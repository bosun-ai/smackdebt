#![forbid(unsafe_code)]

mod json;
mod output;

pub use json::write_json;
pub use output::{TerminalOptions, write_terminal};
