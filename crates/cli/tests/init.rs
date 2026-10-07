use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn command(home: &TempDir) -> Command {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("smackdebt"));
    command
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .env("CODEX_HOME", home.path().join(".codex"))
        .env("CLAUDE_CONFIG_DIR", home.path().join(".claude"));
    command.arg("init");
    command
}

#[test]
fn installs_only_chosen_agents_without_reading_project_config() {
    let home = TempDir::new().unwrap();
    fs::write(home.path().join(".smackdebt.toml"), "broken = [").unwrap();
    command(&home)
        .args(["--agent", "codex", "--agent", "claude-code"])
        .assert()
        .success();
    for agent in [".codex", ".claude"] {
        let content =
            fs::read_to_string(home.path().join(agent).join("skills/smackdebt/SKILL.md")).unwrap();
        assert!(content.contains("name: smackdebt"));
    }
    assert!(!home.path().join(".cursor").exists());
    assert!(!home.path().join(".agents").exists());
}

#[test]
fn remembers_destinations_and_preserves_user_changes_on_update_and_removal() {
    let home = TempDir::new().unwrap();
    let destination = home.path().join("my skills");
    command(&home)
        .args(["--agent", "codex", "--dest"])
        .arg(&destination)
        .assert()
        .success();
    let skill = destination.join("smackdebt/SKILL.md");
    let original = fs::read(&skill).unwrap();
    command(&home)
        .env("CODEX_HOME", home.path().join("moved"))
        .assert()
        .success();
    assert_eq!(fs::read(&skill).unwrap(), original);
    assert!(!home.path().join("moved").exists());
    fs::write(&skill, "my changes").unwrap();
    command(&home)
        .assert()
        .failure()
        .stderr(predicate::str::contains("local edits"));
    command(&home).arg("--uninstall").assert().failure();
    assert_eq!(fs::read_to_string(&skill).unwrap(), "my changes");
    fs::write(&skill, original).unwrap();
    fs::write(destination.join("smackdebt/notes.md"), "keep").unwrap();
    command(&home).arg("--uninstall").assert().success();
    assert!(!skill.exists());
    assert_eq!(
        fs::read_to_string(destination.join("smackdebt/notes.md")).unwrap(),
        "keep"
    );
}

#[test]
fn dry_run_and_missing_selection_do_not_write_files() {
    let home = TempDir::new().unwrap();
    command(&home)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--agent"));
    command(&home)
        .args(["--all", "--dry-run"])
        .assert()
        .success();
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
}

#[test]
fn all_agents_can_be_updated_and_removed_without_a_network_or_binary_change() {
    let home = TempDir::new().unwrap();
    command(&home)
        .args(["--all"])
        .env("PATH", "")
        .assert()
        .success();
    for agent in [".codex", ".claude", ".cursor", ".copilot", ".gemini"] {
        assert!(
            home.path()
                .join(agent)
                .join("skills/smackdebt/SKILL.md")
                .is_file()
        );
    }
    command(&home).env("PATH", "").assert().success();
    command(&home).args(["--uninstall"]).assert().success();
    for agent in [".codex", ".claude", ".cursor", ".copilot", ".gemini"] {
        assert!(
            !home
                .path()
                .join(agent)
                .join("skills/smackdebt/SKILL.md")
                .exists()
        );
    }
}

