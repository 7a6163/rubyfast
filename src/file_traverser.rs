use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::analyzer::{analyze_file, AnalysisResult, ParseError};
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
fn collect_excluded_files(patterns: &[String], base: &Path) -> Vec<PathBuf> {
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
fn is_excluded(file: &Path, excluded: &[PathBuf]) -> bool {
    let file_canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    excluded.contains(&file_canonical)
}
