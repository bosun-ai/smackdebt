use smackdebt_analysis::{
    DependencySyntax, FileAnalysis, LocalUnitId, Measurements, ParseStatus, RecoveredFacts,
    SourceSpan, UnitFact, UnitIdentity, UnitKind, UnitMatchEvidence,
};
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

use crate::cognitive_complexity::CognitiveComplexity;
use crate::cyclomatic_complexity::CyclomaticComplexity;
use crate::language::Language;
use crate::logical_lines::LogicalLines;
use crate::semantic::Syntax;

#[derive(Clone, Copy)]
pub(super) struct Observation {
    pub(super) syntax: Syntax,
    pub(super) nesting: u32,
}

pub(super) struct Scratch {
    pub(super) observations: Vec<Observation>,
    pub(super) nesting_by_depth: Vec<u32>,
    unit_by_depth: Vec<Option<usize>>,
    expression_nesting: Vec<u32>,
    unit_drafts: Vec<UnitDraft>,
    dependencies: Vec<DependencySyntax>,
    fact_bytes: Vec<ByteRange>,
    error_bytes: Vec<ByteRange>,
    queries: [Option<QueryState>; 11],
}

impl Default for Scratch {
    fn default() -> Self {
        Self {
            observations: Vec::new(),
            nesting_by_depth: Vec::new(),
            unit_by_depth: Vec::new(),
            expression_nesting: Vec::new(),
            unit_drafts: Vec::new(),
            dependencies: Vec::new(),
            fact_bytes: Vec::new(),
            error_bytes: Vec::new(),
            queries: std::array::from_fn(|_| None),
        }
    }
}

/// Half-open source byte span of one syntax node.
#[derive(Clone, Copy)]
pub(super) struct ByteRange {
    start: usize,
    end: usize,
}

impl ByteRange {
    /// The span a node occupies.
    ///
    /// A `MISSING` node has no width: it marks the point the parser invented a
    /// token at. That point is a boundary, so the span reaches one byte to
    /// either side of it - a closer invented at the end of a block sits
    /// exactly where the block it grew ends, and the unit that swallowed the
    /// rest of the file has to see it.
    pub(super) fn of(node: Node<'_>) -> Self {
        let start = node.start_byte();
        let end = node.end_byte();
        if start == end {
            return Self {
                start: start.saturating_sub(1),
                end: end + 1,
            };
        }
        Self { start, end }
    }

    const fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

struct QueryState {
    query: Query,
    cursor: QueryCursor,
}

struct UnitDraft {
    identity: UnitIdentity,
    declared_identity: bool,
    match_evidence: UnitMatchEvidence,
    span: SourceSpan,
    measurements: Measurements,
    parent: Option<usize>,
}

pub(super) fn append_expression<L: Language>(
    parser: &mut Parser,
    source: &[u8],
    base_nesting: u32,
    scratch: &mut Scratch,
) -> Result<bool, String> {
    parser
        .set_language(&L::grammar())
        .map_err(|error| format!("language setup failed: {error}"))?;
    let started = std::time::Instant::now();
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "tree-sitter returned no syntax tree".to_owned())?;
    crate::analyzer::record_parser_time(started.elapsed());
    scratch.expression_nesting.clear();
    walk(tree.root_node(), |node, depth| {
        scratch.expression_nesting.truncate(depth);
        let nesting = scratch
            .expression_nesting
            .last()
            .copied()
            .unwrap_or(base_nesting);
        let mut syntax = L::classify(node, source, false).syntax;
        syntax.logical_statement = false;
        scratch.observations.push(Observation { syntax, nesting });
        scratch
            .expression_nesting
            .push(nesting + u32::from(syntax.nests));
        true
    });
    Ok(tree.root_node().has_error())
}

pub(super) fn analyze<L: Language>(
    parser: &mut Parser,
    source: &[u8],
    scratch: &mut Scratch,
) -> Result<FileAnalysis, String> {
    parser
        .set_language(&L::grammar())
        .map_err(|error| format!("language setup failed: {error}"))?;
    let started = std::time::Instant::now();
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "tree-sitter returned no syntax tree".to_owned())?;
    crate::analyzer::record_parser_time(started.elapsed());
    let root = tree.root_node();
    reserve_unit_capacity::<L>(root, source, scratch)?;
    let (units, dependencies) = collect_units::<L>(root, source, 0, 0, scratch);
    Ok(FileAnalysis::with_dependencies(
        L::REPORT_LANGUAGE,
        line_count(source),
        parse_status(root, scratch),
        units,
        dependencies,
    ))
}

