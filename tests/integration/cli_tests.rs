use std::process::Command;

fn cargo_bin() -> Command {
    // The built binary directly (not `cargo run`) so coverage instrumentation applies.
    Command::new(env!("CARGO_BIN_EXE_rubyfast"))
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

#[test]
fn invalid_config_exits_with_error() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join(".rubyfast.yml"),
        "speedups: [not, a, map]\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("a.rb"), "x = 1\n").unwrap();
    let output = cargo_bin()
        .arg(dir.path().to_str().unwrap())
        .output()
        .expect("Failed to run");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error loading config"),
        "Expected config error, got: {}",
        stderr
    );
}

#[test]
fn fix_mode_reports_unwritable_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("readonly.rb");
    std::fs::write(&file, "for x in [1,2,3]; puts x; end\n").unwrap();
    let mut perms = std::fs::metadata(&file).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&file, perms).unwrap();

    // Root (and some filesystems) ignore the read-only bit, so there would be
    // nothing to assert — skip rather than fail.
    if std::fs::write(&file, "unwritable?").is_ok() {
        return;
    }

    let output = cargo_bin()
        .args([file.to_str().unwrap(), "--fix"])
        .output()
        .expect("Failed to run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to write"),
        "Expected write failure, got: {}",
        stderr
    );
    // The offense is still in the file, so it must be reported and fail the run.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "stdout: {}", stdout);
    assert!(stdout.contains("0 offenses fixed"), "stdout: {}", stdout);
    assert!(stdout.contains("For loop"), "stdout: {}", stdout);
}

fn run_fix(source: &str) -> (Option<i32>, String, String) {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("t.rb");
    std::fs::write(&file, source).unwrap();
    let output = cargo_bin()
        .args([file.to_str().unwrap(), "--fix", "--format", "plain"])
        .output()
        .expect("Failed to run");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        std::fs::read_to_string(&file).unwrap(),
    )
}

#[test]
fn fix_mode_exits_0_when_every_fix_applies() {
    let (code, stdout, content) = run_fix("for x in 1..3 do\n  puts x\nend\n");
    assert_eq!(code, Some(0), "stdout: {}", stdout);
    assert_eq!(content, "(1..3).each do |x|\n  puts x\nend\n");
}

#[test]
fn fix_mode_counts_fixes_not_replacements() {
    let (code, stdout, content) = run_fix("arr.select { |x| x }.first\n");
    assert_eq!(code, Some(0), "stdout: {}", stdout);
    assert_eq!(content, "arr.detect { |x| x }\n");
    assert!(stdout.contains("1 offense fixed"), "stdout: {}", stdout);
}

#[test]
fn fix_mode_exits_1_when_an_overlapping_fix_is_skipped() {
    // The include? fix lies inside the for-loop header the for fix rewrites, so only the
    // for fix lands; the include? offense remains and must fail the run.
    let (code, stdout, content) = run_fix("for x in [(1..3).include?(2)]; puts x; end\n");
    assert_eq!(code, Some(1), "stdout: {}", stdout);
    assert_eq!(content, "[(1..3).include?(2)].each do |x| puts x; end\n");
    assert!(stdout.contains("1 offense fixed"), "stdout: {}", stdout);
    assert!(stdout.contains("cover?"), "stdout: {}", stdout);
}

#[test]
fn fix_mode_keeps_for_loop_whose_locals_escape() {
    let src = "for x in arr\n  last = x\nend\nputs last\n";
    let (code, stdout, content) = run_fix(src);
    assert_eq!(code, Some(1), "stdout: {}", stdout);
    assert_eq!(content, src);
}

#[test]
fn fix_mode_keeps_include_on_string_range() {
    let src = "('a'..'z').include?('bb')\n";
    let (code, stdout, content) = run_fix(src);
    assert_eq!(code, Some(1), "stdout: {}", stdout);
    assert_eq!(content, src);
}

/// A repo with a root config excluding `vendor/`, an excluded offense in
/// `vendor/v.rb` and a reported one in `app/a.rb`.
fn repo_with_vendor_exclude() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("vendor")).unwrap();
    std::fs::create_dir_all(dir.path().join("app")).unwrap();
    std::fs::write(
        dir.path().join(".rubyfast.yml"),
        "exclude_paths:\n  - 'vendor/**/*.rb'\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("vendor/v.rb"), "arr.shuffle.first\n").unwrap();
    std::fs::write(dir.path().join("app/a.rb"), "arr.shuffle.first\n").unwrap();
    dir
}

#[test]
fn exclude_paths_resolve_against_the_config_dir() {
    let dir = repo_with_vendor_exclude();
    // (working dir, path argument): every way of pointing at the excluded file.
    for (cwd, arg) in [
        ("", "vendor/v.rb"),
        ("", "vendor"),
        ("vendor", "v.rb"),
        ("vendor", "."),
        ("app", "../vendor"),
    ] {
        let output = cargo_bin()
            .current_dir(dir.path().join(cwd))
            .arg(arg)
            .output()
            .expect("Failed to run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("0 files inspected"),
            "cwd={cwd:?} arg={arg:?} stdout: {stdout}"
        );
    }
}

#[test]
fn exclude_paths_keep_other_files_when_scanning_from_a_subdir() {
    let dir = repo_with_vendor_exclude();
    let output = cargo_bin()
        .current_dir(dir.path().join("app"))
        .arg("..")
        .output()
        .expect("Failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(stdout.contains("1 file inspected"), "stdout: {stdout}");
}
