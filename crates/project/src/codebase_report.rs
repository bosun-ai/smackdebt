//! Accumulating rated files into the one codebase report and composing it.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use smackdebt_analysis::{
    ArchitectureGraph, ArchitectureReportFacts, Coverage, Diagnostic, DiagnosticId, DiagnosticKind,
    DirectoryTree, FileActivity, FileDebt, FileId, FileRecord, Finding, FindingId, HealthCounts,
    HistoryAvailability, HotspotPolicy, Language, PackageContainment, PackageId, PackageRecord,
    ParseStatus, Rating, Report, ReportBuilder as AnalysisReportBuilder, ReportMode, Scope,
    ScopeId, SizeFinding, SizePolicy, SourceCoverageOutcome, SourceRole,
};
use smackdebt_discovery::{DiscoveredFile, Inventory};

use crate::architecture::{ArchitectureBuild, GraphInputs, build_architecture, leakage_findings};
use crate::dependencies::{ManifestFacts, PackageTables, SourceDependencies};
use crate::dormancy::WindowedHistory;
use crate::hierarchy::HierarchyBuilder;
use crate::history_stream::EvolutionInput;
use crate::paths::report_package_path;
use crate::rating::{FileResult, RatedFile, source_coverage};
use crate::resolution_config::ResolutionRules;
use crate::work::AnalysisWork;

pub(crate) struct CodebaseReportBuilder<'a> {
    scopes: Vec<Scope>,
    files: Vec<FileRecord>,
    findings: Vec<Finding>,
    diagnostics: Vec<Diagnostic>,
    file_scopes: Vec<ScopeId>,
    file_sizes: Vec<u64>,
    activity: &'a HashMap<PathBuf, u32>,
    package_ids: Vec<PackageId>,
    dependencies: Vec<SourceDependencies>,
    aliases: ResolutionRules,
    package_roots: Vec<PathBuf>,
    manifest_names: Vec<Option<String>>,
    manifest_paths: Vec<Vec<String>>,
    packages: Vec<PackageRecord>,
    evolution: EvolutionInput,
    file_debt: Vec<FileDebt>,
    size_findings: Vec<SizeFinding>,
    policies: SignalPolicies,
}
/// The policies that own the derived signal tables.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SignalPolicies {
    pub(crate) hotspots: HotspotPolicy,
    pub(crate) size: SizePolicy,
}
/// The measured tree one codebase report is built from: the walked
/// inventory, its analyzable candidates, and the windowed touch counts.
pub(crate) struct CodebaseInputs<'a> {
    pub(crate) inventory: &'a Inventory,
    pub(crate) candidates: &'a [&'a DiscoveredFile],
    pub(crate) activity: &'a HashMap<PathBuf, u32>,
}

/// Where one candidate file sits while its analysis is retained.
struct FileSpot {
    file: FileId,
    scope: ScopeId,
    size_bytes: u64,
    path: String,
}

impl<'a> CodebaseReportBuilder<'a> {
    pub(crate) fn new(
        label: String,
        inputs: CodebaseInputs<'a>,
        aliases: ResolutionRules,
        evolution: EvolutionInput,
        policies: SignalPolicies,
    ) -> Self {
        let CodebaseInputs {
            inventory,
            candidates,
            activity,
        } = inputs;
        let package_roots: Vec<PathBuf> = inventory
            .packages()
            .iter()
            .map(|package| package.root().as_path().to_path_buf())
            .collect();
        let manifest_names: Vec<_> = inventory
            .packages()
            .iter()
            .map(|package| package.manifest_name().map(str::to_owned))
            .collect();
        let manifest_paths: Vec<_> = inventory
            .packages()
            .iter()
            .map(|package| package.declared_paths().to_vec())
            .collect();
        let mut hierarchy = HierarchyBuilder::new(label, &package_roots);
        let packages = package_roots
            .iter()
            .enumerate()
            .map(|(index, root)| {
                PackageRecord::current(
                    PackageId::from_index(index),
                    hierarchy.package_scopes[index],
                    report_package_path(root),
                )
                .with_manifest_name(manifest_names[index].clone())
            })
            .collect();
        let mut package_ids = Vec::with_capacity(candidates.len());
        for file in candidates {
            hierarchy.add_file(file.path().as_path(), file.package().index());
            package_ids.push(file.package());
        }
        let file_scopes = candidates
            .iter()
            .map(|file| hierarchy.file_scopes[file.path().as_path()])
            .collect();
        let file_sizes = candidates.iter().map(|file| file.size_bytes()).collect();

        Self {
            scopes: hierarchy.scopes,
            files: Vec::with_capacity(candidates.len()),
            findings: Vec::with_capacity(candidates.len()),
            diagnostics: Vec::with_capacity(candidates.len()),
            file_scopes,
            file_sizes,
            activity,
            package_ids,
            dependencies: Vec::with_capacity(candidates.len()),
            aliases,
            package_roots,
            manifest_names,
            manifest_paths,
            packages,
            evolution,
            file_debt: Vec::with_capacity(candidates.len()),
            size_findings: Vec::with_capacity(candidates.len()),
            policies,
        }
    }

