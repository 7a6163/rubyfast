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

#[test]
fn format_rule_output() {
    let output = cargo_bin()
        .args(["tests/fixtures/19_for_loop.rb", "--format", "rule"])
        .output()
        .expect("Failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("offense"),
        "Expected offense count in rule output: {}",
        stdout
    );
}

#[test]
fn format_plain_output() {
    let output = cargo_bin()
        .args(["tests/fixtures/19_for_loop.rb", "--format", "plain"])
        .output()
        .expect("Failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("19_for_loop.rb:"),
        "Expected path:line format in plain output: {}",
        stdout
    );
}

#[test]
fn fix_mode_modifies_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("fixable.rb");
    std::fs::write(&file, "for x in [1,2,3]; puts x; end\n").unwrap();
    let output = cargo_bin()
        .args([file.to_str().unwrap(), "--fix"])
        .output()
        .expect("Failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fixed"),
        "Expected 'fixed' in fix output: {}",
        stdout
    );
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(
        content.contains(".each do"),
        "Expected file to be fixed: {}",
        content
    );
}

#[test]
fn fix_mode_reports_unfixable() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("unfixable.rb");
    // sort with block is unfixable
    std::fs::write(&file, "arr.sort { |a, b| a <=> b }\n").unwrap();
    let output = cargo_bin()
        .args([file.to_str().unwrap(), "--fix"])
        .output()
        .expect("Failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cannot be auto-fixed") || stdout.contains("0 offenses fixed"),
        "Expected unfixable note in output: {}",
        stdout
    );
}

#[test]
fn config_disables_rule() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("test.rb"), "for x in [1]; end\n").unwrap();
    std::fs::write(
        dir.path().join(".rubyfast.yml"),
        "speedups:\n  for_loop_vs_each: false\n",
    )
    .unwrap();
    let output = cargo_bin()
        .arg(dir.path().to_str().unwrap())
        .output()
        .expect("Failed to run");
    assert!(
        output.status.success(),
        "Expected exit 0 when rule is disabled. stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
