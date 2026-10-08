use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::skill_install;
use crate::skill_selection::Agent;

pub(crate) fn config_path(agent: Agent, home: &Path) -> PathBuf {
    let root = match agent {
        Agent::Codex => std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex")),
        Agent::ClaudeCode => std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude")),
        Agent::Cursor => home.join(".cursor"),
        Agent::Copilot => std::env::var_os("COPILOT_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".copilot")),
        Agent::Gemini => home.join(".gemini"),
    };
    match agent {
        Agent::Codex | Agent::Cursor => root.join("hooks.json"),
        Agent::ClaudeCode | Agent::Gemini => root.join("settings.json"),
        Agent::Copilot => root.join("hooks/smackdebt.json"),
    }
}

fn event(agent: Agent) -> &'static str {
    match agent {
        Agent::Codex | Agent::ClaudeCode => "Stop",
        Agent::Cursor => "stop",
        Agent::Copilot => "agentStop",
        Agent::Gemini => "AfterAgent",
    }
}

fn command(agent: Agent) -> String {
    format!("smackdebt __hook --agent {}", agent.name())
}

fn entry(agent: Agent) -> Value {
    let command = command(agent);
    match agent {
        Agent::Codex | Agent::ClaudeCode => {
            json!({"hooks": [{"type": "command", "command": command, "timeout": 60}]})
        }
        Agent::Gemini => {
            json!({"hooks": [{"name": "smackdebt-review", "type": "command", "command": command, "timeout": 60000}]})
        }
        Agent::Cursor => json!({"command": command}),
        Agent::Copilot => json!({"type": "command", "bash": command, "timeoutSec": 60}),
    }
}

fn read(path: &Path) -> Result<Value, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Err(format!(
                "will not replace non-file or link: {}",
                path.display()
            ));
        }
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(json!({})),
        Err(error) => return Err(format!("could not inspect {}: {error}", path.display())),
    }
    let bytes =
        fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("could not parse {}: {error}", path.display()))?;
    if !value.is_object() {
        return Err(format!("expected a JSON object in {}", path.display()));
    }
    Ok(value)
}

pub(crate) fn preflight(agent: Agent, home: &Path) -> Result<(), String> {
    let path = config_path(agent, home);
    skill_install::check_path(&path)?;
    let value = read(&path)?;
    if let Some(hooks) = value.get("hooks") {
        let Some(events) = hooks.as_object() else {
            return Err(format!("expected a hooks object in {}", path.display()));
        };
        if events
            .get(event(agent))
            .is_some_and(|entries| !entries.is_array())
        {
            return Err(format!(
                "expected a {} hook list in {}",
                event(agent),
                path.display()
            ));
        }
        if let Some(entries) = events.get(event(agent)).and_then(Value::as_array) {
            for candidate in entries {
                let installed_command = match agent {
                    Agent::Codex | Agent::ClaudeCode | Agent::Gemini => candidate
                        .get("hooks")
                        .and_then(Value::as_array)
                        .and_then(|hooks| hooks.first())
                        .and_then(|hook| hook.get("command")),
                    Agent::Cursor => candidate.get("command"),
                    Agent::Copilot => candidate.get("bash"),
                };
                if installed_command.and_then(Value::as_str) == Some(command(agent).as_str())
                    && candidate != &entry(agent)
                {
                    return Err(format!("hook has local edits: {}", path.display()));
                }
            }
        }
    }
    Ok(())
}

fn edit(agent: Agent, home: &Path, installing: bool) -> Result<(), String> {
    preflight(agent, home)?;
    let path = config_path(agent, home);
    let existed = path.exists();
    let mut value = read(&path)?;
    let root = value.as_object_mut().expect("preflight checked object");
    if !installing && !existed {
        return Ok(());
    }
    if installing && matches!(agent, Agent::Cursor | Agent::Copilot) {
        root.entry("version").or_insert(json!(1));
    }
    let hooks = root.entry("hooks").or_insert(json!({}));
    let events = hooks.as_object_mut().expect("preflight checked hooks");
    let entries = events.entry(event(agent)).or_insert(json!([]));
    let entries = entries.as_array_mut().expect("preflight checked list");
    let managed = entry(agent);
    if installing && entries.contains(&managed) {
        return Ok(());
    }
    let old_len = entries.len();
    entries.retain(|candidate| candidate != &managed);
    if installing {
        entries.push(managed);
    } else if entries.is_empty() {
        events.remove(event(agent));
    }
    if !installing && old_len == entries_len(&value, agent) {
        return Ok(());
    }
    let serialized = serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?;
    skill_install::atomic_write(&path, &serialized)
}

fn entries_len(value: &Value, agent: Agent) -> usize {
    value
        .get("hooks")
        .and_then(|hooks| hooks.get(event(agent)))
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

pub(crate) fn install(agent: Agent, home: &Path) -> Result<(), String> {
    edit(agent, home, true)
}

pub(crate) fn remove(agent: Agent, home: &Path) -> Result<(), String> {
    edit(agent, home, false)
}
