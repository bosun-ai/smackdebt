use smackdebt_analysis::{DependencyKind, DependencySyntax, DependencySyntaxState, SourceSpan};
use tree_sitter::Node;

pub(super) fn quoted(
    node: Node<'_>,
    source: &[u8],
    kind: DependencyKind,
    extensions: &[&str],
    relative_only: bool,
) -> Option<DependencySyntax> {
    let text = node.utf8_text(source).ok()?;
    let target = string_value(text);
    let Some(target) = target else {
        return Some(unresolved(node, kind, text, "dependency target is dynamic"));
    };
    if relative_only && !target.starts_with('.') {
        return Some(external(node, kind, target));
    }
    let dependency = candidates(node, kind, target, path_candidates(target, extensions));
    Some(if target.starts_with('.') {
        dependency.with_internal_intent()
    } else {
        dependency
    })
}

pub(super) fn path_candidates(target: &str, extensions: &[&str]) -> Vec<String> {
    let mut values = Vec::with_capacity(1 + extensions.len() * 2);
    values.push(target.to_owned());
    for extension in extensions {
        let value = format!("{target}{extension}");
        if !values.contains(&value) {
            values.push(value);
        }
        if !extension.is_empty() {
            values.push(format!("{target}/index{extension}"));
        }
    }
    values
}

/// Reads a simple quoted specifier: a literal value between one pair of
/// matching quote characters, with no interpolation and no embedded line
/// break. A quoted value carrying either is not a static specifier, so
/// callers fall back to `specifier_text` over the raw declaration text —
/// which, unlike this line-break rejection, also collapses an embedded tab
/// rather than treating it as reason to give up on the value entirely.
pub(super) fn string_value(text: &str) -> Option<&str> {
    let quote = text
        .chars()
        .next()
        .filter(|quote| matches!(quote, '\'' | '"' | '`'))?;
    let value = text.strip_prefix(quote)?.strip_suffix(quote)?;
    (!value.contains("${") && !value.contains(['\n', '\r'])).then_some(value)
}

/// Reduces syntax text to one legible line for a diagnostic target: the
/// first line, with internal whitespace runs collapsed to a single space,
/// trimmed of leading and trailing whitespace.
///
/// Never truncates — a long single line is left for the renderer to wrap.
fn specifier_text(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn candidates(
    node: Node<'_>,
    kind: DependencyKind,
    target: impl Into<String>,
    values: Vec<String>,
) -> DependencySyntax {
    DependencySyntax::new(
        kind,
        specifier_text(&target.into()),
        span(node),
        DependencySyntaxState::Candidates(values),
    )
}

pub(super) fn external(
    node: Node<'_>,
    kind: DependencyKind,
    target: impl Into<String>,
) -> DependencySyntax {
    DependencySyntax::new(
        kind,
        specifier_text(&target.into()),
        span(node),
        DependencySyntaxState::External,
    )
}

pub(super) fn unresolved(
    node: Node<'_>,
    kind: DependencyKind,
    target: impl Into<String>,
    reason: impl Into<String>,
) -> DependencySyntax {
    DependencySyntax::new(
        kind,
        specifier_text(&target.into()),
        span(node),
        DependencySyntaxState::Unresolved(reason.into()),
    )
}

pub(super) fn span(node: Node<'_>) -> SourceSpan {
    SourceSpan::new(
        node.start_position().row as u32 + 1,
        node.end_position().row as u32 + 1,
    )
}

#[cfg(test)]
mod tests {
    use super::specifier_text;

    #[test]
    fn specifier_text_takes_the_first_line_and_collapses_whitespace() {
        let text = "  first \t line   here\n\tsecond line\n        third";
        assert_eq!(specifier_text(text), "first line here");
    }

    /// `str::lines` only splits on `\n` or `\r\n`, so a lone `\r` stays inside
    /// the "first line" it returns — this proves the whitespace-run collapse
    /// still removes it, which is why `string_value` only needs to reject a
    /// `\n`/`\r\n` line break, not a bare `\r`, as not a static specifier.
    #[test]
    fn specifier_text_collapses_a_lone_carriage_return_within_the_first_line() {
        assert_eq!(specifier_text("foo\rbar"), "foo bar");
    }
}
