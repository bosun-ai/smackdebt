use smackdebt_analysis::{
    CRATE_ROOT_CANDIDATE, DECLARING_FILE_CANDIDATE, DependencyKind, DependencySyntax,
    DependencySyntaxState, SourceSpan, StaticRelationKind,
};
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

pub(super) fn rust(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    let dependency = rust_reference(node, source)?;
    Some(if declared_in_test_scope(node, source) {
        dependency.with_test_scope()
    } else {
        dependency
    })
}

/// Whether the item declaring this reference only exists under `cfg(test)`.
///
/// The rule is syntactic: it reads the outer attributes of the declaring item
/// and of every module that contains it, and never evaluates a configuration
/// predicate or consults enabled features.
fn declared_in_test_scope(node: Node<'_>, source: &[u8]) -> bool {
    outer_attributes_select_test(node, source)
        || ancestors(node)
            .filter(|item| matches!(item.kind(), "mod_item" | "use_declaration"))
            .any(|item| outer_attributes_select_test(item, source))
}

/// The nodes that contain this node, innermost first.
fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(node.parent(), |current| current.parent())
}

/// Reads the run of attributes that immediately precedes an item.
fn outer_attributes_select_test(item: Node<'_>, source: &[u8]) -> bool {
    let mut sibling = item.prev_named_sibling();
    while let Some(current) = sibling {
        match current.kind() {
            "attribute_item" => {
                if attribute_selects_test(current, source) {
                    return true;
                }
            }
            "line_comment" | "block_comment" => {}
            _ => return false,
        }
        sibling = current.prev_named_sibling();
    }
    false
}

/// Whether one attribute is a `cfg` predicate that selects the test configuration.
fn attribute_selects_test(attribute_item: Node<'_>, source: &[u8]) -> bool {
    let Some(attribute) = attribute_item.named_child(0) else {
        return false;
    };
    if attribute.kind() != "attribute" {
        return false;
    }
    let Some(path) = attribute.named_child(0) else {
        return false;
    };
    if path.kind() != "identifier" || path.utf8_text(source).ok() != Some("cfg") {
        return false;
    }
    attribute
        .named_child(1)
        .filter(|arguments| arguments.kind() == "token_tree")
        .is_some_and(|arguments| token_tree_selects_test(arguments, source))
}

/// Whether a `cfg` token tree names `test` outside a negated predicate.
fn token_tree_selects_test(tree: Node<'_>, source: &[u8]) -> bool {
    let mut cursor = tree.walk();
    let mut negated = false;
    for child in tree.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                let text = child.utf8_text(source).ok();
                if text == Some("test") {
                    return true;
                }
                negated = text == Some("not");
            }
            "token_tree" => {
                if !negated && token_tree_selects_test(child, source) {
                    return true;
                }
                negated = false;
            }
            _ => negated = false,
        }
    }
    false
}