    /// Retains one analyzed file's findings, sizes, dependencies, and parse
    /// diagnostics, and answers its coverage and language facts.
    fn add_rated(
        &mut self,
        spot: &FileSpot,
        mut rated: RatedFile,
    ) -> (Coverage, Option<(Language, SourceRole, ParseStatus)>) {
        if rated.signals_verdict {
            self.size_findings.extend(
                self.policies
                    .size
                    .rate_file(spot.file, rated.analysis.source_lines()),
            );
        }
        let mut containers = std::mem::take(&mut rated.container_statements);
        containers.sort_by(|left, right| left.0.cmp(&right.0));
        for (container, statements) in containers {
            self.size_findings.extend(
                self.policies
                    .size
                    .rate_container(spot.file, &container, statements),
            );
        }
        for (unit_index, assessment) in rated.debt {
            let unit = &rated.analysis.units()[unit_index];
            let finding_id = FindingId::from_index(self.findings.len());
            self.scopes[spot.scope.index()].add_finding(finding_id);
            self.findings.push(
                Finding::new(
                    finding_id,
                    spot.file,
                    unit.identity().clone(),
                    unit.span(),
                    unit.measurements(),
                    assessment,
                )
                .with_evidence(rated.role, rated.analysis.parse_status().trust()),
            );
        }
        let analysis = rated.analysis;
        self.dependencies.push(SourceDependencies {
            file: spot.file,
            path: PathBuf::from(&spot.path),
            references: analysis.dependencies().to_vec(),
            role: rated.role,
            trust: analysis.parse_status().trust(),
            language: analysis.language(),
            module_syntax: rated.module_syntax,
        });
        let recovered = matches!(analysis.parse_status(), ParseStatus::Recovered(_));
        let failed = matches!(analysis.parse_status(), ParseStatus::Failed);
        if failed {
            self.add_diagnostic(
                spot.file,
                DiagnosticKind::ParseFailure,
                "parser failed",
                analysis.source_lines(),
            );
        } else if recovered {
            self.add_diagnostic(
                spot.file,
                DiagnosticKind::ParseFailure,
                "parser recovered from syntax errors",
                0,
            );
        }
        (
            source_coverage(&analysis, rated.role).with_bytes(spot.size_bytes, 0),
            Some((
                analysis.language(),
                rated.role,
                analysis.parse_status().clone(),
            )),
        )
    }

    pub(crate) fn add_analysis(&mut self, index: usize, result: FileResult) {
        let scope_id = self.file_scopes[index];
        let spot = FileSpot {
            file: FileId::from_index(index),
            scope: scope_id,
            size_bytes: self.file_sizes[index],
            path: self.scopes[scope_id.index()].name().to_owned(),
        };
        let file_id = spot.file;
        let size_bytes = spot.size_bytes;
        let touches = self.activity.get(Path::new(&spot.path)).copied();
        let mut health = HealthCounts::default();
        let mut rated_units = 0;
        let mut max_rating = Rating::Healthy;
        let (coverage, language) = match result {
            FileResult::Analyzed(rated) => {
                health = rated.health;
                rated_units = rated.rated_units;
                max_rating = rated.max_rating;
                self.add_rated(&spot, rated)
            }
            FileResult::Unsupported { language, role } => {
                self.add_diagnostic(
                    file_id,
                    DiagnosticKind::UnsupportedLanguage,
                    format!("{} uses an unsupported language", spot.path),
                    0,
                );
                (
                    Coverage::classified(1, SourceCoverageOutcome::Unsupported, 0, 0)
                        .with_bytes(size_bytes, size_bytes),
                    Some((language, role, ParseStatus::Failed)),
                )
            }
            FileResult::Failed {
                message,
                role,
                language,
            } => {
                self.add_diagnostic(
                    file_id,
                    DiagnosticKind::UnreadableFile,
                    format!("{}: {message}", spot.path),
                    0,
                );
                (
                    Coverage::classified(1, SourceCoverageOutcome::Failed, 0, 0)
                        .with_bytes(size_bytes, 0),
                    Some((language, role, ParseStatus::Failed)),
                )
            }
            FileResult::RoleConflict { .. } => unreachable!("role conflicts stop composition"),
        };
        let mut file = FileRecord::new(file_id, scope_id, spot.path, coverage, health);
        file = file.with_package(self.package_ids[index]);
        if let Some((language, role, status)) = language {
            file = file.with_language(language).with_source_state(role, status);
        }
        if let Some(touches) = touches {
            file = file.with_activity(FileActivity::new(touches));
        }
        self.file_debt.push(FileDebt::new(
            file_id,
            rated_units,
            max_rating,
            touches.unwrap_or(0),
        ));
        self.scopes[scope_id.index()].add_file(file_id);
        self.files.push(file);
    }

