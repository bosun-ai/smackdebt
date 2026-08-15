use crate::health::Rating;
use crate::report::FileId;

/// The rated debt and windowed change activity of one file.
///
/// Both operands stay integers: no combined score, floating-point value, or
/// invented weight is produced anywhere in hotspot policy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileDebt {
    file: FileId,
    rated_units: u32,
    max_rating: Rating,
    touches: u32,
}

impl FileDebt {
    pub const fn new(file: FileId, rated_units: u32, max_rating: Rating, touches: u32) -> Self {
        Self {
            file,
            rated_units,
            max_rating,
            touches,
        }
    }
    pub const fn file(self) -> FileId {
        self.file
    }
    pub const fn rated_units(self) -> u32 {
        self.rated_units
    }
    pub const fn max_rating(self) -> Rating {
        self.max_rating
    }
    pub const fn touches(self) -> u32 {
        self.touches
    }
}

/// A file whose rated debt meets its change activity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Hotspot {
    file: FileId,
    rating: Rating,
    touches: u32,
}

impl Hotspot {
    pub const fn new(file: FileId, rating: Rating, touches: u32) -> Self {
        Self {
            file,
            rating,
            touches,
        }
    }
    pub const fn file(self) -> FileId {
        self.file
    }
    /// The file's maximum unit rating.
    pub const fn rating(self) -> Rating {
        self.rating
    }
    /// The file's exact touch count inside the analyzed history window.
    pub const fn touches(self) -> u32 {
        self.touches
    }
}

/// The minimum change activity a rated file needs to be hot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HotspotPolicy {
    minimum_touches: u32,
}

impl Default for HotspotPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_MINIMUM_TOUCHES)
    }
}

/// The touch count a rated file reaches before it is hot.
pub const DEFAULT_MINIMUM_TOUCHES: u32 = 5;

impl HotspotPolicy {
    pub const fn new(minimum_touches: u32) -> Self {
        Self { minimum_touches }
    }

    pub const fn minimum_touches(self) -> u32 {
        self.minimum_touches
    }

    /// Derives the hotspot table from tables the report already builds.
    ///
    /// Rows keep the order of the file table, which is the report's
    /// data-stable file order, so serial and parallel runs agree.
    pub fn hotspots(self, files: &[FileDebt]) -> Vec<Hotspot> {
        let mut hotspots = Vec::with_capacity(files.len());
        hotspots.extend(
            files
                .iter()
                .filter(|debt| debt.rated_units() > 0 && debt.touches() >= self.minimum_touches)
                .map(|debt| Hotspot::new(debt.file(), debt.max_rating(), debt.touches())),
        );
        hotspots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debt(index: usize, rated_units: u32, rating: Rating, touches: u32) -> FileDebt {
        FileDebt::new(FileId::from_index(index), rated_units, rating, touches)
    }

    #[test]
    fn a_rated_file_that_changes_often_keeps_both_integer_operands() {
        let hotspots = HotspotPolicy::default().hotspots(&[debt(0, 3, Rating::High, 14)]);
        assert_eq!(
            hotspots,
            [Hotspot::new(FileId::from_index(0), Rating::High, 14)]
        );
    }

    #[test]
    fn the_minimum_touch_count_is_five_and_is_inclusive() {
        assert_eq!(HotspotPolicy::default().minimum_touches(), 5);
        let hotspots = HotspotPolicy::default()
            .hotspots(&[debt(0, 1, Rating::Watch, 4), debt(1, 1, Rating::Watch, 5)]);
        assert_eq!(
            hotspots,
            [Hotspot::new(FileId::from_index(1), Rating::Watch, 5)]
        );
    }

    #[test]
    fn a_file_without_a_rated_unit_is_never_hot() {
        assert!(
            HotspotPolicy::default()
                .hotspots(&[debt(0, 0, Rating::Healthy, 40)])
                .is_empty()
        );
    }

    #[test]
    fn a_rated_file_that_never_changes_is_never_hot() {
        assert!(
            HotspotPolicy::default()
                .hotspots(&[debt(0, 2, Rating::High, 0)])
                .is_empty()
        );
    }

    #[test]
    fn the_minimum_touch_count_is_configurable() {
        let hotspots = HotspotPolicy::new(2).hotspots(&[debt(0, 1, Rating::Watch, 2)]);
        assert_eq!(hotspots.len(), 1);
    }

    #[test]
    fn rows_keep_the_file_table_order() {
        let hotspots = HotspotPolicy::new(1).hotspots(&[
            debt(0, 1, Rating::Watch, 9),
            debt(1, 1, Rating::High, 1),
            debt(2, 1, Rating::Healthy, 3),
        ]);
        assert_eq!(
            hotspots
                .iter()
                .map(|hotspot| hotspot.file())
                .collect::<Vec<_>>(),
            [
                FileId::from_index(0),
                FileId::from_index(1),
                FileId::from_index(2)
            ]
        );
    }
}
