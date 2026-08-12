use std::alloc::System;

use smackdebt_analysis::{
    Coverage, FileId, FileRecord, FindingId, HealthCounts, HealthPolicy, Measurements, Scope,
    ScopeId, ScopeKind, aggregate_scopes,
};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn rating_and_reserved_aggregation_do_not_allocate() {
    let root_id = ScopeId::from_index(0);
    let file_scope_id = ScopeId::from_index(1);
    let file_id = FileId::from_index(0);
    let finding_id = FindingId::from_index(0);
    let mut root = Scope::new(root_id, ScopeKind::Repository, ".", None);
    root.add_child(file_scope_id);
    root.reserve_findings(1);
    let mut file_scope = Scope::new(file_scope_id, ScopeKind::File, "src/lib.rs", Some(root_id));
    file_scope.add_file(file_id);
    file_scope.add_finding(finding_id);
    let mut scopes = vec![root, file_scope];
    let files = vec![FileRecord::new(
        file_id,
        file_scope_id,
        "src/lib.rs",
        Coverage::new(1, 1, 0, 0, 20, 0),
        HealthCounts::new(0, 1, 0),
    )];
    let policy = HealthPolicy::default();

    let region = Region::new(ALLOCATOR);
    let assessment = policy.assess(Measurements::new(16, 2, 20));
    aggregate_scopes(&mut scopes, &files, root_id);
    let allocations = region.change();

    assert_eq!(assessment.rating(), smackdebt_analysis::Rating::Watch);
    assert_eq!(allocations.allocations, 0, "{allocations:?}");
    assert_eq!(allocations.reallocations, 0, "{allocations:?}");
}
