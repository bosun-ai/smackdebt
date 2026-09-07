//! Shared integration fixtures for the module test suites.

use std::fs;
use std::path::Path;
use std::process::Command;

use smackdebt_analysis::{FileId, Report};

pub(crate) fn file_id(report: &smackdebt_analysis::Report, path: &str) -> FileId {
    report
        .files()
        .iter()
        .find(|file| file.path() == path)
        .unwrap_or_else(|| panic!("missing {path}"))
        .id()
}
pub(crate) fn git<const N: usize>(root: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
/// Commit with the authored and landed (committer) dates both pinned, so
/// window fixtures describe the instant history filters compare.
pub(crate) fn git_dated<const N: usize>(root: &Path, date: &str, args: [&str; N]) {
    let output = Command::new("git")
        .args(args)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
pub(crate) fn package_pairs(report: &Report) -> Vec<(String, String)> {
    report
        .package_edges()
        .iter()
        .map(|edge| {
            (
                report.packages()[edge.source().index()].path().to_owned(),
                report.packages()[edge.target().index()].path().to_owned(),
            )
        })
        .collect()
}
pub(crate) fn repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let actual = root.path().join("repo");
    fs::create_dir_all(&actual).unwrap();
    git(&actual, ["init", "-q"]);
    git(&actual, ["config", "user.email", "test@example.invalid"]);
    git(&actual, ["config", "user.name", "Smackdebt Test"]);
    fs::write(
        actual.join("sample.rs"),
        "fn work(value: i32) -> i32 { value + 1 }\n",
    )
    .unwrap();
    git(&actual, ["add", "."]);
    git(&actual, ["commit", "-qm", "initial"]);
    root
}
pub(crate) fn workspace_with_declared_names() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        (
            "crates/core/Cargo.toml",
            "[package]\nname='acme-core'\nversion='0.1.0'\n",
        ),
        (
            "crates/core/src/lib.rs",
            "pub fn core(value: i32) -> i32 { value }\n",
        ),
        (
            "crates/app/Cargo.toml",
            "[package]\nname='acme-app'\nversion='0.1.0'\n[lib]\nname='acme_renamed'\n",
        ),
        (
            "crates/app/src/lib.rs",
            "use acme_core::core;\npub fn app(value: i32) -> i32 { core(value) }\n",
        ),
        (
            "ui/package.json",
            "{\"name\":\"@acme/ui\",\"private\":true}\n",
        ),
        ("ui/index.js", "export const ui = 1;\n"),
        (
            "web/package.json",
            "{\"name\":\"@acme/web\",\"private\":true}\n",
        ),
        (
            "web/index.js",
            "import { ui } from '@acme/ui/button';\nexport const web = ui;\n",
        ),
    ] {
        let file = root.path().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, source).unwrap();
    }
    root
}
pub(crate) fn write_crate(root: &Path, name: &str, entry: &str) {
    fs::create_dir_all(root.join(format!("crates/{name}/src"))).unwrap();
    fs::write(
        root.join(format!("crates/{name}/Cargo.toml")),
        format!("[package]\nname='{name}'\nversion='0.1.0'\n"),
    )
    .unwrap();
    fs::write(root.join(format!("crates/{name}/src/lib.rs")), entry).unwrap();
}
/// A `mod.rs` that re-exports two children which import it back.
pub(crate) fn write_module_component(root: &Path, first_extra: &str) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='component'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("thing")).unwrap();
    fs::write(
        root.join("thing/mod.rs"),
        "mod first;\nmod second;\npub use self::first::first;\npub use self::second::second;\npub fn y() {}\n",
    )
    .unwrap();
    fs::write(
        root.join("thing/first.rs"),
        format!("use super::*;\n{first_extra}pub fn first() {{ y() }}\n"),
    )
    .unwrap();
    fs::write(
        root.join("thing/second.rs"),
        "use super::*;\npub fn second() { y() }\n",
    )
    .unwrap();
}