    pub(crate) fn add_diagnostic(
        &mut self,
        file: FileId,
        kind: DiagnosticKind,
        message: impl Into<String>,
        excluded_lines: u32,
    ) {
        let id = DiagnosticId::from_index(self.diagnostics.len());
        self.diagnostics.push(Diagnostic::new(
            id,
            Some(file),
            kind,
            message,
            excluded_lines,
        ));
    }

    /// Discloses every package whose file closure the node limit skipped.
    ///
    /// The skip is a fact the machine report owes its reader: the package has a
    /// file reach and this report does not state it.
    pub(crate) fn disclose_skipped_closures(&mut self, skipped: &[PackageId]) {
        for package in skipped {
            let path = report_package_path(&self.package_roots[package.index()]);
            self.add_general_diagnostic(
                DiagnosticKind::PropagationSkipped,
                format!("{path} holds more files than one closure may reach over"),
            );
        }
    }

    /// Restates every file the architecture pass proved dormant.
    ///
    /// The role is settled after these tables were filled, so each one is
    /// brought to the answer the file would have carried had its role been
    /// known when it was rated: the file drops the health it contributed, its
    /// findings are re-evidenced so no verdict, card, or offender counts them,
    /// its size findings go with the verdict they were raised for, and the
    /// hotspot table stops seeing rated units. The findings themselves stay,
    /// because a dormant file is still inspectable on its own.
    ///
    /// The churn and coupling tables are deliberately not restated. They were
    /// filled while history streamed, before this role existed - and they hold
    /// nothing to restate, because a file with any row in them was touched
    /// inside the window and so can never be dormant. The pinning test is
    /// `a_dormant_file_has_no_history_row_left_under_the_old_role`.
    pub(crate) fn restate_dormant_files(&mut self, dormant: &BTreeSet<FileId>) {
        if dormant.is_empty() {
            return;
        }
        for file in dormant {
            let record = &mut self.files[file.index()];
            *record = record.clone().in_context_role(SourceRole::Dormant);
            let debt = &mut self.file_debt[file.index()];
            *debt = FileDebt::new(*file, 0, Rating::Healthy, debt.touches());
        }
        for finding in &mut self.findings {
            if dormant.contains(&finding.file()) {
                *finding = finding
                    .clone()
                    .with_evidence(SourceRole::Dormant, finding.trust());
            }
        }
        self.size_findings
            .retain(|finding| !dormant.contains(&finding.file()));
    }

    pub(crate) fn add_general_diagnostic(
        &mut self,
        kind: DiagnosticKind,
        message: impl Into<String>,
    ) {
        let id = DiagnosticId::from_index(self.diagnostics.len());
        self.diagnostics
            .push(Diagnostic::new(id, None, kind, message, 0));
    }

