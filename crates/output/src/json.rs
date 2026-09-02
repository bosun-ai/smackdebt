use std::io::{self, Write};

use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use smackdebt_analysis::{
    ArchitectureComparison, ArchitectureFinding, Comparison, ComparisonParticipation,
    DependencyEdge, Diagnostic, EvolutionaryComparison, EvolutionaryFinding, ExternalDependency,
    FileHistory, FileRecord, Finding, HealthCounts, Measurements, PackageEdge,
    PackageGraphMeasurement, PackageHistory, PackagePresence, PackageRecord, ParseStatus, Report,
    ResolutionDiagnostic, Scope, SourceRole, SourceTrust, UnitKind,
};

use crate::output::{
    comparison_name, diagnostic_name, direction_name, language_name, mode_name, rating_name,
    scope_kind,
};
use smackdebt_analysis::{
    ChangeLeakageFinding, ClaimedFinding, FileChangeCoupling, FileReach, Hotspot,
    KnowledgeConcentrationFinding, OrphanFile, PackageClosure, ProblemAnchor, ProblemCard,
    ProblemEvidence, SizeFinding, SizeSubject, StableDependencyFinding, Verdict, WorstOffender,
};

/// Streams JSON schema version 4 without cloning report strings or arrays.
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
        // The head answers the scope that was asked about, which is the same
        // verdict the terminal renders for the same invocation. Analysis owns
        // the policy; this only selects the scope.
        let verdict = match self.1 {
            Some(scope) => report.scope_verdict(scope),
            None => report
                .verdict()
                .cloned()
                .expect("a built report always carries a root verdict"),
        };
        let mut map = serializer.serialize_map(Some(44))?;
        map.serialize_entry("schema_version", &report.schema_version())?;
        map.serialize_entry("mode", mode_name(report.mode()))?;
        map.serialize_entry("comparison_ref", &report.comparison_ref())?;
        // The head is written before every table so `--json | head` answers the
        // question the tool exists to answer without one index lookup.
        map.serialize_entry("verdict", &VerdictView(&verdict, report.mode()))?;
        map.serialize_entry("summary", &SummaryView(&verdict))?;
        map.serialize_entry("root", &report.root().map(|root| root.get()))?;
        map.serialize_entry("selected_scope", &self.1.map(|id| id.get()))?;
        map.serialize_entry("paths", &report.paths())?;
        map.serialize_entry("packages", &Packages(report.packages()))?;
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
        serialize_graph_comparisons(&mut map, report)?;
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
        map.serialize_entry(
            "knowledge_concentration_findings",
            &KnowledgeConcentrationFindings(report.knowledge_concentration_findings()),
        )?;
        map.serialize_entry(
            "stable_dependency_findings",
            &StableDependencyFindings(report.stable_dependency_findings()),
        )?;
        map.serialize_entry("hotspots", &Hotspots(report.hotspots()))?;
        map.serialize_entry("size_findings", &SizeFindings(report.size_findings()))?;
        map.serialize_entry("orphan_files", &OrphanFiles(report.orphan_files()))?;
        map.serialize_entry(
            "file_change_coupling",
            &FileChangeCouplingRecords(report.file_change_coupling()),
        )?;
        map.serialize_entry(
            "change_leakage_findings",
            &ChangeLeakageFindings(report.change_leakage_findings()),
        )?;
        map.serialize_entry(
            "package_closures",
            &PackageClosures(report.package_closures()),
        )?;
        map.serialize_entry("file_reach", &FileReaches(report.file_reach()))?;
        let core_component =
            (!report.core_members().is_empty()).then_some(CoreComponentView(report.core_members()));
        map.serialize_entry("core_component", &core_component)?;
        map.serialize_entry("problems", &Problems(report.problems()))?;
        map.end()
    }
}

fn serialize_graph_comparisons<M: SerializeMap>(
    map: &mut M,
    report: &Report,
) -> Result<(), M::Error> {
    map.serialize_entry(
        "graph_evidence",
        &GraphEvidenceView(report.graph_evidence()),
    )?;
    if let Some(evidence) = report.diff_graph_evidence() {
        map.serialize_entry("diff_graph_evidence", &DiffGraphEvidenceView(evidence))?;
    }
    map.serialize_entry(
        "propagation_comparisons",
        &PropagationComparisons(report.propagation_comparisons()),
    )?;
    map.serialize_entry(
        "core_comparisons",
        &CoreComparisons(report.core_comparisons()),
    )?;
    map.serialize_entry(
        "change_leakage_comparisons",
        &ChangeLeakageComparisons(report.change_leakage_comparisons()),
    )
}

struct DiffGraphEvidenceView<'a>(&'a smackdebt_analysis::DiffGraphEvidence);
impl Serialize for DiffGraphEvidenceView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("current", &GraphEvidenceView(self.0.current()))?;
        map.serialize_entry("base", &GraphEvidenceView(self.0.base()))?;
        map.serialize_entry(
            "suppressed_propagation",
            &ComparisonSuppressionView(self.0.propagation()),
        )?;
        map.serialize_entry("suppressed_core", &ComparisonSuppressionView(self.0.core()))?;
        map.serialize_entry(
            "suppressed_leakage",
            &ComparisonSuppressionView(self.0.leakage()),
        )?;
        map.end()
    }
}

struct ComparisonSuppressionView(smackdebt_analysis::ComparisonSuppression);
impl Serialize for ComparisonSuppressionView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("total", &self.0.total())?;
        map.serialize_entry("current", &self.0.current())?;
        map.serialize_entry("base", &self.0.base())?;
        map.end()
    }
}

