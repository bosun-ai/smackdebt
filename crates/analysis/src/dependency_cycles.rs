//! Dependency cycles and the size of the largest cyclic component.
//!
//! A cycle finding keeps members and witness edges. Package cycles are High;
//! file cycles are Watch. Graph construction and strongly connected component
//! algorithms supply the members rather than being rerun by these values.
//!
//! Core size is the largest file component relative to the eligible file graph.
//! It is stated at five files and 2% of graph files, both inclusive, and remains
//! descriptive: cycle findings carry the rating. Core comparisons retain member
//! identities and before/after counts so movement can be explained.

#![deny(missing_docs)]

use crate::{ComparisonDirection, DependencyEdgeId, FileId, PackageId, Rating};

/// Whether a cyclic component joins packages or files.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchitectureFindingKind {
    /// A cyclic group of packages, rated High.
    PackageCycle,
    /// A cyclic group of files, rated Watch.
    FileCycle,
}

/// A rated dependency cycle with member identities and witness edges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchitectureFinding {
    id: ArchitectureFindingId,
    kind: ArchitectureFindingKind,
    rating: Rating,
    packages: Vec<PackageId>,
    files: Vec<FileId>,
    witness_edges: Vec<DependencyEdgeId>,
}

impl ArchitectureFinding {
    /// Rates a package cycle High or a file cycle Watch, retaining its members and witnesses.
    pub fn new(
        id: ArchitectureFindingId,
        kind: ArchitectureFindingKind,
        packages: Vec<PackageId>,
        files: Vec<FileId>,
        witness_edges: Vec<DependencyEdgeId>,
    ) -> Self {
        let rating = match kind {
            ArchitectureFindingKind::PackageCycle => Rating::High,
            ArchitectureFindingKind::FileCycle => Rating::Watch,
        };
        Self {
            id,
            kind,
            rating,
            packages,
            files,
            witness_edges,
        }
    }
    /// The row's typed position in its owning report table.
    pub const fn id(&self) -> ArchitectureFindingId {
        self.id
    }
    /// The finding or movement category represented by this row.
    pub const fn kind(&self) -> ArchitectureFindingKind {
        self.kind
    }
    /// The health rating carried by this observation.
    pub const fn rating(&self) -> Rating {
        self.rating
    }
    /// The package identities participating in this observation.
    pub fn packages(&self) -> &[PackageId] {
        &self.packages
    }
    /// The file identities participating in this observation.
    pub fn files(&self) -> &[FileId] {
        &self.files
    }
    /// The file-edge identities that explain this finding.
    pub fn witness_edges(&self) -> &[DependencyEdgeId] {
        &self.witness_edges
    }
}

/// Largest-cycle movement with before/after graph sizes and member identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreComparison {
    id: CoreComparisonId,
    anchor: FileId,
    direction: ComparisonDirection,
    before_core: u32,
    before_files: u32,
    after_core: u32,
    after_files: u32,
    before_members: Vec<FileId>,
    after_members: Vec<FileId>,
}

impl CoreComparison {
    /// Retains the comparison direction and each side's (cycle size, graph size, members).
    pub fn new(
        id: CoreComparisonId,
        anchor: FileId,
        direction: ComparisonDirection,
        before: (u32, u32, Vec<FileId>),
        after: (u32, u32, Vec<FileId>),
    ) -> Self {
        Self {
            id,
            anchor,
            direction,
            before_core: before.0,
            before_files: before.1,
            before_members: before.2,
            after_core: after.0,
            after_files: after.1,
            after_members: after.2,
        }
    }
    /// The row's typed position in its owning report table.
    pub const fn id(&self) -> CoreComparisonId {
        self.id
    }
    /// The stable file identity used to locate the cycle movement.
    pub const fn anchor(&self) -> FileId {
        self.anchor
    }
    /// Whether the comparison represents worse, better, or neutral debt movement.
    pub const fn direction(&self) -> ComparisonDirection {
        self.direction
    }
    /// The (largest-cycle files, graph files) before the change.
    pub const fn before(&self) -> (u32, u32) {
        (self.before_core, self.before_files)
    }
    /// The (largest-cycle files, graph files) after the change.
    pub const fn after(&self) -> (u32, u32) {
        (self.after_core, self.after_files)
    }
    /// The file members of the largest cycle before the change.
    pub fn before_members(&self) -> &[FileId] {
        &self.before_members
    }
    /// The file members of the largest cycle after the change.
    pub fn after_members(&self) -> &[FileId] {
        &self.after_members
    }
}

