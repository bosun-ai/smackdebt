//! Terminal interaction for agent setup; owns no installation decisions or writes.
use std::io::{self, Write};
use std::path::Path;

use crate::skill_selection::{Agent, Installation};
use crate::terminal;
use crate::terminal::ColorChoice;

pub(crate) struct SetupUi {
    interactive: bool,
    uninstall: bool,
    dry_run: bool,
}

impl SetupUi {
    pub(crate) fn new(choice: ColorChoice, uninstall: bool, dry_run: bool) -> Self {
        terminal::configure(choice);
        Self {
            interactive: terminal::interactive(),
            uninstall,
            dry_run,
        }
    }

    pub(crate) fn interactive(&self) -> bool {
        self.interactive
    }

    pub(crate) fn choose(
        &self,
        candidates: &[Installation],
        selected: Vec<Agent>,
        home: &Path,
    ) -> io::Result<Vec<Agent>> {
        cliclack::intro("Smackdebt")?;
        cliclack::log::remark(if self.uninstall {
            "Remove skills from your agents."
        } else {
            "Less spaghetti. Start with your agent."
        })?;
        let action = if self.dry_run {
            "preview"
        } else if self.uninstall {
            "remove"
        } else {
            "install"
        };
        cliclack::log::remark(format!(
            "↑/↓ move · Space select · Enter {action} · Esc cancel"
        ))?;
        let mut picker = cliclack::multiselect(if self.uninstall {
            "Which agent skills should be removed?"
        } else {
            "Which agents should use Smackdebt?"
        })
        .required(false)
        .initial_values(selected);
        for install in candidates {
            picker = picker.item(
                install.agent,
                install.agent.label(),
                display_path(&install.directory, home),
            );
        }
        picker.interact()
    }

    pub(crate) fn result(
        &self,
        action: &str,
        install: &Installation,
        home: &Path,
    ) -> io::Result<()> {
        let message = format!(
            "{action} {}\n{}",
            install.agent.label(),
            display_path(&install.directory, home)
        );
        if self.interactive {
            cliclack::log::success(message)
        } else {
            writeln!(
                io::stdout().lock(),
                "{action} {} skill at {}",
                install.agent.label(),
                install.directory.display()
            )
        }
    }

    pub(crate) fn complete(&self, changed: bool) -> io::Result<()> {
        if !self.interactive {
            return Ok(());
        }
        let message = if self.uninstall {
            "Selected skills removed."
        } else if changed {
            "Ready. Restart your agent to load the skill.\nTry: Use Smackdebt to review this change."
        } else {
            "Everything is up to date."
        };
        self.finish(message)
    }

    pub(crate) fn finish(&self, message: &str) -> io::Result<()> {
        if self.interactive {
            cliclack::outro(message)
        } else {
            writeln!(io::stdout().lock(), "{message}")
        }
    }

    pub(crate) fn error(&self, message: &str) -> io::Result<()> {
        if self.interactive {
            cliclack::outro_cancel(message)
        } else {
            writeln!(io::stderr().lock(), "smackdebt: {message}")
        }
    }
}

fn display_path(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(relative) => format!("~/{}", relative.display()),
        Err(_) => path.display().to_string(),
    }
}
