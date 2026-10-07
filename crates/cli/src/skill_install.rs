use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::skill_selection::Selection;

pub(crate) const SKILL: &str = include_str!("../assets/smackdebt/SKILL.md");
const RECEIPT: &str = ".smackdebt.blake3";

pub(crate) fn check_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path.parent().is_none()
        || path
            .components()
            .any(|component| component == std::path::Component::ParentDir)
    {
        return Err(format!(
            "use an absolute installation path: {}",
            path.display()
        ));
    }
    Ok(())
}

fn inspect(path: &Path, directory: bool) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(format!("will not replace a link: {}", path.display()))
        }
        Ok(metadata) if metadata.is_dir() == directory && (directory || metadata.is_file()) => {
            Ok(())
        }
        Ok(_) => Err(format!("unexpected file type: {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not inspect {}: {error}", path.display())),
    }
}

pub(crate) fn same_directory(left: &Path, right: &Path) -> Result<bool, String> {
    let left_parent = existing_parent(left)?;
    let right_parent = existing_parent(right)?;
    if left.strip_prefix(left_parent) != right.strip_prefix(right_parent) {
        return Ok(false);
    }
    same_file::is_same_file(left_parent, right_parent)
        .map_err(|error| format!("could not compare skill destinations: {error}"))
}

fn existing_parent(path: &Path) -> Result<&Path, String> {
    for parent in path.ancestors() {
        match fs::metadata(parent) {
            Ok(_) => return Ok(parent),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("could not inspect {}: {error}", parent.display())),
        }
    }
    Err(format!(
        "could not find an existing parent for {}",
        path.display()
    ))
}

pub(crate) fn preflight(directory: &Path) -> Result<(), String> {
    check_path(directory)?;
    inspect(directory, true)?;
    let skill = directory.join("SKILL.md");
    let receipt = directory.join(RECEIPT);
    inspect(&skill, false)?;
    inspect(&receipt, false)?;
    if skill.exists() {
        let bytes = fs::read(&skill).map_err(|error| error.to_string())?;
        let hashes = fs::read_to_string(&receipt).map_err(|_| {
            format!(
                "unmanaged skill: {}; move it aside before installing",
                skill.display()
            )
        })?;
        let observed = blake3::hash(&bytes).to_hex();
        if !hashes.lines().any(|hash| hash == observed.as_str()) {
            return Err(format!(
                "skill has local edits: {}; move it aside before updating",
                skill.display()
            ));
        }
    }
    Ok(())
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    inspect(path, false)?;
    let parent = path.parent().ok_or("missing destination directory")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    staged.write_all(bytes).map_err(|error| error.to_string())?;
    staged
        .persist(path)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    Ok(())
}

pub(crate) fn install(directory: &Path) -> Result<(), String> {
    preflight(directory)?;
    let skill = directory.join("SKILL.md");
    let receipt = directory.join(RECEIPT);
    let new_hash = blake3::hash(SKILL.as_bytes()).to_hex();
    let mut hashes = format!("{new_hash}\n");
    if skill.exists() {
        let previous = fs::read(&skill).map_err(|error| error.to_string())?;
        hashes.push_str(&format!("{}\n", blake3::hash(&previous).to_hex()));
    }
    // Save both hashes before replacement so an interrupted update can retry.
    atomic_write(&receipt, hashes.as_bytes())?;
    atomic_write(&skill, SKILL.as_bytes())?;
    atomic_write(&receipt, format!("{new_hash}\n").as_bytes())
}

pub(crate) fn remove(directory: &Path) -> Result<(), String> {
    preflight(directory)?;
    for name in ["SKILL.md", RECEIPT] {
        match fs::remove_file(directory.join(name)) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => {
                return Err(format!(
                    "could not remove {}: {error}",
                    directory.join(name).display()
                ));
            }
        }
    }
    // Leave unrelated files in the skill directory alone.
    let _ = fs::remove_dir(directory);
    Ok(())
}

pub(crate) fn state_path(home: &Path, config: Option<PathBuf>) -> Result<PathBuf, String> {
    let directory = config
        .unwrap_or_else(|| home.join(".config"))
        .join("smackdebt");
    check_path(&directory)?;
    inspect(&directory, true)?;
    Ok(directory.join("agents.toml"))
}

pub(crate) fn load(path: &Path) -> Result<Selection, String> {
    inspect(path, false)?;
    match fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text)
            .map_err(|error| format!("cannot read agent settings {}: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Selection::default()),
        Err(error) => Err(format!("cannot read agent settings: {error}")),
    }
}

pub(crate) fn save(path: &Path, selection: &Selection) -> Result<(), String> {
    atomic_write(
        path,
        toml::to_string(selection)
            .map_err(|error| error.to_string())?
            .as_bytes(),
    )
}