struct Packages<'a>(&'a [PackageRecord]);
impl Serialize for Packages<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for package in self.0 {
            sequence.serialize_element(&PackageView(package))?;
        }
        sequence.end()
    }
}

struct PackageView<'a>(&'a PackageRecord);
impl Serialize for PackageView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("scope", &self.0.scope().get())?;
        map.serialize_entry("path", self.0.path())?;
        map.serialize_entry(
            "presence",
            match self.0.presence() {
                PackagePresence::Current => "current",
                PackagePresence::BaseOnly => "base_only",
            },
        )?;
        // A package whose manifest declares no name carries no key rather than
        // an empty string a consumer could mistake for a declared name.
        if let Some(name) = self.0.manifest_name() {
            map.serialize_entry("manifest_name", name)?;
        }
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
        let mut map = serializer.serialize_map(Some(17))?;
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
            "propagation_comparisons",
            &PropagationComparisonIds(scope.propagation_comparisons()),
        )?;
        map.serialize_entry(
            "core_comparisons",
            &CoreComparisonIds(scope.core_comparisons()),
        )?;
        map.serialize_entry(
            "change_leakage_comparisons",
            &ChangeLeakageComparisonIds(scope.change_leakage_comparisons()),
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

struct PropagationComparisonIds<'a>(&'a [smackdebt_analysis::PropagationComparisonId]);
impl Serialize for PropagationComparisonIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct CoreComparisonIds<'a>(&'a [smackdebt_analysis::CoreComparisonId]);
impl Serialize for CoreComparisonIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct ChangeLeakageComparisonIds<'a>(&'a [smackdebt_analysis::ChangeLeakageComparisonId]);
impl Serialize for ChangeLeakageComparisonIds<'_> {
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
        let mut map = serializer.serialize_map(Some(11))?;
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
        let mut map = serializer.serialize_map(Some(11))?;
        map.serialize_entry("id", &file.id().get())?;
        map.serialize_entry("scope", &file.scope().get())?;
        map.serialize_entry("path", &file.path_id().map(|id| id.get()))?;
        map.serialize_entry("language", &file.language().map(language_name))?;
        map.serialize_entry("coverage", &CoverageView(file.coverage()))?;
        map.serialize_entry("health", &(self.1 as u32 + file.id().get()))?;
        map.serialize_entry("activity", &file.id().get())?;
        map.serialize_entry("package", &file.package().map(|id| id.get()))?;
        map.serialize_entry("role", source_role_name(file.role()))?;
        map.serialize_entry(
            "parse_outcome",
            &file.parse_status().map(parse_outcome_name),
        )?;
        map.serialize_entry("trust", source_trust_name(file.trust()))?;
        map.end()
    }
}