fn rust_reference(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if let Some(member) = use_list_member(node, source) {
        return Some(member);
    }
    let kind = match node.kind() {
        "use_declaration" | "extern_crate_declaration" | "scoped_identifier" => {
            DependencyKind::Import
        }
        "mod_item" => DependencyKind::Module,
        "macro_invocation" => DependencyKind::Include,
        _ => return None,
    };
    // A grouped declaration resolves through its list members, never as one
    // reference naming only the shared prefix.
    if node.kind() == "use_declaration"
        && node
            .child_by_field_name("argument")
            .is_some_and(|argument| matches!(argument.kind(), "use_list" | "scoped_use_list"))
    {
        return None;
    }
    let text = strip_visibility(node.utf8_text(source).ok()?.trim());
    if node.kind() == "scoped_identifier"
        && (node
            .parent()
            .is_some_and(|parent| parent.kind() == "scoped_identifier")
            || has_ancestor(node, "use_declaration"))
    {
        return None;
    }
    if node.kind() == "scoped_identifier"
        && !["crate::", "self::", "super::"]
            .iter()
            .any(|prefix| text.starts_with(prefix))
    {
        return None;
    }
    if node.kind() == "macro_invocation" {
        if !text.starts_with("include!") {
            return None;
        }
        let argument = text
            .strip_prefix("include!(")
            .and_then(|value| value.strip_suffix(')'))
            .map(str::trim);
        let Some(target) = argument
            .filter(|value| {
                let bytes = value.as_bytes();
                bytes.len() >= 2
                    && matches!(bytes.first(), Some(b'\'' | b'\"'))
                    && bytes.first() == bytes.last()
            })
            .and_then(string_value)
            .filter(|target| argument.is_some_and(|value| value.len() == target.len() + 2))
        else {
            return Some(
                unresolved(node, kind, text, "dependency target is dynamic").with_internal_intent(),
            );
        };
        if target.starts_with('/') {
            return Some(
                unresolved(
                    node,
                    kind,
                    target,
                    "absolute include paths are not repository-safe",
                )
                .with_internal_intent(),
            );
        }
        let candidate = if target.starts_with('.') {
            target.to_owned()
        } else {
            format!("./{target}")
        };
        return Some(candidates(node, kind, target, vec![candidate]).with_internal_intent());
    }
    if node.kind() == "extern_crate_declaration" {
        return Some(external(node, kind, text));
    }
    if node.kind() == "mod_item" && !text.ends_with(';') {
        return None;
    }
    let target = text
        .strip_prefix("use ")
        .and_then(|value| value.strip_suffix(';'))
        .or_else(|| {
            text.strip_prefix("mod ")
                .and_then(|value| value.strip_suffix(';'))
        })
        .unwrap_or(text)
        .trim()
        .split(['{', ',', ' '])
        .next()
        .unwrap_or("")
        .trim_end_matches("::");
    if target.is_empty() {
        return Some(unresolved(
            node,
            kind,
            text,
            "dependency target is malformed",
        ));
    }
    Some(resolved_path_reference(
        node,
        kind,
        target,
        node.kind() == "mod_item",
        matches!(node.kind(), "use_declaration" | "scoped_identifier"),
    ))
}

/// Resolves one extracted Rust path into candidates or an external package.
///
/// `is_module` marks a `mod` declaration; `with_parent_candidates` adds the
/// enclosing-module fallbacks an import may resolve through.
fn resolved_path_reference(
    node: Node<'_>,
    kind: DependencyKind,
    target: &str,
    is_module: bool,
    with_parent_candidates: bool,
) -> DependencySyntax {
    let root = target.split("::").next().unwrap_or_default();
    let internal = is_module || matches!(root, "crate" | "self" | "super");
    if !internal {
        return external(node, kind, target);
    }
    // A glob import names the module itself, not a child of it.
    let path = target.strip_suffix("::*").unwrap_or(target);
    if !is_module && path == root {
        let value = match root {
            "crate" => CRATE_ROOT_CANDIDATE,
            "self" => DECLARING_FILE_CANDIDATE,
            _ if inline_module_depth(node) > 0 => DECLARING_FILE_CANDIDATE,
            _ => "./mod.rs",
        };
        return candidates(node, kind, target, vec![value.to_owned()]).with_internal_intent();
    }
    let (prefix, value) = if is_module || path.starts_with("self::") {
        ("./", path.trim_start_matches("self::"))
    } else if path.starts_with("super::") {
        ("../", path.trim_start_matches("super::"))
    } else {
        ("", path.trim_start_matches("crate::"))
    };
    let normalized = format!("{prefix}{}", value.replace("::", "/"));
    let mut values = vec![format!("{normalized}.rs"), format!("{normalized}/mod.rs")];
    if with_parent_candidates {
        let mut parent = normalized.as_str();
        while let Some((prefix, _)) = parent.rsplit_once('/') {
            if prefix.chars().all(|character| character == '.') {
                break;
            }
            values.push(format!("{prefix}.rs"));
            values.push(format!("{prefix}/mod.rs"));
            parent = prefix;
        }
    }
    if root == "crate" {
        values.push(CRATE_ROOT_CANDIDATE.to_owned());
    } else if !is_module && inline_module_depth(node) > 0 {
        values.push(DECLARING_FILE_CANDIDATE.to_owned());
    }
    let dependency = candidates(node, kind, target, values).with_internal_intent();
    if is_module {
        dependency.with_relation(StaticRelationKind::ModuleOwnership)
    } else {
        dependency
    }
}

