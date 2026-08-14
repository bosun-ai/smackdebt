use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};

use assert_cmd::cargo::cargo_bin;

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
    repository.commit(commit(
        "coverage inputs",
        "Coverage Fixture",
        "coverage@example.invalid",
        "2026-03-01T12:00:00Z",
    ));
    repository.make_unreadable("src/failed.js");
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
