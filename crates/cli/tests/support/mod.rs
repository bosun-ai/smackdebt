pub(crate) mod edges;
pub(crate) mod hermetic;

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};

use assert_cmd::cargo::cargo_bin;
use edges::assert_no_dependency_edge_rows;
use hermetic::hermetic_env;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Identity<'a> {
    pub name: &'a str,
    pub address: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Commit<'a> {
    pub message: &'a str,
    pub identity: Identity<'a>,
    pub date: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum WorktreeEdit<'a> {
    Write(&'a str, &'a [u8]),
    Rename(&'a str, &'a str),
    Delete(&'a str),
}

#[derive(Debug)]
pub(crate) struct GeneratedRepository {
    directory: tempfile::TempDir,
}

impl GeneratedRepository {
    pub(crate) fn new(branch: &str) -> Self {
        let directory = tempfile::tempdir().expect("create generated repository");
        run_git(directory.path(), ["init", "-q", "-b", branch], None);
        run_git(
            directory.path(),
            ["config", "user.name", "Smackdebt Acceptance"],
            None,
        );
        run_git(
            directory.path(),
            ["config", "user.email", "acceptance@example.invalid"],
            None,
        );
        run_git(
            directory.path(),
            ["config", "commit.gpgSign", "false"],
            None,
        );
        Self { directory }
    }

