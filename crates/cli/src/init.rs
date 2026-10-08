use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;

use crate::init_ui::SetupUi;
use crate::skill_install;
use crate::skill_selection::{Agent, Installation, Selection};
use crate::terminal::ColorChoice;

#[derive(Debug, Args)]
#[command(
    after_help = "Use ↑/↓ to move, Space to select, and Enter to install. Esc cancels.\n\nExamples:\n  smackdebt init\n  smackdebt init --agent codex --agent claude-code\n  smackdebt init --all --dry-run\n  smackdebt init --uninstall"
)]
pub(crate) struct InitArgs {
    /// Agent to set up. Repeat for multiple agents.
    #[arg(long, value_enum, conflicts_with = "all")]
    agent: Vec<Agent>,
    /// Set up all supported agents.
    #[arg(long)]
    all: bool,
    /// Skills directory for one explicitly selected agent.
    #[arg(long, requires = "agent")]
    dest: Option<PathBuf>,
    /// Show the changes without writing files.
    #[arg(long)]
    dry_run: bool,
    /// Remove managed skills, preserving edited and unrelated files.
    #[arg(long)]
    uninstall: bool,
}

pub(crate) fn run(args: InitArgs, color: ColorChoice) -> ExitCode {
    let ui = SetupUi::new(color, args.uninstall, args.dry_run);
    match execute(args, &ui) {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            let _ = ui.error(&message);
            ExitCode::from(code)
        }
    }
}

fn environment_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

struct Setup {
    home: PathBuf,
    state_path: PathBuf,
    selection: Selection,
}

impl Setup {
    fn load() -> Result<Self, (u8, String)> {
        let home = environment_path("HOME").ok_or((2, "HOME is not set".to_owned()))?;
        let state_path = skill_install::state_path(&home, environment_path("XDG_CONFIG_HOME"))
            .map_err(failure)?;
        let selection = skill_install::load(&state_path).map_err(failure)?;
        Ok(Self {
            home,
            state_path,
            selection,
        })
    }
}

fn execute(args: InitArgs, ui: &SetupUi) -> Result<(), (u8, String)> {
    let mut setup = Setup::load()?;
    let agents = choose_agents(&args, &setup.selection, &setup.home, ui)?;
    if args.dest.is_some() && agents.len() != 1 {
        return Err((2, "--dest requires exactly one --agent".to_owned()));
    }
    if agents.is_empty() {
        ui.finish("No agents selected. Nothing changed.")
            .map_err(output_failure)?;
        return Ok(());
    }
    let installations =
        plan_installations(&args, &agents, &mut setup.selection, &setup.home).map_err(failure)?;
    if args.dry_run {
        for install in installations {
            ui.result(
                if args.uninstall {
                    "Would remove"
                } else {
                    "Would install/update"
                },
                &install,
                &setup.home,
            )
            .map_err(output_failure)?;
        }
        return Ok(());
    }
    apply_installations(&args, &installations, agents, &mut setup, ui)
}

fn apply_installations(
    args: &InitArgs,
    installations: &[Installation],
    agents: Vec<Agent>,
    setup: &mut Setup,
    ui: &SetupUi,
) -> Result<(), (u8, String)> {
    if !args.uninstall {
        setup.selection.selected = agents;
        // Persist intent first; retries finish any partially completed install.
        skill_install::save(&setup.state_path, &setup.selection).map_err(failure)?;
    }
    let mut changed = false;
    for install in installations {
        let (action, updated) = apply(args, install).map_err(failure)?;
        changed |= updated;
        ui.result(action, install, &setup.home)
            .map_err(output_failure)?;
        if args.uninstall {
            setup
                .selection
                .installations
                .retain(|saved| saved.agent != install.agent);
            setup
                .selection
                .selected
                .retain(|agent| *agent != install.agent);
            skill_install::save(&setup.state_path, &setup.selection).map_err(failure)?;
        }
    }
    ui.complete(changed).map_err(output_failure)?;
    Ok(())
}

