#![forbid(unsafe_code)]

mod app;
mod arguments;
mod config;
mod gate_baseline;
mod init;
mod init_ui;
mod progress;
mod skill_install;
mod skill_selection;
mod terminal;

fn main() -> std::process::ExitCode {
    app::main()
}
