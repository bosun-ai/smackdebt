use std::collections::BTreeMap;

use crate::architecture::{ArchitectureFinding, ArchitectureFindingKind, StableDependencyFinding};
use crate::health::{Rating, Signal};
use crate::report::Report;
use crate::size::SizeSubject;

/// One ratcheted debt signal with a frozen identifier.
///
/// Every gate signal is time-invariant: it derives only from the tree being
/// analyzed, never from history, so an unchanged tree produces an unchanged
/// gate result however much wall-clock time passes. History-derived signals
/// such as change coupling and knowledge concentration are excluded by this
/// rule, not by omission — they move with the selected window and would make
/// a committed gate flaky.
///
/// The variants are declared in the byte order of their identifiers so the
/// derived order and the written baseline order are the same order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GateSignal {
    Cognitive,
    ContainerSize,
    Cyclomatic,
    FileCycle,
    FileSize,
    LogicalLines,
    Nesting,
    PackageCycle,
    Parameters,
    StableDependency,
}

impl GateSignal {
    /// Every ratcheted signal, in baseline order.
    pub const ALL: [Self; 10] = [
        Self::Cognitive,
        Self::ContainerSize,
        Self::Cyclomatic,
        Self::FileCycle,
        Self::FileSize,
        Self::LogicalLines,
        Self::Nesting,
        Self::PackageCycle,
        Self::Parameters,
        Self::StableDependency,
    ];

    /// The frozen identifier written in baselines and gate reports.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Cognitive => "cognitive",
            Self::ContainerSize => "container_size",
            Self::Cyclomatic => "cyclomatic",
            Self::FileCycle => "file_cycle",
            Self::FileSize => "file_size",
            Self::LogicalLines => "logical_lines",
            Self::Nesting => "nesting",
            Self::PackageCycle => "package_cycle",
            Self::Parameters => "parameters",
            Self::StableDependency => "stable_dependency",
        }
    }

    /// Resolves a baseline identifier, so an unknown signal never passes.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|signal| signal.id() == id)
    }

    const fn from_unit_signal(signal: Signal) -> Self {
        match signal {
            Signal::CognitiveComplexity => Self::Cognitive,
            Signal::CyclomaticComplexity => Self::Cyclomatic,
            Signal::LogicalLines => Self::LogicalLines,
            Signal::MaxNesting => Self::Nesting,
            Signal::ParameterCount => Self::Parameters,
        }
    }

    const fn from_size_subject(subject: SizeSubject) -> Self {
        match subject {
            SizeSubject::File => Self::FileSize,
            SizeSubject::Container => Self::ContainerSize,
        }
    }
}

/// One ratcheted key with its High and Watch counts.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct GateRow {
    path: String,
    signal: GateSignal,
    high: u32,
    watch: u32,
}

impl GateRow {
    pub fn new(path: impl Into<String>, signal: GateSignal, high: u32, watch: u32) -> Self {
        Self {
            path: path.into(),
            signal,
            high,
            watch,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub const fn signal(&self) -> GateSignal {
        self.signal
    }

    pub const fn high(&self) -> u32 {
        self.high
    }

    pub const fn watch(&self) -> u32 {
        self.watch
    }
}

/// The ratcheted debt of one tree, sorted by path then signal.
///
/// A zero-zero key is meaningless because an absent key already means no
/// debt, so [`GateSnapshot::from_report`] never stores one.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GateSnapshot {
    rows: Vec<GateRow>,
}

impl GateSnapshot {
    /// Wraps rows a validated reader produced in baseline order.
    pub fn new(rows: Vec<GateRow>) -> Self {
        Self { rows }
    }

    pub fn rows(&self) -> &[GateRow] {
        &self.rows
    }

