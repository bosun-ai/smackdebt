#![forbid(unsafe_code)]

mod repository;

pub use repository::{
    Change, ContributorIdentity, GitError, GitRepository, HistoryChange, HistoryCommit,
    HistoryStreamSummary, ObjectReader,
};
