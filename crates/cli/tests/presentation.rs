use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;
use unicode_width::UnicodeWidthStr;

fn snapshot(name: &str, text: &str, width: usize) {
    for line in text.lines() {
        assert!(
            line.width() <= width,
            "{name}: line exceeds {width} columns: {line}"
        );
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    assert_eq!(text, fs::read_to_string(path).unwrap(), "{name}");
}

#[test]
fn help_and_gate_fit_narrow_and_wide_terminals() {
    let project = TempDir::new().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project
            .path()
            .join("src/shipping_with_a_long_descriptive_name.js"),
        "function shipping(a, b) { if (a) { if (b) { return 1; } } return 0; }\n",
    )
    .unwrap();
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[thresholds.cognitive]\nwatch = 1\nhigh = 2\n",
    )
    .unwrap();
    fs::write(
        project.path().join("baseline.tsv"),
        "# smackdebt gate baseline v1\npath\tsignal\thigh\twatch\n",
    )
    .unwrap();
    for width in [50, 80, 120] {
        for (name, args, code) in [
            ("help", vec!["--help"], 0),
            ("gate", vec!["gate", "--baseline", "baseline.tsv"], 3),
            (
                "gate-decorated",
                vec!["gate", "--baseline", "baseline.tsv", "--color", "always"],
                3,
            ),
        ] {
            let output = Command::new(assert_cmd::cargo::cargo_bin!("smackdebt"))
                .current_dir(project.path())
                .args(args)
                .env("HOME", project.path())
                .env("XDG_CONFIG_HOME", project.path())
                .env("COLUMNS", width.to_string())
                .env("NO_COLOR", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_SYSTEM", "/dev/null")
                .assert()
                .code(code)
                .stderr("")
                .get_output()
                .stdout
                .clone();
            let raw = String::from_utf8(output).unwrap();
            let text = console::strip_ansi_codes(&raw);
            if name == "gate-decorated" {
                assert!(raw.contains("\x1b["));
                assert!(text.contains('\u{f062}'));
            }
            snapshot(&format!("{name}-{width}.terminal.txt"), &text, width);
        }
    }
}
