//! One harness-owned home for every acceptance child process.
//!
//! Acceptance evidence is exact bytes, so no machine-level ignore file or git
//! setting may reach a child process. Every invocation of the built command
//! and every fixture git command pins its home directory, user configuration
//! directory, and global and system git configuration here.

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

static HERMETIC_HOME: OnceLock<tempfile::TempDir> = OnceLock::new();

fn hermetic_home() -> &'static Path {
    HERMETIC_HOME
        .get_or_init(|| tempfile::tempdir().expect("create hermetic home"))
        .path()
}

/// Pins the child environment to a harness-owned home and empty git
/// configuration, so evidence bytes are identical on every machine.
pub(crate) fn hermetic_env(command: &mut Command) {
    let home = hermetic_home();
    command
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null");
}
