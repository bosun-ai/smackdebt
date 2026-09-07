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

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;

    #[test]
    fn source_role_precedence_keeps_explicit_rules_ahead_of_generated_javascript() {
        let generated = b"// @generated\nexport function work() {}\n";
        assert_eq!(
            classify_source_role(
                Path::new("tests/work.js"),
                generated,
                &[SourceRoleRule::example("tests/*.js")],
            ),
            Ok(SourceRole::Example)
        );
        assert_eq!(
            classify_source_role(Path::new("tests/work.js"), generated, &[]),
            Ok(SourceRole::Generated)
        );
        assert_eq!(
            classify_source_role(Path::new("tests/work.js"), b"function work() {}", &[]),
            Ok(SourceRole::Test)
        );
        assert_eq!(
            classify_source_role(Path::new("src/work.js"), b"function work() {}", &[]),
            Ok(SourceRole::Primary)
        );
        assert_eq!(
            classify_source_role(
                Path::new("src/vendor.min.js"),
                b"function work() {}",
                &[SourceRoleRule::primary("src/vendor.min.js")],
            ),
            Ok(SourceRole::Primary)
        );
        assert_eq!(
            classify_source_role(Path::new("tests/vendor.min.js"), b"function work() {}", &[],),
            Ok(SourceRole::Generated)
        );
        assert_eq!(
            classify_source_role(
                Path::new("src/client.bundle.ts"),
                b"function work() {}",
                &[],
            ),
            Ok(SourceRole::Primary)
        );
    }
    /// The guard that decides which files the dormancy rule may look at.
    ///
    /// Its two errors are not symmetric: missing module syntax lets a module be
    /// called dormant, while seeing it where there is none only spares a file.
    /// The table therefore leans on the spellings that could be missed.
    #[test]
    fn module_syntax_is_read_from_the_first_word_of_a_line() {
        let script = Path::new("public/js/widget.js");
        for (source, expected, why) in MODULE_SYNTAX_CASES {
            assert_eq!(
                declares_module_syntax(script, source.as_bytes()),
                *expected,
                "{why}: {source:?}"
            );
        }
        assert!(
            !declares_module_syntax(Path::new("src/widget.ts"), b"export const a = 1;\n"),
            "only the JavaScript a runtime loads as written is read at all"
        );
    }
    /// One source, the answer it must produce, and why that answer is right.
    const MODULE_SYNTAX_CASES: &[(&str, bool, &str)] = &[
        (
            "function a() {}\nexport function b() {}\n",
            true,
            "an export anywhere in the file counts",
        ),
        (
            "const a = 1;\nexport {a};\n",
            true,
            "a brace after the keyword counts",
        ),
        (
            "import\"./x\"\n",
            true,
            "a double quote with no space counts",
        ),
        ("import'./x'\n", true, "a single quote with no space counts"),
        (
            "import {\n  thing,\n} from './x';\n",
            true,
            "a multi-line import counts",
        ),
        (
            "import\n  { thing }\nfrom './x';\n",
            true,
            "the keyword ending its own line counts",
        ),
        ("export * from './x';\n", true, "a star counts"),
        ("  export default 1;\n", true, "leading indent is trimmed"),
        (
            "export {a};\r\n",
            true,
            "a carriage return does not hide the keyword",
        ),
        (
            "#!/usr/bin/env node\n(function () {\n  module.exports = 1;\n})();\n",
            false,
            "a UMD or CommonJS wrapper is still a script",
        ),
        (
            "const x = require('./x');\nwindow.x = x;\n",
            false,
            "a plain require is not module syntax the graph reads here",
        ),
        (
            "const exported = 1;\nconst important = 2;\n",
            false,
            "a longer word starting with the keyword is not the keyword",
        ),
        (
            "// export function b() {}\n",
            false,
            "the keyword is not the first word of that line",
        ),
    ];
    #[test]
    fn generated_javascript_content_uses_exact_size_and_density_edges() {
        let exact_edge = vec![b'x'; 65_536];
        for path in [
            "src/client.js",
            "src/client.mjs",
            "src/client.cjs",
            "src/client.jsx",
            "src/client.ts",
            "src/client.tsx",
        ] {
            assert!(
                has_generated_javascript_content(Path::new(path), &exact_edge),
                "{path}",
            );
        }

        let mut exact_lines = Vec::with_capacity(65_536);
        for _ in 0..127 {
            exact_lines.extend(std::iter::repeat_n(b'x', 511));
            exact_lines.push(b'\n');
        }
        exact_lines.extend(std::iter::repeat_n(b'x', 512));
        assert_eq!(exact_lines.len(), 65_536);
        assert!(has_generated_javascript_content(
            Path::new("src/client.js"),
            &exact_lines,
        ));

        let below_size = vec![b'x'; 65_535];
        assert!(!has_generated_javascript_content(
            Path::new("src/client.js"),
            &below_size,
        ));

        let mut below_density = exact_lines;
        below_density[255] = b'\n';
        assert!(!has_generated_javascript_content(
            Path::new("src/client.js"),
            &below_density,
        ));
        assert!(!has_generated_javascript_content(
            Path::new("src/client.vue"),
            &exact_edge,
        ));
    }
    #[test]
    fn ordinary_javascript_content_and_common_directories_remain_primary() {
        let mut multiline = Vec::with_capacity(65_536);
        for _ in 0..256 {
            multiline.extend(std::iter::repeat_n(b'x', 255));
            multiline.push(b'\n');
        }
        assert_eq!(multiline.len(), 65_536);
        assert!(!has_generated_javascript_content(
            Path::new("src/large.js"),
            &multiline,
        ));

        for path in ["public/app.js", "share/tool.js", "assets/editor.js"] {
            assert_eq!(
                classify_source_role(Path::new(path), b"export const value = 1;", &[]),
                Ok(SourceRole::Primary),
                "{path}",
            );
        }
        assert_eq!(
            classify_source_role(
                Path::new("src/authored.js"),
                b"export const compact = true;",
                &[],
            ),
            Ok(SourceRole::Primary)
        );
    }
    #[test]
    fn explicit_role_conflicts_report_every_disagreeing_role() {
        let error = classify_source_role(
            Path::new("src/work.js"),
            b"function work() {}",
            &[
                SourceRoleRule::test("src/*.js"),
                SourceRoleRule::fixture("src/work.js"),
            ],
        )
        .unwrap_err();
        assert_eq!(error, "test, fixture");
    }
}
