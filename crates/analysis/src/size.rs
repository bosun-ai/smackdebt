//! File size in source lines and container size in exclusive logical statements.
//!
//! Default inclusive Watch/High thresholds are 400/800 for files and 300/600 for
//! containers. Healthy observations produce no finding. Source composition
//! supplies trusted parsed measurements in verdict roles. File and container
//! findings remain separate from unit health counts to avoid counting one unit's
//! debt twice. A container name is only allocated once its size is rated.

#![deny(missing_docs)]

use crate::health::{Rating, Thresholds};
use crate::report::FileId;

macro_rules! size_index {
    ($(#[$documentation:meta])* $name:ident) => {
        $(#[$documentation])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            /// Creates an index from a table position.
            pub const fn from_index(index: usize) -> Self {
                Self(index as u32)
            }

            /// Returns the table position represented by this index.
            pub const fn index(self) -> usize {
                self.0 as usize
            }

            /// Returns the compact integer representation.
            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

size_index!(
    /// The identity of one size finding.
    ///
    /// A size finding is identified by its position in the report's size
    /// finding table, the way every other finding family is identified, so a
    /// problem card can link one without copying its path, container name, or
    /// value.
    SizeFindingId
);

/// What a size finding measured.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SizeSubject {
    /// A file in the retained source inventory.
    File,
    /// A named container within a file.
    Container,
}

/// One rated size observation with its exact operand.
///
/// Size findings are rated but stay out of the unit health counts: they measure
/// a file or a container, not a unit, so adding them would double-count debt
/// the unit signals already carry. Only trusted parsed source in a verdict role
/// is sized, so advisory and context source produces no size finding.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SizeFinding {
    file: FileId,
    subject: SizeSubject,
    container: Option<String>,
    value: u32,
    rating: Rating,
}

impl SizeFinding {
    /// The file-table identity this observation describes.
    pub const fn file(&self) -> FileId {
        self.file
    }
    /// The subject whose measurement this value records.
    pub const fn subject(&self) -> SizeSubject {
        self.subject
    }
    /// The container name, for a container finding.
    pub fn container(&self) -> Option<&str> {
        self.container.as_deref()
    }
    /// The exact measured line or statement total.
    pub const fn value(&self) -> u32 {
        self.value
    }
    /// The health rating carried by this observation.
    pub const fn rating(&self) -> Rating {
        self.rating
    }
}

/// Configurable thresholds for file and container size.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SizePolicy {
    file_lines: Thresholds,
    container_lines: Thresholds,
}

impl Default for SizePolicy {
    fn default() -> Self {
        Self::new(Thresholds::new(400, 800), Thresholds::new(300, 600))
    }
}

impl SizePolicy {
    /// Sets separate inclusive threshold pairs for file lines and container statements.
    pub const fn new(file_lines: Thresholds, container_lines: Thresholds) -> Self {
        Self {
            file_lines,
            container_lines,
        }
    }

    /// The inclusive thresholds for physical source lines in a file.
    pub const fn file_lines(self) -> Thresholds {
        self.file_lines
    }

    /// The inclusive thresholds for exclusive logical statements in a container.
    pub const fn container_lines(self) -> Thresholds {
        self.container_lines
    }

    /// Rates one file against its line count.
    pub fn rate_file(self, file: FileId, source_lines: u32) -> Option<SizeFinding> {
        let rating = self.file_lines.level(source_lines);
        (rating != Rating::Healthy).then_some(SizeFinding {
            file,
            subject: SizeSubject::File,
            container: None,
            value: source_lines,
            rating,
        })
    }

    /// Rates one container against its exclusive statement total.
    ///
    /// The container name is owned only once the threshold decides, so a
    /// healthy container costs no allocation.
    pub fn rate_container(
        self,
        file: FileId,
        container: &str,
        statements: u32,
    ) -> Option<SizeFinding> {
        let rating = self.container_lines.level(statements);
        (rating != Rating::Healthy).then_some(SizeFinding {
            file,
            subject: SizeSubject::Container,
            container: Some(container.to_owned()),
            value: statements,
            rating,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> FileId {
        FileId::from_index(0)
    }

    #[test]
    fn file_size_watches_at_four_hundred_lines_and_is_high_at_eight_hundred() {
        let policy = SizePolicy::default();
        assert_eq!(policy.file_lines(), Thresholds::new(400, 800));
        assert!(policy.rate_file(file(), 399).is_none());
        let watch = policy.rate_file(file(), 400).unwrap();
        assert_eq!(watch.rating(), Rating::Watch);
        assert_eq!(watch.value(), 400);
        assert_eq!(watch.subject(), SizeSubject::File);
        assert_eq!(watch.container(), None);
        assert_eq!(
            policy.rate_file(file(), 799).unwrap().rating(),
            Rating::Watch
        );
        assert_eq!(
            policy.rate_file(file(), 800).unwrap().rating(),
            Rating::High
        );
    }

    #[test]
    fn container_size_watches_at_three_hundred_statements_and_is_high_at_six_hundred() {
        let policy = SizePolicy::default();
        assert_eq!(policy.container_lines(), Thresholds::new(300, 600));
        assert!(policy.rate_container(file(), "Worker", 299).is_none());
        let watch = policy.rate_container(file(), "Worker", 300).unwrap();
        assert_eq!(watch.rating(), Rating::Watch);
        assert_eq!(watch.container(), Some("Worker"));
        assert_eq!(
            policy
                .rate_container(file(), "Worker", 599)
                .unwrap()
                .rating(),
            Rating::Watch
        );
        let high = policy.rate_container(file(), "Worker", 640).unwrap();
        assert_eq!(high.rating(), Rating::High);
        assert_eq!(high.value(), 640);
    }

    #[test]
    fn a_small_file_and_a_small_container_create_no_finding() {
        let policy = SizePolicy::default();
        assert!(policy.rate_file(file(), 120).is_none());
        assert!(policy.rate_container(file(), "Worker", 12).is_none());
    }

    #[test]
    fn both_thresholds_are_configurable() {
        let policy = SizePolicy::new(Thresholds::new(10, 20), Thresholds::new(5, 9));
        assert_eq!(
            policy.rate_file(file(), 10).unwrap().rating(),
            Rating::Watch
        );
        assert_eq!(
            policy.rate_container(file(), "Worker", 9).unwrap().rating(),
            Rating::High
        );
    }
}