/// The parse outcome for a tree whose facts were just collected into `scratch`.
///
/// Recovery only clouds the facts when an error region sits on one, so the
/// recorded error spans decide what the recovery cost.
pub(super) fn parse_status(root: Node<'_>, scratch: &Scratch) -> ParseStatus {
    if !root.has_error() {
        return ParseStatus::Parsed;
    }
    ParseStatus::Recovered(recovered_facts(&scratch.error_bytes, &scratch.fact_bytes))
}

/// Whether the recovery cost this parse anything it was read for.
///
/// The comparison can only speak for facts that survived: an error that
/// destroyed a unit outright leaves no span to overlap. The empty-facts rule
/// below catches a parse that lost everything, but one that keeps a single
/// import while every unit is gone still reads as `Intact`. That blind spot is
/// deliberate - closing it would take a guess at what the errors ate - and the
/// file discloses its imperfect parse whatever this answers.
fn recovered_facts(errors: &[ByteRange], facts: &[ByteRange]) -> RecoveredFacts {
    // A parse that yielded nothing offers no ground to place its errors
    // against: whatever the errors swallowed is exactly what is missing.
    if facts.is_empty() {
        return RecoveredFacts::InDoubt;
    }
    if errors
        .iter()
        .any(|error| facts.iter().any(|fact| error.overlaps(*fact)))
    {
        RecoveredFacts::InDoubt
    } else {
        RecoveredFacts::Intact
    }
}

/// The outcome of a parse whose facts are assembled outside this engine, such
/// as the Vue document tree that holds one template unit and hands every
/// reference out of its script elements.
pub(super) fn document_status(
    root: Node<'_>,
    facts: &[ByteRange],
    scratch: &mut Scratch,
) -> ParseStatus {
    if !root.has_error() {
        return ParseStatus::Parsed;
    }
    scratch.error_bytes.clear();
    walk(root, |node, _| {
        if node.is_error() || node.is_missing() {
            scratch.error_bytes.push(ByteRange::of(node));
            return false;
        }
        node.has_error()
    });
    ParseStatus::Recovered(recovered_facts(&scratch.error_bytes, facts))
}

pub(super) fn analyze_included<L: Language>(
    parser: &mut Parser,
    source: &[u8],
    line_offset: u32,
    id_offset: usize,
    scratch: &mut Scratch,
) -> Result<(ParseStatus, Vec<UnitFact>, Vec<DependencySyntax>), String> {
    parser
        .set_language(&L::grammar())
        .map_err(|error| format!("language setup failed: {error}"))?;
    let started = std::time::Instant::now();
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "tree-sitter returned no syntax tree".to_owned())?;
    crate::analyzer::record_parser_time(started.elapsed());
    let root = tree.root_node();
    reserve_unit_capacity::<L>(root, source, scratch)?;
    let (units, dependencies) = collect_units::<L>(root, source, line_offset, id_offset, scratch);
    Ok((parse_status(root, scratch), units, dependencies))
}

pub(super) fn reserve_unit_capacity<L: Language>(
    root: Node<'_>,
    source: &[u8],
    scratch: &mut Scratch,
) -> Result<(), String> {
    let slot = language_slot(L::REPORT_LANGUAGE);
    if scratch.queries[slot].is_none() {
        let query = Query::new(&L::grammar(), L::unit_query())
            .map_err(|error| format!("language query failed: {error}"))?;
        scratch.queries[slot] = Some(QueryState {
            query,
            cursor: QueryCursor::new(),
        });
    }
    let state = scratch.queries[slot].as_mut().expect("query initialized");
    let mut matches = state.cursor.matches(&state.query, root, source);
    let mut units = 0usize;
    while let Some(found) = matches.next() {
        units += found.captures.len();
    }
    scratch.unit_drafts.reserve(units);
    Ok(())
}

