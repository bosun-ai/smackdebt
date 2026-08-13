#![forbid(unsafe_code)]

mod app;
mod arguments;
mod config;
mod terminal;

fn main() -> std::process::ExitCode {
    app::main()
}
