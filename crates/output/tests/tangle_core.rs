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
/// The first thirty-four files close the core; the next two close a smaller
/// cycle. The counts are chosen so the core's card line is fifty-one columns
/// wide with its indent, which is past the narrowest width this product
/// supports: a fixture whose line happened to fit would prove nothing about
/// the width the narrow test exists for.
const CORE: usize = 34;
const KNOT: usize = 2;
const FILES: usize = 210;

fn core() -> Vec<usize> {
    (0..CORE).collect()
}

fn knot() -> Vec<usize> {
    (CORE..CORE + KNOT).collect()
}

/// Two hundred and ten files in one package holding two cycles, with the core
/// the composition found joined on when `stated`, and one package left unread
/// when `holed`.
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
    let (core, knot) = (core(), knot());
    let core_witness = ring(&core, &mut edges);
    let knot_witness = ring(&knot, &mut edges);
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
        vec![cycle(0, &core, core_witness), cycle(1, &knot, knot_witness)],
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
            CoreSize::from_counts(CORE as u32, FILES as u32),
            core.iter()
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
    let stated = report(true, false);
    // A complete graph withholds nothing, so a stated core is never also a
    // counted one: the two halves of the gate are read from the same report.
    assert_eq!(stated.graph_evidence().suppressed_core(), 0);
    let terminal = render(&stated);
    assert!(
        terminal.contains("        34 of 210 files sit in one dependency cycle\n"),
        "{terminal}"
    );
    assert!(
        terminal.contains("        2 files in the cycle\n"),
        "{terminal}"
    );
    assert!(!terminal.contains("34 files in the cycle"), "{terminal}");
    // The core is stated once, beside one subject, rather than on every card
    // whose cycle happens to be that size.
    assert_eq!(terminal.matches("sit in one dependency cycle").count(), 1);
}

/// The card states the core as a fragment, because every line it stacks is
/// one: a closed sentence among lowercase fragments reads as a different kind
/// of claim than the lines around it. The sentence analysis owns keeps its
/// stop for the consumers that state it as a sentence.
#[test]
fn the_card_states_the_core_in_the_fragment_style_its_neighbours_use() {
    let stated = report(true, false);
    let terminal = render(&stated);
    assert!(
        !terminal.contains("sit in one dependency cycle."),
        "{terminal}"
    );
    let core = CoreSize::from_counts(CORE as u32, FILES as u32).expect("a material core");
    assert_eq!(core.sentence(), format!("{}.", core.fragment()));
    assert!(terminal.contains(&core.fragment()), "{terminal}");
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
        terminal.contains("        34 files in the cycle\n"),
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
        terminal.contains("        34 files in the cycle\n"),
        "{terminal}"
    );
}

/// The card states the fact whole at fifty columns, because a core shortened
/// into an ellipsis states nothing.
///
/// This line is fifty-one columns wide with its indent, so the width is
/// genuinely exceeded and the renderer has to break it. Nothing may be lost
/// across that break: the words of the fact, in order, are what the two
/// lines hold between them.
#[test]
fn a_narrow_terminal_breaks_the_core_without_losing_it() {
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
    assert!(!terminal.contains('…'), "{terminal}");
    let fact = "34 of 210 files sit in one dependency cycle";
    assert!(
        !terminal.contains(fact),
        "the fixture must exceed fifty columns or this test proves nothing: {terminal}"
    );
    let broken: Vec<&str> = terminal
        .lines()
        .skip_while(|line| !line.contains("34 of 210"))
        .take(2)
        .collect();
    assert_eq!(
        broken
            .iter()
            .flat_map(|line| line.split_whitespace())
            .collect::<Vec<_>>(),
        fact.split(' ').collect::<Vec<_>>(),
        "{terminal}"
    );
    for line in &broken {
        assert!(line.chars().count() <= 50, "{line:?} in {terminal}");
    }
}

/// The core is stated where a reader can act on it and nowhere else: the
/// verdict head names no subject, so it never carries the fact.
#[test]
fn the_verdict_head_still_states_no_core() {
    let terminal = render(&report(true, false));
    let head = terminal.split("PROBLEMS").next().expect("a verdict head");
    assert!(!head.contains("dependency cycle"), "{head}");
}