const fn language_slot(language: smackdebt_analysis::Language) -> usize {
    match language {
        smackdebt_analysis::Language::C => 0,
        smackdebt_analysis::Language::Cpp => 1,
        smackdebt_analysis::Language::Java => 2,
        smackdebt_analysis::Language::JavaScript => 3,
        smackdebt_analysis::Language::Jsx => 4,
        smackdebt_analysis::Language::Python => 5,
        smackdebt_analysis::Language::Rust => 6,
        smackdebt_analysis::Language::TypeScript => 7,
        smackdebt_analysis::Language::Tsx => 8,
        smackdebt_analysis::Language::Ruby => 9,
        smackdebt_analysis::Language::Vue => 10,
        smackdebt_analysis::Language::Astro
        | smackdebt_analysis::Language::Kotlin
        | smackdebt_analysis::Language::Unknown => 10,
    }
}

fn collect_units<L: Language>(
    root: Node<'_>,
    source: &[u8],
    line_offset: u32,
    id_offset: usize,
    scratch: &mut Scratch,
) -> (Vec<UnitFact>, Vec<DependencySyntax>) {
    scratch.unit_drafts.clear();
    scratch.dependencies.clear();
    scratch.unit_by_depth.clear();
    scratch.fact_bytes.clear();
    scratch.error_bytes.clear();
    // Only a recovered parse has to answer what its errors touched, so clean
    // files never pay for the spans.
    let recovered = root.has_error();
    walk(root, |node, depth| {
        scratch.unit_by_depth.truncate(depth);
        let parent = scratch.unit_by_depth.last().copied().flatten();
        let classification = L::classify(node, source, true);
        let carries_fact = classification.unit.is_some() || classification.dependency.is_some();
        if recovered {
            record_recovery_spans(node, carries_fact, scratch);
        }
        if let Some(dependency) = classification.dependency {
            scratch
                .dependencies
                .push(offset_dependency(dependency, line_offset));
        }
        let child_parent = match classification.unit {
            Some(kind) => {
                let draft = unit_draft::<L>(
                    &UnitSite {
                        node,
                        source,
                        kind,
                        parent,
                        line_offset,
                    },
                    scratch,
                );
                scratch.unit_drafts.push(draft);
                Some(scratch.unit_drafts.len() - 1)
            }
            None => parent,
        };
        scratch.unit_by_depth.push(child_parent);
        true
    });
    let mut units = Vec::with_capacity(scratch.unit_drafts.len());
    units.extend(
        scratch
            .unit_drafts
            .drain(..)
            .enumerate()
            .map(|(index, draft)| {
                UnitFact::new(
                    LocalUnitId::from_index(index + id_offset),
                    draft.identity,
                    draft.span,
                    draft.measurements,
                    draft
                        .parent
                        .map(|parent| LocalUnitId::from_index(parent + id_offset)),
                )
                .with_match_evidence(draft.match_evidence)
            }),
    );
    let mut dependencies = Vec::with_capacity(scratch.dependencies.len());
    dependencies.append(&mut scratch.dependencies);
    (units, dependencies)
}

/// Records the spans a recovered parse is judged by: where its errors sit and
/// where it carried a fact out.
fn record_recovery_spans(node: Node<'_>, carries_fact: bool, scratch: &mut Scratch) {
    if node.is_error() || node.is_missing() {
        scratch.error_bytes.push(ByteRange::of(node));
    }
    if carries_fact {
        scratch.fact_bytes.push(ByteRange::of(node));
    }
}

/// Where one unit sits in the document being collected.
struct UnitSite<'tree, 'source> {
    node: Node<'tree>,
    source: &'source [u8],
    kind: UnitKind,
    parent: Option<usize>,
    line_offset: u32,
}

fn unit_draft<L: Language>(site: &UnitSite<'_, '_>, scratch: &mut Scratch) -> UnitDraft {
    let UnitSite {
        node,
        source,
        kind,
        parent,
        line_offset,
    } = *site;
    let declared_identity = L::has_declared_identity(node, source);
    let measurements = measure::<L>(node, source, scratch);
    let declared_container = enclosing_container::<L>(node, source);
    let display_container = declared_container
        .clone()
        .or_else(|| parent.map(|parent| scratch.unit_drafts[parent].identity.name().to_owned()));
    let name = L::name(node, source);
    let identity = match display_container {
        Some(container) => UnitIdentity::new(name, kind).in_container(container),
        None => UnitIdentity::new(name, kind),
    };
    let enclosing_declared = nearest_declared_unit(parent, &scratch.unit_drafts).cloned();
    let match_evidence = if declared_identity {
        UnitMatchEvidence::declared()
    } else {
        inferred_match_evidence::<L>(
            site,
            declared_container.as_deref(),
            enclosing_declared.as_ref(),
        )
    };
    UnitDraft {
        identity,
        declared_identity,
        match_evidence,
        span: SourceSpan::new(
            node.start_position().row as u32 + 1 + line_offset,
            node.end_position().row as u32 + 1 + line_offset,
        ),
        measurements,
        parent,
    }
}

