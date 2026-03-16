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
    let excluded = collect_excluded_files(&config.exclude_patterns, path);
    let scannable: Vec<PathBuf> = files
        .into_iter()
        .filter(|f| !is_excluded(f, &excluded))
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
    match glob::glob(&pattern) {
        Ok(paths) => paths.filter_map(|entry| entry.ok()).collect(),
        Err(e) => {
            eprintln!("Warning: invalid path pattern '{}': {}", pattern, e);
            vec![]
        }
    }
}

/// Expand exclude patterns relative to a base path, pre-canonicalizing results.
fn collect_excluded_files(patterns: &[String], base: &Path) -> HashSet<PathBuf> {
    patterns
        .iter()
        .flat_map(|pattern| {
            let full_pattern = if Path::new(pattern).is_absolute() {
                pattern.clone()
            } else {
                format!("{}/{}", base.display(), pattern)
            };
            match glob::glob(&full_pattern) {
                Ok(paths) => paths
                    .filter_map(|entry| entry.ok())
                    .map(|p| p.canonicalize().unwrap_or(p))
                    .collect::<Vec<_>>(),
                Err(e) => {
                    eprintln!("Warning: invalid exclude pattern '{}': {}", full_pattern, e);
                    vec![]
                }
            }
        })
        .collect()
}

/// Check if a file should be excluded.
fn is_excluded(file: &Path, excluded: &HashSet<PathBuf>) -> bool {
    let file_canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    excluded.contains(&file_canonical)
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

    #[test]
    fn is_excluded_matching() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        fs::write(&file, "x = 1").unwrap();
        let canonical = file.canonicalize().unwrap();
        let excluded: HashSet<PathBuf> = [canonical].into_iter().collect();
        assert!(is_excluded(&file, &excluded));
    }

    #[test]
    fn is_excluded_not_matching() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        fs::write(&file, "x = 1").unwrap();
        let excluded: HashSet<PathBuf> = HashSet::new();
        assert!(!is_excluded(&file, &excluded));
    }

    #[test]
    fn collect_excluded_files_with_pattern() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("vendor.rb"), "x").unwrap();
        let patterns = vec![format!("{}/*.rb", dir.path().display())];
        let excluded = collect_excluded_files(&patterns, dir.path());
        assert!(!excluded.is_empty());
    }

    #[test]
    fn collect_excluded_files_relative_pattern() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("skip.rb"), "x").unwrap();
        let patterns = vec!["*.rb".to_string()];
        let excluded = collect_excluded_files(&patterns, dir.path());
        assert!(!excluded.is_empty());
    }

    #[test]
    fn collect_excluded_files_invalid_pattern() {
        let excluded = collect_excluded_files(&["[invalid".to_string()], Path::new("."));
        assert!(excluded.is_empty());
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