    pub(crate) fn path(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn apply(&self, edits: &[WorktreeEdit<'_>]) {
        for edit in edits {
            match edit {
                WorktreeEdit::Write(path, bytes) => {
                    let target = self.path().join(path);
                    fs::create_dir_all(target.parent().expect("file parent"))
                        .expect("create fixture directory");
                    fs::write(target, bytes).expect("write fixture file");
                }
                WorktreeEdit::Rename(from, to) => {
                    let target = self.path().join(to);
                    fs::create_dir_all(target.parent().expect("file parent"))
                        .expect("create rename directory");
                    fs::rename(self.path().join(from), target).expect("rename fixture file");
                }
                WorktreeEdit::Delete(path) => {
                    fs::remove_file(self.path().join(path)).expect("delete fixture file");
                }
            }
        }
    }

    pub(crate) fn commit(&self, commit: Commit<'_>) {
        run_git(self.path(), ["add", "-A"], Some(commit));
        run_git(self.path(), ["commit", "-qm", commit.message], Some(commit));
    }

    pub(crate) fn write(&self, path: &str, bytes: &[u8]) {
        self.apply(&[WorktreeEdit::Write(path, bytes)]);
    }

    #[cfg(unix)]
    pub(crate) fn make_unreadable(&self, path: &str) {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(self.path().join(path), fs::Permissions::from_mode(0o000))
            .expect("make fixture source unreadable");
    }

    pub(crate) fn facts<T: serde::de::DeserializeOwned>(&self, name: &str) -> T {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/unified/facts")
            .join(name);
        serde_json::from_slice(&fs::read(path).expect("read fixture facts"))
            .expect("parse fixture facts")
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Invocation {
    arguments: Vec<OsString>,
    columns: u16,
    color: Option<&'static str>,
    jobs: Option<usize>,
    evidence: bool,
    no_color: bool,
}

impl Invocation {
    pub(crate) fn new(arguments: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        Self {
            arguments: arguments.into_iter().map(Into::into).collect(),
            columns: 120,
            color: Some("never"),
            jobs: Some(1),
            evidence: false,
            no_color: true,
        }
    }

    pub(crate) const fn columns(mut self, columns: u16) -> Self {
        self.columns = columns;
        self
    }

    pub(crate) const fn color(mut self, color: &'static str) -> Self {
        self.color = Some(color);
        self
    }

    pub(crate) const fn automatic_color(mut self) -> Self {
        self.color = None;
        self
    }

    pub(crate) const fn without_no_color(mut self) -> Self {
        self.no_color = false;
        self
    }

    pub(crate) const fn automatic_workers(mut self) -> Self {
        self.jobs = None;
        self
    }

    #[cfg(feature = "evidence-stats")]
    pub(crate) const fn evidence(mut self) -> Self {
        self.evidence = true;
        self
    }

    pub(crate) fn run(&self, directory: &Path) -> ProcessResult {
        let mut command = Command::new(cargo_bin!("smackdebt"));
        hermetic_env(&mut command);
        command
            .current_dir(directory)
            .args(&self.arguments)
            .env("COLUMNS", self.columns.to_string())
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0");
        if self.no_color {
            command.env("NO_COLOR", "1");
        } else {
            command.env_remove("NO_COLOR");
        }
        if !self.arguments.iter().any(|value| value == "--json")
            && !self.arguments.iter().any(|value| value == "--color")
            && let Some(color) = self.color
        {
            command.args(["--color", color]);
        }
        if let Some(jobs) = self.jobs
            && !self.arguments.iter().any(|value| value == "--jobs")
        {
            command.args(["--jobs", &jobs.to_string()]);
        }
        if self.evidence {
            command.env("SMACKDEBT_EVIDENCE_STATS", "1");
        }
        let output = command.output().expect("run built command");
        // Every human result this suite produces carries the invariant, so a
        // flow without a committed result cannot reintroduce edge rows either.
        if !self.arguments.iter().any(|value| value == "--json") {
            assert_no_dependency_edge_rows(
                &String::from_utf8_lossy(&output.stdout),
                &format!("{:?}", self.arguments),
            );
        }
        ProcessResult {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ProcessResult {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

impl ProcessResult {
    pub(crate) fn success(&self) {
        assert_eq!(self.status.code(), Some(0), "{}", self.stderr_text());
        assert!(self.stderr.is_empty(), "{}", self.stderr_text());
    }

    pub(crate) fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

pub(crate) fn copy_language_truth_files(repository: &GeneratedRepository) {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../languages/tests/fixtures");
    for name in [
        "go.go",
        "c.c",
        "cpp.cpp",
        "java.java",
        "javascript.js",
        "jsx.jsx",
        "python.py",
        "rust.rs",
        "typescript.ts",
        "tsx.tsx",
        "ruby.rb",
        "vue.vue",
    ] {
        repository.write(
            &format!("src/{name}"),
            &fs::read(fixture_root.join(name)).expect("read language truth fixture"),
        );
    }
    repository.write(
        "package.json",
        br#"{"name":"all-languages","private":true}
"#,
    );
    repository.write(
        ".smackdebt.toml",
        b"[thresholds.cognitive]\nwatch = 1\nhigh = 2\n\n[thresholds.cyclomatic]\nwatch = 1\nhigh = 2\n\n[thresholds.function_lines]\nwatch = 1\nhigh = 2\n",
    );
}

pub(crate) fn source_role_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.write("package.json", b"{}\n");
    repository.write(
        ".smackdebt.toml",
        b"[thresholds.cognitive]\nwatch=1\nhigh=2\n[thresholds.cyclomatic]\nwatch=1\nhigh=2\n[thresholds.function_lines]\nwatch=1\nhigh=2\n",
    );
    let source = b"export function work(a, b) { if (a) { if (b) { return 1; } } return 0; }\n";
    for path in [
        "src/main.js",
        "tests/work.js",
        "examples/work.js",
        "benches/work.js",
        "fixtures/work.js",
    ] {
        repository.write(path, source);
    }
    repository.write(
        "tests/generated.js",
        b"// @generated\nexport function work(a, b) { if (a) { if (b) { return 1; } } return 0; }\n",
    );
    // A vendored library name under a directory no rule reads, so the name is
    // the only thing that can have decided the role.
    repository.write("share/jquery.plugin.js", source);
    repository
}

pub(crate) fn generated_javascript_repository() -> GeneratedRepository {
    fn exact_size_source(nonempty_lines: usize) -> Vec<u8> {
        let mut source =
            b"export function work(a, b) { if (a) { if (b) { return 1; } } return 0; }\n".to_vec();
        for _ in 1..nonempty_lines - 1 {
            source.extend_from_slice(b"//x\n");
        }
        source.extend_from_slice(b"//");
        source.resize(65_536, b'x');
        source
    }

    let repository = GeneratedRepository::new("main");
    repository.write("package.json", b"{}\n");
    repository.write(
        ".smackdebt.toml",
        b"[thresholds.cognitive]\nwatch=1\nhigh=2\n[thresholds.cyclomatic]\nwatch=1\nhigh=2\n[thresholds.function_lines]\nwatch=1\nhigh=2\n[source_roles]\nprimary=['configured.min.js']\n",
    );
    let source = b"export function work(a, b) { if (a) { if (b) { return 1; } } return 0; }\n";
    for path in [
        "bundles/vendor.min.js",
        "bundles/vendor.min.mjs",
        "bundles/vendor.min.cjs",
        "bundles/client.bundle.js",
        "bundles/client.bundle.mjs",
        "bundles/client.bundle.cjs",
        "bundles/client-bundle.js",
        "bundles/client-bundle.mjs",
        "bundles/client-bundle.cjs",
    ] {
        repository.write(path, source);
    }
    let collision = b"items.map(() => { if (ready) { return 1; } return 0; });\nitems.map(() => { if (ready) { return 1; } return 0; });\n";
    repository.write("bundles/collision.bundle.js", collision);
    repository.write("src/dense.tsx", &exact_size_source(128));
    repository.write("authored/large.js", &exact_size_source(129));
    repository.write("authored/compact.js", source);
    repository.write("configured.min.js", source);
    repository.write("src/client.bundle.ts", source);
    repository.write("public/app.js", source);
    repository.write("share/tool.js", source);
    repository.write("assets/editor.js", source);
    repository.write("transitions/from-generated.js", &exact_size_source(128));
    repository.write("transitions/from-primary.js", source);
    repository.commit(Commit {
        message: "test: generated javascript context",
        identity: Identity {
            name: "Generated Fixture",
            address: "generated-fixture@example.invalid",
        },
        date: "2026-01-01T12:00:00Z",
    });
    repository.write(
        "transitions/from-generated.js",
        b"export function work(a, b, c) { if (a) { if (b) { if (c) { return 3; } } } return 0; }\n",
    );
    let mut primary_to_generated = exact_size_source(128);
    let first_line_end = primary_to_generated
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap();
    primary_to_generated.splice(
        ..=first_line_end,
        b"export function work(a, b, c) { if (a) { if (b) { if (c) { return 4; } } } return 0; }\n"
            .iter()
            .copied(),
    );
    primary_to_generated.truncate(65_536);
    repository.write("transitions/from-primary.js", &primary_to_generated);
    let mut changed_collision = b"// changed outside the callbacks\n".to_vec();
    changed_collision.extend_from_slice(collision);
    repository.write("bundles/collision.bundle.js", &changed_collision);
    repository.write(
        "bundles/vendor.min.js",
        b"export function work(a, b, c) { if (a) { if (b) { if (c) { return 2; } } } return 0; }\n",
    );
    repository.commit(Commit {
        message: "test: change generated javascript context",
        identity: Identity {
            name: "Generated Fixture",
            address: "generated-fixture@example.invalid",
        },
        date: "2026-01-02T12:00:00Z",
    });
    repository
}

pub(crate) fn comparison_trust_warning_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.write(
        "package.json",
        b"{\"name\":\"trust-warning\",\"private\":true}\n",
    );
    repository.write("README.txt", b"fixture documentation\n");
    repository.write("docs/note.md", b"fixture documentation\n");
    repository.write("page.astro", b"<h1>Before</h1>\n");
    repository.commit(commit(
        "base",
        "Fixture Author",
        "fixture@example.invalid",
        "2026-02-01T12:00:00Z",
    ));
    repository.write("page.astro", b"<h1>After</h1>\n");
    repository
}

/// A repository whose findings tie until the role class and hot rank keys.
///
/// `src/hot.js` carries fewer statements than `src/cold.js` but changes in five
/// commits, and `spec/cold.js` repeats the cold file under a test role whose
/// path sorts before the primary one.
///
/// `src/rich.js` and `spec/rich.js` are the cold, signal-heavy counterweights:
/// each carries three signals at the watch rating where every other file
/// carries one, so a rank that read the signal counts before role class and hot
/// state would list them above the production debt a reader came for.
pub(crate) fn deepened_signal_repository() -> GeneratedRepository {
    fn statements(name: &str, count: usize) -> Vec<u8> {
        let mut source = format!("export function {name}() {{\n");
        for index in 0..count {
            source.push_str(&format!("  const value{index} = {index};\n"));
        }
        source.push_str("  return 0;\n}\n");
        source.into_bytes()
    }

    /// Source reaching the watch threshold of cognitive complexity, cyclomatic
    /// complexity, and logical lines at once, without reaching any high one.
    fn three_signals(name: &str) -> Vec<u8> {
        let mut source = format!("export function {name}(value) {{\n  let total = 0;\n");
        for index in 0..16 {
            source.push_str(&format!(
                "  if (value === {index}) {{\n    total = total + {index};\n  }}\n"
            ));
        }
        for index in 0..16 {
            source.push_str(&format!(
                "  const step{index} = total + {index};\n  total = step{index};\n"
            ));
        }
        source.push_str("  return total;\n}\n");
        source.into_bytes()
    }

    let repository = GeneratedRepository::new("main");
    repository.write("package.json", b"{\"name\":\"deepened\"}\n");
    repository.write("src/cold.js", &statements("cold", 59));
    repository.write("spec/cold.js", &statements("covered", 59));
    repository.write("src/rich.js", &three_signals("rich"));
    repository.write("spec/rich.js", &three_signals("coveredRich"));
    repository.write("src/hot.js", &statements("hot", 54));
    repository.commit(commit(
        "test: deepened signals",
        "Signal Fixture",
        "signal@example.invalid",
        "2026-01-01T12:00:00Z",
    ));
    for revision in 1..5 {
        repository.write("src/hot.js", &statements("hot", 54 - revision));
        repository.commit(commit(
            "test: change the hot file",
            "Signal Fixture",
            "signal@example.invalid",
            &format!("2026-01-0{}T12:00:00Z", revision + 1),
        ));
    }
    repository
}

/// A repository that produces every derived signal table at once.
///
/// It carries a stable package that depends on a less stable one through two
/// references, an oversized file and container, a file nothing depends on, a
/// rated file touched often enough to be hot, and a package whose commits all
/// come from one contributor.
pub(crate) fn signal_table_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.write(
        ".smackdebt.toml",
        b"[thresholds]\ncognitive = { watch = 1, high = 2 }\ncyclomatic = { watch = 2, high = 4 }\nfunction_lines = { watch = 4, high = 8 }\nfile_lines = { watch = 30, high = 60 }\ncontainer_lines = { watch = 5, high = 10 }\n",
    );
    for package in ["core", "util", "helper", "app", "web"] {
        repository.write(
            &format!("{package}/package.json"),
            format!("{{\"name\":\"{package}\",\"private\":true}}\n").as_bytes(),
        );
    }
    let mut oversized = String::from("export function wide(value) {\n");
    for index in 0..40 {
        oversized.push_str(&format!("  const step{index} = value + {index};\n"));
    }
    oversized.push_str("  return 0;\n}\n");
    let mut container = String::from("export class Wide {\n  work(value) {\n");
    for index in 0..12 {
        container.push_str(&format!("    const step{index} = value + {index};\n"));
    }
    container.push_str("    return 0;\n  }\n}\n");
    repository.apply(&[
        // Two references make the core → util direction count.
        WorktreeEdit::Write(
            "core/main.js",
            b"import one from '../util/one';\nimport two from '../util/two';\nexport default function core(value) { return value ? one(value) : two(value); }\n",
        ),
        WorktreeEdit::Write("core/oversized.js", oversized.as_bytes()),
        WorktreeEdit::Write("core/container.js", container.as_bytes()),
        // Nothing imports this file and it is not a conventional entry file.
        WorktreeEdit::Write(
            "core/unused.js",
            b"export function unused(value) { return value; }\n",
        ),
        WorktreeEdit::Write(
            "util/one.js",
            b"import small from '../helper/small';\nexport default function one(value) { return small(value); }\n",
        ),
        WorktreeEdit::Write(
            "util/two.js",
            b"export default function two(value) { return value; }\n",
        ),
        WorktreeEdit::Write(
            "helper/small.js",
            b"export default function small(value) { return value; }\n",
        ),
        WorktreeEdit::Write(
            "app/main.js",
            b"import core from '../core/main';\nexport const app = core;\n",
        ),
        WorktreeEdit::Write(
            "web/main.js",
            b"import core from '../core/main';\nexport const web = core;\n",
        ),
    ]);
    repository.commit(commit(
        "signal tables",
        "Solo Fixture",
        "solo@example.invalid",
        "2026-05-01T12:00:00Z",
    ));
    // Twelve commits from one contributor make `core` both hot and concentrated,
    // and carry a healthy file along so a hotspot's maximum rating can be
    // healthy: a file is hot when it is rated and changes often, not when it
    // carries debt.
    for revision in 1..=12 {
        repository.write(
            "helper/small.js",
            format!("export default function small(value) {{ return value + {revision} - {revision}; }}\n")
                .as_bytes(),
        );
        repository.write(
            "core/main.js",
            format!(
                "import one from '../util/one';\nimport two from '../util/two';\nexport default function core(value) {{ if (value > {revision}) {{ return one(value); }} return two(value); }}\n"
            )
            .as_bytes(),
        );
        repository.commit(commit(
            "change the hot package",
            "Solo Fixture",
            "solo@example.invalid",
            "2026-05-02T12:00:00Z",
        ));
    }
    repository
}

pub(crate) fn static_architecture_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    let fixture_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static-architecture");
    for path in [
        "app/package.json",
        "app/main.js",
        "app/choice.js",
        "app/choice.ts",
        "app/helper.js",
        "core/package.json",
        "core/main.js",
        "native/Cargo.toml",
        "native/src/lib.rs",
        "native/src/helper.rs",
    ] {
        repository.write(
            path,
            &fs::read(fixture_root.join(path)).expect("read static fixture"),
        );
    }
    repository.commit(Commit {
        message: "test: static architecture",
        identity: Identity {
            name: "Static Fixture",
            address: "static@example.invalid",
        },
        date: "2026-01-01T12:00:00Z",
    });
    repository
}

/// A Rust package whose shipped source also declares an inline test module.
///
/// `src/lib.rs` imports `src/helper.rs` twice: once for the code that ships and
/// once inside `#[cfg(test)] mod tests`, so the report keeps a primary relation
/// and a test relation for one file pair. `src/only_tests.rs` is reached from
/// the test module alone, and `src/shipped.rs` is reached only from production
/// code, so the two roles stay separable in the committed JSON.
///
/// Verdict graphs are primary-only, so the test-role relations below are
/// context: `src/only_tests.rs` is outside the cycle graph while staying a used
/// file for orphan purposes. The repository holds one package, so no package
/// graph value depends on them. `src/only_tests.rs` also carries the test role
/// itself, because `#[cfg(test)] mod only_tests;` is its only declaration.
pub(crate) fn rust_test_scope_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.apply(&[
        WorktreeEdit::Write(
            "Cargo.toml",
            b"[package]\nname = \"scoped\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "src/lib.rs",
            b"mod helper;\nmod shipped;\n#[cfg(test)]\nmod only_tests;\n\nuse crate::helper::ready;\nuse crate::shipped::ship;\n\npub fn run() -> bool {\n    ready() && ship()\n}\n\n#[cfg(test)]\nmod tests {\n    use crate::helper::ready;\n    use crate::only_tests::sample;\n\n    #[test]\n    fn runs() {\n        assert!(ready() && sample());\n    }\n}\n",
        ),
        WorktreeEdit::Write("src/helper.rs", b"pub fn ready() -> bool {\n    true\n}\n"),
        WorktreeEdit::Write("src/shipped.rs", b"pub fn ship() -> bool {\n    true\n}\n"),
        WorktreeEdit::Write("src/only_tests.rs", b"pub fn sample() -> bool {\n    true\n}\n"),
    ]);
    repository.commit(commit(
        "test: rust test scope",
        "Scope Fixture",
        "scope@example.invalid",
        "2026-01-01T12:00:00Z",
    ));
    repository
}

/// A Rust package wired the way the language asks for.
///
/// `src/thing/mod.rs` declares `mod child;` and re-exports from it, and
/// `src/thing/child.rs` imports its parent back with `use super::*`. The two
/// imports run in opposite directions between a pair that also carries a module
/// declaration, so they are one wiring relationship rather than a file cycle:
/// the committed terminal reports no cycle while the JSON keeps every relation.
pub(crate) fn module_wiring_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.apply(&[
        WorktreeEdit::Write(
            "Cargo.toml",
            b"[package]\nname = \"wiring\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "src/lib.rs",
            b"mod thing;\n\npub use crate::thing::run;\n",
        ),
        WorktreeEdit::Write(
            "src/thing/mod.rs",
            b"mod child;\n\npub use self::child::step;\n\npub fn base() -> u32 {\n    1\n}\n\npub fn run() -> u32 {\n    step()\n}\n",
        ),
        WorktreeEdit::Write(
            "src/thing/child.rs",
            b"use super::*;\n\npub fn step() -> u32 {\n    base() + 1\n}\n",
        ),
    ]);
    repository.commit(commit(
        "test: module wiring",
        "Wiring Fixture",
        "wiring@example.invalid",
        "2026-01-01T12:00:00Z",
    ));
    repository
}

/// A Rust workspace whose only return dependencies are test code.
///
/// `alpha` ships a dependency on `beta`; `beta` depends back on `alpha` only
/// from its integration test and from a `#[cfg(test)]` module, so the pair is
/// not a package cycle. `gamma` reaches `alpha` and `zeta` from its test alone,
/// which explains their change coupling without entering a verdict graph;
/// `zeta` declares no entry file, so that reference is a manifest-name one.
///
/// The history is shaped so exactly two package pairs qualify for a coupling
/// finding — `alpha`-`beta` and `alpha`-`gamma` — and both have a code
/// dependency to explain them, the second one from test source only.
pub(crate) fn test_scoped_workspace_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.apply(&[
        WorktreeEdit::Write(
            "crates/alpha/Cargo.toml",
            b"[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "crates/alpha/src/lib.rs",
            b"use beta::helper;\n\npub fn run() -> u32 {\n    helper()\n}\n",
        ),
        WorktreeEdit::Write(
            "crates/beta/Cargo.toml",
            b"[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "crates/beta/src/lib.rs",
            b"pub fn helper() -> u32 {\n    1\n}\n\n#[cfg(test)]\nmod tests {\n    use alpha::run;\n\n    #[test]\n    fn covers() {\n        assert_eq!(run(), 1);\n    }\n}\n",
        ),
        WorktreeEdit::Write(
            "crates/beta/tests/integration.rs",
            b"use alpha::run;\n\n#[test]\nfn integrates() {\n    assert_eq!(run(), 1);\n}\n",
        ),
        WorktreeEdit::Write(
            "crates/gamma/Cargo.toml",
            b"[package]\nname = \"gamma\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "crates/gamma/src/lib.rs",
            b"pub fn gamma() -> u32 {\n    2\n}\n",
        ),
        WorktreeEdit::Write(
            "crates/gamma/tests/integration.rs",
            b"use alpha::run;\nuse zeta::piece;\n\n#[test]\nfn integrates() {\n    assert_eq!(run() + piece(), 3);\n}\n",
        ),
        WorktreeEdit::Write(
            "crates/zeta/Cargo.toml",
            b"[package]\nname = \"zeta\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "crates/zeta/src/piece.rs",
            b"pub fn piece() -> u32 {\n    2\n}\n",
        ),
    ]);
    repository.commit(scope_commit("test: workspace", &scope_date(1)));
    for round in 0..3 {
        repository.write(
            "crates/alpha/src/lib.rs",
            format!("use beta::helper;\n\npub fn run() -> u32 {{\n    helper() + {round}\n}}\n")
                .as_bytes(),
        );
        repository.write(
            "crates/gamma/src/lib.rs",
            format!("pub fn gamma() -> u32 {{\n    2 + {round}\n}}\n").as_bytes(),
        );
        repository.commit(scope_commit(
            "feat: alpha and gamma",
            &scope_date(2 + round),
        ));
    }
    for round in 0..3 {
        repository.write(
            "crates/alpha/src/lib.rs",
            format!(
                "use beta::helper;\n\npub fn run() -> u32 {{\n    helper() + {round} + 1\n}}\n"
            )
            .as_bytes(),
        );
        repository.write(
            "crates/beta/src/lib.rs",
            format!(
                "pub fn helper() -> u32 {{\n    {}\n}}\n\n#[cfg(test)]\nmod tests {{\n    use alpha::run;\n\n    #[test]\n    fn covers() {{\n        assert_eq!(run(), 1);\n    }}\n}}\n",
                round + 1
            )
            .as_bytes(),
        );
        repository.commit(scope_commit("feat: alpha and beta", &scope_date(5 + round)));
    }
    repository
}

fn scope_commit<'a>(message: &'a str, date: &'a str) -> Commit<'a> {
    commit(message, "Scope Fixture", "scope@example.invalid", date)
}

/// The fixture date of one scope-fixture day, which stays within one month.
fn scope_date(day: u32) -> String {
    format!("2026-01-0{day}T12:00:00Z")
}

/// A Rust workspace whose production dependency runs the wrong way.
///
/// `a` is depended on by `x` and depends on `b`, which itself depends on `c`
/// and `d`. That makes `b` the more unstable of the pair, so `a` -> `b` is a
/// stable-dependency violation. The same shape driven by a `#[cfg(test)]`
/// import instead is not a production direction and creates no finding.
pub(crate) fn stable_dependency_repository(test_scoped: bool) -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    let a_source: &[u8] = if test_scoped {
        b"pub fn run() -> u32 {\n    3\n}\n\n#[cfg(test)]\nmod tests {\n    use b::one;\n    use b::two;\n\n    #[test]\n    fn covers() {\n        assert_eq!(one() + two(), 3);\n    }\n}\n"
    } else {
        b"use b::one;\nuse b::two;\n\npub fn run() -> u32 {\n    one() + two()\n}\n"
    };
    repository.apply(&[
        WorktreeEdit::Write(
            "crates/x/Cargo.toml",
            b"[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "crates/x/src/lib.rs",
            b"use a::run;\n\npub fn top() -> u32 {\n    run()\n}\n",
        ),
        WorktreeEdit::Write(
            "crates/a/Cargo.toml",
            b"[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write("crates/a/src/lib.rs", a_source),
        WorktreeEdit::Write(
            "crates/b/Cargo.toml",
            b"[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "crates/b/src/lib.rs",
            b"use c::cee;\nuse d::dee;\n\npub fn one() -> u32 {\n    cee()\n}\n\npub fn two() -> u32 {\n    dee()\n}\n",
        ),
        WorktreeEdit::Write(
            "crates/c/Cargo.toml",
            b"[package]\nname = \"c\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "crates/c/src/lib.rs",
            b"pub fn cee() -> u32 {\n    1\n}\n",
        ),
        WorktreeEdit::Write(
            "crates/d/Cargo.toml",
            b"[package]\nname = \"d\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        WorktreeEdit::Write(
            "crates/d/src/lib.rs",
            b"pub fn dee() -> u32 {\n    2\n}\n",
        ),
    ]);
    repository.commit(scope_commit(
        "test: stable dependency direction",
        &scope_date(1),
    ));
    repository
}

/// A workspace whose packages form one dependency chain beside one package
/// wide enough to state a file reach.
///
/// `a` depends on `b`, `b` on `c`, and `c` on `d`, so a change in `d` reaches
/// four of the six packages, counting `d` itself. `e` depends on nothing and
/// nothing depends on it. `wide` holds a library root that declares twenty
/// modules chained one into the next, so a change in the last module reaches
/// twenty of that package's twenty-one files: the module declarations are
/// ownership rather than use, so the library root itself stays outside the
/// chain.
pub(crate) fn propagation_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    for (name, source) in [
        ("a", "use b::one;\n\npub fn run() -> u32 {\n    one()\n}\n"),
        ("b", "use c::two;\n\npub fn one() -> u32 {\n    two()\n}\n"),
        (
            "c",
            "use d::three;\n\npub fn two() -> u32 {\n    three()\n}\n",
        ),
        ("d", "pub fn three() -> u32 {\n    3\n}\n"),
        ("e", "pub fn alone() -> u32 {\n    1\n}\n"),
    ] {
        repository.write(
            &format!("crates/{name}/Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
                .as_bytes(),
        );
        repository.write(&format!("crates/{name}/src/lib.rs"), source.as_bytes());
    }
    repository.write(
        "crates/wide/Cargo.toml",
        b"[package]\nname = \"wide\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    let mut root = String::new();
    for index in 0..WIDE_PACKAGE_MODULES {
        root.push_str(&format!("mod step{index:02};\n"));
        let source = if index + 1 == WIDE_PACKAGE_MODULES {
            format!("pub fn step{index:02}() -> u32 {{\n    1\n}}\n")
        } else {
            let next = index + 1;
            format!(
                "use crate::step{next:02}::step{next:02};\n\npub fn step{index:02}() -> u32 {{\n    step{next:02}() + 1\n}}\n"
            )
        };
        repository.write(
            &format!("crates/wide/src/step{index:02}.rs"),
            source.as_bytes(),
        );
    }
    repository.write("crates/wide/src/lib.rs", root.as_bytes());
    repository.commit(scope_commit("test: propagation reach", &scope_date(1)));
    repository
}

/// The modules the wide package of [`propagation_repository`] declares, which
/// is one more than the file floor the reach sentence needs once the library
/// root is counted.
const WIDE_PACKAGE_MODULES: usize = 20;

/// A repository whose files hold one import cycle, sized by its caller.
///
/// With `files` files and a cycle of `cycle` of them, the largest component is
/// stated only when it clears both core floors: six of twenty does, and three
/// of two hundred is below the absolute floor and the proportional one alike.
/// A cycle of none writes the same files with no import at all, which is the
/// same tree without the fact.
pub(crate) fn core_repository(files: usize, cycle: usize) -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.write("package.json", b"{\"name\":\"core\",\"private\":true}\n");
    for index in 0..files {
        let source = if index < cycle {
            let next = (index + 1) % cycle;
            format!(
                "import {{ unit{next:03} }} from './unit{next:03}.js';\nexport function unit{index:03}() {{ return unit{next:03}(); }}\n"
            )
        } else {
            format!("export function unit{index:03}() {{ return 1; }}\n")
        };
        repository.write(&format!("src/unit{index:03}.js"), source.as_bytes());
    }
    repository.commit(scope_commit("test: core size", &scope_date(2)));
    repository
}

/// A workspace whose packages import each other by declared manifest name.
pub(crate) fn workspace_manifest_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.apply(&[
        WorktreeEdit::Write("package.json", b"{\"name\":\"@acme/workspace\"}\n"),
        WorktreeEdit::Write("index.js", b"export const workspace = 1;\n"),
        WorktreeEdit::Write(
            "crates/core/Cargo.toml",
            b"[package]\nname = \"acme-core\"\nversion = \"0.1.0\"\n",
        ),
        WorktreeEdit::Write(
            "crates/core/src/lib.rs",
            b"mod helper;\npub use crate::helper::value;\nuse renamed_lib::renamed;\npub fn core(input: i32) -> i32 { renamed(value(input)) }\n",
        ),
        WorktreeEdit::Write(
            "crates/core/src/helper.rs",
            b"pub fn value(input: i32) -> i32 { input + 1 }\n",
        ),
        WorktreeEdit::Write(
            "crates/renamed/Cargo.toml",
            b"[package]\nname = \"acme-renamed\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"renamed_lib\"\n",
        ),
        WorktreeEdit::Write(
            "crates/renamed/src/lib.rs",
            b"use acme_core::core;\npub fn renamed(input: i32) -> i32 { if input > 1 { core(0) } else { input } }\n",
        ),
        WorktreeEdit::Write("ui/package.json", b"{\"name\":\"@acme/ui\"}\n"),
        WorktreeEdit::Write("ui/index.js", b"export const button = 1;\n"),
        WorktreeEdit::Write("web/package.json", b"{\"name\":\"@acme/web\"}\n"),
        WorktreeEdit::Write(
            "web/index.js",
            b"import { button } from '@acme/ui/button';\nexport const web = button;\n",
        ),
        WorktreeEdit::Write(
            "py/pyproject.toml",
            b"[project]\nname = \"acme_py\"\nversion = \"0.1.0\"\n",
        ),
        WorktreeEdit::Write("py/acme_py/__init__.py", b"def value():\n    return 1\n"),
        WorktreeEdit::Write(
            "pyapp/pyproject.toml",
            b"[project]\nname = \"acme-pyapp\"\nversion = \"0.1.0\"\n",
        ),
        WorktreeEdit::Write(
            "pyapp/app.py",
            b"import acme_py\n\n\ndef app():\n    return acme_py.value()\n",
        ),
        WorktreeEdit::Write(
            "rb/acme.gemspec",
            b"Gem::Specification.new do |spec|\n  spec.name = \"acme_rb\"\nend\n",
        ),
        WorktreeEdit::Write("rb/lib/acme_rb.rb", b"def value\n  1\nend\n"),
        WorktreeEdit::Write(
            "rbapp/acme_rbapp.gemspec",
            b"Gem::Specification.new do |spec|\n  spec.name = \"acme_rbapp\"\nend\n",
        ),
        WorktreeEdit::Write("rbapp/lib/app.rb", b"require 'acme_rb'\n\ndef app\n  value\nend\n"),
        WorktreeEdit::Write(
            "dup/one/Cargo.toml",
            b"[package]\nname = \"acme-dup\"\nversion = \"0.1.0\"\n",
        ),
        WorktreeEdit::Write("dup/one/src/lib.rs", b"pub fn thing() -> i32 { 1 }\n"),
        WorktreeEdit::Write(
            "dup/two/Cargo.toml",
            b"[package]\nname = \"acme-dup\"\nversion = \"0.1.0\"\n",
        ),
        WorktreeEdit::Write("dup/two/src/lib.rs", b"pub fn thing() -> i32 { 2 }\n"),
        WorktreeEdit::Write(
            "dupuser/Cargo.toml",
            b"[package]\nname = \"acme-dupuser\"\nversion = \"0.1.0\"\n",
        ),
        WorktreeEdit::Write(
            "dupuser/src/lib.rs",
            b"use acme_dup::thing;\npub fn user() -> i32 { thing() }\n",
        ),
    ]);
    repository.commit(commit(
        "workspace packages",
        "Workspace Fixture",
        "workspace@example.invalid",
        "2026-04-01T12:00:00Z",
    ));
    for version in 2..=3 {
        repository.apply(&[
            WorktreeEdit::Write(
                "crates/core/src/helper.rs",
                format!("pub fn value(input: i32) -> i32 {{ input + {version} }}\n").as_bytes(),
            ),
            WorktreeEdit::Write(
                "crates/renamed/src/lib.rs",
                format!("use acme_core::core;\npub fn renamed(input: i32) -> i32 {{ if input > {version} {{ core(0) }} else {{ input }} }}\n").as_bytes(),
            ),
            WorktreeEdit::Write(
                "index.js",
                format!("export const workspace = {version};\n").as_bytes(),
            ),
        ]);
        repository.commit(commit(
            "compiled packages change together",
            "Workspace Fixture",
            "workspace@example.invalid",
            "2026-04-02T12:00:00Z",
        ));
        repository.apply(&[
            WorktreeEdit::Write(
                "ui/index.js",
                format!("export const button = {version};\n").as_bytes(),
            ),
            WorktreeEdit::Write(
                "rb/lib/acme_rb.rb",
                format!("def value\n  {version}\nend\n").as_bytes(),
            ),
        ]);
        repository.commit(commit(
            "unrelated packages change together",
            "Workspace Fixture",
            "workspace@example.invalid",
            "2026-04-03T12:00:00Z",
        ));
    }
    repository
}

pub(crate) fn evolution_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    for package in ["a", "b", "c"] {
        repository.write(
            &format!("{package}/package.json"),
            format!("{{\"name\":\"{package}\",\"private\":true}}\n").as_bytes(),
        );
    }
    repository.apply(&[
        WorktreeEdit::Write("a/old.js", b"export const a = 1;\n"),
        WorktreeEdit::Write("a/binary.js", b"\0one"),
        WorktreeEdit::Write("b/main.js", b"export const b = 1;\n"),
        WorktreeEdit::Write("c/main.js", b"export const c = 1;\n"),
    ]);
    repository.commit(commit(
        "initial",
        "Alice Example",
        "alice@example.invalid",
        "2026-01-01T12:00:00Z",
    ));
    repository.apply(&[
        WorktreeEdit::Rename("a/old.js", "a/main.js"),
        WorktreeEdit::Write("b/main.js", b"export const b = 2;\n"),
    ]);
    repository.commit(commit(
        "rename together",
        "Alias Person",
        "alias@example.invalid",
        "2026-01-02T12:00:00Z",
    ));
    repository.apply(&[
        WorktreeEdit::Write(
            ".mailmap",
            b"Alice Example <alice@example.invalid> Alias Person <alias@example.invalid>\n",
        ),
        WorktreeEdit::Write("a/main.js", b"export const a = 3;\n"),
        WorktreeEdit::Write("b/main.js", b"export const b = 3;\n"),
    ]);
    repository.commit(commit(
        "together again",
        "Bob Example",
        "bob@example.invalid",
        "2026-01-03T12:00:00Z",
    ));
    repository.apply(&[
        WorktreeEdit::Write("a/main.js", b"export const a = 4;\n"),
        WorktreeEdit::Write("a/binary.js", b"\0two"),
    ]);
    repository.commit(commit(
        "a only one",
        "Alice Example",
        "alice@example.invalid",
        "2026-01-04T12:00:00Z",
    ));
    repository.write("a/main.js", b"export const a = 5;\n");
    repository.commit(commit(
        "a only two",
        "Alice Example",
        "alice@example.invalid",
        "2026-01-05T12:00:00Z",
    ));
    repository.write("b/main.js", b"export const b = 6;\n");
    repository.commit(commit(
        "b only",
        "Bob Example",
        "bob@example.invalid",
        "2026-01-06T12:00:00Z",
    ));
    repository
}