struct DependencyCoverageView(smackdebt_analysis::DependencyCoverage);
impl Serialize for DependencyCoverageView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(12))?;
        map.serialize_entry("internal", &self.0.internal())?;
        map.serialize_entry("external", &self.0.external())?;
        map.serialize_entry("unresolved", &self.0.unresolved())?;
        map.serialize_entry("ambiguous", &self.0.ambiguous())?;
        map.serialize_entry("resolved_internal_uses", &self.0.resolved_internal_uses())?;
        map.serialize_entry(
            "unresolved_internal_uses",
            &self.0.unresolved_internal_uses(),
        )?;
        map.serialize_entry("ambiguous_internal_uses", &self.0.ambiguous_internal_uses())?;
        map.serialize_entry("external_uses", &self.0.external_uses())?;
        map.serialize_entry("unresolved_package_uses", &self.0.unresolved_package_uses())?;
        map.serialize_entry(
            "module_ownership_relations",
            &self.0.module_ownership_relations(),
        )?;
        map.serialize_entry("context_relations", &self.0.context_relations())?;
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
        let mut map = serializer.serialize_map(Some(8))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("source", &self.0.source().get())?;
        map.serialize_entry("target", &self.0.target().get())?;
        map.serialize_entry("references", &self.0.references())?;
        map.serialize_entry("locations", &Locations(self.0.locations()))?;
        map.serialize_entry(
            "relation",
            match self.0.relation() {
                smackdebt_analysis::StaticRelationKind::Uses => "uses",
                smackdebt_analysis::StaticRelationKind::ModuleOwnership => "module_ownership",
            },
        )?;
        map.serialize_entry("role", source_role_name(self.0.role()))?;
        map.serialize_entry("trust", source_trust_name(self.0.trust()))?;
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
        let mut map = serializer.serialize_map(Some(7))?;
        map.serialize_entry("file", &self.0.file().get())?;
        map.serialize_entry("target", self.0.target())?;
        map.serialize_entry("references", &self.0.references())?;
        map.serialize_entry("locations", &Locations(self.0.locations()))?;
        map.serialize_entry("relation", relation_name(self.0.relation()))?;
        map.serialize_entry("role", source_role_name(self.0.role()))?;
        map.serialize_entry("trust", source_trust_name(self.0.trust()))?;
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
        let mut map = serializer.serialize_map(Some(11))?;
        map.serialize_entry("file", &self.0.file().get())?;
        map.serialize_entry("start_line", &self.0.span().start_line())?;
        map.serialize_entry("end_line", &self.0.span().end_line())?;
        map.serialize_entry("target", self.0.target())?;
        map.serialize_entry("kind", kind)?;
        map.serialize_entry("reason", self.0.reason())?;
        map.serialize_entry("references", &self.0.references())?;
        map.serialize_entry("locations", &Locations(self.0.locations()))?;
        map.serialize_entry("relation", relation_name(self.0.relation()))?;
        map.serialize_entry("role", source_role_name(self.0.role()))?;
        map.serialize_entry("trust", source_trust_name(self.0.trust()))?;
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
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("package", &self.0.package().get())?;
        map.serialize_entry("fan_in", &self.0.fan_in())?;
        map.serialize_entry("fan_out", &self.0.fan_out())?;
        map.serialize_entry("reach_in", &self.0.reach_in())?;
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
        map.serialize_entry("eligible_commits", &value.eligible_commits())?;
        map.serialize_entry("mapped_eligible_changes", &value.mapped_eligible_changes())?;
        map.serialize_entry("context_changes", &value.context_changes())?;
        map.serialize_entry("newest_timestamp", &value.newest_timestamp())?;
        map.serialize_entry("oldest_timestamp", &value.oldest_timestamp())?;
        map.serialize_entry("textual_changes", &value.textual_changes())?;
        map.serialize_entry("uncounted_changes", &value.uncounted_changes())?;
        map.serialize_entry("excluded_changes", &value.excluded_changes())?;
        map.serialize_entry("rename_gaps", &value.rename_gaps())?;
        map.serialize_entry("reason", &value.reason())?;
        map.serialize_entry("window_days", &value.window_days())?;
        map.serialize_entry("window_excluded_commits", &value.window_excluded_commits())?;
        map.serialize_entry("bulk_commits", &value.bulk_commits())?;
        map.serialize_entry("declined_pairs", &value.declined_pairs())?;
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
        let mut map = serializer.serialize_map(Some(7))?;
        map.serialize_entry("file", &self.0.file().get())?;
        map.serialize_entry("role", source_role_name(self.0.role()))?;
        map.serialize_entry("trust", source_trust_name(self.0.trust()))?;
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
        let mut map = serializer.serialize_map(Some(7))?;
        map.serialize_entry("package", &self.0.package().get())?;
        map.serialize_entry("role", source_role_name(self.0.role()))?;
        map.serialize_entry("trust", source_trust_name(self.0.trust()))?;
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
        let evidence = self
            .0
            .evidence()
            .expect("retained coupling observations carry source evidence");
        let mut map = serializer.serialize_map(Some(9))?;
        map.serialize_entry("left", &self.0.left().get())?;
        map.serialize_entry("right", &self.0.right().get())?;
        map.serialize_entry("left_role", source_role_name(evidence.left_role()))?;
        map.serialize_entry("left_trust", source_trust_name(evidence.left_trust()))?;
        map.serialize_entry("right_role", source_role_name(evidence.right_role()))?;
        map.serialize_entry("right_trust", source_trust_name(evidence.right_trust()))?;
        map.serialize_entry("shared_commits", &self.0.shared_commits())?;
        map.serialize_entry("union_commits", &self.0.union_commits())?;
        map.end()
    }
}
/// The file pairs that change together often enough to be retained.
///
/// A retained pair is a population a detector reads rather than a statement
/// about design, so it appears here and in no human view. Similarity is not
/// serialized, exactly as it is not for package change coupling: a reader
/// derives it from the two integer operands.
struct FileChangeCouplingRecords<'a>(&'a [FileChangeCoupling]);
impl Serialize for FileChangeCouplingRecords<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for pair in self.0 {
            sequence.serialize_element(&FileChangeCouplingView(*pair))?;
        }
        sequence.end()
    }
}
struct FileChangeCouplingView(FileChangeCoupling);
impl Serialize for FileChangeCouplingView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("left", &self.0.left().get())?;
        map.serialize_entry("right", &self.0.right().get())?;
        map.serialize_entry("shared_commits", &self.0.shared_commits())?;
        map.serialize_entry("union_commits", &self.0.union_commits())?;
        map.serialize_entry("distance", &self.0.distance())?;
        map.end()
    }
}
/// What the change-leakage join decided about those pairs, in the order the
/// rules define.
///
/// A row links the pair it was decided from rather than restating its
/// operands, and carries no reference count, relation kind, resolution
/// outcome, or edge identity: the finding is a statement about design, and the
/// relation tables remain the place a consumer reads relations.
struct ChangeLeakageFindings<'a>(&'a [ChangeLeakageFinding]);
impl Serialize for ChangeLeakageFindings<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for finding in self.0 {
            sequence.serialize_element(&ChangeLeakageFindingView(*finding))?;
        }
        sequence.end()
    }
}
struct ChangeLeakageFindingView(ChangeLeakageFinding);
impl Serialize for ChangeLeakageFindingView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("kind", self.0.kind().id())?;
        map.serialize_entry("coupling", &self.0.coupling().get())?;
        if let Some(interface) = self.0.interface() {
            map.serialize_entry("interface", &interface.get())?;
        }
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
        let mut map = serializer.serialize_map(Some(7))?;
        map.serialize_entry("package", &self.0.package().get())?;
        map.serialize_entry("role", source_role_name(self.0.role()))?;
        map.serialize_entry("trust", source_trust_name(self.0.trust()))?;
        map.serialize_entry("contributor_count", &self.0.contributor_count())?;
        map.serialize_entry("numerator", &self.0.numerator())?;
        map.serialize_entry("denominator", &self.0.denominator())?;
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
        map.serialize_entry("kind", "unexplained_coupling")?;
        map.serialize_entry("rating", rating_name(self.0.rating()))?;
        map.serialize_entry("left", &pair.left().get())?;
        map.serialize_entry("right", &pair.right().get())?;
        map.serialize_entry("shared_commits", &pair.shared_commits())?;
        map.serialize_entry("union_commits", &pair.union_commits())?;
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
        let mut map = serializer.serialize_map(Some(9))?;
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
        map.serialize_entry("relation", &self.0.relation().map(relation_name))?;
        map.serialize_entry("role", &self.0.role().map(source_role_name))?;
        map.serialize_entry("trust", &self.0.trust().map(source_trust_name))?;
        map.serialize_entry("before_references", &self.0.before_references())?;
        map.serialize_entry("after_references", &self.0.after_references())?;
        map.end()
    }
}

