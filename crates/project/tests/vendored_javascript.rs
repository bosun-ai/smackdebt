//! What makes a JavaScript file one the repository carries rather than writes.
//!
//! Every case is one repository whose only difference from the others is the
//! single fact under test, so a role that moves names the signal that moved it.

use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use smackdebt_analysis::{FileRecord, Report, SourceRole};
use smackdebt_project::CodebaseRequest;

/// A library file nothing imports, whose one commit landed before the window.
///
/// It branches often enough to earn a finding, so a role that withdraws it from
/// the verdict is visible in the counts rather than only in the file table.
const LIBRARY: &[u8] = b"function widget(value) {\n  let total = 0;\n  if (value === 1) { total += 1; }\n  if (value === 2) { total += 2; }\n  if (value === 3) { total += 3; }\n  if (value === 4) { total += 4; }\n  if (value === 5) { total += 5; }\n  if (value === 6) { total += 6; }\n  if (value === 7) { total += 7; }\n  if (value === 8) { total += 8; }\n  if (value === 9) { total += 9; }\n  if (value === 10) { total += 10; }\n  if (value === 11) { total += 11; }\n  if (value === 12) { total += 12; }\n  return total;\n}\nwindow.widget = widget;\n";

/// The same code written as a module, which the repository is the author of.
const LIBRARY_MODULE: &[u8] = b"export function widget(value) {\n  let total = 0;\n  if (value === 1) { total += 1; }\n  if (value === 2) { total += 2; }\n  if (value === 3) { total += 3; }\n  if (value === 4) { total += 4; }\n  if (value === 5) { total += 5; }\n  if (value === 6) { total += 6; }\n  if (value === 7) { total += 7; }\n  if (value === 8) { total += 8; }\n  if (value === 9) { total += 9; }\n  if (value === 10) { total += 10; }\n  if (value === 11) { total += 11; }\n  if (value === 12) { total += 12; }\n  return total;\n}\n";