#[test]
fn unmanaged_skills_stop_all_changes_before_installation() {
    let home = TempDir::new().unwrap();
    let directory = home.path().join(".claude/skills/smackdebt");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("SKILL.md"), "user owned").unwrap();
    command(&home)
        .args(["--agent", "codex", "--agent", "claude-code"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unmanaged skill"));
    assert!(!home.path().join(".codex").exists());
    assert_eq!(
        fs::read_to_string(directory.join("SKILL.md")).unwrap(),
        "user owned"
    );
}

#[cfg(unix)]
#[test]
fn refuses_skill_symlinks() {
    let home = TempDir::new().unwrap();
    let target = home.path().join("elsewhere");
    fs::create_dir_all(&target).unwrap();
    let parent = home.path().join(".codex/skills");
    fs::create_dir_all(&parent).unwrap();
    std::os::unix::fs::symlink(&target, parent.join("smackdebt")).unwrap();
    command(&home)
        .args(["--agent", "codex"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("link"));
    assert_eq!(fs::read_dir(target).unwrap().count(), 0);
}

#[test]
fn interrupted_skill_replacement_can_retry_and_refresh_the_receipt() {
    let home = TempDir::new().unwrap();
    command(&home).args(["--agent", "codex"]).assert().success();
    let directory = home.path().join(".codex/skills/smackdebt");
    let skill = directory.join("SKILL.md");
    let receipt = directory.join(".smackdebt.blake3");
    let current = fs::read(&skill).unwrap();
    // Model the state after saving old/new hashes, before replacing the skill.
    let old = b"an older managed skill";
    fs::write(&skill, old).unwrap();
    fs::write(
        &receipt,
        format!("{}\n{}\n", blake3::hash(old), blake3::hash(&current)),
    )
    .unwrap();
    command(&home).assert().success();
    assert_eq!(fs::read(&skill).unwrap(), current);
    assert_eq!(fs::read_to_string(&receipt).unwrap().lines().count(), 1);
}

#[test]
fn corrupt_settings_and_multiple_destination_agents_fail_without_writes() {
    let home = TempDir::new().unwrap();
    command(&home)
        .args(["--agent", "codex", "--agent", "cursor", "--dest"])
        .arg(home.path())
        .assert()
        .code(2);
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
    let state = home.path().join(".config/smackdebt/agents.toml");
    fs::create_dir_all(state.parent().unwrap()).unwrap();
    fs::write(&state, "broken = [").unwrap();
    command(&home)
        .args(["--all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("agent settings"));
    assert!(!home.path().join(".codex").exists());
}

#[test]
fn changing_selection_keeps_previous_installations_available_for_removal() {
    let home = TempDir::new().unwrap();
    command(&home).args(["--agent", "codex"]).assert().success();
    command(&home)
        .args(["--agent", "cursor"])
        .assert()
        .success();
    command(&home)
        .assert()
        .success()
        .stdout(predicate::str::contains("codex").not());
    command(&home).arg("--uninstall").assert().success();
    assert!(
        !home
            .path()
            .join(".codex/skills/smackdebt/SKILL.md")
            .exists()
    );
    assert!(
        !home
            .path()
            .join(".cursor/skills/smackdebt/SKILL.md")
            .exists()
    );
}

#[test]
fn agents_cannot_share_a_custom_destination_that_one_could_remove() {
    let home = TempDir::new().unwrap();
    command(&home)
        .args(["--agent", "codex", "--dest"])
        .arg(home.path())
        .assert()
        .success();
    command(&home)
        .args(["--agent", "cursor", "--dest"])
        .arg(home.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already used"));
    command(&home).arg("--uninstall").assert().success();
}

#[cfg(unix)]
#[test]
fn a_parent_symlink_cannot_hide_a_shared_skill_destination() {
    let home = TempDir::new().unwrap();
    let skills = home.path().join("skills");
    command(&home)
        .args(["--agent", "codex", "--dest"])
        .arg(&skills)
        .assert()
        .success();
    let alias = home.path().join("alias");
    std::os::unix::fs::symlink(&skills, &alias).unwrap();
    command(&home)
        .args(["--agent", "cursor", "--dest"])
        .arg(alias)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already used"));
}

#[cfg(unix)]
#[test]
fn aliases_are_detected_before_the_first_skill_directory_is_created() {
    let home = TempDir::new().unwrap();
    let real = home.path().join("real");
    fs::create_dir(&real).unwrap();
    let alias = home.path().join("alias");
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    command(&home)
        .env("CODEX_HOME", &real)
        .env("CLAUDE_CONFIG_DIR", alias)
        .args(["--agent", "codex", "--agent", "claude-code"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already used"));
    assert!(!real.join("skills").exists());
    assert!(!home.path().join(".config").exists());
}