fn output_failure(error: io::Error) -> (u8, String) {
    (1, format!("could not write setup output: {error}"))
}

fn plan_installations(
    args: &InitArgs,
    agents: &[Agent],
    settings: &mut Selection,
    home: &std::path::Path,
) -> Result<Vec<Installation>, String> {
    let mut installations = Vec::with_capacity(agents.len());
    for &agent in agents {
        let directory = destination(agent, args, settings, home);
        skill_install::preflight(&directory)?;
        for saved in &settings.installations {
            if saved.agent != agent && skill_install::same_directory(&directory, &saved.directory)?
            {
                return Err(format!(
                    "skill destination is already used by {}",
                    saved.agent.name()
                ));
            }
        }
        let installation = Installation { agent, directory };
        settings.remember(installation.clone())?;
        installations.push(installation);
    }
    Ok(installations)
}

fn failure(message: String) -> (u8, String) {
    (1, message)
}

fn destination(
    agent: Agent,
    args: &InitArgs,
    settings: &Selection,
    home: &std::path::Path,
) -> PathBuf {
    if let Some(destination) = &args.dest {
        return destination.join("smackdebt");
    }
    if let Some(saved) = settings.remembered(agent) {
        return saved.directory.clone();
    }
    agent.directory(
        home,
        environment_path("CODEX_HOME"),
        environment_path("CLAUDE_CONFIG_DIR"),
    )
}

fn apply(args: &InitArgs, install: &Installation) -> Result<(&'static str, bool), String> {
    if args.uninstall {
        skill_install::remove(&install.directory)?;
        return Ok(("Removed", true));
    }
    use skill_install::InstallOutcome;
    Ok(match skill_install::install(&install.directory)? {
        InstallOutcome::Installed => ("Installed", true),
        InstallOutcome::Updated => ("Updated", true),
        InstallOutcome::Current => ("Already up to date:", false),
    })
}

fn choose_agents(
    args: &InitArgs,
    settings: &Selection,
    home: &std::path::Path,
    ui: &SetupUi,
) -> Result<Vec<Agent>, (u8, String)> {
    if args.all {
        return Ok(Agent::ALL.to_vec());
    }
    if !args.agent.is_empty() {
        let mut agents = Vec::new();
        for &agent in &args.agent {
            if !agents.contains(&agent) {
                agents.push(agent);
            }
        }
        return Ok(agents);
    }
    let candidates: Vec<_> = if args.uninstall {
        settings.installations.clone()
    } else {
        Agent::ALL
            .iter()
            .map(|&agent| Installation {
                agent,
                directory: destination(agent, args, settings, home),
            })
            .collect()
    };
    if ui.interactive() && !candidates.is_empty() {
        let selected = defaults(args, settings, &candidates);
        return ui.choose(&candidates, selected, home).map_err(|error| {
            if error.kind() == io::ErrorKind::Interrupted {
                (130, "Setup cancelled. Nothing changed.".to_owned())
            } else {
                (1, format!("could not read agent selection: {error}"))
            }
        });
    }
    if args.uninstall {
        return Ok(candidates.iter().map(|install| install.agent).collect());
    }
    if settings.selected.is_empty() {
        return Err((
            2,
            "choose agents with --agent codex (repeat as needed), or --all".to_owned(),
        ));
    }
    Ok(settings.selected.clone())
}

fn defaults(args: &InitArgs, settings: &Selection, candidates: &[Installation]) -> Vec<Agent> {
    if args.uninstall {
        return candidates.iter().map(|install| install.agent).collect();
    }
    if !settings.selected.is_empty() {
        return settings.selected.clone();
    }
    candidates
        .iter()
        .filter(|install| {
            install
                .directory
                .parent()
                .and_then(std::path::Path::parent)
                .is_some_and(std::path::Path::is_dir)
        })
        .map(|install| install.agent)
        .collect()
}
