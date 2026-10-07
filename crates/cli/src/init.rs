use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;

use crate::skill_install;
use crate::skill_selection::{Agent, Installation, Selection};

#[derive(Debug, Args)]
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

pub(crate) fn run(args: InitArgs) -> ExitCode {
    match execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            eprintln!("smackdebt: {message}");
            ExitCode::from(code)
        }
    }
}

fn environment_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn execute(args: InitArgs) -> Result<(), (u8, String)> {
    let home = environment_path("HOME").ok_or((2, "HOME is not set".to_owned()))?;
    let path =
        skill_install::state_path(&home, environment_path("XDG_CONFIG_HOME")).map_err(failure)?;
    let mut settings = skill_install::load(&path).map_err(failure)?;
    let agents = choose_agents(&args, &settings)?;
    if args.dest.is_some() && agents.len() != 1 {
        return Err((2, "--dest requires exactly one --agent".to_owned()));
    }
    if agents.is_empty() {
        println!("No agents selected.");
        return Ok(());
    }
    let installations =
        plan_installations(&args, &agents, &mut settings, &home).map_err(failure)?;
    if args.dry_run {
        for install in installations {
            println!(
                "Would {} {} skill at {}",
                if args.uninstall {
                    "remove"
                } else {
                    "install/update"
                },
                install.agent.name(),
                install.directory.display()
            );
        }
        return Ok(());
    }
    if !args.uninstall {
        settings.selected = agents;
        // Persist intent first; retries finish any partially completed install.
        skill_install::save(&path, &settings).map_err(failure)?;
    }
    for install in installations {
        apply(&args, &install).map_err(failure)?;
        if args.uninstall {
            settings
                .installations
                .retain(|saved| saved.agent != install.agent);
            settings.selected.retain(|agent| *agent != install.agent);
            skill_install::save(&path, &settings).map_err(failure)?;
        }
    }
    Ok(())
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

fn apply(args: &InitArgs, install: &Installation) -> Result<(), String> {
    let action = if args.uninstall {
        skill_install::remove(&install.directory)?;
        "Removed"
    } else {
        skill_install::install(&install.directory)?;
        "Installed/updated"
    };
    println!(
        "{action} {} skill at {}",
        install.agent.name(),
        install.directory.display()
    );
    Ok(())
}

fn choose_agents(args: &InitArgs, settings: &Selection) -> Result<Vec<Agent>, (u8, String)> {
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
    if args.uninstall {
        return Ok(settings
            .installations
            .iter()
            .map(|install| install.agent)
            .collect());
    }
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        return prompt(&settings.selected).map_err(|message| (2, message));
    }
    if settings.selected.is_empty() {
        return Err((
            2,
            "choose agents with --agent codex (repeat as needed), or --all".to_owned(),
        ));
    }
    Ok(settings.selected.clone())
}

fn prompt(saved: &[Agent]) -> Result<Vec<Agent>, String> {
    println!("Choose agents for your account:");
    for (index, agent) in Agent::ALL.iter().enumerate() {
        println!(
            "  {}. [{}] {}",
            index + 1,
            if saved.contains(agent) { "x" } else { " " },
            agent.name()
        );
    }
    print!("Numbers separated by spaces; Enter keeps selection; q cancels: ");
    io::stdout().flush().map_err(|error| error.to_string())?;
    let mut answer = String::new();
    if io::stdin()
        .read_line(&mut answer)
        .map_err(|error| error.to_string())?
        == 0
    {
        return Ok(Vec::new());
    }
    parse_selection(&answer, saved)
}

fn parse_selection(answer: &str, saved: &[Agent]) -> Result<Vec<Agent>, String> {
    let answer = answer.trim();
    if answer == "q" {
        return Ok(Vec::new());
    }
    if answer.is_empty() {
        return Ok(saved.to_vec());
    }
    let mut selected = Vec::new();
    for number in answer.split_whitespace() {
        let index = number.parse::<usize>().ok().and_then(|n| n.checked_sub(1));
        let agent = index
            .and_then(|index| Agent::ALL.get(index))
            .ok_or("choose numbers from 1 to 5, or q to cancel")?;
        if !selected.contains(agent) {
            selected.push(*agent);
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_keeps_choices_cancels_and_rejects_unknown_agents() {
        assert_eq!(
            parse_selection("", &[Agent::Codex]).unwrap(),
            vec![Agent::Codex]
        );
        assert!(parse_selection("q", &[Agent::Codex]).unwrap().is_empty());
        assert_eq!(
            parse_selection("1 2 1", &[]).unwrap(),
            vec![Agent::Codex, Agent::ClaudeCode]
        );
        assert!(parse_selection("0", &[]).is_err());
        assert!(parse_selection("6", &[]).is_err());
    }
}