/// The files the sweeping commit of [`bulk_commit_repository`] adds, chosen so
/// that commit changes thirty change-graph files once the two files it rewrites
/// are counted — five more than the bulk-commit guard admits.
const BULK_FIXTURE_ADDED_FILES: usize = 28;

/// A repository whose history holds one sweeping commit beside five ordinary
/// ones and five that touch neither of their files.
///
/// `left/src/a.js` and `right/src/b.js` change together in five two-file
/// commits, which retains one pair four directories apart. The sixth commit
/// rewrites both of them and adds twenty-eight files, so thirty files enter the
/// change graph at once: the guard holds that commit back from pair
/// accumulation, and the pair's shared and union counts both stay at five,
/// while churn, touches, and package change coupling still count it in full.
///
/// The five three-file commits in `wide` afterwards touch neither of the pair's
/// files, so every pair operand is untouched, and they exist for the guard's
/// other half: the repository's eleven amplification observations are five of
/// two files, five of three, and the sweep's one of thirty, whose nearest-rank
/// median is 3. Without the sweeping commit's one observation the sample would
/// be ten commits at a median of two and the repository would state nothing, so
/// the stated fact is itself the proof that the guard never reached it.
pub(crate) fn bulk_commit_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    for package in ["left", "right", "wide"] {
        repository.write(
            &format!("{package}/package.json"),
            format!("{{\"name\":\"{package}\",\"private\":true}}\n").as_bytes(),
        );
    }
    for version in 1..=5 {
        repository.apply(&[
            WorktreeEdit::Write(
                "left/src/a.js",
                format!("export const a = {version};\n").as_bytes(),
            ),
            WorktreeEdit::Write(
                "right/src/b.js",
                format!("export const b = {version};\n").as_bytes(),
            ),
        ]);
        repository.commit(commit(
            "two files change together",
            "Bulk Fixture",
            "bulk@example.invalid",
            &format!("2026-02-0{version}T12:00:00Z"),
        ));
    }
    let paths: Vec<String> = (0..BULK_FIXTURE_ADDED_FILES)
        .map(|index| {
            let package = if index % 2 == 0 { "left" } else { "right" };
            format!("{package}/src/bulk/unit{index:02}.js")
        })
        .collect();
    let sources: Vec<String> = (0..BULK_FIXTURE_ADDED_FILES)
        .map(|index| format!("export const unit{index:02} = 1;\n"))
        .collect();
    let mut sweep = vec![
        WorktreeEdit::Write("left/src/a.js", b"export const a = 6;\n"),
        WorktreeEdit::Write("right/src/b.js", b"export const b = 6;\n"),
    ];
    sweep.extend(
        paths
            .iter()
            .zip(&sources)
            .map(|(path, source)| WorktreeEdit::Write(path, source.as_bytes())),
    );
    repository.apply(&sweep);
    repository.commit(commit(
        "one sweeping change",
        "Bulk Fixture",
        "bulk@example.invalid",
        "2026-02-06T12:00:00Z",
    ));
    for version in 1..=5 {
        let sources: Vec<String> = (0..3)
            .map(|unit| format!("export const wide{unit} = {version};\n"))
            .collect();
        let edits: Vec<_> = sources
            .iter()
            .enumerate()
            .map(|(unit, source)| WorktreeEdit::Write(WIDE_PATHS[unit], source.as_bytes()))
            .collect();
        repository.apply(&edits);
        repository.commit(commit(
            "three files change together",
            "Bulk Fixture",
            "bulk@example.invalid",
            &format!("2026-02-1{version}T12:00:00Z"),
        ));
    }
    repository
}

