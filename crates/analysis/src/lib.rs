#![forbid(unsafe_code)]

mod comparison;
mod health;
mod report;
mod source;

pub use comparison::{Comparison, ComparisonDirection, ComparisonKind, compare_units};
pub use health::{
    HealthAssessment, HealthCounts, HealthPolicy, Measurements, Rating, Signal, SignalAssessment,
    Thresholds,
};
pub use report::{
    ComparisonId, Coverage, Diagnostic, DiagnosticId, DiagnosticKind, DiffCounts, FileActivity,
    FileId, FileRecord, Finding, FindingId, PackageId, PathId, Report, ReportBuilder, ReportMode,
    Scope, ScopeId, ScopeKind, aggregate_comparisons, aggregate_scopes,
};
pub use source::{
    FileAnalysis, Language, LocalUnitId, ParseStatus, SourceSpan, UnitFact, UnitIdentity, UnitKind,
};
