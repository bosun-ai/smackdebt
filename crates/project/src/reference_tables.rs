//! Resolving every stated reference into edge, external, and diagnostic
//! tables. The machinery is private; resolution enters through
//! `ReferenceResolver` and leaves as `ReferenceRows`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use smackdebt_analysis::{
    DependencyCoverage, DependencyEdge, DependencyEdgeId, DependencySyntax, DependencySyntaxState,
    ExternalDependency, FileId, PackageId, ResolutionDiagnostic, ResolutionIssueKind, SourceRole,
    SourceTrust,
};

use crate::candidates::{candidates_name_an_asset, resolve_candidates};
use crate::dependencies::SourceDependencies;
use crate::manifest_names::{ManifestNameIndex, ManifestNameMatch, package_entry_file};
use crate::paths::package_of;
use crate::resolution_config::ResolutionRules;

const RETAINED_RELATION_LOCATIONS: usize = 3;

type DependencyEdgeKey = (
    FileId,
    FileId,
    smackdebt_analysis::StaticRelationKind,
    SourceRole,
    SourceTrust,
);
type DependencyEdgeValue = (u32, Vec<smackdebt_analysis::SourceSpan>);
type ExternalDependencyKey = (
    FileId,
    String,
    smackdebt_analysis::StaticRelationKind,
    SourceRole,
    SourceTrust,
);
type ResolutionDiagnosticKey = (
    FileId,
    String,
    ResolutionIssueKind,
    String,
    smackdebt_analysis::StaticRelationKind,
    SourceRole,
    SourceTrust,
);
#[derive(Clone, Copy)]
enum RelationResolution {
    ResolvedInternal,
    UnresolvedInternal,
    AmbiguousInternal,
    External,
    UnresolvedPackage,
    /// An internal reference to a file the source languages never analyze.
    AssetReference,
}
#[derive(Default)]
struct DependencyPartitionCounts {
    resolved_internal_uses: u32,
    unresolved_internal_uses: u32,
    ambiguous_internal_uses: u32,
    external_uses: u32,
    unresolved_package_uses: u32,
    module_ownership_relations: u32,
    context_relations: u32,
}
impl DependencyPartitionCounts {
    fn record(
        &mut self,
        relation: smackdebt_analysis::StaticRelationKind,
        role: SourceRole,
        trust: SourceTrust,
        resolution: RelationResolution,
    ) {
        *self.partition(relation, role, trust, resolution) += 1;
    }

    /// The single partition one relation belongs to.
    ///
    /// Every relation lands in exactly one counter, so the partitions add up
    /// to the references the parse found and none is counted twice.
    pub(crate) fn partition(
        &mut self,
        relation: smackdebt_analysis::StaticRelationKind,
        role: SourceRole,
        trust: SourceTrust,
        resolution: RelationResolution,
    ) -> &mut u32 {
        if trust != SourceTrust::Trusted || !role.affects_verdict() {
            return &mut self.context_relations;
        }
        if relation == smackdebt_analysis::StaticRelationKind::ModuleOwnership {
            return &mut self.module_ownership_relations;
        }
        match resolution {
            RelationResolution::ResolvedInternal => &mut self.resolved_internal_uses,
            RelationResolution::UnresolvedInternal => &mut self.unresolved_internal_uses,
            RelationResolution::AmbiguousInternal => &mut self.ambiguous_internal_uses,
            RelationResolution::External => &mut self.external_uses,
            RelationResolution::UnresolvedPackage => &mut self.unresolved_package_uses,
            // An asset reference explains the file it was written in without
            // ever joining the verdict graph, so it stays in the partition
            // that holds relations kept as context.
            RelationResolution::AssetReference => &mut self.context_relations,
        }
    }