/// The three files the trailing commits of [`bulk_commit_repository`] rewrite,
/// which sit in one package no pair operand of that fixture names.
const WIDE_PATHS: [&str; 3] = ["wide/src/one.js", "wide/src/two.js", "wide/src/three.js"];

/// A repository whose commits give three scopes three different answers about
/// what a typical change costs.
///
/// Twelve commits rewrite four files of `core/src`, five rewrite three files of
/// `edge/src`, and ten rewrite two files of `quiet/src`. The repository root
/// therefore sees twenty-seven observations — ten of two, five of three, and
/// twelve of four — whose nearest-rank median is 3, while `core` and `core/src`
/// see twelve of four and state 4. `edge` has too few commits to state
/// anything and `quiet` has enough commits but a median below the floor, so
/// each states nothing rather than something weak.
pub(crate) fn amplification_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    for package in ["core", "edge", "quiet"] {
        repository.write(
            &format!("{package}/package.json"),
            format!("{{\"name\":\"{package}\",\"private\":true}}\n").as_bytes(),
        );
    }
    let mut day = 1;
    for (package, files, commits) in [("core", 4, 12), ("edge", 3, 5), ("quiet", 2, 10)] {
        for version in 1..=commits {
            let paths: Vec<String> = (0..files)
                .map(|unit| format!("{package}/src/unit{unit}.js"))
                .collect();
            let sources: Vec<String> = (0..files)
                .map(|unit| format!("export const unit{unit} = {version};\n"))
                .collect();
            let edits: Vec<_> = paths
                .iter()
                .zip(&sources)
                .map(|(path, source)| WorktreeEdit::Write(path, source.as_bytes()))
                .collect();
            repository.apply(&edits);
            repository.commit(commit(
                "one change of several files",
                "Amplification Fixture",
                "amplification@example.invalid",
                &format!("2026-01-{day:02}T12:00:00Z"),
            ));
            day += 1;
        }
    }
    repository
}

