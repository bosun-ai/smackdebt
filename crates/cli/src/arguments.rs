use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "smackdebt",
    version,
    about = "Find code debt and compare its change"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
    /// Path to inspect.
    pub(crate) path: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) common: Common,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Compare the current worktree with a Git ref.
    Diff(DiffArgs),
}

#[derive(Debug, Args)]
pub(crate) struct DiffArgs {
    /// Git ref. Omit it to use origin/HEAD, main, or master.
    pub(crate) reference: Option<String>,
    /// Limit comparison to this path.
    pub(crate) path: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) common: Common,
}

#[derive(Clone, Debug, Args)]
pub(crate) struct Common {
    /// Write JSON schema version 3.
    #[arg(long)]
    pub(crate) json: bool,
    /// Number of analysis workers. Must be greater than zero.
    #[arg(long, value_parser = parse_jobs)]
    pub(crate) jobs: Option<usize>,
    /// Recent activity window, for example 90d.
    #[arg(long, value_parser = parse_days)]
    pub(crate) history: Option<u32>,
    /// Show every terminal row and retained detail.
    #[arg(long, conflicts_with = "json")]
    pub(crate) all: bool,
    /// Terminal color: auto, always, or never. Defaults to auto.
    #[arg(long, value_enum, conflicts_with = "json")]
    pub(crate) color: Option<ColorChoice>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum ColorChoice {
    Auto,
    Always,
    Never,
}

pub(crate) fn parse_days(value: &str) -> Result<u32, String> {
    let days = value
        .strip_suffix('d')
        .ok_or_else(|| "history must end in d, for example 90d".to_owned())?
        .parse::<u32>()
        .map_err(|_| "history must contain a whole number of days".to_owned())?;
    if days == 0 {
        return Err("history must be greater than zero".to_owned());
    }
    Ok(days)
}

fn parse_jobs(value: &str) -> Result<usize, String> {
    let jobs = value
        .parse::<usize>()
        .map_err(|_| "jobs must be a whole number".to_owned())?;
    if jobs == 0 {
        return Err("jobs must be greater than zero".to_owned());
    }
    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_history_days() {
        assert_eq!(parse_days("180d"), Ok(180));
        assert!(parse_days("0d").is_err());
        assert!(parse_days("90").is_err());
    }

    #[test]
    fn rejects_zero_jobs() {
        assert!(Cli::try_parse_from(["smackdebt", "--jobs", "0"]).is_err());
    }

    #[test]
    fn explicit_color_conflicts_with_json() {
        assert!(Cli::try_parse_from(["smackdebt", "--json", "--color", "always"]).is_err());
    }
}