    /// Composes the report, joining the scope facts that need the one
    /// directory tree this report streamed its history against: the tree is
    /// borrowed rather than rebuilt, so the scope a fact is stated at and the
    /// directory it was accumulated under can never disagree.
    pub(crate) fn finish(mut self, work: &AnalysisWork, directories: &DirectoryTree) -> Report {
        let architecture = build_architecture(
            work,
            GraphInputs {
                files: &self.files,
                dependencies: &self.dependencies,
            },
            &self.aliases,
            PackageTables {
                side_roots: &self.package_roots,
                roots: &self.package_roots,
                manifests: ManifestFacts {
                    names: &self.manifest_names,
                    paths: &self.manifest_paths,
                },
            },
            if self.evolution.coverage.availability() == HistoryAvailability::Unavailable
                || self.evolution.coverage.commits() == 0
            {
                WindowedHistory::Absent
            } else {
                WindowedHistory::Streamed
            },
        );
        self.restate_dormant_files(&architecture.dormant);
        let architecture_findings_for_links = architecture.findings.clone();
        let explanation_pairs = architecture.explanation_pairs.clone();
        self.disclose_skipped_closures(&architecture.skipped_closures);
        work.record_algorithm_pass();
        let containment = PackageContainment::from_paths(
            &self
                .package_roots
                .iter()
                .map(|root| report_package_path(root))
                .collect::<Vec<_>>(),
        );
        let (evolution, amplification) = self.evolution.accumulator.finish(
            self.evolution.coverage,
            self.files.len(),
            self.package_roots.len(),
            &containment,
            &explanation_pairs,
            None,
        );
        // The dependency join runs once, here, where the retained pairs and the
        // graphs both exist: history streamed before the graph was built, so
        // pair accumulation was graph-blind and this is the first point at
        // which a pair can be asked what depends on what.
        let (_, change_leakage_findings, suppressed_leakage) =
            leakage_findings(&architecture, &evolution, &self.files);
        let (suppressed_reach, suppressed_core) = suppressed_propagation(&architecture);
        let graph_evidence = architecture.graph_evidence.clone().with_suppressed(
            suppressed_reach,
            suppressed_core,
            suppressed_leakage,
        );
        // The scope join runs once, here, where the one directory tree the
        // histograms were filed under is still in scope: a rendered scope then
        // reads a table position rather than a tree.
        let scope_amplification =
            smackdebt_analysis::scope_amplification(&self.scopes, directories, &amplification);
        let evolutionary_findings = evolution.findings().to_vec();
        let root = ScopeId::from_index(0);
        let mut builder = AnalysisReportBuilder::with_capacity(
            ReportMode::Codebase,
            self.scopes.len(),
            self.files.len(),
            self.findings.len(),
            self.diagnostics.len(),
            0,
        );
        for scope in self.scopes {
            builder.add_scope(scope);
        }
        builder.set_root(root);
        for file in self.files {
            builder.add_file(file);
        }
        for finding in self.findings {
            builder.add_finding(finding);
        }
        for diagnostic in self.diagnostics {
            builder.add_diagnostic(diagnostic);
        }
        builder.set_architecture(ArchitectureReportFacts::new(
            ArchitectureGraph::new(
                architecture.coverage,
                architecture.file_edges,
                architecture.package_edges,
                architecture.external,
                architecture.diagnostics,
                architecture.measurements,
            ),
            architecture.findings,
            Vec::new(),
        ));
        builder.set_graph_evidence(graph_evidence);
        builder.set_propagation(
            architecture.package_closures,
            architecture.file_reach,
            architecture.core_size,
            if architecture.core_size.is_some() {
                architecture.core_members
            } else {
                Vec::new()
            },
        );
        builder.set_scope_amplification(scope_amplification);
        builder.set_hotspots(self.policies.hotspots.hotspots(&self.file_debt));
        builder.set_orphan_files(architecture.orphans);
        builder.set_stable_dependency_findings(architecture.stable_dependencies);
        builder.set_size_findings(self.size_findings);
        builder.set_evolution(evolution);
        builder.set_change_leakage_findings(change_leakage_findings);
        builder.set_explanation_pairs(explanation_pairs);
        link_evolutionary_findings(&mut builder, root, &self.packages, &evolutionary_findings);
        link_architecture_findings(
            &mut builder,
            &self.packages,
            architecture.finding_links,
            &architecture_findings_for_links,
        );
        builder.set_packages(self.packages);
        builder.finish()
    }
}

/// The propagation and core comparisons incomplete evidence suppresses.
fn suppressed_propagation(architecture: &ArchitectureBuild) -> (u32, u32) {
    let suppressed_reach = architecture
        .package_closures
        .iter()
        .filter(|closure| {
            !architecture
                .graph_evidence
                .package_is_complete(closure.package())
        })
        .count() as u32
        + u32::from(
            !architecture.graph_evidence.is_complete()
                && architecture
                    .measurements
                    .iter()
                    .map(|value| value.reach_in())
                    .max()
                    .and_then(|reach| {
                        smackdebt_analysis::PropagationReach::packages(
                            reach,
                            architecture.measurements.len() as u32,
                        )
                    })
                    .is_some(),
        );
    let suppressed_core =
        u32::from(!architecture.graph_evidence.is_complete() && architecture.core_size.is_some());
    (suppressed_reach, suppressed_core)
}

/// Links every evolutionary finding to the root and its package pair.
fn link_evolutionary_findings(
    builder: &mut AnalysisReportBuilder,
    root: ScopeId,
    packages: &[PackageRecord],
    findings: &[smackdebt_analysis::EvolutionaryFinding],
) {
    for finding in findings {
        let pair = finding.coupling();
        builder.link_evolutionary_finding(root, finding.id());
        builder.link_evolutionary_finding(packages[pair.left().index()].scope(), finding.id());
        builder.link_evolutionary_finding(packages[pair.right().index()].scope(), finding.id());
    }
}

