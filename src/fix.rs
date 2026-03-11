use std::path::Path;

use lib_ruby_parser::{ErrorLevel, Parser};

/// A single byte-range replacement in a source file.
#[derive(Debug, Clone)]
pub struct Replacement {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// A fix consisting of one or more replacements to apply atomically.
#[derive(Debug, Clone)]
pub struct Fix {
    pub replacements: Vec<Replacement>,
}

impl Fix {
    pub fn single(start: usize, end: usize, text: impl Into<String>) -> Self {
        Self {
            replacements: vec![Replacement {
                start,
                end,
                text: text.into(),
            }],
        }
    }

    pub fn two(
        start1: usize,
        end1: usize,
        text1: impl Into<String>,
        start2: usize,
        end2: usize,
        text2: impl Into<String>,
    ) -> Self {
        Self {
            replacements: vec![
                Replacement {
                    start: start1,
                    end: end1,
                    text: text1.into(),
                },
                Replacement {
                    start: start2,
                    end: end2,
                    text: text2.into(),
                },
            ],
        }
    }
}

/// Apply a set of fixes to source bytes. Returns the fixed source.
/// Fixes are applied in reverse byte order to preserve offsets.
/// Overlapping replacements are skipped.
pub fn apply_fixes(source: &[u8], fixes: &[Fix]) -> Vec<u8> {
    // Flatten all replacements and sort by start descending
    let mut replacements: Vec<&Replacement> = fixes.iter().flat_map(|f| &f.replacements).collect();

    replacements.sort_by(|a, b| b.start.cmp(&a.start));

    let mut result = source.to_vec();
    let mut last_start = usize::MAX;

    for r in &replacements {
        // Skip overlapping or out-of-bounds replacements
        if r.end > last_start || r.start > result.len() || r.end > result.len() {
            continue;
        }
        result.splice(r.start..r.end, r.text.bytes());
        last_start = r.start;
    }

    result
}

/// Verify that the given source parses without fatal errors.
pub fn verify_syntax(source: &[u8]) -> bool {
    let result = Parser::new(source.to_vec(), Default::default()).do_parse();
    !result
        .diagnostics
        .iter()
        .any(|d| d.level == ErrorLevel::Error)
}

/// Apply fixes to a file: read -> fix -> verify syntax -> write.
/// Returns the number of fixes applied, or an error.
pub fn apply_fixes_to_file(path: &Path, fixes: &[Fix]) -> Result<usize, String> {
    if fixes.is_empty() {
        return Ok(0);
    }

    let source =
        std::fs::read(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let fixed = apply_fixes(&source, fixes);

    if !verify_syntax(&fixed) {
        return Err(format!(
            "Fix would produce invalid syntax in {}; skipping",
            path.display()
        ));
    }

    std::fs::write(path, &fixed)
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;

    Ok(fixes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_replacement() {
        let source = b"hello world";
        let fix = Fix::single(6, 11, "rust");
        let result = apply_fixes(source, &[fix]);
        assert_eq!(result, b"hello rust");
    }

    #[test]
    fn multiple_non_overlapping() {
        let source = b"foo.bar.baz";
        let fixes = vec![Fix::single(0, 3, "qux"), Fix::single(8, 11, "quux")];
        let result = apply_fixes(source, &fixes);
        assert_eq!(result, b"qux.bar.quux");
    }

    #[test]
    fn overlapping_skipped() {
        let source = b"abcdefgh";
        let fixes = vec![
            Fix::single(2, 6, "XX"), // replace cdef with XX
            Fix::single(4, 8, "YY"), // overlaps — should be skipped
        ];
        // Because we sort descending, 4..8 is processed first, then 2..6 overlaps
        // Actually: sorted descending by start: 4..8 first (start=4), then 2..6 (start=2)
        // 4..8 replaces "efgh" -> "YY", result = "abcdYY", last_start=4
        // 2..6 has end=6 > last_start=4, so it's skipped
        let result = apply_fixes(source, &fixes);
        assert_eq!(result, b"abcdYY");
    }

    #[test]
    fn verify_valid_ruby() {
        assert!(verify_syntax(b"x = 1 + 2"));
    }

    #[test]
    fn verify_invalid_ruby() {
        assert!(!verify_syntax(b"def def def"));
    }

    #[test]
    fn two_replacements_in_one_fix() {
        let source = b"arr.map { |x| [x] }.flatten(1)";
        // Rename .map -> .flat_map and delete .flatten(1)
        let fix = Fix::two(
            4, 7, "flat_map", // "map" -> "flat_map"
            19, 30, "", // delete ".flatten(1)"
        );
        let result = apply_fixes(source, &[fix]);
        assert_eq!(result, b"arr.flat_map { |x| [x] }");
    }

    #[test]
    fn apply_fixes_empty_fixes() {
        let source = b"hello world";
        let result = apply_fixes(source, &[]);
        assert_eq!(result, source);
    }

    #[test]
    fn apply_fixes_out_of_bounds_skipped() {
        let source = b"short";
        let fix = Fix::single(10, 20, "big");
        let result = apply_fixes(source, &[fix]);
        assert_eq!(result, b"short");
    }

    #[test]
    fn apply_fixes_to_file_no_fixes() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "x = 1").unwrap();
        let result = apply_fixes_to_file(&file, &[]).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn apply_fixes_to_file_valid_fix() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "for x in [1]; end").unwrap();
        // Replace "for x in [1]; " with "[1].each do |x|; "
        let fix = Fix::single(0, 14, "[1].each do |x|;");
        let result = apply_fixes_to_file(&file, &[fix]).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn apply_fixes_to_file_syntax_error_skipped() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "x = 1 + 2").unwrap();
        // This fix produces invalid syntax
        let fix = Fix::single(0, 9, "def def def");
        let result = apply_fixes_to_file(&file, &[fix]);
        assert!(result.is_err());
    }

    #[test]
    fn apply_fixes_to_file_nonexistent_file() {
        let fix = Fix::single(0, 3, "x");
        let result = apply_fixes_to_file(Path::new("/nonexistent.rb"), &[fix]);
        assert!(result.is_err());
    }
}