    /// Counts every ratcheted signal in one report.
    ///
    /// Unit signals count per rated unit at each signal's own rating and are
    /// attributed to the unit's file; only findings that affect the verdict
    /// count, and a unit tripping two signals produces two independent keys.
    /// Size findings count per subject at the sized file. Cycle and
    /// stable-dependency findings are attributed to their first witness,
    /// which is the source file of their first witness edge.
    pub fn from_report(report: &Report) -> Self {
        let mut counts: BTreeMap<(&str, GateSignal), (u32, u32)> = BTreeMap::new();
        for finding in report
            .findings()
            .iter()
            .filter(|finding| finding.affects_verdict())
        {
            let path = report.files()[finding.file().index()].path();
            for signal in finding.assessment().signals() {
                add(
                    &mut counts,
                    path,
                    GateSignal::from_unit_signal(signal.signal()),
                    signal.rating(),
                );
            }
        }
        for finding in report.size_findings() {
            add(
                &mut counts,
                report.files()[finding.file().index()].path(),
                GateSignal::from_size_subject(finding.subject()),
                finding.rating(),
            );
        }
        for finding in report.architecture_findings() {
            if let Some(path) = cycle_witness_path(report, finding) {
                add(
                    &mut counts,
                    path,
                    match finding.kind() {
                        ArchitectureFindingKind::PackageCycle => GateSignal::PackageCycle,
                        ArchitectureFindingKind::FileCycle => GateSignal::FileCycle,
                    },
                    finding.rating(),
                );
            }
        }
        for finding in report.stable_dependency_findings() {
            if let Some(path) = stable_witness_path(report, finding) {
                add(
                    &mut counts,
                    path,
                    GateSignal::StableDependency,
                    finding.rating(),
                );
            }
        }
        let rows = counts
            .into_iter()
            .filter(|(_, (high, watch))| *high > 0 || *watch > 0)
            .map(|((path, signal), (high, watch))| GateRow::new(path, signal, high, watch))
            .collect();
        Self { rows }
    }
}

fn add<'report>(
    counts: &mut BTreeMap<(&'report str, GateSignal), (u32, u32)>,
    path: &'report str,
    signal: GateSignal,
    rating: Rating,
) {
    let entry = counts.entry((path, signal)).or_default();
    match rating {
        Rating::High => entry.0 += 1,
        Rating::Watch => entry.1 += 1,
        Rating::Healthy => {}
    }
}

/// The path of a cycle's first witness: the source file of its first witness
/// edge, falling back to its first named file.
fn cycle_witness_path<'report>(
    report: &'report Report,
    finding: &ArchitectureFinding,
) -> Option<&'report str> {
    finding
        .witness_edges()
        .first()
        .and_then(|id| report.dependency_edges().get(id.index()))
        .map(|edge| edge.source())
        .or_else(|| finding.files().first().copied())
        .map(|file| report.files()[file.index()].path())
}

/// The path of a stable-dependency finding's first witness, falling back to
/// the depending package's path.
fn stable_witness_path<'report>(
    report: &'report Report,
    finding: &StableDependencyFinding,
) -> Option<&'report str> {
    finding
        .witness_edges()
        .first()
        .and_then(|id| report.dependency_edges().get(id.index()))
        .map(|edge| report.files()[edge.source().index()].path())
        .or_else(|| {
            report
                .packages()
                .get(finding.source().index())
                .map(|package| package.path())
        })
}

/// One key whose counts differ between the baseline and the observed tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateDelta {
    path: String,
    signal: GateSignal,
    baseline_high: u32,
    high: u32,
    baseline_watch: u32,
    watch: u32,
}

impl GateDelta {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub const fn signal(&self) -> GateSignal {
        self.signal
    }

    pub const fn baseline_high(&self) -> u32 {
        self.baseline_high
    }

    pub const fn high(&self) -> u32 {
        self.high
    }

    pub const fn baseline_watch(&self) -> u32 {
        self.baseline_watch
    }

    pub const fn watch(&self) -> u32 {
        self.watch
    }
}