/// Links every architecture finding to the scopes it names.
fn link_architecture_findings(
    builder: &mut AnalysisReportBuilder,
    packages: &[PackageRecord],
    links: Vec<(ScopeId, smackdebt_analysis::ArchitectureFindingId)>,
    findings: &[smackdebt_analysis::ArchitectureFinding],
) {
    for (scope, finding) in links {
        builder.link_architecture_finding(scope, finding);
    }
    for finding in findings {
        for package in finding.packages() {
            builder.link_architecture_finding(packages[package.index()].scope(), finding.id());
        }
        for file in finding.files() {
            builder.link_architecture_finding(builder.files()[file.index()].scope(), finding.id());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use crate::test_support::git;
    use std::fs;

    #[test]
    fn concentrated_package_knowledge_is_a_watch_finding_of_counts_only() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "owner@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Sole Owner"]);
        for revision in 0..10 {
            fs::write(
                repository_path.join("owned.rs"),
                format!("pub fn owned() -> i32 {{ {revision} }}\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let analyzed = analyze_codebase(&CodebaseRequest::new(repository_path)).unwrap();
        let report = analyzed.report();
        let findings = report.knowledge_concentration_findings();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rating(), Rating::Watch);
        let concentration = findings[0].concentration();
        assert_eq!(
            (
                concentration.contributor_count(),
                concentration.numerator(),
                concentration.denominator()
            ),
            (1, 10, 10)
        );
        assert!(report.evolutionary_findings().is_empty());
        // No contributor identity reaches any retained report value.
        let retained = format!("{report:?}");
        assert!(!retained.contains("Sole Owner"));
        assert!(!retained.contains("owner@example.invalid"));
    }
    #[test]
    fn hotspots_cross_rated_files_with_their_windowed_touch_count() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("cold.rs"), "pub fn cold() {}\n").unwrap();
        for revision in 0..5 {
            fs::write(
                repository_path.join("hot.rs"),
                format!("pub fn hot(value: i32) -> i32 {{ value + {revision} }}\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let report = analyze_codebase(&CodebaseRequest::new(repository_path)).unwrap();
        let report = report.report();
        let named = |file: FileId| report.files()[file.index()].path().to_owned();
        let hotspots: Vec<_> = report
            .hotspots()
            .iter()
            .map(|hotspot| (named(hotspot.file()), hotspot.touches()))
            .collect();
        assert_eq!(hotspots, [("hot.rs".to_owned(), 5)]);
        assert!(
            report.is_hotspot(
                report
                    .files()
                    .iter()
                    .find(|file| file.path() == "hot.rs")
                    .unwrap()
                    .id()
            )
        );

        let below_boundary = analyze_codebase(
            &CodebaseRequest::new(repository_path).with_minimum_hotspot_touches(6),
        )
        .unwrap();
        assert!(below_boundary.report().hotspots().is_empty());
    }
    #[test]
    fn file_and_container_size_are_rated_outside_the_unit_health_counts() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("big.rs"),
            "struct Worker;\nimpl Worker {\n    fn one(&self) {\n        let a = 1;\n        let b = 2;\n        let c = 3;\n    }\n    fn two(&self) {\n        let d = 4;\n        let e = 5;\n    }\n}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "size"]);

        let report = analyze_codebase(
            &CodebaseRequest::new(repository_path).with_size_thresholds((10, 20), (4, 6)),
        )
        .unwrap();
        let report = report.report();
        let findings: Vec<_> = report
            .size_findings()
            .iter()
            .map(|finding| {
                (
                    report.files()[finding.file().index()].path().to_owned(),
                    finding.subject(),
                    finding.container().map(str::to_owned),
                    finding.value(),
                    finding.rating(),
                )
            })
            .collect();
        assert_eq!(
            findings,
            [
                (
                    "big.rs".to_owned(),
                    smackdebt_analysis::SizeSubject::File,
                    None,
                    12,
                    Rating::Watch
                ),
                (
                    "big.rs".to_owned(),
                    smackdebt_analysis::SizeSubject::Container,
                    Some("Worker".to_owned()),
                    5,
                    Rating::Watch
                ),
            ]
        );
        // Size findings never enter the unit verdict counts.
        let root_scope = report.root().unwrap();
        assert_eq!(
            report.scopes()[root_scope.index()].health(),
            HealthCounts::new(2, 0, 0)
        );
    }
}
