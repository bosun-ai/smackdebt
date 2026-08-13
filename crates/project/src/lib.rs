#![forbid(unsafe_code)]

#[cfg(feature = "evidence-stats")]
mod evidence;
mod project;
mod requests;

#[cfg(feature = "evidence-stats")]
pub use evidence::{
    EvidenceSnapshot, record_renderer_entry, reset as reset_evidence, snapshot as evidence_snapshot,
};

pub use requests::{
    CodebaseRequest, DiffRequest, ExecutionWidth, ProjectError, ProjectReport, WorkStats,
};

#[doc(hidden)]
pub use smackdebt_languages::{parser_time_ns, reset_parser_time};
