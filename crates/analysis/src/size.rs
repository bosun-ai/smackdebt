use crate::health::{Rating, Thresholds};
use crate::report::FileId;

/// What a size finding measured.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SizeSubject {
    File,
    Container,
}

/// One rated size observation with its exact operand.
///
/// Size findings are rated but stay out of the unit health counts: they measure
/// a file or a container, not a unit, so adding them would double-count debt
/// the unit signals already carry.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SizeFinding {
    file: FileId,
    subject: SizeSubject,
    container: Option<String>,
    value: u32,
    rating: Rating,
}

impl SizeFinding {
    pub const fn file(&self) -> FileId {
        self.file
    }
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
    pub const fn new(file_lines: Thresholds, container_lines: Thresholds) -> Self {
        Self {
            file_lines,
            container_lines,
        }
    }

    pub const fn file_lines(self) -> Thresholds {
        self.file_lines
    }

    pub const fn container_lines(self) -> Thresholds {
        self.container_lines
    }

    /// Rates one file against its line count.
    pub fn rate_file(self, file: FileId, source_lines: u32) -> Option<SizeFinding> {
        Self::finding(self.file_lines, file, SizeSubject::File, None, source_lines)
    }

    /// Rates one container against its exclusive statement total.
    pub fn rate_container(
        self,
        file: FileId,
        container: &str,
        statements: u32,
    ) -> Option<SizeFinding> {
        Self::finding(
            self.container_lines,
            file,
            SizeSubject::Container,
            Some(container.to_owned()),
            statements,
        )
    }

    fn finding(
        thresholds: Thresholds,
        file: FileId,
        subject: SizeSubject,
        container: Option<String>,
        value: u32,
    ) -> Option<SizeFinding> {
        let rating = thresholds.level(value);
        (rating != Rating::Healthy).then_some(SizeFinding {
            file,
            subject,
            container,
            value,
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
