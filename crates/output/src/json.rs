use std::io::{self, Write};

use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use smackdebt_analysis::{
    ArchitectureComparison, ArchitectureFinding, Comparison, DependencyEdge, Diagnostic,
    EvolutionaryComparison, EvolutionaryFinding, ExternalDependency, FileHistory, FileRecord,
    Finding, HealthCounts, Measurements, PackageEdge, PackageGraphMeasurement, PackageHistory,
    Report, ResolutionDiagnostic, Scope, UnitKind,
};

use crate::output::{
    comparison_name, diagnostic_name, direction_name, language_name, mode_name, rating_name,
    scope_kind,
};

/// Streams JSON schema version 2 without cloning report strings or arrays.
pub fn write_json(
    writer: &mut impl Write,
    report: &Report,
    selected_scope: Option<smackdebt_analysis::ScopeId>,
) -> io::Result<()> {
    let mut serializer = serde_json::Serializer::new(writer);
    ReportView(report, selected_scope)
        .serialize(&mut serializer)
        .map_err(io::Error::other)
}

struct ReportView<'a>(&'a Report, Option<smackdebt_analysis::ScopeId>);

impl Serialize for ReportView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let report = self.0;
        let mut map = serializer.serialize_map(Some(27))?;
        map.serialize_entry("schema_version", &report.schema_version())?;
        map.serialize_entry("mode", mode_name(report.mode()))?;
        map.serialize_entry("root", &report.root().map(|root| root.get()))?;
        map.serialize_entry("selected_scope", &self.1.map(|id| id.get()))?;
        map.serialize_entry("paths", &report.paths())?;
        map.serialize_entry("scopes", &Scopes(report.scopes()))?;
        map.serialize_entry("files", &Files(report.files(), report.scopes().len()))?;
        map.serialize_entry("findings", &Findings(report.findings()))?;
        map.serialize_entry("diagnostics", &Diagnostics(report.diagnostics()))?;
        map.serialize_entry("comparisons", &Comparisons(report.comparisons()))?;
        map.serialize_entry("health", &HealthRecords(report))?;
        map.serialize_entry("activity", &ActivityRecords(report.files()))?;
        map.serialize_entry(
            "dependency_coverage",
            &DependencyCoverageView(report.dependency_coverage()),
        )?;
        map.serialize_entry(
            "dependency_edges",
            &DependencyEdges(report.dependency_edges()),
        )?;
        map.serialize_entry("package_edges", &PackageEdges(report.package_edges()))?;
        map.serialize_entry(
            "external_dependencies",
            &ExternalDependencies(report.external_dependencies()),
        )?;
        map.serialize_entry(
            "resolution_diagnostics",
            &ResolutionDiagnostics(report.resolution_diagnostics()),
        )?;
        map.serialize_entry("package_graph", &PackageGraph(report.package_graph()))?;
        map.serialize_entry(
            "architecture_findings",
            &ArchitectureFindings(report.architecture_findings()),
        )?;
        map.serialize_entry(
            "architecture_comparisons",
            &ArchitectureComparisons(report.architecture_comparisons()),
        )?;
        map.serialize_entry(
            "history_coverage",
            &HistoryCoverageView(report.history_coverage()),
        )?;
        map.serialize_entry("file_history", &FileHistoryRecords(report.file_history()))?;
        map.serialize_entry(
            "package_history",
            &PackageHistoryRecords(report.package_history()),
        )?;
        map.serialize_entry(
            "change_coupling",
            &ChangeCouplingRecords(report.change_coupling()),
        )?;
        map.serialize_entry(
            "contributor_concentration",
            &ConcentrationRecords(report.contributor_concentration()),
        )?;
        map.serialize_entry(
            "evolutionary_findings",
            &EvolutionaryFindings(report.evolutionary_findings()),
        )?;
        map.serialize_entry(
            "evolutionary_comparisons",
            &EvolutionaryComparisons(report.evolutionary_comparisons()),
        )?;
        map.end()
    }
}

struct Scopes<'a>(&'a [Scope]);
impl Serialize for Scopes<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for scope in self.0 {
            sequence.serialize_element(&ScopeView(scope))?;
        }
        sequence.end()
    }
}