/// The file the window's commit touches, so the repository is worked on.
const WORKED_ON: &[u8] = b"export function run(value) { return value + 1; }\n";

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    /// A repository holding one worked-on module and one candidate library.
    ///
    /// The library's only commit is old enough to fall outside the default
    /// ninety-day window while the second commit stays inside it, so the window
    /// holds evidence and the library is absent from it.
    fn new(library: &str) -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        fixture.git(["init", "-q"]);
        fixture.git(["config", "user.email", "test@example.invalid"]);
        fixture.git(["config", "user.name", "Smackdebt Test"]);
        fixture.write("package.json", b"{\"name\":\"fixture\",\"private\":true}\n");
        fixture.write(library, LIBRARY);
        fixture.commit(days_ago(400));
        fixture.write("src/app.js", WORKED_ON);
        fixture.commit(days_ago(1));
        fixture
    }

    fn write(&self, path: &str, bytes: &[u8]) {
        let file = self.root.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, bytes).unwrap();
    }

    fn git<const N: usize>(&self, arguments: [&str; N]) {
        self.dated_git("", arguments);
    }

    fn commit(&self, date: String) {
        self.dated_git(&date, ["add", "-A"]);
        self.dated_git(&date, ["commit", "-qm", "fixture"]);
    }

    fn dated_git<const N: usize>(&self, date: &str, arguments: [&str; N]) {
        let output = Command::new("git")
            .args(arguments)
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
            .current_dir(self.root.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn report(&self) -> Report {
        CodebaseRequest::new(self.root.path())
            .analyze()
            .unwrap()
            .report()
            .clone()
    }
}

/// A landed instant the history window compares against, in Git's raw format.
fn days_ago(days: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("@{} +0000", now - days * 86_400)
}

fn role_of(report: &Report, path: &str) -> SourceRole {
    report
        .files()
        .iter()
        .find(|file| file.path() == path)
        .unwrap_or_else(|| panic!("no file record for {path}"))
        .role()
}

/// The units the verdict rated, which is what a context role withdraws.
fn verdict_checked(report: &Report) -> u32 {
    report
        .verdict()
        .expect("a codebase report answers")
        .counts()
        .checked()
}

#[test]
fn a_cold_unimported_javascript_library_is_vendored() {
    let fixture = Fixture::new("public/js/widget.js");
    let report = fixture.report();
    assert_eq!(
        role_of(&report, "public/js/widget.js"),
        SourceRole::Vendored
    );
    assert_eq!(role_of(&report, "src/app.js"), SourceRole::Primary);
}

#[test]
fn a_vendored_library_leaves_the_verdict_while_staying_in_the_file_table() {
    let vendored = Fixture::new("public/js/widget.js");
    let report = vendored.report();
    // The same library under a name the repository imports keeps its findings
    // inside the verdict, so the difference in counts is the role alone.
    let imported = Fixture::new("src/lib/widget.js");
    imported.write(
        "src/app.js",
        b"import './lib/widget.js';\nexport function run(value) { return window.widget(value); }\n",
    );
    imported.commit(days_ago(1));
    let imported = imported.report();

    assert_eq!(role_of(&imported, "src/lib/widget.js"), SourceRole::Primary);
    assert!(
        verdict_checked(&report) < verdict_checked(&imported),
        "vendored source still counts: {} vs {}",
        verdict_checked(&report),
        verdict_checked(&imported)
    );
    let record = report
        .files()
        .iter()
        .find(|file| file.path() == "public/js/widget.js")
        .expect("the file table keeps every selected file");
    assert_eq!(
        record.health().high(),
        0,
        "vendored health left the verdict"
    );
    assert_eq!(
        record.coverage().context_files(),
        1,
        "vendored lines are context rather than clean"
    );
    assert!(
        report
            .findings()
            .iter()
            .any(|finding| finding.file() == record.id()),
        "the file keeps its findings for its own inspection"
    );
}

#[test]
fn a_javascript_library_the_window_touched_stays_primary() {
    let fixture = Fixture::new("public/js/widget.js");
    let mut edited = b"// one line the window recorded\n".to_vec();
    edited.extend_from_slice(LIBRARY);
    fixture.write("public/js/widget.js", &edited);
    fixture.commit(days_ago(2));
    let report = fixture.report();
    assert_eq!(role_of(&report, "public/js/widget.js"), SourceRole::Primary);
}

#[test]
fn a_jquery_name_is_vendored_however_the_repository_uses_it() {
    let fixture = Fixture::new("public/js/jquery.floatThead.js");
    fixture.write(
        "src/app.js",
        b"import './jquery.floatThead.js';\nexport function run(value) { return value + 1; }\n",
    );
    fixture.write("src/jquery.floatThead.js", LIBRARY);
    fixture.commit(days_ago(1));
    let report = fixture.report();
    assert_eq!(
        role_of(&report, "src/jquery.floatThead.js"),
        SourceRole::Vendored,
        "an imported jQuery plugin edited inside the window is still vendored"
    );
    assert_eq!(
        role_of(&report, "public/js/jquery.floatThead.js"),
        SourceRole::Vendored
    );
}

#[test]
fn a_module_the_repository_wrote_is_an_orphan_rather_than_a_vendored_copy() {
    let fixture = Fixture::new("public/js/widget.js");
    fixture.write("public/js/widget.js", LIBRARY_MODULE);
    let report = fixture.report();
    assert_eq!(
        role_of(&report, "public/js/widget.js"),
        SourceRole::Primary,
        "a file that states its own exports is one the graph can speak about"
    );
}

#[test]
fn typescript_is_never_vendored_by_evidence() {
    let fixture = Fixture::new("public/js/widget.ts");
    let report = fixture.report();
    assert_eq!(
        role_of(&report, "public/js/widget.ts"),
        SourceRole::Primary,
        "authored TypeScript stays authored however cold it is"
    );
}

#[test]
fn an_entry_file_no_one_imports_stays_primary() {
    let fixture = Fixture::new("public/js/index.js");
    let report = fixture.report();
    assert_eq!(role_of(&report, "public/js/index.js"), SourceRole::Primary);
}

#[test]
fn a_file_a_tool_loads_by_name_stays_primary() {
    for name in ["postcss.config.js", ".dependency-cruiser.cjs"] {
        let fixture = Fixture::new(name);
        let report = fixture.report();
        assert_eq!(
            role_of(&report, name),
            SourceRole::Primary,
            "{name} is found by its name, so nothing importing it proves nothing"
        );
    }
}

#[test]
fn a_script_the_manifest_runs_stays_primary() {
    let fixture = Fixture::new("tools/release.js");
    fixture.write(
        "package.json",
        b"{\"name\":\"fixture\",\"private\":true,\"scripts\":{\"release\":\"node tools/release.js --dry-run\"}}\n",
    );
    let report = fixture.report();
    assert_eq!(
        role_of(&report, "tools/release.js"),
        SourceRole::Primary,
        "a script the package runs is owned by the package"
    );
}

#[test]
fn an_empty_history_window_classifies_nothing() {
    let fixture = Fixture {
        root: tempfile::tempdir().unwrap(),
    };
    fixture.git(["init", "-q"]);
    fixture.git(["config", "user.email", "test@example.invalid"]);
    fixture.git(["config", "user.name", "Smackdebt Test"]);
    fixture.write("package.json", b"{\"name\":\"fixture\",\"private\":true}\n");
    fixture.write("public/js/widget.js", LIBRARY);
    fixture.write("src/app.js", WORKED_ON);
    fixture.commit(days_ago(400));
    let report = fixture.report();
    assert_eq!(
        role_of(&report, "public/js/widget.js"),
        SourceRole::Primary,
        "a window with no commits proves nothing about any file"
    );
}

#[test]
fn a_vendored_library_is_not_reported_as_an_orphan() {
    let fixture = Fixture::new("public/js/widget.js");
    let report = fixture.report();
    let vendored = report
        .files()
        .iter()
        .find(|file| file.path() == "public/js/widget.js")
        .map(FileRecord::id)
        .unwrap();
    assert!(
        report
            .orphan_files()
            .iter()
            .all(|orphan| orphan.file() != vendored),
        "a file the repository does not own is not its orphan"
    );
}

#[test]
fn a_repository_without_javascript_pays_nothing_for_the_rule() {
    let fixture = Fixture {
        root: tempfile::tempdir().unwrap(),
    };
    fixture.git(["init", "-q"]);
    fixture.git(["config", "user.email", "test@example.invalid"]);
    fixture.git(["config", "user.name", "Smackdebt Test"]);
    fixture.write(
        "Cargo.toml",
        b"[package]\nname='fixture'\nversion='0.1.0'\n",
    );
    fixture.write("src/lib.rs", b"pub fn work(value: i32) -> i32 { value }\n");
    fixture.commit(days_ago(400));
    fixture.write(
        "src/other.rs",
        b"pub fn more(value: i32) -> i32 { value }\n",
    );
    fixture.commit(days_ago(1));
    let report = fixture.report();
    assert!(
        report
            .files()
            .iter()
            .all(|file| file.role() != SourceRole::Vendored)
    );
    assert!(Path::new("src/other.rs").is_relative());
}
