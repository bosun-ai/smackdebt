use std::fs;
use std::path::Path;
use std::process::Command as Process;

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

fn git(path: &Path, args: &[&str]) {
    let output = Process::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "test@example.com"]);
    git(repo.path(), &["config", "user.name", "Test"]);
    fs::write(
        repo.path().join("Cargo.toml"),
        "[package]\nname = \"sample\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::create_dir(repo.path().join("src")).unwrap();
    fs::write(
        repo.path().join("src/lib.rs"),
        "pub fn calculate(x: i32) -> i32 { x }\n",
    )
    .unwrap();
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "start"]);
    repo
}

fn worsened_source() -> String {
    let mut source = String::from("pub fn calculate(x: i32) -> i32 {\n");
    for threshold in 0..10 {
        source.push_str(&format!("if x > {threshold} {{\n"));
    }
    source.push_str("return x;\n");
    for _ in 0..10 {
        source.push_str("}\n");
    }
    source.push_str("x\n}\n");
    source
}

fn hook(repo: &TempDir, agent: &str, payload: Value) -> Value {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("smackdebt"));
    let output = command
        .current_dir(repo.path())
        .env("XDG_CACHE_HOME", repo.path().join(".git/cache").join(agent))
        .args(["__hook", "--agent", agent])
        .write_stdin(payload.to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn hook_requests_one_focused_review_for_worsened_code() {
    let repo = repository();
    let cwd = repo.path().to_str().unwrap();
    assert_eq!(hook(&repo, "codex", json!({"cwd": cwd})), json!({}));
    fs::write(repo.path().join("src/lib.rs"), worsened_source()).unwrap();
    let first = hook(&repo, "codex", json!({"cwd": cwd}));
    assert_eq!(first["decision"], "block");
    assert!(first["reason"].as_str().unwrap().contains("worse 1"));
    assert_eq!(hook(&repo, "codex", json!({"cwd": cwd})), json!({}));
    assert_eq!(
        hook(
            &repo,
            "codex",
            json!({"cwd": cwd, "stop_hook_active": true})
        ),
        json!({})
    );
}

#[test]
fn cursor_uses_the_workspace_root_and_ignores_followup_turns() {
    let repo = repository();
    fs::write(repo.path().join("src/lib.rs"), worsened_source()).unwrap();
    let cwd = repo.path().to_str().unwrap();
    let result = hook(
        &repo,
        "cursor",
        json!({"workspace_roots": [cwd], "status": "completed", "loop_count": 0}),
    );
    assert!(
        result["followup_message"]
            .as_str()
            .is_some_and(|message| message.contains("Smackdebt"))
    );
    assert_eq!(
        hook(
            &repo,
            "cursor",
            json!({"workspace_roots": [cwd], "status": "completed", "loop_count": 1})
        ),
        json!({})
    );
}

#[test]
fn each_agent_receives_its_supported_review_response() {
    let repo = repository();
    fs::write(repo.path().join("src/lib.rs"), worsened_source()).unwrap();
    let cwd = repo.path().to_str().unwrap();
    for agent in ["codex", "claude-code", "cursor", "copilot", "gemini"] {
        let payload = if agent == "cursor" {
            json!({"workspace_roots": [cwd], "status": "completed"})
        } else {
            json!({"cwd": cwd})
        };
        let output = hook(&repo, agent, payload);
        let reason = match agent {
            "claude-code" => output["hookSpecificOutput"]["additionalContext"].as_str(),
            "cursor" => output["followup_message"].as_str(),
            _ => output["reason"].as_str(),
        };
        assert!(
            reason.is_some_and(|text| text.contains("worse 1")),
            "{agent}: {output}"
        );
        if agent == "gemini" {
            assert_eq!(output["decision"], "deny");
        }
    }
}