struct ScopeView<'a>(&'a Scope);
impl Serialize for ScopeView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let scope = self.0;
        let mut map = serializer.serialize_map(Some(14))?;
        map.serialize_entry("id", &scope.id().get())?;
        map.serialize_entry("kind", scope_kind(scope.kind()))?;
        map.serialize_entry("parent", &scope.parent().map(|id| id.get()))?;
        map.serialize_entry("children", &ChildIds(scope.children()))?;
        map.serialize_entry("path", &scope.path().map(|id| id.get()))?;
        map.serialize_entry("findings", &FindingIds(scope.findings()))?;
        map.serialize_entry("comparisons", &ComparisonIds(scope.comparisons()))?;
        map.serialize_entry(
            "architecture_findings",
            &ArchitectureFindingIds(scope.architecture_findings()),
        )?;
        map.serialize_entry(
            "architecture_comparisons",
            &ArchitectureComparisonIds(scope.architecture_comparisons()),
        )?;
        map.serialize_entry(
            "evolutionary_findings",
            &EvolutionaryFindingIds(scope.evolutionary_findings()),
        )?;
        map.serialize_entry(
            "evolutionary_comparisons",
            &EvolutionaryComparisonIds(scope.evolutionary_comparisons()),
        )?;
        map.serialize_entry("coverage", &CoverageView(scope.coverage()))?;
        map.serialize_entry("health", &scope.id().get())?;
        map.serialize_entry("diff", &DiffView(scope.diff()))?;
        map.end()
    }
}
struct EvolutionaryFindingIds<'a>(&'a [smackdebt_analysis::EvolutionaryFindingId]);
impl Serialize for EvolutionaryFindingIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}
struct EvolutionaryComparisonIds<'a>(&'a [smackdebt_analysis::EvolutionaryComparisonId]);
impl Serialize for EvolutionaryComparisonIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct ArchitectureFindingIds<'a>(&'a [smackdebt_analysis::ArchitectureFindingId]);
impl Serialize for ArchitectureFindingIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}
struct ArchitectureComparisonIds<'a>(&'a [smackdebt_analysis::ArchitectureComparisonId]);
impl Serialize for ArchitectureComparisonIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct FindingIds<'a>(&'a [smackdebt_analysis::FindingId]);
impl Serialize for FindingIds<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct ComparisonIds<'a>(&'a [smackdebt_analysis::ComparisonId]);
impl Serialize for ComparisonIds<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct DiffView(smackdebt_analysis::DiffCounts);
impl Serialize for DiffView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("worse", &self.0.worse())?;
        map.serialize_entry("better", &self.0.better())?;
        map.serialize_entry("changed", &self.0.changed())?;
        map.serialize_entry("total", &self.0.total())?;
        map.end()
    }
}

struct ChildIds<'a>(&'a [smackdebt_analysis::ScopeId]);

impl Serialize for ChildIds<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for child in self.0 {
            sequence.serialize_element(&child.get())?;
        }
        sequence.end()
    }
}

struct Files<'a>(&'a [FileRecord], usize);
impl Serialize for Files<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for file in self.0 {
            sequence.serialize_element(&FileView(file, self.1))?;
        }
        sequence.end()
    }
}

struct FileView<'a>(&'a FileRecord, usize);
impl Serialize for FileView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let file = self.0;
        let mut map = serializer.serialize_map(Some(8))?;
        map.serialize_entry("id", &file.id().get())?;
        map.serialize_entry("scope", &file.scope().get())?;
        map.serialize_entry("path", &file.path_id().map(|id| id.get()))?;
        map.serialize_entry("language", &file.language().map(language_name))?;
        map.serialize_entry("coverage", &CoverageView(file.coverage()))?;
        map.serialize_entry("health", &(self.1 as u32 + file.id().get()))?;
        map.serialize_entry("activity", &file.id().get())?;
        map.serialize_entry("package", &file.package().map(|id| id.get()))?;
        map.end()
    }
}

struct DependencyCoverageView(smackdebt_analysis::DependencyCoverage);
impl Serialize for DependencyCoverageView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("internal", &self.0.internal())?;
        map.serialize_entry("external", &self.0.external())?;
        map.serialize_entry("unresolved", &self.0.unresolved())?;
        map.serialize_entry("ambiguous", &self.0.ambiguous())?;
        map.serialize_entry("total", &self.0.total())?;
        map.end()
    }
}

