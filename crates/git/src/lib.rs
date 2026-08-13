#![forbid(unsafe_code)]

mod repository;

pub use repository::{Change, FileActivity, GitError, GitRepository, ObjectReader};
