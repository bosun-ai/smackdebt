//! Where the repository's core is stated, over the terminal bytes a reader
//! sees.
//!
//! The core is the largest cycle of the whole file graph. It is a superlative
//! rather than an action, so it stays beside the cycle it names: these read
//! that it lands on that cycle's card, on no other card, and nowhere at all
//! when the graph it is a superlative over has a hole in it.

use smackdebt_analysis::{
    ArchitectureFinding, ArchitectureFindingId, ArchitectureFindingKind, ArchitectureGraph,
    ArchitectureReportFacts, CoreSize, Coverage, DependencyCoverage, DependencyEdge,
    DependencyEdgeId, FileId, FileRecord, GraphEvidence, HealthCounts, PackageId, PackageRecord,
    Report, ReportBuilder, ReportMode, Scope, ScopeId, ScopeKind, SourceSpan,
};
use smackdebt_output::{TerminalOptions, write_terminal};

const ROOT: ScopeId = ScopeId::from_index(0);
const PACKAGE: ScopeId = ScopeId::from_index(1);
/// Files zero through four close the core; five and six close a smaller cycle.
const CORE: [usize; 5] = [0, 1, 2, 3, 4];
const KNOT: [usize; 2] = [5, 6];
const FILES: usize = 10;

/// Ten files in one package holding two cycles, with the core the composition
/// found joined on when `stated`, and one package left unread when `holed`.
///
/// The two cycles differ in size, so a card stating the core's words for the
/// smaller one is a wrong claim rather than the same claim twice.
fn report(stated: bool, holed: bool) -> Report {
    let mut builder = ReportBuilder::new(ReportMode::Codebase);
    let mut repository = Scope::new(ROOT, ScopeKind::Repository, ".", None);
    repository.add_child(PACKAGE);
    builder.add_scope(repository);
    builder.add_scope(Scope::new(PACKAGE, ScopeKind::Package, "app", Some(ROOT)));
    builder.set_root(ROOT);
    let package = PackageId::from_index(0);
    builder.set_packages(vec![PackageRecord::current(package, PACKAGE, "app")]);
    for index in 0..FILES {
        let id = FileId::from_index(index);
        builder.add_file(
            FileRecord::new(
                id,
                PACKAGE,
                format!("app/unit{index}.rs"),
                Coverage::new(1, 1, 0, 0, 10, 0),
                HealthCounts::default(),
            )
            .with_package(package),
        );
        builder.link_file(PACKAGE, id);
    }
    let mut edges = Vec::new();
    let ring = |members: &[usize], edges: &mut Vec<DependencyEdge>| -> Vec<DependencyEdgeId> {
        members
            .iter()
            .enumerate()
            .map(|(step, &member)| {
                let id = DependencyEdgeId::from_index(edges.len());
                edges.push(DependencyEdge::new(
                    id,
                    FileId::from_index(member),
                    FileId::from_index(members[(step + 1) % members.len()]),
                    1,
                    vec![SourceSpan::new(1, 1)],
                ));
                id
            })
            .collect()
    };
    let core_witness = ring(&CORE, &mut edges);
    let knot_witness = ring(&KNOT, &mut edges);
    let cycle = |index: usize, members: &[usize], witness: Vec<DependencyEdgeId>| {
        ArchitectureFinding::new(
            ArchitectureFindingId::from_index(index),
            ArchitectureFindingKind::FileCycle,
            vec![package],
            members
                .iter()
                .map(|&member| FileId::from_index(member))
                .collect(),
            witness,
        )
    };
    builder.set_architecture(ArchitectureReportFacts::new(
        ArchitectureGraph::new(
            DependencyCoverage::new(FILES as u32, 0, 0, 0, 0, 0, 0),
            edges,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
        vec![cycle(0, &CORE, core_witness), cycle(1, &KNOT, knot_witness)],
        Vec::new(),
    ));
    for index in 0..2 {
        builder.link_architecture_finding(ROOT, ArchitectureFindingId::from_index(index));
    }
    if holed {
        // One package whose imports could not all be followed is a hole in the
        // one graph the core is a superlative over, and the report counts the
        // core it therefore withholds.
        builder.set_graph_evidence(
            GraphEvidence::new(vec![package], 0, 1, 0, Vec::new()).with_suppressed(0, 1, 0),
        );
    }
    if stated {
        builder.set_propagation(
            Vec::new(),
            Vec::new(),
            CoreSize::from_counts(CORE.len() as u32, FILES as u32),
            CORE.iter()
                .map(|&member| FileId::from_index(member))
                .collect(),
        );
    }
    builder.finish()
}

fn render(report: &Report) -> String {
    let mut bytes = Vec::new();
    write_terminal(&mut bytes, report, Some(ROOT), TerminalOptions::default()).unwrap();
    String::from_utf8(bytes).unwrap()
}

/// The core lands on the card of its own cycle, in the words analysis froze on
/// the value, and the other cycle keeps its own size.
#[test]
fn the_largest_cycle_says_so_and_every_other_cycle_states_its_size() {
    let terminal = render(&report(true, false));
    assert!(
        terminal.contains("        5 of 10 files sit in one dependency cycle.\n"),
        "{terminal}"
    );
    assert!(
        terminal.contains("        2 files in the cycle\n"),
        "{terminal}"
    );
    assert!(!terminal.contains("5 files in the cycle"), "{terminal}");
    // The core is stated once, beside one subject, rather than on every card
    // whose cycle happens to be that size.
    assert_eq!(terminal.matches("sit in one dependency cycle").count(), 1);
}

/// A report whose composition found no material core states two cycle sizes
/// and no core anywhere.
#[test]
fn a_report_without_a_core_states_only_the_sizes_of_its_cycles() {
    let terminal = render(&report(false, false));
    assert!(
        !terminal.contains("sit in one dependency cycle"),
        "{terminal}"
    );
    assert!(
        terminal.contains("        5 files in the cycle\n"),
        "{terminal}"
    );
    assert!(
        terminal.contains("        2 files in the cycle\n"),
        "{terminal}"
    );
}

/// A graph with a hole in it states no core: an import that could not be
/// followed could grow a cycle or make a different one the largest. What the
/// report counts as withheld and what it states therefore agree.
#[test]
fn an_incomplete_graph_withholds_the_core_it_counts_as_withheld() {
    let holed = report(true, true);
    assert_eq!(holed.graph_evidence().suppressed_core(), 1);
    let terminal = render(&holed);
    assert!(
        !terminal.contains("sit in one dependency cycle"),
        "{terminal}"
    );
    assert!(
        terminal.contains("        5 files in the cycle\n"),
        "{terminal}"
    );
}

/// The card states the fact whole at fifty columns, because a core shortened
/// into an ellipsis states nothing.
#[test]
fn a_narrow_terminal_states_the_whole_core() {
    let stated = report(true, false);
    let mut bytes = Vec::new();
    write_terminal(
        &mut bytes,
        &stated,
        Some(ROOT),
        TerminalOptions::new(50, false, false),
    )
    .unwrap();
    let terminal = String::from_utf8(bytes).unwrap();
    assert!(
        terminal.contains("        5 of 10 files sit in one dependency cycle.\n"),
        "{terminal}"
    );
    assert!(!terminal.contains('…'), "{terminal}");
}

/// The core is stated where a reader can act on it and nowhere else: the
/// verdict head names no subject, so it never carries the fact.
#[test]
fn the_verdict_head_still_states_no_core() {
    let terminal = render(&report(true, false));
    let head = terminal.split("PROBLEMS").next().expect("a verdict head");
    assert!(!head.contains("dependency cycle"), "{head}");
}