/// One reference per imported item of a grouped `use` list.
///
/// Grammar members are the direct named children of a `use_list`; path
/// segments live under `scoped_use_list` or `scoped_identifier` parents and
/// never match here, so every member is counted exactly once.
fn use_list_member(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if node.parent()?.kind() != "use_list" {
        return None;
    }
    let prefix = use_list_prefix(node, source);
    let target = match node.kind() {
        "identifier" | "scoped_identifier" | "use_wildcard" => {
            joined_path(prefix, node.utf8_text(source).ok()?.trim())
        }
        // An `as` clause references the original item, never the alias.
        "use_as_clause" => joined_path(
            prefix,
            node.child_by_field_name("path")?
                .utf8_text(source)
                .ok()?
                .trim(),
        ),
        // A `self` member imports the enclosing list's own path.
        "self" => prefix?,
        _ => return None,
    };
    Some(resolved_path_reference(
        node,
        DependencyKind::Import,
        &target,
        false,
        true,
    ))
}

/// The `::`-joined path the enclosing `use` lists prefix a member with.
fn use_list_prefix(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut prefix = String::new();
    for ancestor in ancestors(node) {
        match ancestor.kind() {
            "scoped_use_list" => {
                if let Some(text) = ancestor
                    .child_by_field_name("path")
                    .and_then(|path| path.utf8_text(source).ok())
                {
                    if prefix.is_empty() {
                        prefix = text.trim().to_owned();
                    } else {
                        prefix = format!("{}::{prefix}", text.trim());
                    }
                }
            }
            "use_declaration" => break,
            _ => {}
        }
    }
    (!prefix.is_empty()).then_some(prefix)
}

/// Joins an optional list prefix and one member path with `::`.
fn joined_path(prefix: Option<String>, member: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}::{member}"),
        None => member.to_owned(),
    }
}

/// Removes a leading Rust visibility modifier so it cannot become a target.
fn strip_visibility(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("pub") else {
        return text;
    };
    let Some(arguments) = rest.strip_prefix('(') else {
        return if rest.starts_with(char::is_whitespace) {
            rest.trim_start()
        } else {
            text
        };
    };
    let mut depth = 1usize;
    for (index, character) in arguments.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return arguments[index + 1..].trim_start();
                }
            }
            _ => {}
        }
    }
    text
}

/// Counts the inline modules that contain this node inside its own file.
fn inline_module_depth(node: Node<'_>) -> usize {
    ancestors(node)
        .filter(|parent| parent.kind() == "mod_item")
        .count()
}

fn has_ancestor(node: Node<'_>, kind: &str) -> bool {
    ancestors(node).any(|parent| parent.kind() == kind)
}

pub(super) fn python(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if !matches!(node.kind(), "import_statement" | "import_from_statement") {
        return None;
    }
    let text = node.utf8_text(source).ok()?.trim();
    let target = if let Some(rest) = text.strip_prefix("from ") {
        let module = rest.split_whitespace().next().unwrap_or("");
        if module.chars().all(|character| character == '.') {
            let imported = rest
                .split_once(" import ")
                .map(|(_, value)| value.split([',', ' ']).next().unwrap_or(""))
                .unwrap_or("");
            return if imported.is_empty() {
                Some(unresolved(
                    node,
                    DependencyKind::Import,
                    text,
                    "dependency target is malformed",
                ))
            } else {
                let target = format!("{module}{imported}");
                let levels = module.len();
                let prefix = if levels == 1 {
                    "./".to_owned()
                } else {
                    "../".repeat(levels.saturating_sub(1))
                };
                Some(
                    candidates(
                        node,
                        DependencyKind::Import,
                        &target,
                        vec![
                            format!("{prefix}{imported}.py"),
                            format!("{prefix}{imported}/__init__.py"),
                        ],
                    )
                    .with_internal_intent(),
                )
            };
        }
        module
    } else {
        text.strip_prefix("import ")
            .unwrap_or("")
            .split([',', ' '])
            .next()
            .unwrap_or("")
    };
    if target.is_empty() {
        return Some(unresolved(
            node,
            DependencyKind::Import,
            text,
            "dependency target is malformed",
        ));
    }
    if !target.starts_with('.') {
        return Some(candidates(
            node,
            DependencyKind::Import,
            target,
            dotted_candidates(target),
        ));
    }
    let levels = target
        .chars()
        .take_while(|character| *character == '.')
        .count();
    let module = target.trim_start_matches('.').replace('.', "/");
    let prefix = if levels == 1 {
        "./".to_owned()
    } else {
        "../".repeat(levels.saturating_sub(1))
    };
    Some(
        candidates(
            node,
            DependencyKind::Import,
            target,
            vec![
                format!("{prefix}{module}.py"),
                format!("{prefix}{module}/__init__.py"),
            ],
        )
        .with_internal_intent(),
    )
}

