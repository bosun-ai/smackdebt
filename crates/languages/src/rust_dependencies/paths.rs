use super::ancestors;
use crate::dependency::{candidates, external};
use smackdebt_analysis::{
    CRATE_ROOT_CANDIDATE, DECLARING_FILE_CANDIDATE, DependencyKind, DependencySyntax,
    PARENT_MODULE_CANDIDATE, StaticRelationKind,
};
use tree_sitter::Node;

/// Resolves one extracted Rust path into candidates or an external package.
///
/// `is_module` marks a `mod` declaration; `with_parent_candidates` adds the
/// enclosing-module fallbacks an import may resolve through.
pub(super) fn resolved_path_reference(
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
    let inline = ancestors(node).any(|node| node.kind() == "mod_item");
    if !is_module && path == root {
        let values = root_candidates(root, inline);
        return candidates(node, kind, target, values).with_internal_intent();
    }
    let normalized = module_path(path, is_module);
    let mut values = vec![format!("{normalized}.rs"), format!("{normalized}/mod.rs")];
    if with_parent_candidates {
        append_parent_candidates(&mut values, &normalized, path, inline);
    }
    if root == "crate" {
        values.push(CRATE_ROOT_CANDIDATE.to_owned());
    } else if !is_module && inline {
        values.push(DECLARING_FILE_CANDIDATE.to_owned());
    }
    let dependency = candidates(node, kind, target, values).with_internal_intent();
    if is_module {
        dependency.with_relation(StaticRelationKind::ModuleOwnership)
    } else {
        dependency
    }
}

/// The candidates for the module a rooted path counts its segments from.
///
/// The root module owns no file named after it, so the walk that turns
/// `crate::a::b::Item` into `a/b.rs` has nothing left to offer once it reaches
/// `.` or `..`.  Both roots still name a file: `self` is the declaring file,
/// and `super` is the module that declares it, which lives either in
/// `../mod.rs` or beside the declaring file's own directory — a spelling only
/// the resolver knows, so it is named symbolically.
///
/// A chain of several `super` segments normalizes to one `../` today, so the
/// parent module it would name is the wrong one, two or more levels below the
/// module the path counts from.  Such a path is offered no parent at all: a
/// trailing `super` segment is the module itself rather than an item in it, so
/// counting the leading segments is the only reading that sees the difference
/// between `super::Item` and `super::super`.
fn root_module_candidates(prefix: &str, path: &str, inline: bool) -> Vec<String> {
    let single_super = path
        .split("::")
        .take_while(|segment| *segment == "super")
        .count()
        == 1;
    match prefix {
        // Inside an inline module the declaring file is already the root, and
        // the reference carries it as its own fallback.
        _ if inline => Vec::new(),
        "." => vec![DECLARING_FILE_CANDIDATE.to_owned()],
        ".." if single_super => vec![
            format!("{prefix}/mod.rs"),
            PARENT_MODULE_CANDIDATE.to_owned(),
        ],
        _ => Vec::new(),
    }
}

fn module_path(path: &str, is_module: bool) -> String {
    let (prefix, value) = if is_module || path.starts_with("self::") {
        ("./", path.trim_start_matches("self::"))
    } else if path.starts_with("super::") {
        ("../", path.trim_start_matches("super::"))
    } else {
        ("", path.trim_start_matches("crate::"))
    };
    format!("{prefix}{}", value.replace("::", "/"))
}

fn append_parent_candidates(values: &mut Vec<String>, normalized: &str, path: &str, inline: bool) {
    let mut parent = normalized;
    while let Some((prefix, _)) = parent.rsplit_once('/') {
        // An all-dots prefix is the path's own root, which owns no `.rs`
        // file of its name and cannot be walked any further.
        if prefix.chars().all(|character| character == '.') {
            values.extend(root_module_candidates(prefix, path, inline));
            break;
        }
        values.push(format!("{prefix}.rs"));
        values.push(format!("{prefix}/mod.rs"));
        parent = prefix;
    }
}

fn root_candidates(root: &str, inline: bool) -> Vec<String> {
    match root {
        "crate" => vec![CRATE_ROOT_CANDIDATE.to_owned()],
        "self" => vec![DECLARING_FILE_CANDIDATE.to_owned()],
        _ if inline => vec![DECLARING_FILE_CANDIDATE.to_owned()],
        _ => vec!["./mod.rs".to_owned(), PARENT_MODULE_CANDIDATE.to_owned()],
    }
}