struct PropagationComparisons<'a>(&'a [smackdebt_analysis::PropagationComparison]);
impl Serialize for PropagationComparisons<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for comparison in self.0 {
            sequence.serialize_element(&PropagationComparisonView(*comparison))?;
        }
        sequence.end()
    }
}
struct PropagationComparisonView(smackdebt_analysis::PropagationComparison);
impl Serialize for PropagationComparisonView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(9))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("direction", direction_name(self.0.direction()))?;
        match self.0.subject() {
            smackdebt_analysis::PropagationSubject::Package { source } => {
                map.serialize_entry("subject", "package")?;
                map.serialize_entry("source", &source.get())?;
                map.serialize_entry("package", &Option::<u32>::None)?;
            }
            smackdebt_analysis::PropagationSubject::File { package, source } => {
                map.serialize_entry("subject", "file")?;
                map.serialize_entry("source", &source.get())?;
                map.serialize_entry("package", &Some(package.get()))?;
            }
        }
        let before = self.0.before();
        let after = self.0.after();
        map.serialize_entry("before_reached", &before.0)?;
        map.serialize_entry("before_total", &before.1)?;
        map.serialize_entry("after_reached", &after.0)?;
        map.serialize_entry("after_total", &after.1)?;
        map.end()
    }
}

struct CoreComparisons<'a>(&'a [smackdebt_analysis::CoreComparison]);
impl Serialize for CoreComparisons<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for comparison in self.0 {
            sequence.serialize_element(&CoreComparisonView(comparison))?;
        }
        sequence.end()
    }
}
struct CoreComparisonView<'a>(&'a smackdebt_analysis::CoreComparison);
impl Serialize for CoreComparisonView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(10))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("direction", direction_name(self.0.direction()))?;
        map.serialize_entry("anchor", &self.0.anchor().get())?;
        let before = self.0.before();
        let after = self.0.after();
        map.serialize_entry("before_core", &before.0)?;
        map.serialize_entry("before_files", &before.1)?;
        map.serialize_entry("after_core", &after.0)?;
        map.serialize_entry("after_files", &after.1)?;
        map.serialize_entry("before_members", &FileIds(self.0.before_members()))?;
        map.serialize_entry("after_members", &FileIds(self.0.after_members()))?;
        map.end()
    }
}

struct ChangeLeakageComparisons<'a>(&'a [smackdebt_analysis::ChangeLeakageComparison]);
impl Serialize for ChangeLeakageComparisons<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for comparison in self.0 {
            sequence.serialize_element(&ChangeLeakageComparisonView(*comparison))?;
        }
        sequence.end()
    }
}
struct ChangeLeakageComparisonView(smackdebt_analysis::ChangeLeakageComparison);
impl Serialize for ChangeLeakageComparisonView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("kind", self.0.kind().id())?;
        map.serialize_entry("direction", direction_name(self.0.direction()))?;
        map.serialize_entry("left", &self.0.left().get())?;
        map.serialize_entry("right", &self.0.right().get())?;
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
        let mut map = serializer.serialize_map(Some(11))?;
        map.serialize_entry("id", &finding.id().get())?;
        map.serialize_entry("file", &finding.file().get())?;
        map.serialize_entry("name", finding.identity().name())?;
        map.serialize_entry("container", &finding.identity().container())?;
        map.serialize_entry("kind", unit_kind_name(finding.identity().kind()))?;
        map.serialize_entry("start_line", &finding.span().start_line())?;
        map.serialize_entry("end_line", &finding.span().end_line())?;
        map.serialize_entry("rating", rating_name(finding.assessment().rating()))?;
        map.serialize_entry("measurements", &MeasurementsView(finding.measurements()))?;
        map.serialize_entry("role", source_role_name(finding.role()))?;
        map.serialize_entry("trust", source_trust_name(finding.trust()))?;
        map.end()
    }
}

fn source_role_name(role: SourceRole) -> &'static str {
    match role {
        SourceRole::Primary => "primary",
        SourceRole::Test => "test",
        SourceRole::Example => "example",
        SourceRole::Benchmark => "benchmark",
        SourceRole::Fixture => "fixture",
        SourceRole::Generated => "generated",
    }
}

fn parse_outcome_name(status: &ParseStatus) -> &'static str {
    match status {
        ParseStatus::Parsed => "parsed",
        ParseStatus::Recovered => "recovered",
        ParseStatus::Failed => "failed",
    }
}

fn relation_name(relation: smackdebt_analysis::StaticRelationKind) -> &'static str {
    match relation {
        smackdebt_analysis::StaticRelationKind::Uses => "uses",
        smackdebt_analysis::StaticRelationKind::ModuleOwnership => "module_ownership",
    }
}

