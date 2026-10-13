//! Reads the small literal gem load-path contract without evaluating Ruby.
use tree_sitter::{Node, Parser};

/// Extracts literal load paths from an unconditional Gem::Specification block.
/// Configurations requiring Ruby evaluation are rejected rather than guessed.
pub fn ruby_gem_require_paths(source: &str) -> Result<Vec<String>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_ruby::LANGUAGE.into())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(source, None)
        .ok_or("Ruby gem configuration could not be parsed")?;
    if tree.root_node().has_error() {
        return Err("Ruby gem configuration contains syntax errors".to_owned());
    }
    let mut paths = vec!["lib".to_owned()];
    let mut cursor = tree.walk();
    loop {
        let node = cursor.node();
        if node.kind() == "call" && text_field(node, "method", source) == Some("require_paths") {
            paths = literal_assignment(node, source)
                .ok_or("Ruby gem require_paths are not unconditional literal assignments")?;
        }
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return Ok(paths);
            }
        }
    }
}

fn text_field<'a>(node: Node<'_>, field: &str, source: &'a str) -> Option<&'a str> {
    node.child_by_field_name(field)?
        .utf8_text(source.as_bytes())
        .ok()
}

fn literal_assignment(call: Node<'_>, source: &str) -> Option<Vec<String>> {
    let assignment = call.parent()?;
    if assignment.kind() != "assignment" || assignment.child_by_field_name("left") != Some(call) {
        return None;
    }
    let block = specification_block(assignment, source)?;
    let parameters = block.child_by_field_name("parameters")?;
    if parameters.named_child_count() != 1 {
        return None;
    }
    let parameter = parameters.named_child(0)?;
    if parameter.kind() != "identifier"
        || text_field(call, "receiver", source) != parameter.utf8_text(source.as_bytes()).ok()
    {
        return None;
    }
    let value = assignment.child_by_field_name("right")?;
    if value.kind() != "array" {
        return None;
    }
    literal_paths(value, source)
}

fn specification_block<'tree>(assignment: Node<'tree>, source: &str) -> Option<Node<'tree>> {
    let body = assignment.parent()?;
    let block = body.parent()?;
    let constructor = block.parent()?;
    if !matches!(body.kind(), "body_statement" | "block_body")
        || !matches!(block.kind(), "do_block" | "block")
        || constructor.kind() != "call"
        || constructor.parent()?.kind() != "program"
        || text_field(constructor, "receiver", source) != Some("Gem::Specification")
        || text_field(constructor, "method", source) != Some("new")
    {
        return None;
    }
    Some(block)
}

fn literal_paths(value: Node<'_>, source: &str) -> Option<Vec<String>> {
    let mut paths = Vec::new();
    let mut cursor = value.walk();
    for item in value.named_children(&mut cursor) {
        if item.kind() != "comment" {
            paths.push(literal_path(item, source)?);
        }
    }
    Some(paths)
}

fn literal_path(item: Node<'_>, source: &str) -> Option<String> {
    if item.kind() != "string" {
        return None;
    }
    let mut cursor = item.walk();
    if item
        .named_children(&mut cursor)
        .any(|child| child.kind() != "string_content")
    {
        return None;
    }
    let raw = item.utf8_text(source.as_bytes()).ok()?;
    let path = raw
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .or_else(|| {
            raw.strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })?;
    if path.contains('\\')
        || std::path::Path::new(path).is_absolute()
        || path.split('/').any(|part| part == "..")
    {
        return None;
    }
    Some(path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::ruby_gem_require_paths;
    #[test]
    fn literal_arrays_allow_multiple_lines_and_comments() {
        assert_eq!(
            ruby_gem_require_paths(
                "Gem::Specification.new do |spec|\n spec.require_paths = [\n 'app', # source\n \"shared\",\n ]\nend"
            ),
            Ok(vec!["app".to_owned(), "shared".to_owned()])
        );
    }
    #[test]
    fn comments_and_strings_are_not_load_path_assignments() {
        assert_eq!(
            ruby_gem_require_paths(
                "# spec.require_paths = compute()\nGem::Specification.new do |spec|\n spec.summary = \"spec.require_paths = ['decoy']\"\nend"
            ),
            Ok(vec!["lib".to_owned()])
        );
    }
    #[test]
    fn control_flow_and_mutations_require_ruby_evaluation() {
        for statement in [
            "if false; spec.require_paths = ['decoy']; end",
            "spec.require_paths = ['decoy'] if false",
            "while false; spec.require_paths = ['decoy']; end",
            "def paths; spec.require_paths = ['decoy']; end",
            "spec.require_paths << 'decoy'",
            "spec.require_paths.push('decoy')",
            "spec.require_paths = compute()",
            "spec.require_paths = [\"#{path}\"]",
            "spec.require_paths = ['../outside']",
        ] {
            let source = format!("Gem::Specification.new do |spec|\n{statement}\nend");
            assert!(ruby_gem_require_paths(&source).is_err(), "{statement}");
        }
        assert!(ruby_gem_require_paths("spec.require_paths = ['decoy']").is_err());
    }
}
