use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Args;
use serde_json::{Value, json};

use crate::skill_install;
use crate::skill_selection::Agent;

const MIN_INTERVAL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Args)]
pub(crate) struct HookArgs {
    #[arg(long, value_enum)]
    agent: Agent,
}

pub(crate) fn run(args: HookArgs) -> ExitCode {
    let mut input = Vec::new();
    let _ = io::stdin().take(1024 * 1024).read_to_end(&mut input);
    let payload: Value = serde_json::from_slice(&input).unwrap_or(Value::Null);
    let result = review(args.agent, &payload).unwrap_or_else(|| json!({}));
    println!("{result}");
    ExitCode::SUCCESS
}

fn review(agent: Agent, payload: &Value) -> Option<Value> {
    if already_reviewed(payload) {
        return None;
    }
    let cwd = working_directory(agent, payload)?;
    let root = repository_root(&cwd)?;
    let change = WorktreeChange::detect(&root)?;
    let report = diff_report(&root)?;
    change.mark_checked();
    if !has_regressions(&report) {
        return None;
    }
    let excerpt: String = report.chars().take(3000).collect();
    let reason = format!(
        "{}\n{excerpt}",
        include_str!("../assets/hooks/review-message.txt").trim_end()
    );
    Some(match agent {
        Agent::Codex | Agent::Copilot => {
            json!({"decision": "block", "reason": reason})
        }
        Agent::Gemini => json!({"decision": "deny", "reason": reason}),
        Agent::ClaudeCode => {
            json!({"hookSpecificOutput": {"hookEventName": "Stop", "additionalContext": reason}})
        }
        Agent::Cursor => json!({"followup_message": reason}),
    })
}

fn already_reviewed(payload: &Value) -> bool {
    payload.get("stop_hook_active").and_then(Value::as_bool) == Some(true)
        || payload
            .get("loop_count")
            .and_then(Value::as_u64)
            .is_some_and(|count| count > 0)
        || payload
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status != "completed")
}

fn repository_root(cwd: &Path) -> Option<PathBuf> {
    let root = git(cwd, &["rev-parse", "--show-toplevel"])?;
    Some(PathBuf::from(String::from_utf8(root).ok()?.trim()))
}

fn diff_report(root: &Path) -> Option<String> {
    let output = Command::new(std::env::current_exe().ok()?)
        .current_dir(root)
        .args(["diff", "HEAD", "--top", "3", "--color", "never"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
}

struct WorktreeChange {
    cache: PathBuf,
    now: u64,
    fingerprint: String,
}

impl WorktreeChange {
    fn detect(root: &Path) -> Option<Self> {
        let status = git(root, &["status", "--porcelain=v1", "-z", "-uall"])?;
        if status.is_empty() {
            return None;
        }
        let fingerprint = fingerprint(root, &status)?;
        let cache = cache_path(root)?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        if recently_checked(&cache, now, &fingerprint) {
            return None;
        }
        // A failed or interrupted check can retry after the interval.
        skill_install::atomic_write(&cache, format!("{now} failed\n").as_bytes()).ok()?;
        Some(Self {
            cache,
            now,
            fingerprint,
        })
    }

    fn mark_checked(&self) {
        let _ = skill_install::atomic_write(
            &self.cache,
            format!("{} {}\n", self.now, self.fingerprint).as_bytes(),
        );
    }
}

fn recently_checked(cache: &Path, now: u64, fingerprint: &str) -> bool {
    let Ok(previous) = fs::read_to_string(cache) else {
        return false;
    };
    let mut fields = previous.split_whitespace();
    let at = fields.next().and_then(|text| text.parse::<u64>().ok());
    let previous_fingerprint = fields.next();
    previous_fingerprint == Some(fingerprint)
        || at.is_some_and(|at| now.saturating_sub(at) < MIN_INTERVAL.as_secs())
}

fn working_directory(agent: Agent, payload: &Value) -> Option<PathBuf> {
    if agent == Agent::Cursor {
        if let Some(path) = std::env::var_os("CURSOR_PROJECT_DIR") {
            return Some(PathBuf::from(path));
        }
        return payload
            .get("workspace_roots")?
            .as_array()?
            .first()?
            .as_str()
            .map(PathBuf::from);
    }
    payload
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
}

fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn fingerprint(root: &Path, status: &[u8]) -> Option<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(status);
    for record in status
        .split(|byte| *byte == 0)
        .filter(|record| record.starts_with(b"?? "))
    {
        let name = std::str::from_utf8(&record[3..]).ok()?;
        let metadata = fs::metadata(root.join(name)).ok()?;
        hasher.update(&metadata.len().to_le_bytes());
        if let Ok(modified) = metadata
            .modified()
            .and_then(|time| time.duration_since(UNIX_EPOCH).map_err(io::Error::other))
        {
            hasher.update(&modified.as_nanos().to_le_bytes());
        }
    }
    let mut child = Command::new("git")
        .current_dir(root)
        .args(["diff", "HEAD", "--binary", "--no-ext-diff", "--"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let output = child.stdout.take()?;
    hasher.update_reader(output).ok()?;
    child
        .wait()
        .ok()?
        .success()
        .then(|| hasher.finalize().to_hex().to_string())
}

fn cache_path(root: &Path) -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    let name = blake3::hash(root.as_os_str().as_encoded_bytes()).to_hex();
    Some(base.join("smackdebt/hooks").join(name.to_string()))
}

fn has_regressions(report: &str) -> bool {
    report.lines().any(|line| {
        line.split_once("worse ")
            .and_then(|(_, rest)| rest.split_whitespace().next())
            .and_then(|count| count.parse::<usize>().ok())
            .is_some_and(|count| count > 0)
    })
}

#[cfg(test)]
mod tests {
    use super::has_regressions;

    #[test]
    fn only_new_debt_requests_another_review() {
        assert!(!has_regressions("worse 0 · better 2 · changed 2"));
        assert!(has_regressions("worse 1 · better 0 · changed 1"));
    }
}