struct DependencyEdges<'a>(&'a [DependencyEdge]);
impl Serialize for DependencyEdges<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for edge in self.0 {
            sequence.serialize_element(&DependencyEdgeView(edge))?;
        }
        sequence.end()
    }
}
struct DependencyEdgeView<'a>(&'a DependencyEdge);
impl Serialize for DependencyEdgeView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("source", &self.0.source().get())?;
        map.serialize_entry("target", &self.0.target().get())?;
        map.serialize_entry("references", &self.0.references())?;
        map.serialize_entry("locations", &Locations(self.0.locations()))?;
        map.end()
    }
}
struct Locations<'a>(&'a [smackdebt_analysis::SourceSpan]);
impl Serialize for Locations<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for span in self.0 {
            sequence.serialize_element(&[span.start_line(), span.end_line()])?;
        }
        sequence.end()
    }
}

struct PackageEdges<'a>(&'a [PackageEdge]);
impl Serialize for PackageEdges<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for edge in self.0 {
            sequence.serialize_element(&PackageEdgeView(edge))?;
        }
        sequence.end()
    }
}
struct PackageEdgeView<'a>(&'a PackageEdge);
impl Serialize for PackageEdgeView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("source", &self.0.source().get())?;
        map.serialize_entry("target", &self.0.target().get())?;
        map.serialize_entry("file_pairs", &self.0.file_pairs())?;
        map.serialize_entry("references", &self.0.references())?;
        map.serialize_entry("file_edges", &DependencyEdgeIds(self.0.file_edges()))?;
        map.end()
    }
}
struct DependencyEdgeIds<'a>(&'a [smackdebt_analysis::DependencyEdgeId]);
impl Serialize for DependencyEdgeIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct ExternalDependencies<'a>(&'a [ExternalDependency]);
impl Serialize for ExternalDependencies<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&ExternalDependencyView(value))?;
        }
        sequence.end()
    }
}
struct ExternalDependencyView<'a>(&'a ExternalDependency);
impl Serialize for ExternalDependencyView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("file", &self.0.file().get())?;
        map.serialize_entry("target", self.0.target())?;
        map.serialize_entry("references", &self.0.references())?;
        map.end()
    }
}
struct ResolutionDiagnostics<'a>(&'a [ResolutionDiagnostic]);
impl Serialize for ResolutionDiagnostics<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&ResolutionDiagnosticView(value))?;
        }
        sequence.end()
    }
}
struct ResolutionDiagnosticView<'a>(&'a ResolutionDiagnostic);
impl Serialize for ResolutionDiagnosticView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let kind = match self.0.kind() {
            smackdebt_analysis::ResolutionIssueKind::Unresolved => "unresolved",
            smackdebt_analysis::ResolutionIssueKind::Ambiguous => "ambiguous",
        };
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("file", &self.0.file().get())?;
        map.serialize_entry("start_line", &self.0.span().start_line())?;
        map.serialize_entry("end_line", &self.0.span().end_line())?;
        map.serialize_entry("target", self.0.target())?;
        map.serialize_entry("kind", kind)?;
        map.serialize_entry("reason", self.0.reason())?;
        map.end()
    }
}
struct PackageGraph<'a>(&'a [PackageGraphMeasurement]);
impl Serialize for PackageGraph<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&PackageGraphView(*value))?;
        }
        sequence.end()
    }
}
struct PackageGraphView(PackageGraphMeasurement);
impl Serialize for PackageGraphView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("package", &self.0.package().get())?;
        map.serialize_entry("fan_in", &self.0.fan_in())?;
        map.serialize_entry("fan_out", &self.0.fan_out())?;
        let instability = self
            .0
            .instability()
            .map(|value| (value.numerator(), value.denominator()));
        map.serialize_entry("instability", &instability)?;
        map.end()
    }
}
struct ArchitectureFindings<'a>(&'a [ArchitectureFinding]);

struct HistoryCoverageView<'a>(&'a smackdebt_analysis::HistoryCoverage);
impl Serialize for HistoryCoverageView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = self.0;
        let mut map = serializer.serialize_map(Some(10))?;
        let availability = match value.availability() {
            smackdebt_analysis::HistoryAvailability::Complete => "complete",
            smackdebt_analysis::HistoryAvailability::Incomplete => "incomplete",
            smackdebt_analysis::HistoryAvailability::Unavailable => "unavailable",
        };
        map.serialize_entry("availability", availability)?;
        map.serialize_entry("revision", &value.revision())?;
        map.serialize_entry("commits", &value.commits())?;
        map.serialize_entry("newest_timestamp", &value.newest_timestamp())?;
        map.serialize_entry("oldest_timestamp", &value.oldest_timestamp())?;
        map.serialize_entry("textual_changes", &value.textual_changes())?;
        map.serialize_entry("uncounted_changes", &value.uncounted_changes())?;
        map.serialize_entry("excluded_paths", &value.excluded_paths())?;
        map.serialize_entry("rename_gaps", &value.rename_gaps())?;
        map.serialize_entry("reason", &value.reason())?;
        map.end()
    }
}