fn source_trust_name(trust: SourceTrust) -> &'static str {
    match trust {
        SourceTrust::Trusted => "trusted",
        SourceTrust::Advisory => "advisory",
        SourceTrust::Failed => "failed",
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
        let span = comparison.span();
        let mut map = serializer.serialize_map(Some(13))?;
        map.serialize_entry("id", &comparison.id().get())?;
        map.serialize_entry("file", &comparison.file().map(|id| id.get()))?;
        map.serialize_entry("start_line", &span.map(|span| span.start_line()))?;
        map.serialize_entry("end_line", &span.map(|span| span.end_line()))?;
        map.serialize_entry("name", comparison.identity().name())?;
        map.serialize_entry("container", &comparison.identity().container())?;
        map.serialize_entry("unit_kind", unit_kind_name(comparison.identity().kind()))?;
        map.serialize_entry("kind", comparison_name(comparison.kind()))?;
        map.serialize_entry("direction", direction_name(comparison.direction()))?;
        map.serialize_entry(
            "participation",
            match comparison.participation() {
                ComparisonParticipation::Verdict => "verdict",
                ComparisonParticipation::Context => "context",
            },
        )?;
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
        let mut map = serializer.serialize_map(Some(11))?;
        map.serialize_entry("selected_files", &self.0.selected_files())?;
        map.serialize_entry("analyzed_files", &self.0.analyzed_files())?;
        map.serialize_entry("clean_files", &self.0.clean_files())?;
        map.serialize_entry("recovered_files", &self.0.recovered_files())?;
        map.serialize_entry("unsupported_files", &self.0.unsupported_files())?;
        map.serialize_entry("failed_files", &self.0.failed_files())?;
        map.serialize_entry("context_files", &self.0.context_files())?;
        map.serialize_entry("source_lines", &self.0.source_lines())?;
        map.serialize_entry("excluded_lines", &self.0.excluded_lines())?;
        map.serialize_entry("selected_bytes", &self.0.selected_bytes())?;
        map.serialize_entry("unsupported_bytes", &self.0.unsupported_bytes())?;
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
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("cognitive_complexity", &self.0.cognitive_complexity())?;
        map.serialize_entry("cyclomatic_complexity", &self.0.cyclomatic_complexity())?;
        map.serialize_entry("logical_lines", &self.0.logical_lines())?;
        map.serialize_entry("max_nesting", &self.0.max_nesting())?;
        map.serialize_entry("parameter_count", &self.0.parameter_count())?;
        map.end()
    }
}

/// The denormalized verdict head.
///
/// The tier id and its sentence are analysis-owned bytes, so a machine
/// consumer and the terminal state the same answer for the same report.
struct VerdictView<'a>(&'a Verdict, smackdebt_analysis::ReportMode);
impl Serialize for VerdictView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        let tier = match self.0.diff_tier() {
            Some(tier) => tier.id(),
            None => self.0.tier().id(),
        };
        map.serialize_entry("tier", tier)?;
        map.serialize_entry("sentence", self.0.sentence())?;
        if let Some(qualifier) = self.0.qualifier() {
            map.serialize_entry("qualifier", &QualifierView(qualifier))?;
        }
        if let Some(share) = self.0.share() {
            map.serialize_entry("share", &ShareView(share))?;
        }
        if let Some(reach) = self.0.reach() {
            map.serialize_entry("reach", &ReachView(reach))?;
        }
        if let Some(core) = self.0.core_size() {
            map.serialize_entry("core_size", &CoreSizeView(core))?;
        }
        if let Some(amplification) = self.0.amplification() {
            map.serialize_entry("amplification", &AmplificationView(amplification))?;
        }
        map.serialize_entry("mode", mode_name(self.1))?;
        map.end()
    }
}

/// The unsupported-coverage qualifier beside the tier and sentence.
///
/// The sentence bytes are analysis-owned, so a machine consumer and the
/// terminal state the same qualifier for the same report.
struct QualifierView<'a>(&'a smackdebt_analysis::CoverageQualifier);
impl Serialize for QualifierView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("sentence", self.0.sentence())?;
        map.serialize_entry("detail", &self.0.detail())?;
        map.serialize_entry("selected_files", &self.0.selected_files())?;
        map.serialize_entry("analyzed_files", &self.0.analyzed_files())?;
        map.serialize_entry("share_permille", &self.0.share_permille())?;
        if let Some(language) = self.0.largest_language() {
            map.serialize_entry("largest_language", language)?;
        }
        map.end()
    }
}

/// The repository frame a sub-scope verdict carries.
///
/// The sentence bytes are analysis-owned, so a machine consumer and the
/// terminal state the same fraction for the same scope.
struct ShareView(smackdebt_analysis::VerdictShare);
impl Serialize for ShareView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("sentence", &self.0.sentence())?;
        map.serialize_entry("high", &self.0.high())?;
        map.serialize_entry("repository_high", &self.0.repository_high())?;
        map.end()
    }
}

/// How far a change reaches from the selected scope.
///
/// The sentence bytes are analysis-owned, so a machine consumer and the
/// terminal state the same reach for the same scope. Both counts include the
/// changed node, which is what the sentence says.
struct ReachView(smackdebt_analysis::PropagationReach);
impl Serialize for ReachView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("sentence", &self.0.sentence())?;
        map.serialize_entry("reached", &self.0.reached())?;
        map.serialize_entry("total", &self.0.total())?;
        map.end()
    }
}