crate::table_index::table_index!(
    /// The position of one architecture finding in its report table.
    ArchitectureFindingId
);

crate::table_index::table_index!(
    /// The position of one core comparison in its report table.
    CoreComparisonId
);

/// The largest file dependency cycle, against the graph it sits in.
///
/// A core is descriptive: it is never rated, creates no finding, and changes
/// no verdict. The per-component cycle findings and their witnesses are what a
/// reader acts on; this states how much of the codebase moves together.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CoreSize {
    core: u32,
    files: u32,
}

impl CoreSize {
    /// Completes a core size when the largest cycle is both absolutely and
    /// proportionally worth stating.
    ///
    /// A three-file cycle is a local knot rather than a core, and a cycle that
    /// is a rounding error of the codebase says nothing about the codebase, so
    /// each floor rules out one of those.
    pub const fn from_counts(core: u32, files: u32) -> Option<Self> {
        if core < CORE_SIZE_FILES {
            return None;
        }
        if core as u64 * 100 < files as u64 * CORE_SIZE_PERCENT as u64 {
            return None;
        }
        Some(Self { core, files })
    }

    /// The exact sentence every consumer prints for this core.
    pub fn sentence(self) -> String {
        format!("{}.", self.fragment())
    }

    /// The same fact without its full stop, which is what a card states.
    ///
    /// A card stacks lowercase fragments — `a change here reaches 11 files`,
    /// `2 files in the cycle` — and a closed sentence among them reads as a
    /// different kind of claim than the ones around it. A verdict-level
    /// statement is a sentence and keeps the stop.
    pub fn fragment(self) -> String {
        format!(
            "{} of {} files sit in one dependency cycle",
            self.core, self.files
        )
    }

    /// The files the largest cycle holds.
    pub const fn core(self) -> u32 {
        self.core
    }

    /// The files the file dependency graph is built over.
    pub const fn files(self) -> u32 {
        self.files
    }
}

/// The files the largest dependency cycle holds before it is a core.
pub const CORE_SIZE_FILES: u32 = 5;

/// The percent of the graph's files the largest cycle holds before it is a
/// core, so a cycle that is a rounding error of the codebase stays silent.
pub const CORE_SIZE_PERCENT: u32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_core_is_material_only_above_both_of_its_floors() {
        assert!(
            CoreSize::from_counts(4, 20).is_none(),
            "four files are a knot rather than a core"
        );
        assert!(
            CoreSize::from_counts(3, 200).is_none(),
            "three of two hundred is below both floors"
        );
        assert!(
            CoreSize::from_counts(5, 300).is_none(),
            "five of three hundred is below the proportional floor alone"
        );
        let exactly = CoreSize::from_counts(5, 250).expect("exactly two percent is material");
        assert_eq!(exactly.core(), 5);
        assert_eq!(exactly.files(), 250);
        let core = CoreSize::from_counts(34, 210).expect("a large core");
        assert_eq!(
            core.sentence(),
            "34 of 210 files sit in one dependency cycle."
        );
        // The two forms are one fact: the fragment a card stacks, and the
        // sentence a verdict-level statement closes.
        assert_eq!(
            core.fragment(),
            "34 of 210 files sit in one dependency cycle"
        );
    }
}