struct FileHistoryRecords<'a>(&'a [FileHistory]);
impl Serialize for FileHistoryRecords<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&FileHistoryView(*value))?;
        }
        sequence.end()
    }
}
struct FileHistoryView(FileHistory);
impl Serialize for FileHistoryView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("file", &self.0.file().get())?;
        map.serialize_entry("touches", &self.0.touches())?;
        map.serialize_entry("added_lines", &self.0.added_lines())?;
        map.serialize_entry("deleted_lines", &self.0.deleted_lines())?;
        map.serialize_entry("uncounted_changes", &self.0.uncounted_changes())?;
        map.end()
    }
}
struct PackageHistoryRecords<'a>(&'a [PackageHistory]);
impl Serialize for PackageHistoryRecords<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&PackageHistoryView(*value))?;
        }
        sequence.end()
    }
}
struct PackageHistoryView(PackageHistory);
impl Serialize for PackageHistoryView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("package", &self.0.package().get())?;
        map.serialize_entry("touches", &self.0.touches())?;
        map.serialize_entry("added_lines", &self.0.added_lines())?;
        map.serialize_entry("deleted_lines", &self.0.deleted_lines())?;
        map.serialize_entry("uncounted_changes", &self.0.uncounted_changes())?;
        map.end()
    }
}
struct ChangeCouplingRecords<'a>(&'a [smackdebt_analysis::ChangeCoupling]);
impl Serialize for ChangeCouplingRecords<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&ChangeCouplingView(*value))?;
        }
        sequence.end()
    }
}
struct ChangeCouplingView(smackdebt_analysis::ChangeCoupling);
impl Serialize for ChangeCouplingView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("left", &self.0.left().get())?;
        map.serialize_entry("right", &self.0.right().get())?;
        map.serialize_entry("shared_commits", &self.0.shared_commits())?;
        map.serialize_entry("union_commits", &self.0.union_commits())?;
        map.serialize_entry("similarity", &self.0.similarity())?;
        map.end()
    }
}
struct ConcentrationRecords<'a>(&'a [smackdebt_analysis::ContributorConcentration]);
impl Serialize for ConcentrationRecords<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&ConcentrationView(*value))?;
        }
        sequence.end()
    }
}
struct ConcentrationView(smackdebt_analysis::ContributorConcentration);
impl Serialize for ConcentrationView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("package", &self.0.package().get())?;
        map.serialize_entry("contributor_count", &self.0.contributor_count())?;
        map.serialize_entry("numerator", &self.0.numerator())?;
        map.serialize_entry("denominator", &self.0.denominator())?;
        map.serialize_entry("ratio", &self.0.ratio())?;
        map.end()
    }
}
struct EvolutionaryFindings<'a>(&'a [EvolutionaryFinding]);
impl Serialize for EvolutionaryFindings<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&EvolutionaryFindingView(*value))?;
        }
        sequence.end()
    }
}
struct EvolutionaryFindingView(EvolutionaryFinding);
impl Serialize for EvolutionaryFindingView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let pair = self.0.coupling();
        let mut map = serializer.serialize_map(Some(7))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("rating", "watch")?;
        map.serialize_entry("left", &pair.left().get())?;
        map.serialize_entry("right", &pair.right().get())?;
        map.serialize_entry("shared_commits", &pair.shared_commits())?;
        map.serialize_entry("union_commits", &pair.union_commits())?;
        map.serialize_entry("similarity", &pair.similarity())?;
        map.end()
    }
}
struct EvolutionaryComparisons<'a>(&'a [EvolutionaryComparison]);
impl Serialize for EvolutionaryComparisons<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&EvolutionaryComparisonView(*value))?;
        }
        sequence.end()
    }
}
struct EvolutionaryComparisonView(EvolutionaryComparison);
impl Serialize for EvolutionaryComparisonView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let pair = self.0.coupling();
        let kind = match self.0.kind() {
            smackdebt_analysis::EvolutionaryComparisonKind::FindingIntroduced => {
                "finding_introduced"
            }
            smackdebt_analysis::EvolutionaryComparisonKind::FindingRemoved => "finding_removed",
        };
        let mut map = serializer.serialize_map(Some(8))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("kind", kind)?;
        map.serialize_entry("direction", direction_name(self.0.direction()))?;
        map.serialize_entry("left", &pair.left().get())?;
        map.serialize_entry("right", &pair.right().get())?;
        map.serialize_entry("shared_commits", &pair.shared_commits())?;
        map.serialize_entry("union_commits", &pair.union_commits())?;
        map.serialize_entry("similarity", &pair.similarity())?;
        map.end()
    }
}