/// The largest file dependency cycle, against the graph it sits in.
struct CoreSizeView(smackdebt_analysis::CoreSize);
impl Serialize for CoreSizeView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("sentence", &self.0.sentence())?;
        map.serialize_entry("core", &self.0.core())?;
        map.serialize_entry("files", &self.0.files())?;
        map.end()
    }
}

/// What a typical change to the selected scope touches.
///
/// The sentence bytes are analysis-owned, so a machine consumer and the
/// terminal state the same median for the same scope, and the commit count is
/// the sample it was taken from.
struct AmplificationView(smackdebt_analysis::ChangeAmplification);
impl Serialize for AmplificationView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("sentence", &self.0.sentence())?;
        map.serialize_entry("median", &self.0.median())?;
        map.serialize_entry("commits", &self.0.commits())?;
        map.end()
    }
}

/// The denormalized summary head.
///
/// Every value here also exists in a table. The duplication is bounded, it is
/// produced from the same completed verdict, and it is what lets a consumer
/// answer the common question without a join.
struct SummaryView<'a>(&'a Verdict);
impl Serialize for SummaryView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let counts = self.0.counts();
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("checked", &counts.checked())?;
        map.serialize_entry("high", &counts.high())?;
        map.serialize_entry("watch", &counts.watch())?;
        map.serialize_entry("high_architecture", &counts.high_architecture_findings())?;
        map.serialize_entry("debt_diff", &DebtDiffView(self.0))?;
        map.serialize_entry("worst", &WorstOffenders(self.0.worst()))?;
        map.end()
    }
}

/// The reconciled debt-diff counts, each labeled by its word and printed even
/// when it is zero.
struct DebtDiffView<'a>(&'a Verdict);
impl Serialize for DebtDiffView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let total = self.0.facts().total();
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("worse", &total.worse())?;
        map.serialize_entry("better", &total.better())?;
        map.serialize_entry("changed", &total.changed())?;
        map.serialize_entry("total", &total.total())?;
        map.end()
    }
}

struct WorstOffenders<'a>(&'a [WorstOffender]);
impl Serialize for WorstOffenders<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for offender in self.0 {
            sequence.serialize_element(&WorstOffenderView(offender))?;
        }
        sequence.end()
    }
}
struct WorstOffenderView<'a>(&'a WorstOffender);
impl Serialize for WorstOffenderView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let identity = self.0.identity();
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("path", self.0.path())?;
        map.serialize_entry(
            "name",
            &identity.map(smackdebt_analysis::UnitIdentity::name),
        )?;
        map.serialize_entry(
            "container",
            &identity.and_then(smackdebt_analysis::UnitIdentity::container),
        )?;
        map.serialize_entry(
            "unit_kind",
            &identity.map(|identity| unit_kind_name(identity.kind())),
        )?;
        map.serialize_entry("reason", self.0.reason().id())?;
        map.end()
    }
}

/// The file reach of every package whose value is material.
///
/// A package below the file floor and a package the closure node limit skipped
/// each have no row, so a consumer joins this table by package index rather
/// than by position.
struct PackageClosures<'a>(&'a [PackageClosure]);
impl Serialize for PackageClosures<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for closure in self.0 {
            sequence.serialize_element(&PackageClosureView(*closure))?;
        }
        sequence.end()
    }
}
struct PackageClosureView(PackageClosure);
impl Serialize for PackageClosureView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("package", &self.0.package().get())?;
        map.serialize_entry("source", &self.0.source().get())?;
        map.serialize_entry("files", &self.0.files())?;
        map.serialize_entry("reach", &self.0.reach())?;
        map.end()
    }
}

struct GraphEvidenceView<'a>(&'a smackdebt_analysis::GraphEvidence);
impl Serialize for GraphEvidenceView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(9))?;
        map.serialize_entry(
            "status",
            if self.0.is_complete() {
                "complete"
            } else {
                "incomplete"
            },
        )?;
        let packages: Vec<_> = self
            .0
            .incomplete_packages()
            .iter()
            .map(|package| package.get())
            .collect();
        map.serialize_entry("incomplete_packages", &packages)?;
        map.serialize_entry("parse_failures", &self.0.parse_failures())?;
        map.serialize_entry("unresolved_internal", &self.0.unresolved_internal())?;
        map.serialize_entry("ambiguous_internal", &self.0.ambiguous_internal())?;
        map.serialize_entry(
            "configuration_failures",
            &GraphConfigurationFailures(self.0.configuration_failures()),
        )?;
        map.serialize_entry("suppressed_reach", &self.0.suppressed_reach())?;
        map.serialize_entry("suppressed_core", &self.0.suppressed_core())?;
        map.serialize_entry("suppressed_leakage", &self.0.suppressed_leakage())?;
        map.end()
    }
}

struct GraphConfigurationFailures<'a>(&'a [smackdebt_analysis::GraphConfigurationFailure]);
impl Serialize for GraphConfigurationFailures<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for failure in self.0 {
            sequence.serialize_element(&GraphConfigurationFailureView(failure))?;
        }
        sequence.end()
    }
}

struct GraphConfigurationFailureView<'a>(&'a smackdebt_analysis::GraphConfigurationFailure);
impl Serialize for GraphConfigurationFailureView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("package", &self.0.package().get())?;
        map.serialize_entry("reason", self.0.reason())?;
        map.end()
    }
}

