#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use serde::Deserialize;
use smackdebt_output::{TerminalOptions, write_json, write_terminal};
use smackdebt_project::{
    CodebaseRequest, DiffRequest, ExecutionWidth, ProjectError, analyze_codebase, analyze_diff,
};

#[cfg(feature = "allocation-stats")]
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
#[cfg(feature = "allocation-stats")]
use std::alloc::System;

#[cfg(feature = "allocation-stats")]
#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Debug, Parser)]
#[command(
    name = "smackdebt",
    version,
    about = "Find code debt and compare its change"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to inspect.
    path: Option<PathBuf>,

    #[command(flatten)]
    common: Common,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Compare the current worktree with a Git ref.
    Diff(DiffArgs),
}

#[derive(Debug, Args)]
struct DiffArgs {
    /// Git ref. Omit it to use origin/HEAD, main, or master.
    reference: Option<String>,

    /// Limit comparison to this path.
    path: Option<PathBuf>,

    #[command(flatten)]
    common: Common,
}

#[derive(Clone, Debug, Args)]
struct Common {
    /// Write JSON schema version 1.
    #[arg(long)]
    json: bool,

    /// Number of analysis workers. Must be greater than zero.
    #[arg(long, value_parser = parse_jobs)]
    jobs: Option<usize>,

    /// Recent activity window, for example 90d.
    #[arg(long, value_parser = parse_days)]
    history: Option<u32>,

    /// Show every terminal row and retained detail.
    #[arg(long, conflicts_with = "json")]
    all: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectConfig {
    history: Option<String>,
    #[serde(default)]
    exclude: Vec<String>,
    thresholds: Option<ThresholdConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThresholdConfig {
    cognitive: Option<LimitConfig>,
    cyclomatic: Option<LimitConfig>,
    function_lines: Option<LimitConfig>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitConfig {
    watch: u32,
    high: u32,
}

fn parse_days(value: &str) -> Result<u32, String> {
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

fn main() -> ExitCode {
    let exit = run(std::env::args_os());
    #[cfg(feature = "allocation-stats")]
    eprintln!("smackdebt allocation stats: {:?}", ALLOCATOR.stats());
    exit
}

fn run(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
    let cli = match Cli::try_parse_from(arguments) {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return ExitCode::from(2);
        }
    };

    let config_path = match &cli.command {
        None => cli.path.clone().unwrap_or_else(|| PathBuf::from(".")),
        Some(Command::Diff(args)) => args
            .path
            .clone()
            .or_else(|| cli.path.clone())
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "smackdebt: {message}");
            return ExitCode::from(2);
        }
    };

    let (result, json, all) = match cli.command {
        None => {
            let request = match cli.path {
                Some(path) => CodebaseRequest::new(path),
                None => CodebaseRequest::automatic("."),
            };
            let request = apply_codebase_common(request, &cli.common, &config);
            (analyze_codebase(&request), cli.common.json, cli.common.all)
        }
        Some(Command::Diff(args)) => {
            let json = args.common.json;
            let mut request = match args.path {
                Some(path) => DiffRequest::new(path),
                None => DiffRequest::automatic("."),
            };
            if let Some(reference) = args.reference {
                request = request.with_reference(reference);
            }
            if let Some(width) = execution_width(args.common.jobs) {
                request = request.with_width(width);
            }
            request = apply_diff_config(request, &config);
            (analyze_diff(&request), json, args.common.all)
        }
    };

    match result {
        Ok(result) => {
            let mut stdout = io::BufWriter::new(io::stdout().lock());
            let rendered = if json {
                write_json(&mut stdout, result.report()).and_then(|()| writeln!(stdout))
            } else {
                let width = terminal_width();
                write_terminal(&mut stdout, result.report(), TerminalOptions { width, all })
            };
            match rendered {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => fail(&ProjectError::Inspect {
                    path: PathBuf::from("standard output"),
                    source: error,
                }),
            }
        }
        Err(error) => fail(&error),
    }
}

fn apply_codebase_common(
    request: CodebaseRequest,
    common: &Common,
    config: &ProjectConfig,
) -> CodebaseRequest {
    let configured_history = config
        .history
        .as_deref()
        .and_then(|value| parse_days(value).ok());
    let request = request
        .with_history_days(common.history.or(configured_history).unwrap_or(90))
        .with_excludes(config.exclude.clone());
    let request = apply_codebase_thresholds(request, config);
    match execution_width(common.jobs) {
        Some(width) => request.with_width(width),
        None => request,
    }
}

fn apply_codebase_thresholds(request: CodebaseRequest, config: &ProjectConfig) -> CodebaseRequest {
    let (cognitive, cyclomatic, lines) = configured_thresholds(config);
    request.with_thresholds(cognitive, cyclomatic, lines)
}

fn apply_diff_config(request: DiffRequest, config: &ProjectConfig) -> DiffRequest {
    let (cognitive, cyclomatic, lines) = configured_thresholds(config);
    request.with_thresholds(cognitive, cyclomatic, lines)
}

fn configured_thresholds(config: &ProjectConfig) -> ((u32, u32), (u32, u32), (u32, u32)) {
    let thresholds = config.thresholds.as_ref();
    let pair = |value: Option<LimitConfig>, fallback| {
        value.map_or(fallback, |limit| (limit.watch, limit.high))
    };
    (
        pair(thresholds.and_then(|value| value.cognitive), (15, 25)),
        pair(thresholds.and_then(|value| value.cyclomatic), (11, 21)),
        pair(thresholds.and_then(|value| value.function_lines), (50, 100)),
    )
}

fn load_config(selected: &std::path::Path) -> Result<ProjectConfig, String> {
    let start = if selected.is_file() {
        selected.parent().unwrap_or(selected)
    } else {
        selected
    };
    let mut directory = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("cannot resolve current directory: {error}"))?
            .join(start)
    };
    loop {
        let candidate = directory.join(".smackdebt.toml");
        if candidate.is_file() {
            let source = fs::read_to_string(&candidate)
                .map_err(|error| format!("cannot read {}: {error}", candidate.display()))?;
            let config: ProjectConfig = toml::from_str(&source)
                .map_err(|error| format!("invalid {}: {error}", candidate.display()))?;
            validate_config(&config)?;
            return Ok(config);
        }
        if !directory.pop() {
            return Ok(ProjectConfig::default());
        }
    }
}

fn validate_config(config: &ProjectConfig) -> Result<(), String> {
    if let Some(history) = &config.history {
        parse_days(history)?;
    }
    if let Some(thresholds) = &config.thresholds {
        for (name, limit) in [
            ("cognitive", thresholds.cognitive),
            ("cyclomatic", thresholds.cyclomatic),
            ("function_lines", thresholds.function_lines),
        ] {
            if let Some(limit) = limit
                && (limit.watch == 0 || limit.high <= limit.watch)
            {
                return Err(format!(
                    "thresholds.{name} requires watch greater than zero and high greater than watch"
                ));
            }
        }
    }
    Ok(())
}

fn execution_width(jobs: Option<usize>) -> Option<ExecutionWidth> {
    jobs.and_then(ExecutionWidth::fixed)
}

fn terminal_width() -> usize {
    if !io::stdout().is_terminal() {
        return 100;
    }
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|width| *width >= 40)
        .unwrap_or(100)
}

fn fail(error: &ProjectError) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "smackdebt: {error}");
    ExitCode::from(1)
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
        let result = Cli::try_parse_from(["smackdebt", "--jobs", "0"]);
        assert!(result.is_err());
    }
}
