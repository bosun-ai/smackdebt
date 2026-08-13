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

pub(super) fn rust(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    let kind = match node.kind() {
        "use_declaration" | "extern_crate_declaration" => DependencyKind::Import,
        "mod_item" => DependencyKind::Module,
        "macro_invocation" => DependencyKind::Include,
        _ => return None,
    };
    let text = node.utf8_text(source).ok()?.trim();
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
    let internal = node.kind() == "mod_item"
        || target.starts_with("crate::")
        || target.starts_with("self::")
        || target.starts_with("super::");
    if !internal {
        return Some(external(node, kind, target));
    }
    let (prefix, value) = if node.kind() == "mod_item" || target.starts_with("self::") {
        ("./", target.trim_start_matches("self::"))
    } else if target.starts_with("super::") {
        ("../", target.trim_start_matches("super::"))
    } else {
        ("", target.trim_start_matches("crate::"))
    };
    let normalized = format!("{prefix}{}", value.replace("::", "/"));
    let mut values = vec![format!("{normalized}.rs"), format!("{normalized}/mod.rs")];
    if node.kind() == "use_declaration" {
        let mut parent = normalized.as_str();
        while let Some((prefix, _)) = parent.rsplit_once('/') {
            values.push(format!("{prefix}.rs"));
            values.push(format!("{prefix}/mod.rs"));
            parent = prefix;
        }
    }
    Some(candidates(node, kind, target, values).with_internal_intent())
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
    let text = node.utf8_text(source).ok()?.trim();
    let (kind, relative) = if text.starts_with("require_relative") {
        (DependencyKind::Require, true)
    } else if text.starts_with("require") {
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

fn string_value(text: &str) -> Option<&str> {
    for quote in ['\'', '"', '`'] {
        if let Some(start) = text.find(quote) {
            let rest = &text[start + 1..];
            let end = rest.find(quote)?;
            let value = &rest[..end];
            if !value.contains("${") {
                return Some(value);
            }
        }
    }
    None
}

fn candidates(
    node: Node<'_>,
    kind: DependencyKind,
    target: impl Into<String>,
    values: Vec<String>,
) -> DependencySyntax {
    DependencySyntax::new(
        kind,
        target,
        span(node),
        DependencySyntaxState::Candidates(values),
    )
}

fn external(node: Node<'_>, kind: DependencyKind, target: impl Into<String>) -> DependencySyntax {
    DependencySyntax::new(kind, target, span(node), DependencySyntaxState::External)
}

fn unresolved(
    node: Node<'_>,
    kind: DependencyKind,
    target: impl Into<String>,
    reason: impl Into<String>,
) -> DependencySyntax {
    DependencySyntax::new(
        kind,
        target,
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
