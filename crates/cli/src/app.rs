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
    if std::env::var_os("SMACKDEBT_ALLOCATION_STATS").is_some() {
        smackdebt_project::reset_parser_time();
    }
    let exit = run(std::env::args_os());
    #[cfg(feature = "allocation-stats")]
    if std::env::var_os("SMACKDEBT_ALLOCATION_STATS").is_some() {
        eprintln!("smackdebt allocation stats: {:?}", ALLOCATOR.stats());
    }
    #[cfg(feature = "allocation-stats")]
    if std::env::var_os("SMACKDEBT_ALLOCATION_STATS").is_some() {
        eprintln!(
            "smackdebt parser stats: {{\"parser_time_ns\":{}}}",
            smackdebt_project::parser_time_ns()
        );
    }
    exit
}

fn run(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
    #[cfg(feature = "evidence-stats")]
    let evidence_enabled = std::env::var_os("SMACKDEBT_EVIDENCE_STATS").is_some();
    #[cfg(feature = "evidence-stats")]
    if evidence_enabled || std::env::var_os("SMACKDEBT_ALLOCATION_STATS").is_some() {
        smackdebt_project::reset_evidence();
    }
    let cli = match Cli::try_parse_from(arguments) {
        Ok(cli) => cli,
        Err(error) => {
            let exit = error.exit_code();
            let _ = error.print();
            return ExitCode::from(u8::try_from(exit).unwrap_or(2));
        }
    };

    if cli.common.json && cli.common.all
        || cli
            .command
            .as_ref()
            .is_some_and(|Command::Diff(args)| args.common.json && args.common.all)
    {
        return fail_with("--all cannot be used with --json", 2);
    }
    let selected_path = match &cli.command {
        None => cli.path.as_deref(),
        Some(Command::Diff(args)) => args.path.as_deref().or(cli.path.as_deref()),
    };
    if let Some(path) = selected_path.filter(|path| !path.exists()) {
        return fail_with(&format!("path not found: {}", path.display()), 1);
    }

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
            if std::env::var_os("SMACKDEBT_ALLOCATION_STATS").is_some() {
                let stats = smackdebt_project::evidence_snapshot();
                eprintln!(
                    "smackdebt project stats: {{\"inventory_walks\":{},\"inventory_visits\":{},\"source_reads\":{},\"object_reads\":{},\"git_processes\":{},\"parser_visits\":{},\"algorithm_passes\":{}}}",
                    stats.inventory_walks(),
                    stats.inventory_visits(),
                    stats.source_reads(),
                    stats.object_reads(),
                    stats.git_processes(),
                    stats.parser_visits(),
                    stats.algorithm_passes(),
                );
            }
            #[cfg(feature = "evidence-stats")]
            let before_render = evidence_enabled.then(smackdebt_project::evidence_snapshot);
            let stdout_is_terminal = io::stdout().is_terminal();
            let mut stdout = io::BufWriter::new(io::stdout().lock());
            let rendered = if common.json {
                #[cfg(feature = "evidence-stats")]
                if evidence_enabled {
                    smackdebt_project::record_renderer_entry();
                }
                write_json(&mut stdout, result.report(), result.selected_scope())
                    .and_then(|()| writeln!(stdout))
            } else {
                let width =
                    terminal::width(stdout_is_terminal, std::env::var("COLUMNS").ok().as_deref());
                let choice = common.color.unwrap_or(ColorChoice::Auto);
                let color = terminal::color(
                    choice,
                    stdout_is_terminal,
                    std::env::var_os("NO_COLOR").is_some(),
                );
                let decorations = terminal::decorations(choice, stdout_is_terminal);
                #[cfg(feature = "evidence-stats")]
                if evidence_enabled {
                    smackdebt_project::record_renderer_entry();
                }
                write_terminal(
                    &mut stdout,
                    result.report(),
                    result.selected_scope(),
                    TerminalOptions::new(width, common.all, color).with_decorations(decorations),
                )
            };
            match rendered.and_then(|()| stdout.flush()) {
                Ok(()) => {
                    #[cfg(feature = "evidence-stats")]
                    if evidence_enabled {
                        let before_render = before_render.expect("evidence snapshot");
                        let after_render = smackdebt_project::evidence_snapshot();
                        let render = after_render.since(before_render);
                        eprintln!(
                            "smackdebt evidence stats: {{\"inventory_walks\":{},\"inventory_visits\":{},\"source_reads\":{},\"object_reads\":{},\"git_processes\":{},\"parser_visits\":{},\"algorithm_passes\":{},\"renderer_entries\":{},\"render_inventory_walks\":{},\"render_inventory_visits\":{},\"render_source_reads\":{},\"render_object_reads\":{},\"render_git_processes\":{},\"render_parser_visits\":{},\"render_algorithm_passes\":{},\"render_renderer_entries\":{}}}",
                            after_render.inventory_walks(),
                            after_render.inventory_visits(),
                            after_render.source_reads(),
                            after_render.object_reads(),
                            after_render.git_processes(),
                            after_render.parser_visits(),
                            after_render.algorithm_passes(),
                            after_render.renderer_entries(),
                            render.inventory_walks(),
                            render.inventory_visits(),
                            render.source_reads(),
                            render.object_reads(),
                            render.git_processes(),
                            render.parser_visits(),
                            render.algorithm_passes(),
                            render.renderer_entries(),
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => match stdout_failure(&error) {
                    // A reader that stopped reading, such as `smackdebt | head`,
                    // is not an error the user has to see.
                    StdoutFailure::ReaderLeft => ExitCode::SUCCESS,
                    StdoutFailure::Reportable => fail(&ProjectError::Inspect {
                        path: PathBuf::from("standard output"),
                        source: error,
                    }),
                },
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
        .with_excludes(config.exclude.clone())
        .with_role_rules(config.role_rules());
    let request = apply_codebase_thresholds(request, config);
    match execution_width(common.jobs) {
        Some(width) => request.with_width(width),
        None => request,
    }
}

fn apply_codebase_thresholds(request: CodebaseRequest, config: &ProjectConfig) -> CodebaseRequest {
    let (cognitive, cyclomatic, lines, nesting, parameters) = config.thresholds();
    let (file_lines, container_lines) = config.size_thresholds();
    request
        .with_thresholds(cognitive, cyclomatic, lines, nesting, parameters)
        .with_size_thresholds(file_lines, container_lines)
        .with_minimum_hotspot_touches(config.minimum_hotspot_touches())
}

fn apply_diff_common(request: DiffRequest, common: &Common, config: &ProjectConfig) -> DiffRequest {
    let configured_history = config
        .history
        .as_deref()
        .and_then(|value| parse_days(value).ok());
    let (cognitive, cyclomatic, lines, nesting, parameters) = config.thresholds();
    request
        .with_history_days(common.history.or(configured_history).unwrap_or(90))
        .with_role_rules(config.role_rules())
        .with_thresholds(cognitive, cyclomatic, lines, nesting, parameters)
}

fn execution_width(jobs: Option<usize>) -> Option<ExecutionWidth> {
    jobs.and_then(ExecutionWidth::fixed)
}

fn fail(error: &ProjectError) -> ExitCode {
    let message = match error {
        ProjectError::Inspect { path, source } => {
            format!("could not read {}: {source}", path.display())
        }
        ProjectError::WorkerPool(source) => format!("could not start analysis: {source}"),
        ProjectError::Git(source) => format!("could not compare changes: {source}"),
        ProjectError::MissingReference => {
            "no comparison branch was found; pass one, for example `smackdebt diff main`".to_owned()
        }
        // Git's own command, status, and fatal output stay out of the message:
        // the ref the user typed is the only fixable value.
        ProjectError::UnknownReference(reference) => format!("Git ref not found: {reference}"),
        ProjectError::SourceRoleConflict { path, roles } => {
            format!(
                "source roles conflict for {}: {roles}; update .smackdebt.toml",
                path.display()
            )
        }
    };
    let status = if matches!(error, ProjectError::SourceRoleConflict { .. }) {
        2
    } else {
        1
    };
    fail_with(&message, status)
}

/// Writes one exact user-facing line to standard error with no usage tail.
fn fail_with(message: &str, status: u8) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "smackdebt: {message}");
    ExitCode::from(status)
}

/// How a failed standard-output write is answered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StdoutFailure {
    /// The reader closed the pipe, which every Unix tool exits quietly on.
    ReaderLeft,
    /// Any other IO failure, which the user has to see.
    Reportable,
}

fn stdout_failure(error: &io::Error) -> StdoutFailure {
    match error.kind() {
        io::ErrorKind::BrokenPipe => StdoutFailure::ReaderLeft,
        _ => StdoutFailure::Reportable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_reader_is_quiet_and_every_other_write_failure_is_reported() {
        assert_eq!(
            stdout_failure(&io::Error::from(io::ErrorKind::BrokenPipe)),
            StdoutFailure::ReaderLeft
        );
        for kind in [
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::StorageFull,
            io::ErrorKind::InvalidData,
            io::ErrorKind::Other,
        ] {
            assert_eq!(
                stdout_failure(&io::Error::from(kind)),
                StdoutFailure::Reportable,
                "{kind:?}"
            );
        }
    }
}