/// A repository whose history states one leakage finding of each kind and
/// every case that must state nothing.
///
/// Seven commits rewrite `app/interface.js` beside the `web/src/follower.js`
/// that imports it, and five more rewrite the interface beside its test, so the
/// pair shares 7 of 12 commits three directories apart and the interface leaks.
/// Six commits rewrite `data/src/model.js` beside `data/store/lib/keys.js`, and
/// three rewrite the model alone, so that pair shares 6 of 9 commits three
/// directories apart with nothing in either package that reaches the other.
///
/// A second unlinked pair with the same counts states the other half of the
/// claiming rule. `data/src/rules.js` is complex enough for a finding of its
/// own, so it already carries a card before any history is read and its hidden
/// finding is one more line on that card rather than a card of its own. Its
/// partner `data/store/lib/cache.js` therefore reaches a reader only through
/// that line, which is what makes the line name it.
///
/// The nesting of `data/store` inside `data` is deliberate. Two packages that
/// keep changing together with no dependency are a `shotgun_pair` as well, and
/// a nested pair is never a coupling row, so the file findings add exactly one
/// card to each affected scope and the strict-launch count states that.
///
/// Three cases must produce nothing. `web/src/one.js` and `web/src/two.js`
/// change together five times inside one directory, which is what a directory
/// is for. `app/tests/unit/interface.test.js` imports the interface and changes
/// with it five times, which is good practice rather than leakage. The Rust
/// files of `wiring` are joined only by module declarations — `src/lib.rs`
/// declares `a`, which declares `b` — so the pair two directories apart that
/// shares all six of its commits is connected and never named. One further
/// pair, `edge/src/a.js` with `edge/lib/b.js`, is retained at three shared
/// commits and reaches no detector floor, so it exists in the machine report
/// and in no human view.
///
/// Authorship alternates so no package concentrates on one contributor, and no
/// package reaches the concentration commit floor with a single author.
pub(crate) fn change_leakage_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.write(
        ".smackdebt.toml",
        b"[thresholds.cognitive]\nwatch = 2\nhigh = 5\n",
    );
    for package in ["app", "web", "data", "data/store", "edge"] {
        let name = package.replace('/', "-");
        repository.write(
            &format!("{package}/package.json"),
            format!("{{\"name\":\"{name}\",\"private\":true}}\n").as_bytes(),
        );
    }
    repository.write(
        "wiring/Cargo.toml",
        b"[package]\nname = \"wiring\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    let mut day = 0;
    for (message, files, count) in LEAKAGE_COMMITS {
        leakage_commits(&repository, &mut day, message, files, count);
    }
    repository
}

