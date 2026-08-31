//! The invariant that keeps dependency edges out of every human view.
//!
//! A relationship is a graph fact, not a decision, so no human view may state
//! one as a row at any scope, in any mode, or at any detail level. The check is
//! an invariant applied to every terminal result the suites produce and to
//! every committed terminal result, rather than an assertion on selected cases,
//! so a future view cannot reintroduce the rows quietly.
//!
//! # The one carve-out
//!
//! A co-change finding names the two files it is *about*: a `hidden_coupling`
//! card heads `<left> ↔ <right>`, and a leaking interface's evidence names the
//! follower that changed with it. A finding's subject is the identity of the
//! thing measured, not a graph row — the same principle that already lets a
//! cycle witness print file paths and lets an `unstable_dependency` card head
//! on a package pair. The carve-out is limited to the two file identities, so
//! [`assert_no_dependency_edge_rows`] still proves that no reference count,
//! relation kind, resolution outcome, or ownership wording travels with them.
//!
//! # The blindspot the carve-out did not need
//!
//! Mechanically, the ban half of this check never saw a `↔` pair: it looks for
//! two file paths joined by an arrow, by ` owns `, or for the single-reference
//! import fact. That is deliberate rather than an oversight. Every row the
//! deleted relation sections wrote was directed — `<source> → <target> ·
//! 1 import` and `<source> owns <target>` — because a relation has a direction
//! and a row that dropped it would state nothing. `↔` is the wording this
//! product reserves for a symmetric fact, so an edge row cannot return through
//! it without first ceasing to be an edge. The shape is not left unchecked
//! either: a `↔` row of two file paths is exactly the row
//! [`assert_no_relation_facts`] proves carries none of the facts an edge row
//! carried.

/// The indent every wrapped and stacked line of a row carries.
const CONTINUATION: &str = "        ";

/// The two wordings that make a row a co-change finding about files.
const CO_CHANGE_FACTS: [&str; 2] = ["changed with it in", "no dependency either way"];

/// Asserts that `text` states no dependency edge as a row.
///
/// Three shapes are banned: a head joining two repository file paths with an
/// arrow, the ownership wording, and the single-reference import fact. Package
/// identities joined by an arrow are not edges — a stable-dependency card
/// states one — and a cycle witness legitimately stacks arrows, so both stay
/// allowed. The one further shape this admits is the two file paths a
/// co-change finding names, which the module documentation states in full.
pub(crate) fn assert_no_dependency_edge_rows(text: &str, context: &str) {
    for line in text.lines() {
        for fact in line.trim().split(" · ") {
            assert!(
                fact != "1 import",
                "{context}: single-reference import fact in {line}"
            );
        }
    }
    for row in rows(text) {
        assert!(!row.contains(" owns "), "{context}: ownership row {row}");
        assert_no_relation_facts(&row, context);
        // A cycle witness is stacked evidence of one card or one comparison,
        // so its arrows belong to that fact rather than to a row of their own.
        if row.contains("dependency cycle") || row.contains("circular dependency") {
            continue;
        }
        for (offset, _) in row.match_indices(" → ") {
            let source = row[..offset].rsplit(' ').next().unwrap_or_default();
            let target = row[offset + " → ".len()..]
                .split(' ')
                .next()
                .unwrap_or_default();
            assert!(
                !(is_file_path(source) && is_file_path(target)),
                "{context}: edge row {row}"
            );
        }
    }
}

/// Asserts that a row naming two files as a co-change finding carries none of
/// the facts the deleted relation rows carried.
///
/// This is the proof that the carve-out is a finding's subject rather than a
/// relation row returning under a new head: the two paths are allowed, and
/// nothing that describes a relation between them is. An aggregate count such
/// as `8 files import this` stays allowed, because the accepted rule keeps
/// aggregate problem evidence and removes the per-relation row.
fn assert_no_relation_facts(row: &str, context: &str) {
    if !CO_CHANGE_FACTS.iter().any(|fact| row.contains(fact)) {
        return;
    }
    for fact in row.split(" · ") {
        let reference_count = fact
            .strip_suffix(" imports")
            .or_else(|| fact.strip_suffix(" import"))
            .is_some_and(|count| count.chars().all(|value| value.is_ascii_digit()));
        let relation_kind = fact.contains(" owns ") || fact.contains(" uses ");
        let outcome = fact == "could not be matched" || fact == "matched more than one file";
        assert!(
            !(reference_count || relation_kind || outcome || fact.contains(" → ")),
            "{context}: co-change row states the relation fact {fact:?} in {row}"
        );
    }
}

/// Reads every wrapped row back into the one line it was written from.
///
/// A row continues on indented lines, and a long path breaks after a path
/// separator rather than at a space, so a line ending in one continues without
/// the space a word break restores.
fn rows(text: &str) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    for line in text.lines() {
        match rows.last_mut().filter(|_| line.starts_with(CONTINUATION)) {
            Some(row) => {
                if !row.ends_with('/') {
                    row.push(' ');
                }
                row.push_str(line.trim());
            }
            None => rows.push(line.trim().to_owned()),
        }
    }
    rows
}

/// Whether one arrow operand names a repository file rather than a package.
///
/// A file path carries an extension on its last segment; a package identity,
/// a measurement operand, and an unresolved import specifier do not. A
/// diagnostic anchor writes `path:line`, which no edge row ever wrote, so a
/// retained unresolved or ambiguous row stays readable.
fn is_file_path(value: &str) -> bool {
    let segment = value.rsplit('/').next().unwrap_or_default();
    segment.contains('.') && !segment.contains(':') && !value.starts_with('.')
}
