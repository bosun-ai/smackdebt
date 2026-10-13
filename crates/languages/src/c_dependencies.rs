//! C-family include syntax; search order belongs to project resolution.
use smackdebt_analysis::{
    DependencyKind, DependencySyntax, DependencySyntaxState, IncludeStyle, SourceSpan,
};
use tree_sitter::Node;

pub(crate) fn include(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if node.kind() != "preproc_include" {
        return None;
    }
    let path = node.child_by_field_name("path")?;
    let text = path.utf8_text(source).ok()?.trim();
    let (target, state) = match path.kind() {
        "string_literal" => (
            text.trim_matches('"'),
            DependencySyntaxState::Include(IncludeStyle::Quoted),
        ),
        "system_lib_string" => (
            text.trim_start_matches('<').trim_end_matches('>'),
            DependencySyntaxState::Include(IncludeStyle::Angle),
        ),
        _ => (
            text,
            DependencySyntaxState::Unresolved("include target requires macro expansion".to_owned()),
        ),
    };
    let target = if matches!(state, DependencySyntaxState::Unresolved(_)) {
        std::borrow::Cow::Owned(target.split_whitespace().collect::<Vec<_>>().join(" "))
    } else {
        std::borrow::Cow::Borrowed(target)
    };
    let internal = matches!(
        state,
        DependencySyntaxState::Include(IncludeStyle::Quoted) | DependencySyntaxState::Unresolved(_)
    );
    let dependency = DependencySyntax::new(
        DependencyKind::Include,
        target,
        SourceSpan::new(
            node.start_position().row as u32 + 1,
            node.end_position().row as u32 + 1,
        ),
        state,
    );
    Some(if internal {
        dependency.with_internal_intent()
    } else {
        dependency
    })
}