/// The interface whose importers follow it, complex enough for one High
/// finding under the fixture's own thresholds so the file already carries a
/// card before any history is read.
const LEAKY_INTERFACE: &str = "export function thing(value) {\n  if (value) {\n    if (value > 1) {\n      if (value > {}) {\n        return 3;\n      }\n    }\n  }\n  return 0;\n}\n";

/// The file of a hidden pair that already carries a card, complex enough for
/// one Watch finding under the fixture's own thresholds and no more, so the
/// card exists before history is read and the hidden finding joins it.
const CLAIMING_FILE: &str = "export function rule(value) {\n  if (value) {\n    if (value > {}) {\n      return 1;\n    }\n  }\n  return 0;\n}\n";

/// The same file at a version the paired commits never wrote, so the commits
/// that rewrite it alone are changes rather than no-ops, exactly as the model's
/// own solo template is.
const CLAIMING_FILE_ALONE: &str = "export function rule(value) {\n  if (value) {\n    if (value > 1{}) {\n      return 1;\n    }\n  }\n  return 0;\n}\n";

/// One commit group: a message, the files it rewrites together as a path
/// beside the source template whose `{}` becomes the version, and how many
/// times it rewrites them.
type LeakageCommits = (&'static str, &'static [(&'static str, &'static str)], usize);