/// How a unit that declares no identity of its own is paired across versions.
///
/// A unit carries every kind of evidence it has, not the best one: an anchor
/// follows the unit through an edit, its exact syntax follows it through a
/// move, and the matcher needs the second when an anchor is shared by sibling
/// units it cannot otherwise tell apart.
fn inferred_match_evidence<L: Language>(
    site: &UnitSite<'_, '_>,
    declared_container: Option<&str>,
    enclosing_declared: Option<&UnitIdentity>,
) -> UnitMatchEvidence {
    let evidence =
        L::match_anchor(site.node, site.source).map_or_else(UnitMatchEvidence::none, |anchor| {
            UnitMatchEvidence::semantic(declared_container, enclosing_declared, site.kind, anchor)
        });
    match site
        .source
        .get(site.node.start_byte()..site.node.end_byte())
    {
        Some(syntax) => evidence.with_exact_syntax(syntax),
        None => evidence,
    }
}

fn nearest_declared_unit(mut parent: Option<usize>, drafts: &[UnitDraft]) -> Option<&UnitIdentity> {
    while let Some(index) = parent {
        let draft = &drafts[index];
        if draft.declared_identity {
            return Some(&draft.identity);
        }
        parent = draft.parent;
    }
    None
}

fn offset_dependency(dependency: DependencySyntax, line_offset: u32) -> DependencySyntax {
    if line_offset == 0 {
        return dependency;
    }
    let span = dependency.span();
    dependency.with_span(SourceSpan::new(
        span.start_line() + line_offset,
        span.end_line() + line_offset,
    ))
}

fn measure<L: Language>(root: Node<'_>, source: &[u8], scratch: &mut Scratch) -> Measurements {
    scratch.observations.clear();
    scratch.nesting_by_depth.clear();
    walk(root, |node, depth| {
        scratch.nesting_by_depth.truncate(depth);
        let nesting = scratch.nesting_by_depth.last().copied().unwrap_or(0);
        let classification = L::classify(node, source, false);
        if node.id() != root.id() && classification.unit.is_some() {
            return false;
        }
        let syntax = classification.syntax;
        scratch.observations.push(Observation { syntax, nesting });
        scratch
            .nesting_by_depth
            .push(nesting + u32::from(syntax.nests));
        true
    });
    let mut metrics = MetricState::default();
    for observation in scratch.observations.iter().copied() {
        metrics.observe(observation.syntax, observation.nesting);
    }
    metrics.finish(L::parameter_count(root, source))
}

#[derive(Default)]
pub(super) struct MetricState {
    cognitive: CognitiveComplexity,
    cyclomatic: CyclomaticComplexity,
    logical: LogicalLines,
    max_nesting: u32,
}

impl MetricState {
    pub(super) fn observe(&mut self, syntax: crate::semantic::Syntax, nesting: u32) {
        if syntax.logical_statement {
            self.cognitive.begin_statement();
        }
        self.cognitive.observe(syntax.cognitive, nesting);
        self.cognitive.observe(syntax.secondary_cognitive, nesting);
        self.cyclomatic.observe(syntax.decision);
        self.logical.observe(syntax.logical_statement);
        // Depth starts at zero at the unit body and grows through the same
        // nesting events cognitive complexity already uses.
        let depth = nesting + u32::from(syntax.nests);
        if depth > self.max_nesting {
            self.max_nesting = depth;
        }
    }

    pub(super) fn finish(self, parameter_count: u32) -> Measurements {
        Measurements::new(
            self.cognitive.finish(),
            self.cyclomatic.finish(),
            self.logical.finish(),
        )
        .with_shape(self.max_nesting, parameter_count)
    }
}

