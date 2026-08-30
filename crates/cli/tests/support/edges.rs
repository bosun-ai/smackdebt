//! The invariant that keeps dependency edges out of every human view.
//!
//! A relationship is a graph fact, not a decision, so no human view may state
//! one as a row at any scope, in any mode, or at any detail level. The check is
//! an invariant applied to every terminal result the suites produce and to
//! every committed terminal result, rather than an assertion on selected cases,
//! so a future view cannot reintroduce the rows quietly.

/// The indent every wrapped and stacked line of a row carries.
const CONTINUATION: &str = "        ";

/// Asserts that `text` states no dependency edge as a row.
///
/// Three shapes are banned: a head joining two repository file paths with an
/// arrow, the ownership wording, and the single-reference import fact. Package
/// identities joined by an arrow are not edges — a stable-dependency row states
/// one — and a cycle witness legitimately stacks arrows, so both stay allowed.
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
        // A cycle witness is stacked evidence of one finding, so its arrows
        // belong to the finding rather than to a row of their own.
        if row.contains("dependency cycle") {
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