/// Every commit group of [`change_leakage_repository`].
const LEAKAGE_COMMITS: [LeakageCommits; 9] = [
    (
        "the interface and the importer that follows it",
        &[
            ("app/interface.js", LEAKY_INTERFACE),
            (
                "web/src/follower.js",
                "import { thing } from '../../app/interface.js';\n\nexport const follower = thing({});\n",
            ),
        ],
        7,
    ),
    (
        "the interface and the test that exercises it",
        &[
            ("app/interface.js", LEAKY_INTERFACE),
            (
                "app/tests/unit/interface.test.js",
                "import { thing } from '../../interface.js';\n\nexport const checked = thing({});\n",
            ),
        ],
        5,
    ),
    (
        "two files no dependency connects",
        &[
            ("data/src/model.js", "export const model = {};\n"),
            ("data/store/lib/keys.js", "export const keys = {};\n"),
        ],
        6,
    ),
    (
        "the model alone",
        &[("data/src/model.js", "export const model = 1{};\n")],
        3,
    ),
    (
        "a file that already carries a card and the file it changes with",
        &[
            ("data/src/rules.js", CLAIMING_FILE),
            ("data/store/lib/cache.js", "export const cache = {};\n"),
        ],
        6,
    ),
    (
        "the rules alone",
        &[("data/src/rules.js", CLAIMING_FILE_ALONE)],
        3,
    ),
    (
        "two files of one directory",
        &[
            ("web/src/one.js", "export const one = {};\n"),
            ("web/src/two.js", "export const two = {};\n"),
        ],
        5,
    ),
    (
        "a pair below every detector floor",
        &[
            ("edge/src/a.js", "export const a = {};\n"),
            ("edge/lib/b.js", "export const b = {};\n"),
        ],
        3,
    ),
    (
        "a parent and the module its child declares",
        &[
            (
                "wiring/src/lib.rs",
                "mod a;\n\npub const VERSION: u32 = {};\n",
            ),
            ("wiring/src/a/mod.rs", "mod b;\n\npub use self::b::value;\n"),
            (
                "wiring/src/a/b/mod.rs",
                "pub fn value() -> u32 {\n    {}\n}\n",
            ),
        ],
        6,
    ),
];

/// Rewrites one group's files together, `count` times, alternating authors.
///
/// A file whose source never changes is written every time and committed once,
/// because Git records the change rather than the write: that is how the module
/// declaration between the wiring files exists without changing with them.
fn leakage_commits(
    repository: &GeneratedRepository,
    day: &mut usize,
    message: &str,
    files: &[(&str, &str)],
    count: usize,
) {
    for version in 1..=count {
        let sources: Vec<String> = files
            .iter()
            .map(|(_, template)| template.replace("{}", &version.to_string()))
            .collect();
        let edits: Vec<_> = files
            .iter()
            .zip(&sources)
            .map(|((path, _), source)| WorktreeEdit::Write(path, source.as_bytes()))
            .collect();
        repository.apply(&edits);
        let (name, address) = if (*day).is_multiple_of(2) {
            ("Ada Fixture", "ada@example.invalid")
        } else {
            ("Bo Fixture", "bo@example.invalid")
        };
        repository.commit(commit(message, name, address, &leakage_date(*day)));
        *day += 1;
    }
}

/// One fixed date per commit, so the fixture states the same history whenever
/// it runs.
fn leakage_date(index: usize) -> String {
    let day = index + 1;
    if day <= 31 {
        format!("2026-01-{day:02}T12:00:00Z")
    } else {
        format!("2026-02-{:02}T12:00:00Z", day - 31)
    }
}

