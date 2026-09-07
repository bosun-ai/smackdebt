#![forbid(unsafe_code)]

#[cfg(feature = "evidence-stats")]
mod evidence;
mod manifest_names;
mod paths;
mod project;
mod requests;
mod work;

#[cfg(feature = "evidence-stats")]
pub use evidence::{
    EvidenceSnapshot, record_renderer_entry, reset as reset_evidence, snapshot as evidence_snapshot,
};

pub use requests::{
    CodebaseRequest, DiffRequest, ExecutionWidth, ProjectError, ProjectReport, SourceRoleRule,
    WorkStats,
};
pub use smackdebt_analysis::DEFAULT_MINIMUM_TOUCHES;
pub use smackdebt_analysis::{GateComparison, GateRow, GateSignal, GateSnapshot};

#[doc(hidden)]
pub use smackdebt_languages::{parser_time_ns, reset_parser_time};
