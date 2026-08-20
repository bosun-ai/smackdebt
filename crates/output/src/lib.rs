#![forbid(unsafe_code)]

mod gate;
mod json;
mod output;

pub use gate::{write_gate, write_gate_json};
pub use json::write_json;
pub use output::{TerminalOptions, write_terminal};
