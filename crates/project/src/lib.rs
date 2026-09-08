#![forbid(unsafe_code)]

mod architecture;
mod candidates;
mod codebase;
mod codebase_report;
mod core_comparisons;
mod cycle_findings;
mod dependencies;
mod diff;
mod diff_changes;
mod diff_comparisons;
mod diff_findings;
mod diff_graphs;
mod diff_impact;
mod diff_report;
mod diff_source;
mod dormancy;
#[cfg(feature = "evidence-stats")]
mod evidence;
mod go_project;
mod hierarchy;
mod history_stream;
mod manifest_names;
mod named_dependencies;
mod package_graph;
mod paths;
mod php_project;
mod project_metadata;
mod propagation_comparisons;
mod rating;
mod reference_tables;
mod requests;
mod resolution_config;
mod resolution_rules;
mod roles;
mod rust_layout;
mod selection;
mod source_units;
mod test_scope;
#[cfg(test)]
mod test_support;
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
pub use smackdebt_analysis::{HealthPolicy, Thresholds};

#[doc(hidden)]
pub use smackdebt_languages::{parser_time_ns, reset_parser_time};