impl Serialize for ArchitectureFindings<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&ArchitectureFindingView(value))?;
        }
        sequence.end()
    }
}
struct ArchitectureFindingView<'a>(&'a ArchitectureFinding);
impl Serialize for ArchitectureFindingView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("id", &self.0.id().get())?;
        let kind = match self.0.kind() {
            smackdebt_analysis::ArchitectureFindingKind::PackageCycle => "package_cycle",
            smackdebt_analysis::ArchitectureFindingKind::FileCycle => "file_cycle",
        };
        map.serialize_entry("kind", kind)?;
        map.serialize_entry("rating", rating_name(self.0.rating()))?;
        map.serialize_entry("packages", &PackageIds(self.0.packages()))?;
        map.serialize_entry("files", &FileIds(self.0.files()))?;
        map.serialize_entry("witness_edges", &DependencyEdgeIds(self.0.witness_edges()))?;
        map.end()
    }
}
struct ArchitectureComparisons<'a>(&'a [ArchitectureComparison]);
impl Serialize for ArchitectureComparisons<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            sequence.serialize_element(&ArchitectureComparisonView(value))?;
        }
        sequence.end()
    }
}
struct ArchitectureComparisonView<'a>(&'a ArchitectureComparison);
impl Serialize for ArchitectureComparisonView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("id", &self.0.id().get())?;
        let kind = match self.0.kind() {
            smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded => "edge_added",
            smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved => "edge_removed",
            smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced => "cycle_introduced",
            smackdebt_analysis::ArchitectureComparisonKind::CycleRemoved => "cycle_removed",
        };
        map.serialize_entry("kind", kind)?;
        map.serialize_entry("direction", direction_name(self.0.direction()))?;
        map.serialize_entry("packages", &PackageIds(self.0.packages()))?;
        map.serialize_entry("witness", &PackageIds(self.0.witness()))?;
        map.serialize_entry("files", &FileIds(self.0.files()))?;
        map.end()
    }
}
struct PackageIds<'a>(&'a [smackdebt_analysis::PackageId]);
impl Serialize for PackageIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}
struct FileIds<'a>(&'a [smackdebt_analysis::FileId]);
impl Serialize for FileIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct HealthRecords<'a>(&'a Report);

impl Serialize for HealthRecords<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let report = self.0;
        let mut sequence = serializer.serialize_seq(Some(
            report.scopes().len().saturating_add(report.files().len()),
        ))?;
        for scope in report.scopes() {
            sequence.serialize_element(&HealthRecord(scope.id().get(), scope.health()))?;
        }
        let offset = report.scopes().len() as u32;
        for file in report.files() {
            sequence.serialize_element(&HealthRecord(offset + file.id().get(), file.health()))?;
        }
        sequence.end()
    }
}

struct HealthRecord(u32, HealthCounts);

impl Serialize for HealthRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("id", &self.0)?;
        map.serialize_entry("healthy", &self.1.healthy())?;
        map.serialize_entry("watch", &self.1.watch())?;
        map.serialize_entry("high", &self.1.high())?;
        map.end()
    }
}

struct ActivityRecords<'a>(&'a [FileRecord]);

impl Serialize for ActivityRecords<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for file in self.0 {
            sequence.serialize_element(&ActivityRecord(file))?;
        }
        sequence.end()
    }
}

struct ActivityRecord<'a>(&'a FileRecord);

impl Serialize for ActivityRecord<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("file", &self.0.id().get())?;
        map.serialize_entry(
            "touches",
            &self.0.activity().map(|activity| activity.touches()),
        )?;
        map.end()
    }
}