pub(crate) fn worktree_change_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    for package in ["a", "b", "c", "d", "e", "f", "gone", "h", "i"] {
        repository.write(
            &format!("{package}/package.json"),
            format!("{{\"name\":\"{package}\",\"private\":true}}\n").as_bytes(),
        );
    }
    repository.write(
        ".smackdebt.toml",
        b"[thresholds.cognitive]\nwatch = 2\nhigh = 5\n\n[thresholds.cyclomatic]\nwatch = 2\nhigh = 5\n\n[thresholds.function_lines]\nwatch = 2\nhigh = 5\n",
    );
    repository.apply(&[
        WorktreeEdit::Write(
            "a/main.js",
            b"import b from '../b/main';\nexport default function a(value) {\n  if (value) {\n    if (value > 1) return b(value);\n  }\n  return 0;\n}\n",
        ),
        WorktreeEdit::Write(
            "b/main.js",
            b"import a from '../a/main';\nexport default function b(value) { return value ? a(0) : 0; }\n",
        ),
        WorktreeEdit::Write(
            "c/main.js",
            b"import d from '../d/main';\nexport default function c(value) { const result = d(value); return result; }\n",
        ),
        WorktreeEdit::Write(
            "d/main.js",
            b"export default function d(value) { return value; }\n",
        ),
        WorktreeEdit::Write(
            "e/old.js",
            b"export function renamed(value) { return value; }\n",
        ),
        WorktreeEdit::Write(
            "f/deleted.js",
            b"export function removed(value) { return value; }\n",
        ),
        WorktreeEdit::Write("h/main.js", b"export const h = 1;\n"),
        WorktreeEdit::Write("i/main.js", b"export const i = 1;\n"),
    ]);
    repository.commit(commit(
        "base",
        "First Fixture",
        "first@example.invalid",
        "2026-02-01T12:00:00Z",
    ));
    repository.apply(&[
        WorktreeEdit::Write("h/main.js", b"export const h = 2;\n"),
        WorktreeEdit::Write("i/main.js", b"export const i = 2;\n"),
    ]);
    repository.commit(commit(
        "coupled one",
        "Second Fixture",
        "second@example.invalid",
        "2026-02-02T12:00:00Z",
    ));
    repository.apply(&[
        WorktreeEdit::Write("h/main.js", b"export const h = 3;\n"),
        WorktreeEdit::Write("i/main.js", b"export const i = 3;\n"),
    ]);
    repository.commit(commit(
        "coupled two",
        "Second Fixture",
        "second@example.invalid",
        "2026-02-03T12:00:00Z",
    ));
    repository.apply(&[
        WorktreeEdit::Write("h/main.js", b"export const h = 4;\n"),
        WorktreeEdit::Write("i/main.js", b"export const i = 4;\n"),
    ]);
    repository.commit(commit(
        "coupled three",
        "First Fixture",
        "first@example.invalid",
        "2026-02-04T12:00:00Z",
    ));
    repository.apply(&[
        WorktreeEdit::Write(
            "a/main.js",
            b"import c from '../c/main';\nexport default function a(value) { return c(value); }\n",
        ),
        WorktreeEdit::Write(
            "b/main.js",
            b"export default function b(value) {\n  if (value) {\n    if (value > 1) {\n      if (value > 2) return value;\n    }\n  }\n  return 0;\n}\n",
        ),
        WorktreeEdit::Write(
            "d/main.js",
            b"import c from '../c/main';\nexport default function d(value) { return c(value); }\n",
        ),
        WorktreeEdit::Write(
            "c/main.js",
            b"import d from '../d/main';\nexport default function c(value) { const result = d(value); const copy = result; return copy; }\n",
        ),
        WorktreeEdit::Write(
            "h/main.js",
            b"import { i } from '../i/main';\nexport const h = i;\n",
        ),
        WorktreeEdit::Rename("e/old.js", "e/new.js"),
        WorktreeEdit::Write(
            "e/new.js",
            b"export function renamed(value) { if (value) { if (value > 1) return value; } return 0; }\n",
        ),
        WorktreeEdit::Delete("f/deleted.js"),
        WorktreeEdit::Delete("gone/package.json"),
        WorktreeEdit::Write("new/package.json", b"{\"name\":\"new\",\"private\":true}\n"),
        WorktreeEdit::Write(
            "new/untracked.js",
            b"export function untracked(value) { return value; }\n",
        ),
    ]);
    repository
}

pub(crate) fn ref_diff_repository() -> GeneratedRepository {
    let repository = worktree_change_repository();
    repository.commit(commit(
        "committed analysis change",
        "Review Fixture",
        "review@example.invalid",
        "2026-02-05T12:00:00Z",
    ));
    repository
}

#[cfg(unix)]
pub(crate) fn coverage_failure_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.write(
        "package.json",
        b"{\"name\":\"coverage\",\"private\":true}\n",
    );
    repository.write("src/good.js", b"export function good() { return 1; }\n");
    repository.write("src/failed.js", b"export function failed() { return 1; }\n");
    repository.write("src/unsupported.kt", b"fun unsupported() = 1\n");
    // One file the parser recovers from where nothing it measured was
    // touched: the report counts it as recovered and still trusts it.
    repository.write(
        "src/tail.js",
        b"export function tail(value) { return value; }\n\nexport function unterminated(\n",
    );
    repository.commit(commit(
        "coverage inputs",
        "Coverage Fixture",
        "coverage@example.invalid",
        "2026-03-01T12:00:00Z",
    ));
    repository.make_unreadable("src/failed.js");
    repository
}

/// A repository whose worst area is a directory rather than a file.
///
/// `wide/` holds thirty files that each carry one High finding, so the root
/// view's `next:` line proposes a directory holding more cards than the last
/// ladder rung. A file scope is exempt from the ladder, so a fixture whose
/// worst area is a single file cannot prove that following the report's own
/// advice stays inside the budget.
pub(crate) fn wide_directory_repository() -> GeneratedRepository {
    let repository = GeneratedRepository::new("main");
    repository.write("package.json", b"{\"name\":\"wide\",\"private\":true}\n");
    repository.write(
        ".smackdebt.toml",
        b"[thresholds]\ncognitive = { watch = 2, high = 4 }\ncyclomatic = { watch = 2, high = 4 }\n",
    );
    repository.write("thin/main.js", b"export const thin = 1;\n");
    for index in 0..30 {
        repository.write(
            &format!("wide/unit-{index}.js"),
            format!(
                "export function unit{index}(value) {{\n  if (value > 1) {{\n    if (value > 2) {{\n      if (value > 3) {{\n        return {index};\n      }}\n    }}\n  }}\n  return 0;\n}}\n"
            )
            .as_bytes(),
        );
    }
    repository.commit(commit(
        "test: wide directory",
        "Wide Fixture",
        "wide@example.invalid",
        "2026-01-01T12:00:00Z",
    ));
    repository
}

pub(crate) fn shallow_clone(source: &GeneratedRepository) -> GeneratedRepository {
    let directory = tempfile::tempdir().expect("create shallow fixture checkout");
    let source_url = format!("file://{}", source.path().display());
    run_git(
        directory.path(),
        ["clone", "-q", "--depth", "1", &source_url, "."],
        None,
    );
    GeneratedRepository { directory }
}

fn commit<'a>(message: &'a str, name: &'a str, address: &'a str, date: &'a str) -> Commit<'a> {
    Commit {
        message,
        identity: Identity { name, address },
        date,
    }
}

fn run_git<const N: usize>(directory: &Path, arguments: [&str; N], commit: Option<Commit<'_>>) {
    let mut command = Command::new("git");
    hermetic_env(&mut command);
    command
        .args(arguments)
        .current_dir(directory)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    if let Some(commit) = commit {
        command
            .env("GIT_AUTHOR_NAME", commit.identity.name)
            .env("GIT_AUTHOR_EMAIL", commit.identity.address)
            .env("GIT_COMMITTER_NAME", commit.identity.name)
            .env("GIT_COMMITTER_EMAIL", commit.identity.address)
            .env("GIT_AUTHOR_DATE", commit.date)
            .env("GIT_COMMITTER_DATE", commit.date);
    }
    let output = command.output().expect("run fixture git command");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}
