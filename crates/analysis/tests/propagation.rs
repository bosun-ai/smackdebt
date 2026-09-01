//! The scope a propagation fact is stated at, over the public surface.
//!
//! One completed report answers every scope it holds, so these read a built
//! report rather than reaching into the analysis that filled it: the rule
//! under test is which scope states which fact, and that is what a consumer
//! sees.

use smackdebt_analysis::{
    ArchitectureGraph, ArchitectureReportFacts, CoreSize, DependencyCoverage,
    PackageGraphMeasurement, PackageId, PackageRecord, PropagationReach, Report, ReportBuilder,
    ReportMode, Scope, ScopeId, ScopeKind, close_over_packages,
};

/// Builds one report per case: one package scope per name holding the
/// files `sizes` gives it, chained one file into the next inside each
/// package and with no dependency between packages, one directory scope
/// under the first package, and the package reach-in counts `reach_in`
/// records.
///
/// Scope ids run root, one per package, then the directory.
fn packaged_report(
    names: &[&str],
    sizes: &[usize],
    reach_in: &[u32],
    core: Option<CoreSize>,
) -> Report {
    let mut builder = ReportBuilder::new(ReportMode::Codebase);
    let directory = ScopeId::from_index(names.len() + 1);
    let mut root = Scope::new(ScopeId::from_index(0), ScopeKind::Repository, ".", None);
    let (mut scopes, mut packages, mut graph) = (Vec::new(), Vec::new(), Vec::new());
    let (mut members, mut edges) = (Vec::new(), Vec::new());
    for (index, name) in names.iter().enumerate() {
        let (package, id) = (PackageId::from_index(index), ScopeId::from_index(index + 1));
        root.add_child(id);
        let mut scope = Scope::new(id, ScopeKind::Package, *name, Some(ScopeId::from_index(0)));
        if index == 0 {
            scope.add_child(directory);
        }
        scopes.push(scope);
        packages.push(PackageRecord::current(package, id, *name));
        graph.push(PackageGraphMeasurement::new(package, 0, 0).with_reach_in(reach_in[index]));
        let first = members.len();
        members.extend(std::iter::repeat_n(Some(package), sizes[index]));
        edges.extend((first..members.len().saturating_sub(1)).map(|file| (file, file + 1)));
    }
    scopes.push(Scope::new(
        directory,
        ScopeKind::Directory,
        "app/src",
        Some(ScopeId::from_index(1)),
    ));
    builder.add_scope(root);
    for scope in scopes {
        builder.add_scope(scope);
    }
    builder.set_root(ScopeId::from_index(0));
    builder.set_packages(packages);
    builder.set_architecture(ArchitectureReportFacts::new(
        ArchitectureGraph::new(
            DependencyCoverage::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            graph,
        ),
        Vec::new(),
        Vec::new(),
    ));
    let closures = close_over_packages(names.len(), &members, &edges);
    builder.set_propagation(closures.closures().to_vec(), Vec::new(), core, Vec::new());
    builder.finish()
}

/// The sentence one scope of a report states, if it states one.
fn reach_of(report: &Report, scope: usize) -> Option<String> {
    report
        .scope_verdict(ScopeId::from_index(scope))
        .reach()
        .map(PropagationReach::sentence)
}

/// One report answers every scope from the tables it was built with, so
/// the facts land where their scope rule allows them and nowhere else.
#[test]
fn the_propagation_facts_reach_the_root_and_a_package_scope_and_nothing_else() {
    // Twenty files chained inside the first package: the last is reached by
    // all twenty, counting itself.
    let report = packaged_report(
        &["app", "core", "web"],
        &[20, 0, 0],
        &[3, 1, 1],
        CoreSize::from_counts(34, 210),
    );
    let root = report.scope_verdict(ScopeId::from_index(0));
    assert_eq!(
        root.reach().map(PropagationReach::sentence),
        Some("A change in one package can reach 3 of 3 packages.".to_owned())
    );
    assert_eq!(
        root.core_size().map(CoreSize::sentence),
        Some("34 of 210 files sit in one dependency cycle.".to_owned())
    );
    assert_eq!(
        reach_of(&report, 1),
        Some("A change here can reach 20 of 20 files in this package.".to_owned())
    );
    assert!(
        report
            .scope_verdict(ScopeId::from_index(1))
            .core_size()
            .is_none(),
        "the core is a root fact"
    );
    // A package without a material closure, and a directory, state neither.
    for other in [2, 3, 4] {
        let verdict = report.scope_verdict(ScopeId::from_index(other));
        assert!(verdict.reach().is_none(), "scope {other} states no reach");
        assert!(
            verdict.core_size().is_none(),
            "scope {other} states no core"
        );
    }
    // Rendering a scope a second time reads the same tables and closes over
    // nothing, so the two answers are the same answer.
    let first = reach_of(&report, 1);
    assert_eq!(reach_of(&report, 1), first);
}

/// A repository of one package is that package, and no consumer can select
/// its package scope, so its root states the file reach rather than
/// nothing. A repository of several packages never borrows the sentence.
#[test]
fn a_repository_that_is_one_package_states_that_package_reach_at_its_root() {
    let one = packaged_report(&["."], &[20], &[1], None);
    assert_eq!(
        reach_of(&one, 0),
        Some("A change here can reach 20 of 20 files in this package.".to_owned())
    );
    // One file short of the floor there is no closure row and so no fact.
    let small = packaged_report(&["."], &[19], &[1], None);
    assert!(
        reach_of(&small, 0).is_none(),
        "a package below the file floor states nothing at any scope"
    );
    // Three independent packages have no package reach to state, and the
    // one material closure among them is not the repository's answer.
    let several = packaged_report(&["app", "core", "web"], &[20, 1, 1], &[1, 1, 1], None);
    assert!(
        reach_of(&several, 0).is_none(),
        "the root of a multi-package repository is not one of its packages"
    );
    assert_eq!(
        reach_of(&several, 1),
        Some("A change here can reach 20 of 20 files in this package.".to_owned()),
        "that package still answers for itself"
    );
}