struct Findings<'a>(&'a [Finding]);
impl Serialize for Findings<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for finding in self.0 {
            sequence.serialize_element(&FindingView(finding))?;
        }
        sequence.end()
    }
}

struct FindingView<'a>(&'a Finding);
impl Serialize for FindingView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let finding = self.0;
        let mut map = serializer.serialize_map(Some(9))?;
        map.serialize_entry("id", &finding.id().get())?;
        map.serialize_entry("file", &finding.file().get())?;
        map.serialize_entry("name", finding.identity().name())?;
        map.serialize_entry("container", &finding.identity().container())?;
        map.serialize_entry("kind", unit_kind_name(finding.identity().kind()))?;
        map.serialize_entry("start_line", &finding.span().start_line())?;
        map.serialize_entry("end_line", &finding.span().end_line())?;
        map.serialize_entry("rating", rating_name(finding.assessment().rating()))?;
        map.serialize_entry("measurements", &MeasurementsView(finding.measurements()))?;
        map.end()
    }
}

struct Diagnostics<'a>(&'a [Diagnostic]);
impl Serialize for Diagnostics<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for diagnostic in self.0 {
            sequence.serialize_element(&DiagnosticView(diagnostic))?;
        }
        sequence.end()
    }
}

struct DiagnosticView<'a>(&'a Diagnostic);
impl Serialize for DiagnosticView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let diagnostic = self.0;
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("id", &diagnostic.id().get())?;
        map.serialize_entry("file", &diagnostic.file().map(|id| id.get()))?;
        map.serialize_entry("kind", diagnostic_name(diagnostic.kind()))?;
        map.serialize_entry("message", diagnostic.message())?;
        map.serialize_entry("excluded_lines", &diagnostic.excluded_lines())?;
        map.end()
    }
}

struct Comparisons<'a>(&'a [Comparison]);
impl Serialize for Comparisons<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for comparison in self.0 {
            sequence.serialize_element(&ComparisonView(comparison))?;
        }
        sequence.end()
    }
}

struct ComparisonView<'a>(&'a Comparison);
impl Serialize for ComparisonView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let comparison = self.0;
        let mut map = serializer.serialize_map(Some(10))?;
        map.serialize_entry("id", &comparison.id().get())?;
        map.serialize_entry("file", &comparison.file().map(|id| id.get()))?;
        map.serialize_entry("name", comparison.identity().name())?;
        map.serialize_entry("container", &comparison.identity().container())?;
        map.serialize_entry("unit_kind", unit_kind_name(comparison.identity().kind()))?;
        map.serialize_entry("kind", comparison_name(comparison.kind()))?;
        map.serialize_entry("direction", direction_name(comparison.direction()))?;
        map.serialize_entry("before", &comparison.before().map(MeasurementsView))?;
        map.serialize_entry("after", &comparison.after().map(MeasurementsView))?;
        map.serialize_entry("ratings", &RatingsView(comparison))?;
        map.end()
    }
}

struct RatingsView<'a>(&'a Comparison);
impl Serialize for RatingsView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("before", &self.0.before_rating().map(rating_name))?;
        map.serialize_entry("after", &self.0.after_rating().map(rating_name))?;
        map.end()
    }
}

struct CoverageView(smackdebt_analysis::Coverage);
impl Serialize for CoverageView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("selected_files", &self.0.selected_files())?;
        map.serialize_entry("analyzed_files", &self.0.analyzed_files())?;
        map.serialize_entry("unsupported_files", &self.0.unsupported_files())?;
        map.serialize_entry("failed_files", &self.0.failed_files())?;
        map.serialize_entry("source_lines", &self.0.source_lines())?;
        map.serialize_entry("excluded_lines", &self.0.excluded_lines())?;
        map.end()
    }
}

fn unit_kind_name(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Function => "function",
        UnitKind::Method => "method",
        UnitKind::Closure => "closure",
        UnitKind::Lambda => "lambda",
        UnitKind::SyntheticTopLevel => "synthetic_top_level",
        UnitKind::Template => "template",
    }
}

struct MeasurementsView(Measurements);
impl Serialize for MeasurementsView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("cognitive_complexity", &self.0.cognitive_complexity())?;
        map.serialize_entry("cyclomatic_complexity", &self.0.cyclomatic_complexity())?;
        map.serialize_entry("logical_lines", &self.0.logical_lines())?;
        map.end()
    }
}
