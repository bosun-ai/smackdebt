use std::ffi::OsString;
use std::io::{self, IsTerminal, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use smackdebt_output::{TerminalOptions, write_gate, write_gate_json, write_json, write_terminal};
use smackdebt_project::{
    CodebaseRequest, DiffRequest, ExecutionWidth, GateComparison, GateSnapshot, ProjectError,
};

#[cfg(feature = "allocation-stats")]
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
#[cfg(feature = "allocation-stats")]
use std::alloc::System;

use crate::arguments::{Cli, ColorChoice, Command, Common, GateArgs, parse_days};
use crate::config::{self, ProjectConfig};
use crate::gate_baseline;
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
    reset_evidence_counters();
    let (cli, selected, config) = match prepare(arguments) {
        Ok(prepared) => prepared,
        Err(exit) => return exit,
    };
    let (result, common) = match cli.command {
        None => {
            let request = codebase_request(cli.path, &cli.common, &config);
            (request.analyze(), cli.common)
        }
        Some(Command::Gate(args)) => return run_gate(args, selected, &config),
        Some(Command::Diff(args)) => {
            let request = diff_request(args.path, args.reference, &args.common, &config);
            (request.analyze(), args.common)
        }
    };
    match result {
        Ok(result) => render(&result, &common),
        Err(error) => fail(&error),
    }
}

/// Clears the work counters before a measured run when either stats
/// instrumentation asks for them.
#[cfg(feature = "evidence-stats")]
fn reset_evidence_counters() {
    if std::env::var_os("SMACKDEBT_EVIDENCE_STATS").is_some()
        || std::env::var_os("SMACKDEBT_ALLOCATION_STATS").is_some()
    {
        smackdebt_project::reset_evidence();
    }
}

/// The codebase analysis one invocation asks for.
fn codebase_request(
    path: Option<PathBuf>,
    common: &Common,
    config: &ProjectConfig,
) -> CodebaseRequest {
    let request = match path {
        Some(path) => CodebaseRequest::new(path),
        None => CodebaseRequest::automatic("."),
    };
    apply_codebase_common(request, common, config)
}

/// Parses the invocation, validates the selected path, and loads project
/// configuration, or answers with the exit status the failure earned.
fn prepare(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<(Cli, Option<PathBuf>, ProjectConfig), ExitCode> {
    let cli = match Cli::try_parse_from(arguments) {
        Ok(cli) => cli,
        Err(error) => {
            let exit = error.exit_code();
            let _ = error.print();
            return Err(ExitCode::from(u8::try_from(exit).unwrap_or(2)));
        }
    };
    if json_all_conflict(&cli) {
        return Err(fail_with("--all cannot be used with --json", 2));
    }
    let (selected, config_root) = selected_paths(&cli);
    if let Some(path) = selected.as_deref().filter(|path| !path.exists()) {
        return Err(fail_with(&format!("path not found: {}", path.display()), 1));
    }
    let config = match config::load(&config_root) {
        Ok(config) => config,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "smackdebt: {message}");
            return Err(ExitCode::from(2));
        }
    };
    Ok((cli, selected, config))
}

/// Whether any command combines `--all` with `--json`, which no flow accepts.
fn json_all_conflict(cli: &Cli) -> bool {
    cli.common.json && cli.common.all
        || matches!(&cli.command, Some(Command::Diff(args)) if args.common.json && args.common.all)
}

/// The path the user selected, if any, and the directory configuration is
/// loaded from.
fn selected_paths(cli: &Cli) -> (Option<PathBuf>, PathBuf) {
    let selected = match &cli.command {
        None => cli.path.clone(),
        Some(Command::Diff(args)) => args.path.clone().or_else(|| cli.path.clone()),
        Some(Command::Gate(args)) => args.path.clone().or_else(|| cli.path.clone()),
    };
    let config_root = selected.clone().unwrap_or_else(|| PathBuf::from("."));
    (selected, config_root)
}

/// The diff analysis one invocation asks for.
fn diff_request(
    path: Option<PathBuf>,
    reference: Option<String>,
    common: &Common,
    config: &ProjectConfig,
) -> DiffRequest {
    let mut request = match path {
        Some(path) => DiffRequest::new(path),
        None => DiffRequest::automatic("."),
    };
    if let Some(reference) = reference {
        request = request.with_reference(reference);
    }
    if let Some(width) = execution_width(common.jobs) {
        request = request.with_width(width);
    }
    apply_diff_common(request, common, config)
}

