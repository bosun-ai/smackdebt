#![forbid(unsafe_code)]

mod analyzer;
mod ruby;
mod upstream;
mod vue;

pub use analyzer::{AnalysisError, Analyzer};