    pub(crate) fn finish(self) -> DependencyCoverage {
        DependencyCoverage::new(
            self.resolved_internal_uses,
            self.unresolved_internal_uses,
            self.ambiguous_internal_uses,
            self.external_uses,
            self.unresolved_package_uses,
            self.module_ownership_relations,
            self.context_relations,
        )
    }
}
/// What one unmatched reference means after the manifest-name index is read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManifestReference {
    /// No unique internal package declares the referenced name.
    Absent,
    /// Several internal packages declare the referenced name.
    Ambiguous,
    /// The matched package presents this file as its entry point.
    Entry(FileId),
    /// The matched package has no resolvable entry file.
    Package(PackageId),
}
/// The tables one architecture pass fills while it partitions references.
#[derive(Default)]
pub(crate) struct ReferenceTables {
    coverage: DependencyPartitionCounts,
    edge_values: BTreeMap<DependencyEdgeKey, DependencyEdgeValue>,
    external_values: BTreeMap<ExternalDependencyKey, DependencyEdgeValue>,
    diagnostic_values: BTreeMap<ResolutionDiagnosticKey, DependencyEdgeValue>,
    internal_issue_files: BTreeSet<FileId>,
    manifest_package_values:
        BTreeMap<(PackageId, PackageId), (std::collections::BTreeSet<FileId>, u32)>,
    /// Manifest-name package pairs that explain change coupling without
    /// entering a verdict graph, such as a dev-dependency test import.
    manifest_explanation_pairs: BTreeSet<(PackageId, PackageId)>,
}
/// The role one reference carries as evidence.
///
/// A reference declared under a test configuration is at least test code, so a
/// primary file's `#[cfg(test)]` imports become test evidence while a file that
/// already carries a later role keeps it.
fn evidence_role(reference: &DependencySyntax, dependencies: &SourceDependencies) -> SourceRole {
    if reference.scope() == smackdebt_analysis::DependencyScope::Test {
        dependencies.role.demoted_by_test_scope()
    } else {
        dependencies.role
    }
}
impl ReferenceTables {
    fn record_internal(
        &mut self,
        source: FileId,
        target: FileId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
    ) {
        let role = evidence_role(reference, dependencies);
        self.coverage.record(
            reference.relation(),
            role,
            dependencies.trust,
            RelationResolution::ResolvedInternal,
        );
        if source == target {
            return;
        }
        let value = self
            .edge_values
            .entry((
                source,
                target,
                reference.relation(),
                role,
                dependencies.trust,
            ))
            .or_default();
        value.0 += 1;
        if value.1.len() < RETAINED_RELATION_LOCATIONS {
            value.1.push(reference.span());
        }
    }

    pub(crate) fn record_external(
        &mut self,
        source: FileId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
    ) {
        let role = evidence_role(reference, dependencies);
        self.coverage.record(
            reference.relation(),
            role,
            dependencies.trust,
            RelationResolution::External,
        );
        let value = self
            .external_values
            .entry((
                source,
                reference.target().to_owned(),
                reference.relation(),
                role,
                dependencies.trust,
            ))
            .or_default();
        value.0 += 1;
        if value.1.len() < RETAINED_RELATION_LOCATIONS {
            value.1.push(reference.span());
        }
    }

    /// Records an internal reference that only names a package, not a file.
    pub(crate) fn record_package(
        &mut self,
        source: PackageId,
        target: PackageId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
    ) {
        let source_file = dependencies.file;
        let role = evidence_role(reference, dependencies);
        self.coverage.record(
            reference.relation(),
            role,
            dependencies.trust,
            RelationResolution::ResolvedInternal,
        );
        if source == target
            || !(reference.relation() == smackdebt_analysis::StaticRelationKind::Uses
                && role.affects_verdict()
                && dependencies.trust == SourceTrust::Trusted)
        {
            return;
        }
        self.manifest_explanation_pairs.insert((source, target));
        if role != SourceRole::Primary {
            return;
        }
        let value = self
            .manifest_package_values
            .entry((source, target))
            .or_default();
        value.0.insert(source_file);
        value.1 += 1;
    }

