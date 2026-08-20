use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "smackdebt",
    version,
    about = "Find costly code and see whether a change made it better"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
    /// Show one path.
    pub(crate) path: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) common: Common,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Compare your current work with a Git ref.
    Diff(DiffArgs),
}

#[derive(Debug, Args)]
pub(crate) struct DiffArgs {
    /// Git ref. If omitted, uses origin/HEAD, main, or master.
    pub(crate) reference: Option<String>,
    /// Show one changed path.
    pub(crate) path: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) common: Common,
}

#[derive(Clone, Debug, Args)]
pub(crate) struct Common {
    /// Write the complete JSON report.
    #[arg(long)]
    pub(crate) json: bool,
    /// Number of workers to use.
    #[arg(long, value_parser = parse_jobs)]
    pub(crate) jobs: Option<usize>,
    /// Recent activity window, such as 90d.
    #[arg(long, value_parser = parse_days)]
    pub(crate) history: Option<u32>,
    /// Show all useful terminal detail.
    #[arg(long)]
    pub(crate) all: bool,
    /// Show up to this many findings.
    #[arg(long, value_parser = parse_top, conflicts_with_all = ["json", "all"])]
    pub(crate) top: Option<usize>,
    /// Glyph color: auto, always, or never.
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
        .ok_or_else(|| "use days, for example 90d".to_owned())?
        .parse::<u32>()
        .map_err(|_| "use a whole number of days".to_owned())?;
    if days == 0 {
        return Err("use at least one day".to_owned());
    }
    Ok(days)
}

fn parse_top(value: &str) -> Result<usize, String> {
    let top = value
        .parse::<usize>()
        .map_err(|_| "use a whole number greater than zero".to_owned())?;
    if top == 0 {
        return Err("use a whole number greater than zero".to_owned());
    }
    Ok(top)
}

fn parse_jobs(value: &str) -> Result<usize, String> {
    let jobs = value
        .parse::<usize>()
        .map_err(|_| "use a whole number greater than zero".to_owned())?;
    if jobs == 0 {
        return Err("use a whole number greater than zero".to_owned());
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
    fn all_and_json_are_rejected_by_the_command_not_by_usage_text() {
        let cli = Cli::try_parse_from(["smackdebt", "--json", "--all"]).unwrap();
        assert!(cli.common.json && cli.common.all);
    }

    #[test]
    fn explicit_color_conflicts_with_json() {
        assert!(Cli::try_parse_from(["smackdebt", "--json", "--color", "always"]).is_err());
    }

    #[test]
    fn rejects_a_zero_finding_limit() {
        assert!(Cli::try_parse_from(["smackdebt", "--top", "0"]).is_err());
        let cli = Cli::try_parse_from(["smackdebt", "--top", "5"]).unwrap();
        assert_eq!(cli.common.top, Some(5));
    }

    #[test]
    fn a_finding_limit_conflicts_with_json_and_with_all() {
        assert!(Cli::try_parse_from(["smackdebt", "--top", "5", "--json"]).is_err());
        assert!(Cli::try_parse_from(["smackdebt", "--top", "5", "--all"]).is_err());
    }
}