/// The exact repository-wide reach of each candidate file.
///
/// A file outside the candidate set has no row, so this table is never a
/// complete reach index.
struct FileReaches<'a>(&'a [FileReach]);
impl Serialize for FileReaches<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for reach in self.0 {
            sequence.serialize_element(&FileReachView(*reach))?;
        }
        sequence.end()
    }
}
struct FileReachView(FileReach);
impl Serialize for FileReachView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("file", &self.0.file().get())?;
        map.serialize_entry("reach", &self.0.reach())?;
        map.end()
    }
}

#[derive(Clone, Copy)]
struct CoreComponentView<'a>(&'a [smackdebt_analysis::FileId]);
impl Serialize for CoreComponentView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("anchor", &self.0[0].get())?;
        let members: Vec<_> = self.0.iter().map(|file| file.get()).collect();
        map.serialize_entry("members", &members)?;
        map.end()
    }
}

struct Hotspots<'a>(&'a [Hotspot]);
impl Serialize for Hotspots<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for hotspot in self.0 {
            sequence.serialize_element(&HotspotView(*hotspot))?;
        }
        sequence.end()
    }
}
struct HotspotView(Hotspot);
impl Serialize for HotspotView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("file", &self.0.file().get())?;
        map.serialize_entry("rating", rating_name(self.0.rating()))?;
        map.serialize_entry("touches", &self.0.touches())?;
        map.end()
    }
}

struct SizeFindings<'a>(&'a [SizeFinding]);
impl Serialize for SizeFindings<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for finding in self.0 {
            sequence.serialize_element(&SizeFindingView(finding))?;
        }
        sequence.end()
    }
}
struct SizeFindingView<'a>(&'a SizeFinding);
impl Serialize for SizeFindingView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("file", &self.0.file().get())?;
        map.serialize_entry(
            "subject",
            match self.0.subject() {
                SizeSubject::File => "file",
                SizeSubject::Container => "container",
            },
        )?;
        map.serialize_entry("container", &self.0.container())?;
        map.serialize_entry("value", &self.0.value())?;
        map.serialize_entry("rating", rating_name(self.0.rating()))?;
        map.end()
    }
}

/// The descriptive orphan table, which is file indexes and nothing else.
struct OrphanFiles<'a>(&'a [OrphanFile]);
impl Serialize for OrphanFiles<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for orphan in self.0 {
            sequence.serialize_element(&orphan.file().get())?;
        }
        sequence.end()
    }
}

struct StableDependencyFindings<'a>(&'a [StableDependencyFinding]);
impl Serialize for StableDependencyFindings<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for finding in self.0 {
            sequence.serialize_element(&StableDependencyFindingView(finding))?;
        }
        sequence.end()
    }
}
struct StableDependencyFindingView<'a>(&'a StableDependencyFinding);
impl Serialize for StableDependencyFindingView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let evidence = self.0.evidence();
        let mut map = serializer.serialize_map(Some(11))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("kind", "stable_dependency_violation")?;
        map.serialize_entry("rating", rating_name(self.0.rating()))?;
        map.serialize_entry("source", &self.0.source().get())?;
        map.serialize_entry("target", &self.0.target().get())?;
        // The degree operands are published instead of an instability ratio, so
        // a consumer compares stability at whatever precision it chooses.
        map.serialize_entry("source_fan_in", &evidence.source().fan_in())?;
        map.serialize_entry("source_fan_out", &evidence.source().fan_out())?;
        map.serialize_entry("target_fan_in", &evidence.target().fan_in())?;
        map.serialize_entry("target_fan_out", &evidence.target().fan_out())?;
        map.serialize_entry("references", &evidence.references())?;
        map.serialize_entry("witness_edges", &DependencyEdgeIds(self.0.witness_edges()))?;
        map.end()
    }
}

struct KnowledgeConcentrationFindings<'a>(&'a [KnowledgeConcentrationFinding]);
impl Serialize for KnowledgeConcentrationFindings<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for finding in self.0 {
            sequence.serialize_element(&KnowledgeConcentrationFindingView(*finding))?;
        }
        sequence.end()
    }
}
struct KnowledgeConcentrationFindingView(KnowledgeConcentrationFinding);
impl Serialize for KnowledgeConcentrationFindingView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let concentration = self.0.concentration();
        let mut map = serializer.serialize_map(Some(9))?;
        map.serialize_entry("id", &self.0.id().get())?;
        map.serialize_entry("kind", "knowledge_concentration")?;
        map.serialize_entry("rating", rating_name(self.0.rating()))?;
        map.serialize_entry("package", &concentration.package().get())?;
        map.serialize_entry("role", source_role_name(concentration.role()))?;
        map.serialize_entry("trust", source_trust_name(concentration.trust()))?;
        map.serialize_entry("contributor_count", &concentration.contributor_count())?;
        map.serialize_entry("numerator", &concentration.numerator())?;
        map.serialize_entry("denominator", &concentration.denominator())?;
        map.end()
    }
}

/// The ranked problem table, where a row's position is that card's identity.
struct Problems<'a>(&'a [ProblemCard]);
impl Serialize for Problems<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for card in self.0 {
            sequence.serialize_element(&ProblemView(card))?;
        }
        sequence.end()
    }
}

