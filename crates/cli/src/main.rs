#![forbid(unsafe_code)]

mod app;
mod arguments;
mod config;
mod gate_baseline;
mod terminal;

fn main() -> std::process::ExitCode {
    app::main()
}