/// Streams one finished report in the selected format and maps the write
/// outcome to an exit status.
fn render(result: &smackdebt_project::ProjectReport, common: &Common) -> ExitCode {
    #[cfg(feature = "allocation-stats")]
    print_allocation_run_stats();
    #[cfg(feature = "evidence-stats")]
    let evidence_enabled = std::env::var_os("SMACKDEBT_EVIDENCE_STATS").is_some();
    #[cfg(feature = "evidence-stats")]
    let before_render = evidence_enabled.then(smackdebt_project::evidence_snapshot);
    let stdout_is_terminal = io::stdout().is_terminal();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    #[cfg(feature = "evidence-stats")]
    if evidence_enabled {
        smackdebt_project::record_renderer_entry();
    }
    let rendered = if common.json {
        write_json(&mut stdout, result.report(), result.selected_scope())
            .and_then(|()| writeln!(stdout))
    } else {
        write_terminal(
            &mut stdout,
            result.report(),
            result.selected_scope(),
            terminal_options(common, stdout_is_terminal),
        )
    };
    match rendered.and_then(|()| stdout.flush()) {
        Ok(()) => {
            #[cfg(feature = "evidence-stats")]
            if evidence_enabled {
                print_evidence_stats(before_render.expect("evidence snapshot"));
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

/// The terminal options one run renders with, from the flags and the
/// environment.
fn terminal_options(common: &Common, stdout_is_terminal: bool) -> TerminalOptions {
    let width = terminal::width(stdout_is_terminal, std::env::var("COLUMNS").ok().as_deref());
    let choice = common.color.unwrap_or(ColorChoice::Auto);
    let color = terminal::color(
        choice,
        stdout_is_terminal,
        std::env::var_os("NO_COLOR").is_some(),
    );
    let decorations = terminal::decorations(choice, stdout_is_terminal);
    TerminalOptions::new(width, common.all, color)
        .with_decorations(decorations)
        .with_top(common.top.and_then(NonZeroUsize::new))
}

/// Prints the composition work counters when allocation profiling asks.
#[cfg(feature = "allocation-stats")]
fn print_allocation_run_stats() {
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
}

/// Prints what rendering added on top of composition, from the snapshot taken
/// before the write.
#[cfg(feature = "evidence-stats")]
fn print_evidence_stats(before_render: smackdebt_project::EvidenceSnapshot) {
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

/// Runs the ratchet gate: analyze, compare against the committed baseline,
/// and exit 3 when any ratcheted counter exceeds it. `--update` writes the
/// observed snapshot verbatim instead of comparing, so the baseline moves
/// only on request and a clean check run never dirties the working tree.
fn run_gate(args: GateArgs, selected: Option<PathBuf>, config: &ProjectConfig) -> ExitCode {
    let baseline_path = args.baseline.unwrap_or_else(|| match &selected {
        Some(path) => path.join(gate_baseline::BASELINE_FILE_NAME),
        None => PathBuf::from(gate_baseline::BASELINE_FILE_NAME),
    });
    let baseline = if args.update {
        None
    } else {
        match read_baseline(&baseline_path) {
            Ok(baseline) => Some(baseline),
            Err(exit) => return exit,
        }
    };
    let result = match gate_request(selected, config, args.jobs).analyze() {
        Ok(result) => result,
        Err(error) => return fail(&error),
    };
    let observed = GateSnapshot::from_report(result.report());
    let Some(baseline) = baseline else {
        return write_baseline(&baseline_path, &observed);
    };
    let comparison = GateComparison::between(&baseline, &observed);
    render_gate(&baseline_path.display().to_string(), &comparison, args.json)
}

/// Writes the observed snapshot verbatim, so the baseline moves only on
/// request.
fn write_baseline(path: &Path, observed: &GateSnapshot) -> ExitCode {
    match std::fs::write(path, gate_baseline::render(observed)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail_with(&format!("could not write {}: {error}", path.display()), 1),
    }
}

/// Streams the gate comparison and answers with the exit status the contract
/// promises: 3 on regression, quiet success when the reader left.
fn render_gate(baseline_name: &str, comparison: &GateComparison, json: bool) -> ExitCode {
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let rendered = if json {
        write_gate_json(&mut stdout, baseline_name, comparison).and_then(|()| writeln!(stdout))
    } else {
        write_gate(&mut stdout, baseline_name, comparison)
    }
    .and_then(|()| stdout.flush());
    match rendered {
        Err(error) if stdout_failure(&error) == StdoutFailure::Reportable => {
            fail(&ProjectError::Inspect {
                path: PathBuf::from("standard output"),
                source: error,
            })
        }
        // A reader that stopped reading never erases the gate's exit status,
        // which is the contract a check pipeline depends on.
        _ if comparison.regressed() => ExitCode::from(3),
        _ => ExitCode::SUCCESS,
    }
}

/// Reads and parses the committed baseline, or answers with the exit status
/// the failure earned.
fn read_baseline(path: &Path) -> Result<GateSnapshot, ExitCode> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(fail_with(
                &format!("baseline not found: {}", path.display()),
                2,
            ));
        }
        Err(error) => {
            return Err(fail(&ProjectError::Inspect {
                path: path.to_path_buf(),
                source: error,
            }));
        }
    };
    gate_baseline::parse(&text).map_err(|message| fail_with(&message, 2))
}

/// The codebase analysis one gate run measures, with config-driven defaults.
fn gate_request(
    selected: Option<PathBuf>,
    config: &ProjectConfig,
    jobs: Option<usize>,
) -> CodebaseRequest {
    let request = match selected {
        Some(path) => CodebaseRequest::new(path),
        None => CodebaseRequest::automatic("."),
    };
    let configured_history = config
        .history
        .as_deref()
        .and_then(|value| parse_days(value).ok());
    let request = request
        .with_history_days(configured_history.unwrap_or(90))
        .with_excludes(config.exclude.clone())
        .with_role_rules(config.role_rules());
    let request = apply_codebase_thresholds(request, config);
    match execution_width(jobs) {
        Some(width) => request.with_width(width),
        None => request,
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
        ProjectError::NoSourceFiles(path) => {
            format!("no source files found under: {}", path.display())
        }
        ProjectError::NotSourceFile(path) => format!("not a source file: {}", path.display()),
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