fn enclosing_container<L: Language>(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if L::is_container(candidate) {
            return Some(L::name(candidate, source));
        }
        parent = candidate.parent();
    }
    None
}

pub(super) fn walk<'tree>(root: Node<'tree>, mut visit: impl FnMut(Node<'tree>, usize) -> bool) {
    let mut cursor = root.walk();
    let mut depth = 0usize;
    loop {
        if visit(cursor.node(), depth) && cursor.goto_first_child() {
            depth += 1;
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return;
            }
            depth -= 1;
        }
    }
}

fn line_count(source: &[u8]) -> u32 {
    if source.is_empty() {
        0
    } else {
        source.iter().filter(|&&byte| byte == b'\n').count() as u32
            + u32::from(source.last() != Some(&b'\n'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rust_language::Rust;

    #[test]
    fn consecutive_files_reuse_query_cursor_traversal_and_result_capacity() {
        let mut parser = Parser::new();
        let mut scratch = Scratch::default();
        analyze::<Rust>(&mut parser, b"fn one() { if true {} }", &mut scratch).unwrap();
        let query = scratch.queries[language_slot(smackdebt_analysis::Language::Rust)]
            .as_ref()
            .unwrap();
        let cursor_address = std::ptr::from_ref(&query.cursor);
        let observation_capacity = scratch.observations.capacity();
        let result_capacity = scratch.unit_drafts.capacity();

        analyze::<Rust>(&mut parser, b"fn two() { while false {} }", &mut scratch).unwrap();

        let query = scratch.queries[language_slot(smackdebt_analysis::Language::Rust)]
            .as_ref()
            .unwrap();
        assert_eq!(std::ptr::from_ref(&query.cursor), cursor_address);
        assert!(scratch.observations.capacity() >= observation_capacity);
        assert!(scratch.unit_drafts.capacity() >= result_capacity);
        assert!(!scratch.nesting_by_depth.is_empty());
    }

    #[test]
    fn moving_a_reference_to_its_document_position_keeps_every_evidence_field() {
        let dependency = DependencySyntax::new(
            smackdebt_analysis::DependencyKind::Import,
            "crate::helper",
            SourceSpan::new(2, 2),
            smackdebt_analysis::DependencySyntaxState::Candidates(vec!["helper.rs".to_owned()]),
        )
        .with_internal_intent()
        .with_relation(smackdebt_analysis::StaticRelationKind::ModuleOwnership)
        .with_test_scope();

        let moved = offset_dependency(dependency.clone(), 3);

        assert_eq!(moved.span(), SourceSpan::new(5, 5));
        assert_eq!(moved.scope(), smackdebt_analysis::DependencyScope::Test);
        assert_eq!(
            moved.intent(),
            smackdebt_analysis::DependencyIntent::Internal
        );
        assert_eq!(
            moved.relation(),
            smackdebt_analysis::StaticRelationKind::ModuleOwnership
        );
        assert_eq!(offset_dependency(dependency.clone(), 0), dependency);
    }

    fn shapes(source: &str) -> Vec<(u32, u32)> {
        let mut parser = Parser::new();
        let mut scratch = Scratch::default();
        analyze::<Rust>(&mut parser, source.as_bytes(), &mut scratch)
            .unwrap()
            .units()
            .iter()
            .map(|unit| {
                (
                    unit.measurements().max_nesting(),
                    unit.measurements().parameter_count(),
                )
            })
            .collect()
    }

    #[test]
    fn maximum_nesting_counts_the_deepest_entered_nesting_events() {
        assert_eq!(
            shapes("fn work(a: i32) { if a > 0 { while a > 1 { for _ in 0..a {} } } }"),
            [(3, 1)]
        );
    }

    #[test]
    fn a_flat_unit_reports_no_nesting_and_its_declared_parameters() {
        assert_eq!(shapes("fn work(a: i32, b: i32) -> i32 { a + b }"), [(0, 2)]);
        assert_eq!(shapes("fn work() {}"), [(0, 0)]);
    }

    #[test]
    fn a_separately_rated_closure_keeps_its_own_depth_out_of_its_parent() {
        assert_eq!(
            shapes("fn work(a: i32) { let deep = |b: i32| if b > 0 { while b > 1 {} }; }"),
            [(0, 1), (2, 1)]
        );
    }
}