    fn record_diagnostic(
        &mut self,
        source: FileId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
        resolution: RelationResolution,
        kind: ResolutionIssueKind,
        reason: &str,
    ) {
        let role = evidence_role(reference, dependencies);
        self.coverage
            .record(reference.relation(), role, dependencies.trust, resolution);
        if dependencies.trust == SourceTrust::Trusted
            && role == SourceRole::Primary
            && matches!(
                resolution,
                RelationResolution::UnresolvedInternal | RelationResolution::AmbiguousInternal
            )
        {
            self.internal_issue_files.insert(source);
        }
        record_resolution_diagnostic(
            &mut self.diagnostic_values,
            source,
            reference,
            role,
            dependencies.trust,
            kind,
            reason,
        );
    }

    /// Records an internal reference that no repository file matched.
    ///
    /// A reference looked for under a name whose extension the source
    /// languages never analyze is an asset reference: the repository may well
    /// hold the file, but discovery only inventories source, so no lookup
    /// could ever have matched it. Calling that unresolved would claim a hole
    /// in the dependency graph the code does not have, so it is disclosed
    /// under its own reason instead.
    fn record_unmatched_internal(
        &mut self,
        source: FileId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
        candidates: &[String],
    ) {
        let (resolution, kind, reason) = if candidates_name_an_asset(candidates) {
            (
                RelationResolution::AssetReference,
                ResolutionIssueKind::Asset,
                "target is an asset",
            )
        } else {
            (
                RelationResolution::UnresolvedInternal,
                ResolutionIssueKind::Unresolved,
                "no repository file matches",
            )
        };
        self.record_diagnostic(source, reference, dependencies, resolution, kind, reason);
    }