pub(super) fn java(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if node.kind() != "import_declaration" {
        return None;
    }
    let text = node.utf8_text(source).ok()?.trim();
    let target = text
        .strip_prefix("import ")
        .unwrap_or(text)
        .trim_start_matches("static ")
        .trim_end_matches(';')
        .trim_end_matches(".*");
    Some(candidates(
        node,
        DependencyKind::Import,
        target,
        vec![
            format!("{}.java", target.replace('.', "/")),
            format!("src/main/java/{}.java", target.replace('.', "/")),
            format!("src/test/java/{}.java", target.replace('.', "/")),
        ],
    ))
}

pub(super) fn include(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if node.kind() != "preproc_include" {
        return None;
    }
    let text = node.utf8_text(source).ok()?;
    if text.contains('<') {
        let target = text
            .split_once('<')
            .and_then(|(_, rest)| rest.split_once('>'))
            .map_or(text, |(target, _)| target);
        return Some(external(node, DependencyKind::Include, target));
    }
    quoted(node, source, DependencyKind::Include, &[""], false).map(|dependency| {
        match dependency.state() {
            DependencySyntaxState::Candidates(values) => DependencySyntax::new(
                dependency.kind(),
                dependency.target(),
                dependency.span(),
                DependencySyntaxState::Candidates(
                    values.iter().map(|value| format!("./{value}")).collect(),
                ),
            )
            .with_internal_intent(),
            _ => dependency,
        }
    })
}

pub(super) fn ruby(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if node.kind() != "call" {
        return None;
    }
    if node.child_by_field_name("receiver").is_some() {
        return None;
    }
    let method = node.child_by_field_name("method")?.utf8_text(source).ok()?;
    let (kind, relative) = if method == "require_relative" {
        (DependencyKind::Require, true)
    } else if method == "require" {
        (DependencyKind::Require, false)
    } else {
        return None;
    };
    quoted(node, source, kind, &[".rb", "/init.rb"], !relative).map(|dependency| {
        if relative {
            match dependency.state() {
                DependencySyntaxState::Candidates(values) => DependencySyntax::new(
                    dependency.kind(),
                    dependency.target(),
                    dependency.span(),
                    DependencySyntaxState::Candidates(
                        values.iter().map(|value| format!("./{value}")).collect(),
                    ),
                )
                .with_internal_intent(),
                _ => dependency,
            }
        } else if matches!(dependency.state(), DependencySyntaxState::Candidates(_)) {
            external(node, kind, dependency.target())
        } else {
            dependency
        }
    })
}

fn dotted_candidates(target: &str) -> Vec<String> {
    let path = target.replace('.', "/");
    vec![format!("{path}.py"), format!("{path}/__init__.py")]
}

fn path_candidates(target: &str, extensions: &[&str]) -> Vec<String> {
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
fn string_value(text: &str) -> Option<&str> {
    for quote in ['\'', '"', '`'] {
        if let Some(start) = text.find(quote) {
            let rest = &text[start + 1..];
            let end = rest.find(quote)?;
            let value = &rest[..end];
            if !value.contains("${") && !value.contains('\n') && !value.contains('\r') {
                return Some(value);
            }
        }
    }
    None
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

// The three constructors below are where every `DependencySyntax` in this
// crate is built, from every language and every extraction path — a
// splitter or prefix-strip elsewhere in this file only ever produces a
// `&str` that flows into one of them as `target`. Reducing `target` to a
// single legible line here, once, is what guarantees the raw declaration
// text a splitter failed to fully parse (an unbraced multi-line Rust `use`,
// a backslash-continued Python import, a comment-interrupted Java import)
// can never carry a newline, carriage return, or tab into a retained
// dependency target — without auditing, and re-auditing, every splitter.

fn candidates(
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

fn external(node: Node<'_>, kind: DependencyKind, target: impl Into<String>) -> DependencySyntax {
    DependencySyntax::new(
        kind,
        specifier_text(&target.into()),
        span(node),
        DependencySyntaxState::External,
    )
}

fn unresolved(
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

fn span(node: Node<'_>) -> SourceSpan {
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