/// One problem card, which links findings the report already holds rather than
/// restating them, so every value here is an index, a count, or a frozen id.
struct ProblemView<'a>(&'a ProblemCard);
impl Serialize for ProblemView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let card = self.0;
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("pattern", card.pattern().id())?;
        map.serialize_entry("rating", rating_name(card.rating()))?;
        // Visibility is a string because version 4 serializes integers and
        // strings only, and because a third value would otherwise open a
        // version rather than a member.
        map.serialize_entry("visibility", card.visibility().id())?;
        map.serialize_entry("anchor", &ProblemAnchorView(card.anchor()))?;
        map.serialize_entry("evidence", &ProblemEvidenceList(card.evidence()))?;
        map.serialize_entry("claimed", &ClaimedFindings(card.claimed_findings()))?;
        map.end()
    }
}

/// What a card is about, naming its kind and carrying only the indexes that
/// kind implies.
struct ProblemAnchorView<'a>(&'a ProblemAnchor);
impl Serialize for ProblemAnchorView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        match self.0 {
            ProblemAnchor::File(file) => {
                map.serialize_entry("kind", "file")?;
                map.serialize_entry("file", &file.get())?;
            }
            ProblemAnchor::Files(files) => {
                map.serialize_entry("kind", "files")?;
                map.serialize_entry("files", &FileIds(files))?;
            }
            ProblemAnchor::Package(package) => {
                map.serialize_entry("kind", "package")?;
                map.serialize_entry("package", &package.get())?;
            }
            ProblemAnchor::PackagePair(left, right) => {
                map.serialize_entry("kind", "package_pair")?;
                map.serialize_entry("packages", &[left.get(), right.get()])?;
            }
        }
        map.end()
    }
}

/// The card's facts in the order analysis stored them, head first.
struct ProblemEvidenceList<'a>(&'a [ProblemEvidence]);
impl Serialize for ProblemEvidenceList<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for fact in self.0 {
            sequence.serialize_element(&ProblemEvidenceView(*fact))?;
        }
        sequence.end()
    }
}

/// One fact a card states.
///
/// A fact that links a finding names the table it indexes, so a consumer
/// resolves it with one lookup; a fact the report already measured carries its
/// integer instead. Two functions partition the vocabulary between those two
/// shapes, so a kind neither of them classifies writes no member at all and
/// schema validation fails rather than a wrong kind being invented.
struct ProblemEvidenceView(ProblemEvidence);
impl Serialize for ProblemEvidenceView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        match (linked_finding(self.0), measured_fact(self.0)) {
            (Some((table, index)), _) => {
                map.serialize_entry("kind", table)?;
                map.serialize_entry("index", &index)?;
            }
            (None, Some((kind, value))) => {
                map.serialize_entry("kind", kind)?;
                map.serialize_entry("value", &value)?;
            }
            // The two functions partition the whole vocabulary, so this arm is
            // reached only by a kind added to one enum and to neither of them.
            // Writing no member makes schema validation fail, which is the
            // correct end state but a long way from the cause; the assert names
            // the cause where it happened.
            (None, None) => debug_assert!(false, "every evidence kind is linked or measured"),
        }
        map.end()
    }
}

/// The table one evidence link indexes and the position it names, which is the
/// same table a claim of that finding names.
const fn linked_finding(fact: ProblemEvidence) -> Option<(&'static str, u32)> {
    Some(match fact {
        ProblemEvidence::Finding(id) => ("findings", id.get()),
        ProblemEvidence::Size(id) => ("size_findings", id.get()),
        ProblemEvidence::Architecture(id) => ("architecture_findings", id.get()),
        ProblemEvidence::StableDependency(id) => ("stable_dependency_findings", id.get()),
        ProblemEvidence::Coupling(id) => ("evolutionary_findings", id.get()),
        ProblemEvidence::Knowledge(id) => ("knowledge_concentration_findings", id.get()),
        ProblemEvidence::ChangeLeakage(id) => ("change_leakage_findings", id.get()),
        _ => return None,
    })
}

/// The kind one measured fact names and the integer it carries.
const fn measured_fact(fact: ProblemEvidence) -> Option<(&'static str, u32)> {
    Some(match fact {
        ProblemEvidence::FanIn(value) => ("fan_in", value),
        ProblemEvidence::FanOut(value) => ("fan_out", value),
        ProblemEvidence::Hot(value) => ("hot", value),
        ProblemEvidence::RatedUnits(value) => ("rated_units", value),
        ProblemEvidence::Members(value) => ("members", value),
        ProblemEvidence::ReachIn(value) => ("reach_in", value),
        ProblemEvidence::Followers(value) => ("followers", value),
        _ => return None,
    })
}

/// The findings one card claimed, each naming the table it indexes and its
/// position in that table, so the claim audit resolves without a join.
struct ClaimedFindings<'a>(&'a [ClaimedFinding]);
impl Serialize for ClaimedFindings<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for claim in self.0 {
            sequence.serialize_element(&ClaimedFindingView(*claim))?;
        }
        sequence.end()
    }
}

struct ClaimedFindingView(ClaimedFinding);
impl Serialize for ClaimedFindingView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (table, index) = match self.0 {
            ClaimedFinding::Source(id) => ("findings", id.get()),
            ClaimedFinding::Size(id) => ("size_findings", id.get()),
            ClaimedFinding::Architecture(id) => ("architecture_findings", id.get()),
            ClaimedFinding::StableDependency(id) => ("stable_dependency_findings", id.get()),
            ClaimedFinding::Coupling(id) => ("evolutionary_findings", id.get()),
            ClaimedFinding::Knowledge(id) => ("knowledge_concentration_findings", id.get()),
            ClaimedFinding::ChangeLeakage(id) => ("change_leakage_findings", id.get()),
        };
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("table", table)?;
        map.serialize_entry("index", &index)?;
        map.end()
    }
}
