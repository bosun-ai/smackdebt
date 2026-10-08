//! Exercise the actual terminal protocol, with isolated user directories.
#[cfg(unix)]
#[test]
fn interactive_setup_works_with_real_keyboard_input() {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/terminal_session.py"
        ))
        .arg(assert_cmd::cargo::cargo_bin!("smackdebt"))
        .output()
        .expect("Python 3 is required for terminal acceptance tests");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
