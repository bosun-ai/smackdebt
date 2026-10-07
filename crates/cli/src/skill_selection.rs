use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Agent {
    Codex,
    ClaudeCode,
    Cursor,
    Copilot,
    Gemini,
}

impl Agent {
    pub(crate) const ALL: [Self; 5] = [
        Self::Codex,
        Self::ClaudeCode,
        Self::Cursor,
        Self::Copilot,
        Self::Gemini,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
            Self::Cursor => "cursor",
            Self::Copilot => "copilot",
            Self::Gemini => "gemini",
        }
    }

    pub(crate) fn directory(
        self,
        home: &Path,
        codex: Option<PathBuf>,
        claude: Option<PathBuf>,
    ) -> PathBuf {
        let root = match self {
            Self::Codex => codex.unwrap_or_else(|| home.join(".codex")),
            Self::ClaudeCode => claude.unwrap_or_else(|| home.join(".claude")),
            Self::Cursor => home.join(".cursor"),
            Self::Copilot => home.join(".copilot"),
            Self::Gemini => home.join(".gemini"),
        };
        root.join("skills/smackdebt")
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selection {
    pub(crate) selected: Vec<Agent>,
    pub(crate) installations: Vec<Installation>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Installation {
    pub(crate) agent: Agent,
    pub(crate) directory: PathBuf,
}

impl Selection {
    pub(crate) fn remembered(&self, agent: Agent) -> Option<&Installation> {
        self.installations
            .iter()
            .find(|install| install.agent == agent)
    }

    pub(crate) fn remember(&mut self, install: Installation) -> Result<(), String> {
        if let Some(other) = self
            .installations
            .iter()
            .find(|saved| saved.agent != install.agent && saved.directory == install.directory)
        {
            return Err(format!(
                "skill destination is already used by {}",
                other.agent.name()
            ));
        }
        if let Some(previous) = self.remembered(install.agent) {
            if previous.directory != install.directory {
                return Err(format!(
                    "{} is installed at {}; uninstall it before changing its destination",
                    install.agent.name(),
                    previous.directory.display()
                ));
            }
        } else {
            self.installations.push(install);
        }
        Ok(())
    }
}
