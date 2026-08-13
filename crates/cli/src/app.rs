use std::ffi::OsString;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use smackdebt_output::{TerminalOptions, write_json, write_terminal};
use smackdebt_project::{CodebaseRequest, DiffRequest, ExecutionWidth, ProjectError};

#[cfg(feature = "allocation-stats")]
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
#[cfg(feature = "allocation-stats")]
use std::alloc::System;

use crate::arguments::{Cli, ColorChoice, Command, Common, parse_days};
use crate::config::{self, ProjectConfig};
use crate::terminal;

#[cfg(feature = "allocation-stats")]
#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

pub(crate) fn main() -> ExitCode {
    #[cfg(feature = "allocation-stats")]
    smackdebt_project::reset_parser_time();
    let exit = run(std::env::args_os());
    #[cfg(feature = "allocation-stats")]
    eprintln!("smackdebt allocation stats: {:?}", ALLOCATOR.stats());
    #[cfg(feature = "allocation-stats")]
    eprintln!(
        "smackdebt parser stats: {{\"parser_time_ns\":{}}}",
        smackdebt_project::parser_time_ns()
    );
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
    let config = match config::load(&config_path) {
        Ok(config) => config,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "smackdebt: {message}");
            return ExitCode::from(2);
        }
    };

    let (result, common) = match cli.command {
        None => {
            let request = match cli.path {
                Some(path) => CodebaseRequest::new(path),
                None => CodebaseRequest::automatic("."),
            };
            let request = apply_codebase_common(request, &cli.common, &config);
            (request.analyze(), cli.common)
        }
        Some(Command::Diff(args)) => {
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
            request = apply_diff_common(request, &args.common, &config);
            (request.analyze(), args.common)
        }
    };

    match result {
        Ok(result) => {
            #[cfg(feature = "allocation-stats")]
            eprintln!(
                "smackdebt project stats: {{\"inventory_walks\":{},\"inventory_visits\":{},\"source_reads\":{},\"git_processes\":{}}}",
                result.stats().inventory_walks(),
                result.stats().inventory_visits(),
                result.stats().source_reads(),
                result.stats().git_processes(),
            );
            let stdout_is_terminal = io::stdout().is_terminal();
            let mut stdout = io::BufWriter::new(io::stdout().lock());
            let rendered = if common.json {
                write_json(&mut stdout, result.report(), result.selected_scope())
                    .and_then(|()| writeln!(stdout))
            } else {
                let width =
                    terminal::width(stdout_is_terminal, std::env::var("COLUMNS").ok().as_deref());
                let color = terminal::color(
                    common.color.unwrap_or(ColorChoice::Auto),
                    stdout_is_terminal,
                    std::env::var_os("NO_COLOR").is_some(),
                );
                write_terminal(
                    &mut stdout,
                    result.report(),
                    result.selected_scope(),
                    TerminalOptions::new(width, common.all, color),
                )
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
    let (cognitive, cyclomatic, lines) = config.thresholds();
    request.with_thresholds(cognitive, cyclomatic, lines)
}

fn apply_diff_common(request: DiffRequest, common: &Common, config: &ProjectConfig) -> DiffRequest {
    let configured_history = config
        .history
        .as_deref()
        .and_then(|value| parse_days(value).ok());
    let (cognitive, cyclomatic, lines) = config.thresholds();
    request
        .with_history_days(common.history.or(configured_history).unwrap_or(90))
        .with_thresholds(cognitive, cyclomatic, lines)
}

fn execution_width(jobs: Option<usize>) -> Option<ExecutionWidth> {
    jobs.and_then(ExecutionWidth::fixed)
}

fn fail(error: &ProjectError) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "smackdebt: {error}");
    ExitCode::from(1)
}
