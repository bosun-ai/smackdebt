//! Source-role classification: what a path, its content, and the configured
//! rules say a file is.

use std::path::Path;

use smackdebt_analysis::SourceRole;
use smackdebt_discovery::{
    generic_source_roles, glob_matches, has_generated_javascript_name,
    has_vendored_javascript_name, is_runtime_javascript_path,
};

use smackdebt_languages::Analyzer;

use crate::requests::SourceRoleRule;

/// The roles explicit configuration states for one path, in rule order.
pub(crate) fn matching_role_rules<'a>(
    path: &Path,
    rules: &'a [SourceRoleRule],
) -> impl Iterator<Item = SourceRole> + 'a {
    let normalized = path.to_string_lossy().replace('\\', "/");
    rules
        .iter()
        .filter(move |rule| glob_matches(rule.pattern(), &normalized))
        .map(SourceRoleRule::role)
}
pub(crate) fn classify_source_role(
    path: &Path,
    source: &[u8],
    rules: &[SourceRoleRule],
) -> Result<SourceRole, String> {
    let mut explicit: Vec<_> = matching_role_rules(path, rules).collect();
    explicit.sort();
    explicit.dedup();
    if !explicit.is_empty() {
        return one_role(explicit);
    }
    if Analyzer::has_generated_marker(path, source) {
        return Ok(SourceRole::Generated);
    }
    if has_generated_javascript_name(path) || has_generated_javascript_content(path, source) {
        return Ok(SourceRole::Generated);
    }
    if has_vendored_javascript_name(path) {
        return Ok(SourceRole::Vendored);
    }
    let generic = generic_source_roles(path);
    if generic.is_empty() {
        Ok(SourceRole::Primary)
    } else {
        one_role(generic)
    }
}
/// Whether the source is written as a module rather than as a plain script.
///
/// A module states its own imports and exports, so the dependency graph can see
/// whether anything uses it: nothing importing a module is an orphan, a fact
/// the report already states. A script states nothing - a page or a build tool
/// loads it by name - so no import could ever have named it and an empty fan-in
/// is what such a file is supposed to look like. Only a script can therefore be
/// read as dormant.
///
/// The test reads the first word of each line, not every occurrence, so the
/// `module.exports` a UMD wrapper indents inside a function leaves the file the
/// script it is. That prefix reading is the whole of the rule: it is a cheap
/// approximation of a statement position, not a parse, and the `export` opening
/// a line inside a template literal counts too.
///
/// The two errors are not symmetric. Missing module syntax lets a module be
/// called dormant, which is wrong about a file the team may be working on;
/// seeing it where there is none only spares a file from a rule of absences.
/// So the accepted follow set is generous - whitespace, a brace, a star, a
/// parenthesis, either quote, a carriage return, or the end of the line, which
/// is how a multi-line `import` opens. Only the JavaScript a runtime loads as
/// written is read at all, because that is the only source the dormancy rule
/// can classify.
pub(crate) fn declares_module_syntax(path: &Path, source: &[u8]) -> bool {
    if !is_runtime_javascript_path(path) {
        return false;
    }
    source.split(|byte| *byte == b'\n').any(|line| {
        let line = line.trim_ascii_start();
        ["export", "import"].iter().any(|keyword| {
            line.strip_prefix(keyword.as_bytes()).is_some_and(|rest| {
                matches!(
                    rest.first(),
                    None | Some(b' ' | b'\t' | b'\r' | b'{' | b'*' | b'(' | b'"' | b'\'')
                )
            })
        })
    })
}
pub(crate) fn has_generated_javascript_content(path: &Path, source: &[u8]) -> bool {
    const MINIMUM_BYTES: usize = 65_536;
    const MINIMUM_BYTES_PER_NONEMPTY_LINE: usize = 512;

    let supported_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx"));
    if !supported_extension || source.len() < MINIMUM_BYTES {
        return false;
    }

    let nonempty_lines = source
        .split(|byte| *byte == b'\n')
        .filter(|line| line.iter().any(|byte| !byte.is_ascii_whitespace()))
        .count();
    nonempty_lines != 0
        && nonempty_lines
            .checked_mul(MINIMUM_BYTES_PER_NONEMPTY_LINE)
            .is_some_and(|minimum| source.len() >= minimum)
}
pub(crate) fn role_for_unavailable_source(
    path: &Path,
    rules: &[SourceRoleRule],
) -> Result<SourceRole, String> {
    classify_source_role(path, &[], rules)
}
pub(crate) fn one_role(roles: Vec<SourceRole>) -> Result<SourceRole, String> {
    if roles.len() == 1 {
        Ok(roles[0])
    } else {
        Err(roles
            .iter()
            .map(|role| format!("{role:?}").to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(", "))
    }
}
