use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::analyzer::{AnalysisResult, ParseError, analyze_file};
use crate::config::Config;

/// Result of traversing and analyzing all files.
#[derive(Debug)]
pub struct TraversalResult {
    pub results: Vec<AnalysisResult>,
    pub parse_errors: Vec<ParseError>,
    pub files_inspected: usize,
}

impl TraversalResult {
    pub fn total_offenses(&self) -> usize {
        self.results.iter().map(|r| r.offenses.len()).sum()
    }

    pub fn has_offenses(&self) -> bool {
        self.total_offenses() > 0
    }
}

/// Find all .rb files, filter by config, and analyze them in parallel.
pub fn traverse_and_analyze(path: &Path, config: &Config) -> TraversalResult {
    let files = collect_ruby_files(path);
    let exclude_root = config.exclude_root.as_deref().unwrap_or(path);
    let excludes = Excludes::new(&config.exclude_patterns, exclude_root);
    let scannable: Vec<PathBuf> = files
        .into_iter()
        .filter(|f| !excludes.contains(f))
        .collect();

    let files_inspected = scannable.len();

    let file_results: Vec<Result<AnalysisResult, ParseError>> = scannable
        .par_iter()
        .map(|f| analyze_file(f, config))
        .collect();

    let mut results = Vec::new();
    let mut parse_errors = Vec::new();

    for result in file_results {
        match result {
            Ok(analysis) => results.push(analysis),
            Err(err) => parse_errors.push(err),
        }
    }

    // Sort by path for deterministic output with rayon
    results.sort_by(|a, b| a.path.cmp(&b.path));
    parse_errors.sort_by(|a, b| a.path.cmp(&b.path));

    TraversalResult {
        results,
        parse_errors,
        files_inspected,
    }
}

/// Collect all .rb files under a path.
fn collect_ruby_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }

    // Escape glob metacharacters in the base path to avoid pattern errors
    let escaped = glob::Pattern::escape(&path.display().to_string());
    let pattern = format!("{}/**/*.rb", escaped);
    // The pattern is escaped above, so it can never be invalid.
    glob::glob(&pattern)
        .map(|paths| paths.filter_map(|entry| entry.ok()).collect())
        .unwrap_or_default()
}

/// Compiled `exclude_paths`.
///
/// Relative patterns are matched against each file's path relative to `root`, not expanded
/// on disk: on Windows the canonical root is a `\\?\C:\...` verbatim path that glob can't
/// expand. Absolute patterns are expanded on disk as written.
struct Excludes {
    root: PathBuf,
    relative: Vec<glob::Pattern>,
    absolute: HashSet<PathBuf>,
}

impl Excludes {
    fn new(patterns: &[String], root: &Path) -> Self {
        let mut relative = Vec::new();
        let mut absolute = HashSet::new();
        for pattern in patterns {
            let parsed = if Path::new(pattern).is_absolute() {
                glob::glob(pattern).map(|paths| {
                    absolute.extend(paths.filter_map(Result::ok).map(|p| canonical(&p)));
                })
            } else {
                glob::Pattern::new(pattern).map(|p| relative.push(p))
            };
            if let Err(e) = parsed {
                eprintln!("Warning: invalid exclude pattern '{}': {}", pattern, e);
            }
        }
        Self {
            root: canonical(root),
            relative,
            absolute,
        }
    }

    fn contains(&self, file: &Path) -> bool {
        // `*` stays within one path component, as it does when globbing.
        let opts = glob::MatchOptions {
            require_literal_separator: true,
            ..Default::default()
        };
        let file = canonical(file);
        self.absolute.contains(&file)
            || file
                .strip_prefix(&self.root)
                .is_ok_and(|rel| self.relative.iter().any(|p| p.matches_path_with(rel, opts)))
    }
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn traversal_result_total_offenses() {
        let result = TraversalResult {
            results: vec![
                crate::analyzer::AnalysisResult {
                    path: "a.rb".to_string(),
                    offenses: vec![
                        crate::offense::Offense::new(crate::offense::OffenseKind::GsubVsTr, 1),
                        crate::offense::Offense::new(crate::offense::OffenseKind::GsubVsTr, 2),
                    ],
                },
                crate::analyzer::AnalysisResult {
                    path: "b.rb".to_string(),
                    offenses: vec![crate::offense::Offense::new(
                        crate::offense::OffenseKind::GsubVsTr,
                        1,
                    )],
                },
            ],
            parse_errors: vec![],
            files_inspected: 2,
        };
        assert_eq!(result.total_offenses(), 3);
        assert!(result.has_offenses());
    }

    #[test]
    fn traversal_result_no_offenses() {
        let result = TraversalResult {
            results: vec![],
            parse_errors: vec![],
            files_inspected: 0,
        };
        assert_eq!(result.total_offenses(), 0);
        assert!(!result.has_offenses());
    }

