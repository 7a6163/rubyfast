use std::process::Command;

fn cargo_bin() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--"]);
    cmd
}

#[test]
fn exit_code_0_on_clean_file() {
    let output = cargo_bin()
        .arg("tests/fixtures/clean.rb")
        .output()
        .expect("Failed to run");
    assert!(
        output.status.success(),
        "Expected exit 0, got {}. stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn exit_code_1_on_offense() {
    let output = cargo_bin()
        .arg("tests/fixtures/19_for_loop.rb")
        .output()
        .expect("Failed to run");
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit 1. stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn nonexistent_path_prints_error() {
    let output = cargo_bin()
        .arg("/nonexistent/path")
        .output()
        .expect("Failed to run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No such file or directory"),
        "Expected 'No such file or directory' in stderr: {}",
        stderr
    );
}

#[test]
fn scans_directory_recursively() {
    let output = cargo_bin()
        .arg("tests/fixtures")
        .output()
        .expect("Failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should find offenses across multiple files
    assert!(stdout.contains("offenses detected"), "stdout: {}", stdout);
    assert!(stdout.contains("files inspected"), "stdout: {}", stdout);
}

#[test]
fn statistics_line_present() {
    let output = cargo_bin()
        .arg("tests/fixtures/clean.rb")
        .output()
        .expect("Failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 file inspected"),
        "Expected '1 file inspected' in: {}",
        stdout
    );
    assert!(
        stdout.contains("0 offenses detected"),
        "Expected '0 offenses detected' in: {}",
        stdout
    );
}
