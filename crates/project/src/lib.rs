#![forbid(unsafe_code)]

mod architecture;
mod candidates;
mod cycle_findings;
mod dependencies;
mod dormancy;
#[cfg(feature = "evidence-stats")]
mod evidence;
mod history_stream;
mod manifest_names;
mod package_graph;
mod paths;
mod project;
mod rating;
mod reference_tables;
mod requests;
mod resolution_config;
mod roles;
mod rust_layout;
mod source_units;
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