    /// Records a reference no path candidate matched.
    fn record_unmatched(
        &mut self,
        source: FileId,
        source_package: PackageId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
        resolution: ManifestReference,
    ) {
        match resolution {
            ManifestReference::Entry(target) => {
                self.record_internal(source, target, reference, dependencies);
            }
            ManifestReference::Package(target) => {
                self.record_package(source_package, target, reference, dependencies);
            }
            ManifestReference::Ambiguous => self.record_diagnostic(
                source,
                reference,
                dependencies,
                RelationResolution::AmbiguousInternal,
                ResolutionIssueKind::Ambiguous,
                "several packages declare this name",
            ),
            ManifestReference::Absent => self.record_external(source, reference, dependencies),
        }
    }
}
/// Resolves one unmatched reference against the declared package names.
fn resolve_manifest_name(
    reference: &DependencySyntax,
    dependencies: &SourceDependencies,
    manifest_index: &ManifestNameIndex,
    manifest_names: &[Option<String>],
    package_roots: &[PathBuf],
    index: &BTreeMap<PathBuf, FileId>,
) -> ManifestReference {
    if reference.relation() != smackdebt_analysis::StaticRelationKind::Uses {
        return ManifestReference::Absent;
    }
    match manifest_index.resolve(reference.target(), dependencies.language) {
        ManifestNameMatch::Absent => ManifestReference::Absent,
        ManifestNameMatch::Ambiguous => ManifestReference::Ambiguous,
        ManifestNameMatch::Package(position) => {
            let root = &package_roots[position];
            let name = manifest_names[position].as_deref().unwrap_or_default();
            package_entry_file(root, name, index).map_or(
                ManifestReference::Package(PackageId::from_index(position)),
                ManifestReference::Entry,
            )
        }
    }
}
fn record_resolution_diagnostic(
    values: &mut BTreeMap<ResolutionDiagnosticKey, DependencyEdgeValue>,
    source: FileId,
    reference: &DependencySyntax,
    role: SourceRole,
    trust: SourceTrust,
    kind: ResolutionIssueKind,
    reason: &str,
) {
    let value = values
        .entry((
            source,
            reference.target().to_owned(),
            kind,
            reason.to_owned(),
            reference.relation(),
            role,
            trust,
        ))
        .or_default();
    value.0 += 1;
    if value.1.len() < RETAINED_RELATION_LOCATIONS {
        value.1.push(reference.span());
    }
}
/// Everything one reference resolution reads: the path index, the manifest
/// names, the package positions, and the resolution rules.
pub(crate) struct ReferenceResolver<'a> {
    pub(crate) index: &'a BTreeMap<PathBuf, FileId>,
    pub(crate) manifest_index: &'a ManifestNameIndex,
    pub(crate) manifest_names: &'a [Option<String>],
    pub(crate) side_package_roots: &'a [PathBuf],
    pub(crate) package_roots: &'a [PathBuf],
    pub(crate) aliases: &'a ResolutionRules,
}
impl ReferenceResolver<'_> {
    /// Resolves every reference the dependency table states, in table order.
    pub(crate) fn resolve(&self, dependencies: &[SourceDependencies]) -> ReferenceTables {
        let mut tables = ReferenceTables::default();
        for dependencies in dependencies {
            let source_package = package_of(
                &dependencies.path,
                self.side_package_roots,
                self.package_roots,
            );
            for reference in &dependencies.references {
                self.record(&mut tables, source_package, dependencies, reference);
            }
        }
        tables
    }

    /// Records what one reference settles to: an internal edge, an external
    /// dependency, or a diagnostic.
    fn record(
        &self,
        tables: &mut ReferenceTables,
        source_package: PackageId,
        dependencies: &SourceDependencies,
        reference: &DependencySyntax,
    ) {
        let source = dependencies.file;
        match reference.state() {
            DependencySyntaxState::External => {
                let resolution = self.manifest_resolution(reference, dependencies);
                tables.record_unmatched(
                    source,
                    source_package,
                    reference,
                    dependencies,
                    resolution,
                );
            }
            DependencySyntaxState::Unresolved(reason) => {
                let resolution =
                    if reference.intent() == smackdebt_analysis::DependencyIntent::Internal {
                        RelationResolution::UnresolvedInternal
                    } else {
                        RelationResolution::UnresolvedPackage
                    };
                tables.record_diagnostic(
                    source,
                    reference,
                    dependencies,
                    resolution,
                    ResolutionIssueKind::Unresolved,
                    reason,
                );
            }
            DependencySyntaxState::Candidates(candidates) => {
                let matches =
                    resolve_candidates(&dependencies.path, candidates, self.index, self.aliases);
                match matches.as_slice() {
                    [] if reference.intent() == smackdebt_analysis::DependencyIntent::Internal => {
                        tables.record_unmatched_internal(
                            source,
                            reference,
                            dependencies,
                            candidates,
                        );
                    }
                    [] => {
                        let resolution = self.manifest_resolution(reference, dependencies);
                        tables.record_unmatched(
                            source,
                            source_package,
                            reference,
                            dependencies,
                            resolution,
                        );
                    }
                    [target] => {
                        tables.record_internal(source, *target, reference, dependencies);
                    }
                    _ => {
                        tables.record_diagnostic(
                            source,
                            reference,
                            dependencies,
                            RelationResolution::AmbiguousInternal,
                            ResolutionIssueKind::Ambiguous,
                            "several repository files match",
                        );
                    }
                }
            }
        }
    }

    /// The manifest-name reading of one reference no repository file matched.
    fn manifest_resolution(
        &self,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
    ) -> ManifestReference {
        resolve_manifest_name(
            reference,
            dependencies,
            self.manifest_index,
            self.manifest_names,
            self.package_roots,
            self.index,
        )
    }
}
/// The package-level joins manifest names contributed without file edges.
pub(crate) struct ManifestJoins {
    pub(crate) package_values: BTreeMap<(PackageId, PackageId), (BTreeSet<FileId>, u32)>,
    pub(crate) explanation_pairs: BTreeSet<(PackageId, PackageId)>,
}
/// The rows the resolved reference tables state, ready for the report.
pub(crate) struct ReferenceRows {
    pub(crate) coverage: DependencyCoverage,
    pub(crate) diagnostics: Vec<ResolutionDiagnostic>,
    pub(crate) file_edges: Vec<DependencyEdge>,
    pub(crate) external: Vec<ExternalDependency>,
    pub(crate) internal_issue_files: BTreeSet<FileId>,
    pub(crate) manifest: ManifestJoins,
}
impl ReferenceTables {
    /// Finishes the accumulated tables into their report rows.
    pub(crate) fn into_rows(self) -> ReferenceRows {
        ReferenceRows {
            coverage: self.coverage.finish(),
            diagnostics: self
                .diagnostic_values
                .into_iter()
                .map(
                    |(
                        (source, target, kind, reason, relation, role, trust),
                        (references, locations),
                    )| {
                        ResolutionDiagnostic::new(source, locations[0], target, kind, reason)
                            .with_evidence(relation, role, trust)
                            .with_occurrences(references, locations)
                    },
                )
                .collect(),
            file_edges: self
                .edge_values
                .into_iter()
                .enumerate()
                .map(
                    |(
                        edge_index,
                        ((source, target, relation, role, trust), (references, locations)),
                    )| {
                        DependencyEdge::new(
                            DependencyEdgeId::from_index(edge_index),
                            source,
                            target,
                            references,
                            locations,
                        )
                        .with_relation(relation)
                        .with_evidence(role, trust)
                    },
                )
                .collect(),
            external: self
                .external_values
                .into_iter()
                .map(
                    |((file, target, relation, role, trust), (references, locations))| {
                        ExternalDependency::new(file, target, references)
                            .with_evidence(locations, relation, role, trust)
                    },
                )
                .collect(),
            internal_issue_files: self.internal_issue_files,
            manifest: ManifestJoins {
                package_values: self.manifest_package_values,
                explanation_pairs: self.manifest_explanation_pairs,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use std::fs;

    #[test]
    fn an_import_of_a_non_source_file_is_an_asset_rather_than_a_hole() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            ("app/package.json", "{\"name\":\"app\"}\n"),
            // Three assets an importer reads for their bytes: one carries a
            // query suffix, one a fragment, one neither.
            (
                "app/main.ts",
                "import raw from './config.yaml?raw';\nimport icon from './logo.svg#glyph';\nimport theme from './theme.css';\nimport helper from './helper';\nexport default [raw, icon, theme, helper];\n",
            ),
            ("app/helper.ts", "export default 1;\n"),
            ("app/config.yaml", "name: fixture\n"),
            ("app/logo.svg", "<svg />\n"),
            ("app/theme.css", ".a { color: red; }\n"),
            ("core/package.json", "{\"name\":\"core\"}\n"),
            // The scope guard: a source extension that matches nothing, and a
            // target that names no extension at all, are still holes.
            (
                "core/main.ts",
                "import gone from './gone.ts';\nimport absent from './absent';\nexport default [gone, absent];\n",
            ),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let rows: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .map(|value| {
                format!(
                    "{} · {:?} · {}",
                    value.target(),
                    value.kind(),
                    value.reason()
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                "./config.yaml?raw · Asset · target is an asset",
                "./logo.svg#glyph · Asset · target is an asset",
                "./theme.css · Asset · target is an asset",
                "./absent · Unresolved · no repository file matches",
                "./gone.ts · Unresolved · no repository file matches",
            ],
            "an asset keeps its disclosure under its own reason"
        );

        let coverage = report.dependency_coverage();
        assert_eq!(
            coverage.unresolved_internal_uses(),
            2,
            "only the two holes are unresolved"
        );
        assert_eq!(
            coverage.context_relations(),
            3,
            "the assets stay counted, outside the verdict graph"
        );
        assert_eq!(coverage.resolved_internal_uses(), 1);

        let evidence = report.graph_evidence();
        assert_eq!(evidence.unresolved_internal(), 2);
        let core = report
            .files()
            .iter()
            .find(|file| file.path() == "core/main.ts")
            .and_then(smackdebt_analysis::FileRecord::package)
            .expect("the hole belongs to a package");
        assert_eq!(
            evidence.incomplete_packages(),
            [core],
            "importing an asset leaves its package complete"
        );
    }
    #[test]
    fn an_asset_is_read_from_the_paths_a_reference_was_looked_for_under() {
        // The spellings each language hands the resolver. A path language
        // offers the written name plus the extensions it knows; a language
        // that reads dotted module notation offers only the paths it derived
        // from that name, and the written form never appears at all.
        let path = |target: &str| {
            let mut values = vec![target.to_owned()];
            for extension in [".js", ".ts"] {
                values.push(format!("{target}{extension}"));
                values.push(format!("{target}/index{extension}"));
            }
            values
        };
        let module = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };

        for (target, candidates, asset, reading) in [
            (
                "./x.yaml?raw",
                path("./x.yaml?raw"),
                true,
                "a query suffix is stripped before the extension is read",
            ),
            (
                "./x.md",
                path("./x.md"),
                true,
                "a plain unclaimed extension needs no suffix",
            ),
            (
                "./capabilities",
                path("./capabilities"),
                false,
                "a spelling that names no extension claims nothing",
            ),
            (
                "./missing.ts",
                path("./missing.ts"),
                false,
                "a source extension that matched nothing is still a hole",
            ),
            // `from ..core import thing`. Read as a path the target carries
            // the extension `core`, so only the candidates show it is a module
            // name and that the file it misses is a real hole.
            (
                "..core",
                module(&["../core.py", "../core/__init__.py"]),
                false,
                "dotted module notation never reaches here as a path",
            ),
            (
                ".missing.thing",
                module(&["./missing/thing.py", "./missing/thing/__init__.py"]),
                false,
                "a dotted module chain is not a path either",
            ),
            // Pinned rather than preferred: an absent `./webpack.config.js`
            // imported as `./webpack.config` reads as an asset, because
            // `config` is an extension no language claims. The candidates
            // cannot settle it — the literal spelling is one of them. It costs
            // a reader nothing: the row keeps its target and its line in JSON,
            // and is only held out of a count that would otherwise claim a
            // broken graph on a guess.
            (
                "./webpack.config",
                path("./webpack.config"),
                true,
                "an unclaimed extension on a written path reads as an asset",
            ),
        ] {
            assert_eq!(
                candidates_name_an_asset(&candidates),
                asset,
                "{target}: {reading}"
            );
        }
    }
    #[test]
    fn a_python_relative_import_of_a_missing_module_is_still_a_hole() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            ("pyproject.toml", "[project]\nname='service'\n"),
            ("src/__init__.py", "\n"),
            ("src/api/__init__.py", "\n"),
            // `..core` names the module `src/core`, which nothing declares.
            // Read as a path it would carry the extension `core` and vanish
            // under an asset row, taking the package's incompleteness with it.
            (
                "src/api/handler.py",
                "from ..core import thing\n\n\ndef handle():\n    return thing\n",
            ),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let rows: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .map(|value| {
                format!(
                    "{} · {:?} · {}",
                    value.target(),
                    value.kind(),
                    value.reason()
                )
            })
            .collect();
        assert_eq!(rows, ["..core · Unresolved · no repository file matches"]);
        assert_eq!(report.dependency_coverage().unresolved_internal_uses(), 1);

        let evidence = report.graph_evidence();
        assert!(
            !evidence.is_complete(),
            "a missing Python module leaves the graph incomplete"
        );
        assert_eq!(evidence.unresolved_internal(), 1);
        assert_eq!(evidence.incomplete_packages().len(), 1);
    }
}