/// The exact comparison of an observed snapshot against a committed baseline.
///
/// An absent key counts as zero on either side, so new debt in a file the
/// baseline never named is a regression and a deleted file reads as an
/// improvement to zero. A key whose High or Watch count rises is a
/// regression even when its other counter falls; a key is an improvement
/// only when nothing rose and something fell. Equal keys are unchanged and
/// never reported.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GateComparison {
    regressions: Vec<GateDelta>,
    improvements: Vec<GateDelta>,
}

impl GateComparison {
    pub fn between(baseline: &GateSnapshot, observed: &GateSnapshot) -> Self {
        let mut comparison = Self::default();
        let mut baseline_rows = baseline.rows().iter().peekable();
        let mut observed_rows = observed.rows().iter().peekable();
        loop {
            let ordering = match (baseline_rows.peek(), observed_rows.peek()) {
                (None, None) => break,
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(before), Some(after)) => {
                    (before.path(), before.signal()).cmp(&(after.path(), after.signal()))
                }
            };
            let (before, after) = match ordering {
                std::cmp::Ordering::Less => (baseline_rows.next(), None),
                std::cmp::Ordering::Greater => (None, observed_rows.next()),
                std::cmp::Ordering::Equal => (baseline_rows.next(), observed_rows.next()),
            };
            comparison.record(before, after);
        }
        comparison
    }

    fn record(&mut self, before: Option<&GateRow>, after: Option<&GateRow>) {
        let key = before.or(after).expect("one side of a gate key");
        let (baseline_high, baseline_watch) =
            before.map_or((0, 0), |row| (row.high(), row.watch()));
        let (high, watch) = after.map_or((0, 0), |row| (row.high(), row.watch()));
        if high == baseline_high && watch == baseline_watch {
            return;
        }
        let delta = GateDelta {
            path: key.path().to_owned(),
            signal: key.signal(),
            baseline_high,
            high,
            baseline_watch,
            watch,
        };
        if high > baseline_high || watch > baseline_watch {
            self.regressions.push(delta);
        } else {
            self.improvements.push(delta);
        }
    }

    pub fn regressions(&self) -> &[GateDelta] {
        &self.regressions
    }

    pub fn improvements(&self) -> &[GateDelta] {
        &self.improvements
    }

    /// Whether any counter exceeded the baseline, which fails the gate.
    pub fn regressed(&self) -> bool {
        !self.regressions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture::{
        ArchitectureFindingKind, ArchitectureGraph, ArchitectureReportFacts, DependencyCoverage,
        DependencyEdge, DependencyEdgeId, PackageGraphMeasurement, StableDependencyEvidence,
        StableDependencyFindingId,
    };
    use crate::report::{
        Coverage, FileId, FileRecord, Finding, FindingId, PackageId, PackageRecord, ReportBuilder,
        ReportMode, Scope, ScopeId, ScopeKind,
    };
    use crate::size::SizePolicy;
    use crate::source::{ParseStatus, SourceRole, SourceSpan, SourceTrust, UnitIdentity, UnitKind};
    use crate::{ArchitectureFindingId, HealthCounts, HealthPolicy, Measurements};

    fn snapshot(rows: &[(&str, GateSignal, u32, u32)]) -> GateSnapshot {
        GateSnapshot::new(
            rows.iter()
                .map(|(path, signal, high, watch)| GateRow::new(*path, *signal, *high, *watch))
                .collect(),
        )
    }

    #[test]
    fn signal_identifiers_are_frozen_and_declared_in_baseline_order() {
        let ids: Vec<&str> = GateSignal::ALL.iter().map(|signal| signal.id()).collect();
        assert_eq!(
            ids,
            [
                "cognitive",
                "container_size",
                "cyclomatic",
                "file_cycle",
                "file_size",
                "logical_lines",
                "nesting",
                "package_cycle",
                "parameters",
                "stable_dependency",
            ]
        );
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "variant order must match identifier order");
        for signal in GateSignal::ALL {
            assert_eq!(GateSignal::from_id(signal.id()), Some(signal));
        }
        assert_eq!(GateSignal::from_id("coupling"), None);
        assert_eq!(GateSignal::from_id("concentration"), None);
    }

    #[test]
    fn a_new_key_regresses_against_an_absent_baseline_key() {
        let baseline = snapshot(&[]);
        let observed = snapshot(&[("src/new.rs", GateSignal::Cognitive, 1, 0)]);
        let comparison = GateComparison::between(&baseline, &observed);
        assert!(comparison.regressed());
        assert_eq!(comparison.regressions().len(), 1);
        assert!(comparison.improvements().is_empty());
        let delta = &comparison.regressions()[0];
        assert_eq!(delta.path(), "src/new.rs");
        assert_eq!((delta.baseline_high(), delta.high()), (0, 1));
        assert_eq!((delta.baseline_watch(), delta.watch()), (0, 0));
    }

    #[test]
    fn a_deleted_file_reads_as_an_improvement_to_zero() {
        let baseline = snapshot(&[("src/gone.rs", GateSignal::Nesting, 2, 1)]);
        let observed = snapshot(&[]);
        let comparison = GateComparison::between(&baseline, &observed);
        assert!(!comparison.regressed());
        assert_eq!(comparison.improvements().len(), 1);
        let delta = &comparison.improvements()[0];
        assert_eq!(delta.path(), "src/gone.rs");
        assert_eq!((delta.baseline_high(), delta.high()), (2, 0));
        assert_eq!((delta.baseline_watch(), delta.watch()), (1, 0));
    }

    #[test]
    fn equal_keys_are_unchanged_and_unreported() {
        let rows = [("src/same.rs", GateSignal::Cyclomatic, 3, 5)];
        let comparison = GateComparison::between(&snapshot(&rows), &snapshot(&rows));
        assert!(!comparison.regressed());
        assert!(comparison.regressions().is_empty());
        assert!(comparison.improvements().is_empty());
    }

    #[test]
    fn a_lower_counter_is_an_improvement_that_is_never_applied_for_the_caller() {
        let baseline = snapshot(&[("src/better.rs", GateSignal::Cognitive, 3, 5)]);
        let observed = snapshot(&[("src/better.rs", GateSignal::Cognitive, 2, 5)]);
        let comparison = GateComparison::between(&baseline, &observed);
        assert!(!comparison.regressed());
        assert_eq!(comparison.improvements().len(), 1);
        assert_eq!(comparison.improvements()[0].high(), 2);
    }

    #[test]
    fn one_rising_counter_regresses_even_when_the_other_falls() {
        let baseline = snapshot(&[("src/mixed.rs", GateSignal::Parameters, 3, 5)]);
        let observed = snapshot(&[("src/mixed.rs", GateSignal::Parameters, 2, 6)]);
        let comparison = GateComparison::between(&baseline, &observed);
        assert!(comparison.regressed());
        assert!(comparison.improvements().is_empty());
    }

    /// Builds a two-file report carrying one verdict unit finding, one
    /// test-role unit finding, both size subjects, both cycle kinds, and one
    /// stable-dependency finding.
    fn gate_report() -> Report {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let mut packages = Vec::new();
        for (index, path) in ["src/a.rs", "src/b.rs"].iter().enumerate() {
            let scope = ScopeId::from_index(index + 1);
            builder.add_scope(Scope::new(scope, ScopeKind::File, *path, Some(root)));
            let file = FileId::from_index(index);
            let package = PackageId::from_index(index);
            packages.push(PackageRecord::current(package, scope, *path));
            builder.add_file(
                FileRecord::new(
                    file,
                    scope,
                    *path,
                    Coverage::new(1, 1, 0, 0, 10, 0),
                    HealthCounts::new(0, 0, 1),
                )
                .with_package(package)
                .with_source_state(SourceRole::Primary, ParseStatus::Parsed),
            );
            builder.link_file(scope, file);
        }
        builder.set_packages(packages);
        // High cognitive with Watch nesting on one unit: two independent keys.
        let measurements = Measurements::new(30, 1, 1).with_shape(5, 0);
        builder.add_finding(
            Finding::new(
                FindingId::from_index(0),
                FileId::from_index(0),
                UnitIdentity::new("work", UnitKind::Function),
                SourceSpan::new(1, 4),
                measurements,
                HealthPolicy::default().assess(measurements),
            )
            .with_evidence(SourceRole::Primary, SourceTrust::Trusted),
        );
        // A fixture-role unit never affects the verdict, so it never counts.
        builder.add_finding(
            Finding::new(
                FindingId::from_index(1),
                FileId::from_index(1),
                UnitIdentity::new("fixture", UnitKind::Function),
                SourceSpan::new(1, 4),
                measurements,
                HealthPolicy::default().assess(measurements),
            )
            .with_evidence(SourceRole::Fixture, SourceTrust::Trusted),
        );
        let policy = SizePolicy::default();
        builder.set_size_findings(vec![
            policy
                .rate_file(FileId::from_index(0), 900)
                .expect("high file size"),
            policy
                .rate_container(FileId::from_index(1), "Worker", 320)
                .expect("watch container size"),
        ]);
        let edges = vec![
            DependencyEdge::new(
                DependencyEdgeId::from_index(0),
                FileId::from_index(0),
                FileId::from_index(1),
                2,
                Vec::new(),
            ),
            DependencyEdge::new(
                DependencyEdgeId::from_index(1),
                FileId::from_index(1),
                FileId::from_index(0),
                1,
                Vec::new(),
            ),
        ];
        let findings = vec![
            crate::ArchitectureFinding::new(
                ArchitectureFindingId::from_index(0),
                ArchitectureFindingKind::PackageCycle,
                vec![PackageId::from_index(0), PackageId::from_index(1)],
                Vec::new(),
                vec![DependencyEdgeId::from_index(0)],
            ),
            crate::ArchitectureFinding::new(
                ArchitectureFindingId::from_index(1),
                ArchitectureFindingKind::FileCycle,
                Vec::new(),
                vec![FileId::from_index(1), FileId::from_index(0)],
                vec![DependencyEdgeId::from_index(1)],
            ),
        ];
        builder.set_architecture(ArchitectureReportFacts::new(
            ArchitectureGraph::new(
                DependencyCoverage::default(),
                edges,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            findings,
            Vec::new(),
        ));
        builder.set_stable_dependency_findings(vec![StableDependencyFinding::new(
            StableDependencyFindingId::from_index(0),
            PackageId::from_index(0),
            PackageId::from_index(1),
            StableDependencyEvidence::new(
                PackageGraphMeasurement::new(PackageId::from_index(0), 3, 1),
                PackageGraphMeasurement::new(PackageId::from_index(1), 1, 2),
                2,
            ),
            vec![DependencyEdgeId::from_index(0)],
        )]);
        builder.finish()
    }

    #[test]
    fn a_snapshot_counts_each_signal_at_its_own_rating_for_verdict_findings_only() {
        let report = gate_report();
        let snapshot = GateSnapshot::from_report(&report);
        let rows: Vec<(&str, GateSignal, u32, u32)> = snapshot
            .rows()
            .iter()
            .map(|row| (row.path(), row.signal(), row.high(), row.watch()))
            .collect();
        assert_eq!(
            rows,
            [
                ("src/a.rs", GateSignal::Cognitive, 1, 0),
                ("src/a.rs", GateSignal::FileSize, 1, 0),
                ("src/a.rs", GateSignal::Nesting, 0, 1),
                ("src/a.rs", GateSignal::PackageCycle, 1, 0),
                ("src/a.rs", GateSignal::StableDependency, 0, 1),
                ("src/b.rs", GateSignal::ContainerSize, 0, 1),
                ("src/b.rs", GateSignal::FileCycle, 0, 1),
            ]
        );
    }

    #[test]
    fn an_unchanged_snapshot_compares_clean_against_itself() {
        let report = gate_report();
        let observed = GateSnapshot::from_report(&report);
        let comparison = GateComparison::between(&observed.clone(), &observed);
        assert!(!comparison.regressed());
        assert!(comparison.improvements().is_empty());
    }
}
