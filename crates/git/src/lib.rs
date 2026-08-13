#![forbid(unsafe_code)]

#[cfg(feature = "evidence-stats")]
mod evidence;
mod repository;

#[cfg(feature = "evidence-stats")]
pub use evidence::{
    EvidenceSnapshot as GitEvidenceSnapshot, reset as reset_evidence, snapshot as evidence_snapshot,
};

pub use repository::{
    Change, ContributorIdentity, GitError, GitRepository, HistoryChange, HistoryCommit,
    HistoryStreamSummary, ObjectReader,
};