    #[test]
    fn collect_ruby_files_single_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        fs::write(&file, "x = 1").unwrap();
        let files = collect_ruby_files(&file);
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn collect_ruby_files_directory() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.rb"), "x = 1").unwrap();
        fs::write(dir.path().join("b.rb"), "y = 2").unwrap();
        fs::write(dir.path().join("c.txt"), "not ruby").unwrap();
        let files = collect_ruby_files(dir.path());
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn collect_ruby_files_nested() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("deep.rb"), "z = 3").unwrap();
        let files = collect_ruby_files(dir.path());
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn collect_ruby_files_no_rb() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("readme.md"), "hello").unwrap();
        let files = collect_ruby_files(dir.path());
        assert!(files.is_empty());
    }

    /// A tempdir holding `top.rb` and `vendor/lib/v.rb`.
    fn tree() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("vendor/lib")).unwrap();
        fs::write(dir.path().join("top.rb"), "x").unwrap();
        fs::write(dir.path().join("vendor/lib/v.rb"), "x").unwrap();
        dir
    }

    fn excluded(patterns: &[&str], root: &Path, file: &Path) -> bool {
        let patterns: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
        Excludes::new(&patterns, root).contains(file)
    }

    #[test]
    fn relative_pattern_matches_under_the_root() {
        let dir = tree();
        let v = dir.path().join("vendor/lib/v.rb");
        assert!(excluded(&["vendor/**/*.rb"], dir.path(), &v));
        assert!(excluded(&["vendor/lib/v.rb"], dir.path(), &v));
        assert!(!excluded(
            &["vendor/**/*.rb"],
            dir.path(),
            &dir.path().join("top.rb")
        ));
        // Resolved against the root, not the file's own directory.
        assert!(!excluded(&["v.rb"], dir.path(), &v));
    }

    #[test]
    fn star_does_not_cross_directories() {
        let dir = tree();
        let v = dir.path().join("vendor/lib/v.rb");
        assert!(!excluded(&["*.rb"], dir.path(), &v));
        assert!(!excluded(&["vendor/*.rb"], dir.path(), &v));
        assert!(excluded(&["*.rb"], dir.path(), &dir.path().join("top.rb")));
    }

    #[test]
    fn file_outside_the_root_is_not_excluded() {
        let dir = tree();
        let root = dir.path().join("vendor");
        assert!(!excluded(&["**/*.rb"], &root, &dir.path().join("top.rb")));
    }

    #[test]
    fn absolute_pattern_is_expanded_on_disk() {
        let dir = tree();
        let pattern = format!(
            "{}/*.rb",
            glob::Pattern::escape(&dir.path().display().to_string())
        );
        let other = TempDir::new().unwrap();
        assert!(excluded(
            &[&pattern],
            other.path(),
            &dir.path().join("top.rb")
        ));
        assert!(!excluded(
            &[&pattern],
            other.path(),
            &dir.path().join("vendor/lib/v.rb")
        ));
    }

    #[test]
    fn invalid_patterns_are_skipped() {
        let dir = tree();
        let invalid_abs = format!("{}/[", dir.path().display());
        let e = Excludes::new(&["[invalid".to_string(), invalid_abs], dir.path());
        assert!(e.relative.is_empty() && e.absolute.is_empty());
    }

    #[test]
    fn traverse_and_analyze_with_tempdir() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.rb"), "for x in [1]; end").unwrap();
        let config = Config::default();
        let result = traverse_and_analyze(dir.path(), &config);
        assert_eq!(result.files_inspected, 1);
        assert!(result.has_offenses());
    }

    #[test]
    fn traverse_and_analyze_clean_file() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("clean.rb"), "x = 1 + 2").unwrap();
        let config = Config::default();
        let result = traverse_and_analyze(dir.path(), &config);
        assert_eq!(result.files_inspected, 1);
        assert!(!result.has_offenses());
    }

    #[test]
    fn traverse_and_analyze_with_exclusion() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.rb"), "for x in [1]; end").unwrap();
        let config = Config::parse_yaml(&format!(
            "exclude_paths:\n  - '{}/*.rb'\n",
            dir.path().display()
        ))
        .unwrap();
        let result = traverse_and_analyze(dir.path(), &config);
        assert_eq!(result.files_inspected, 0);
    }

    #[test]
    fn traverse_and_analyze_unreadable_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("unreadable.rb");
        // Create a symlink to a nonexistent target to simulate unreadable file
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/nonexistent_target_12345", &file).unwrap();
            let config = Config::default();
            let result = traverse_and_analyze(dir.path(), &config);
            assert_eq!(result.files_inspected, 1);
            assert!(!result.parse_errors.is_empty());
        }
    }

    #[test]
    fn traverse_and_analyze_parse_error() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("bad.rb"), "def def def").unwrap();
        let config = Config::default();
        let result = traverse_and_analyze(dir.path(), &config);
        assert_eq!(result.files_inspected, 1);
        // Prism always produces an AST even with errors, but our analyzer skips
        // analysis when errors are detected, returning empty offenses.
        assert!(result.results.iter().all(|r| r.offenses.is_empty()));
    }
}
