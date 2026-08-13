#![forbid(unsafe_code)]

mod project;
mod requests;

pub use requests::{
    CodebaseRequest, DiffRequest, ExecutionWidth, ProjectError, ProjectReport, WorkStats,
};

#[doc(hidden)]
pub use smackdebt_languages::{parser_time_ns, reset_parser_time};
