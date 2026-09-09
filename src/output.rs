use std::collections::BTreeMap;

use colored::Colorize;

use crate::analyzer::ParseError;
use crate::cli::OutputFormat;
use crate::file_traverser::TraversalResult;
use crate::offense::OffenseKind;

/// Print analysis results using the selected output format.
pub fn print_results(result: &TraversalResult, format: &OutputFormat) {
    print!("{}", format_results(result, format));
}

/// Render analysis results using the selected output format.
fn format_results(result: &TraversalResult, format: &OutputFormat) -> String {
    let mut out = match format {
        OutputFormat::File => format_results_by_file(result),
        OutputFormat::Rule => format_results_by_rule(result),
        OutputFormat::Plain => format_results_plain(result),
    };

    if !result.parse_errors.is_empty() {
        out.push_str(&format_parse_errors(&result.parse_errors));
    }

    out.push_str(&format_statistics(result));
    out
}

/// `--format file` — group offenses by file path.
///
/// ```text
/// app/controllers/concerns/lottery_common.rb
///   L13  Hash#fetch with second argument is slower than Hash#fetch with block
///
/// tests/fixtures/19_for_loop.rb
///   L1   For loop is slower than using each (fixable)
/// ```
fn format_results_by_file(result: &TraversalResult) -> String {
    let mut out = String::new();
    for analysis in &result.results {
        if analysis.offenses.is_empty() {
            continue;
        }
        out.push_str(&format!("{}\n", analysis.path.bold()));
        for offense in &analysis.offenses {
            let fixable_tag = if offense.kind.is_fixable() {
                format!(" {}", "(fixable)".green())
            } else {
                String::new()
            };
            out.push_str(&format!(
                "  {}  {}{}\n",
                format!("L{}", offense.line).cyan(),
                offense.kind.explanation(),
                fixable_tag
            ));
        }
        out.push('\n');
    }
    out
}

/// `--format rule` — group offenses by rule kind.
///
/// ```text
/// Hash#fetch with second argument is slower than Hash#fetch with block. (5 offenses)
///   app/controllers/api/v1/health_articles_controller.rb:11
///   app/controllers/concerns/lottery_common.rb:13
/// ```
fn format_results_by_rule(result: &TraversalResult) -> String {
    let mut out = String::new();
    let mut grouped: BTreeMap<OffenseKind, Vec<(String, usize)>> = BTreeMap::new();

    for analysis in &result.results {
        for offense in &analysis.offenses {
            grouped
                .entry(offense.kind)
                .or_default()
                .push((analysis.path.clone(), offense.line));
        }
    }

    for (kind, locations) in &grouped {
        let count = locations.len();
        out.push_str(&format!(
            "{} ({} {})\n",
            kind.explanation().yellow(),
            count,
            pluralize("offense", count)
        ));
        for (path, line) in locations {
            out.push_str(&format!("  {}:{}\n", path, line));
        }
        out.push('\n');
    }
    out
}

/// `--format plain` — one offense per line (original format, for grep/reviewdog).
///
/// ```text
/// app/controllers/api/v1/health_articles_controller.rb:11 Hash#fetch with second argument ...
/// ```
fn format_results_plain(result: &TraversalResult) -> String {
    let mut out = String::new();
    for analysis in &result.results {
        if analysis.offenses.is_empty() {
            continue;
        }
        for offense in &analysis.offenses {
            let location = format!("{}:{}", analysis.path, offense.line);
            out.push_str(&format!(
                "{} {}.\n",
                location.red(),
                offense.kind.explanation()
            ));
        }
        out.push('\n');
    }
    out
}

fn format_parse_errors(errors: &[ParseError]) -> String {
    let mut out = String::from(
        "rubyfast was unable to process some files because the\n\
         internal parser is not able to read some characters or\n\
         has timed out. Unprocessable files were:\n\
         -----------------------------------------------------\n",
    );
    for err in errors {
        out.push_str(&format!("{} - {}\n", err.path, err.message));
    }
    out.push('\n');
    out
}

struct StatsParts {
    files_str: String,
    colored_offenses: String,
    parse_errors_str: Option<String>,
}

impl StatsParts {
    fn build(result: &TraversalResult) -> Self {
        let files = result.files_inspected;
        let offenses = result.total_offenses();
        let parse_errors = result.parse_errors.len();

        let files_str = format!("{} {} inspected", files, pluralize("file", files));
        let offenses_str = format!("{} {} detected", offenses, pluralize("offense", offenses));
        let colored_offenses = if offenses == 0 {
            offenses_str.green().to_string()
        } else {
            offenses_str.red().to_string()
        };
        let parse_errors_str = if parse_errors > 0 {
            Some(
                format!(
                    "{} unparsable {} found",
                    parse_errors,
                    pluralize("file", parse_errors)
                )
                .red()
                .to_string(),
            )
        } else {
            None
        };

        Self {
            files_str,
            colored_offenses,
            parse_errors_str,
        }
    }
}

fn format_statistics(result: &TraversalResult) -> String {
    let stats = StatsParts::build(result);

    let fixable: usize = result
        .results
        .iter()
        .flat_map(|r| &r.offenses)
        .filter(|o| o.kind.is_fixable())
        .count();

    let fixable_hint = if fixable > 0 {
        format!(
            ", {}",
            format!(
                "{} {} autocorrectable (run rubyfast --fix)",
                fixable,
                pluralize("offense", fixable)
            )
            .yellow()
        )
    } else {
        String::new()
    };

    match &stats.parse_errors_str {
        Some(errors_str) => format!(
            "{}, {}, {}{}\n",
            stats.files_str.green(),
            stats.colored_offenses,
            errors_str,
            fixable_hint
        ),
        None => format!(
            "{}, {}{}\n",
            stats.files_str.green(),
            stats.colored_offenses,
            fixable_hint
        ),
    }
}

/// Print results when --fix mode is active.
pub fn print_fix_results(
    result: &TraversalResult,
    total_fixed: usize,
    total_errors: usize,
    format: &OutputFormat,
) {
    print!(
        "{}",
        format_fix_results(result, total_fixed, total_errors, format)
    );
}

/// Render results when --fix mode is active.
fn format_fix_results(
    result: &TraversalResult,
    total_fixed: usize,
    total_errors: usize,
    format: &OutputFormat,
) -> String {
    // Unfixable offenses are still reported, using the selected format
    let unfixable_result = filter_unfixable(result);
    let mut out = match format {
        OutputFormat::File => format_results_by_file(&unfixable_result),
        OutputFormat::Rule => format_results_by_rule(&unfixable_result),
        OutputFormat::Plain => format_results_plain(&unfixable_result),
    };

    if !result.parse_errors.is_empty() {
        out.push_str(&format_parse_errors(&result.parse_errors));
    }

    out.push_str(&format_fix_statistics(result, total_fixed, total_errors));
    out
}

/// Build a TraversalResult containing only unfixable offenses.
fn filter_unfixable(result: &TraversalResult) -> TraversalResult {
    use crate::analyzer::AnalysisResult;

    let results = result
        .results
        .iter()
        .map(|analysis| {
            let offenses = analysis
                .offenses
                .iter()
                .filter(|o| o.fix.is_none())
                .cloned()
                .collect();
            AnalysisResult {
                path: analysis.path.clone(),
                offenses,
            }
        })
        .collect();

    TraversalResult {
        results,
        parse_errors: vec![],
        files_inspected: result.files_inspected,
    }
}

fn format_fix_statistics(
    result: &TraversalResult,
    total_fixed: usize,
    total_errors: usize,
) -> String {
    let stats = StatsParts::build(result);
    let offenses = result.total_offenses();
    let fixable: usize = result
        .results
        .iter()
        .flat_map(|r| &r.offenses)
        .filter(|o| o.fix.is_some())
        .count();

    let fixed_str = format!(
        "{} {} fixed",
        total_fixed,
        pluralize("offense", total_fixed)
    );
    let colored_fixed = if total_fixed > 0 {
        fixed_str.green().to_string()
    } else {
        fixed_str.to_string()
    };

    let unfixable = offenses.saturating_sub(fixable);
    if total_errors > 0 {
        let err_str = format!(
            "{} {} skipped (syntax error after fix)",
            total_errors,
            pluralize("file", total_errors)
        );
        format!(
            "{}, {}, {}, {}\n",
            stats.files_str.green(),
            stats.colored_offenses,
            colored_fixed,
            err_str.yellow()
        )
    } else if unfixable > 0 {
        let unfixable_str = format!(
            "{} {} cannot be auto-fixed",
            unfixable,
            pluralize("offense", unfixable)
        );
        format!(
            "{}, {}, {}, {}\n",
            stats.files_str.green(),
            stats.colored_offenses,
            colored_fixed,
            unfixable_str.yellow()
        )
    } else {
        format!(
            "{}, {}, {}\n",
            stats.files_str.green(),
            stats.colored_offenses,
            colored_fixed
        )
    }
}

fn pluralize(word: &str, count: usize) -> String {
    if count == 1 {
        word.to_string()
    } else {
        format!("{}s", word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::AnalysisResult;
    use crate::fix::Fix;
    use crate::offense::{Offense, OffenseKind};

    #[test]
    fn pluralize_singular() {
        assert_eq!(pluralize("file", 1), "file");
    }

    #[test]
    fn pluralize_plural() {
        assert_eq!(pluralize("file", 0), "files");
        assert_eq!(pluralize("offense", 2), "offenses");
    }

    fn make_result(offenses: Vec<Offense>) -> TraversalResult {
        TraversalResult {
            results: vec![AnalysisResult {
                path: "test.rb".to_string(),
                offenses,
            }],
            parse_errors: vec![],
            files_inspected: 1,
        }
    }

    fn make_result_with_parse_errors(
        offenses: Vec<Offense>,
        parse_errors: Vec<ParseError>,
    ) -> TraversalResult {
        TraversalResult {
            results: vec![AnalysisResult {
                path: "test.rb".to_string(),
                offenses,
            }],
            parse_errors,
            files_inspected: 1,
        }
    }

    fn parse_error() -> ParseError {
        ParseError {
            path: "broken.rb".to_string(),
            message: "boom".to_string(),
        }
    }

    #[test]
    fn format_by_file_lists_path_line_and_fixable_tag() {
        let result = make_result(vec![
            Offense::new(OffenseKind::SortVsSortBy, 7),
            Offense::with_fix(OffenseKind::ForLoopVsEach, 9, Fix::single(0, 3, "x")),
        ]);
        let out = format_results_by_file(&result);
        assert!(out.contains("test.rb"));
        assert!(out.contains("L7"));
        assert!(out.contains("L9"));
        assert!(out.contains(OffenseKind::SortVsSortBy.explanation()));
        assert!(out.contains("(fixable)"));
    }

    #[test]
    fn format_by_rule_groups_and_counts() {
        let result = make_result(vec![
            Offense::new(OffenseKind::SortVsSortBy, 1),
            Offense::new(OffenseKind::SortVsSortBy, 4),
        ]);
        let out = format_results_by_rule(&result);
        assert!(out.contains("(2 offenses)"));
        assert!(out.contains("test.rb:1"));
        assert!(out.contains("test.rb:4"));
    }

    #[test]
    fn format_plain_is_one_line_per_offense() {
        let result = make_result(vec![
            Offense::new(OffenseKind::SortVsSortBy, 1),
            Offense::new(OffenseKind::GsubVsTr, 2),
        ]);
        let out = format_results_plain(&result);
        assert!(out.contains("test.rb:1"));
        assert!(out.contains("test.rb:2"));
        // A file with no offenses contributes nothing.
        assert!(format_results_plain(&make_result(vec![])).is_empty());
    }

    #[test]
    fn format_parse_errors_lists_each_file() {
        let out = format_parse_errors(&[parse_error()]);
        assert!(out.contains("unable to process some files"));
        assert!(out.contains("broken.rb - boom"));
    }

    #[test]
    fn parse_error_banner_only_when_there_are_parse_errors() {
        let clean = format_results(&make_result(vec![]), &OutputFormat::File);
        assert!(!clean.contains("unable to process some files"));

        let broken = format_results(
            &make_result_with_parse_errors(vec![], vec![parse_error()]),
            &OutputFormat::File,
        );
        assert!(broken.contains("unable to process some files"));
    }

    #[test]
    fn parse_error_banner_in_fix_mode_only_when_there_are_parse_errors() {
        let clean = format_fix_results(&make_result(vec![]), 0, 0, &OutputFormat::File);
        assert!(!clean.contains("unable to process some files"));

        let broken = format_fix_results(
            &make_result_with_parse_errors(vec![], vec![parse_error()]),
            0,
            0,
            &OutputFormat::File,
        );
        assert!(broken.contains("unable to process some files"));
    }

    #[test]
    fn statistics_counts_files_offenses_and_fixables() {
        let out = format_statistics(&make_result(vec![
            Offense::new(OffenseKind::SortVsSortBy, 1),
            Offense::with_fix(OffenseKind::ForLoopVsEach, 2, Fix::single(0, 3, "x")),
        ]));
        assert!(out.contains("1 file inspected"));
        assert!(out.contains("2 offenses detected"));
        assert!(out.contains("1 offense autocorrectable"));
    }

    #[test]
    fn statistics_omits_autocorrectable_hint_when_none_are_fixable() {
        let out = format_statistics(&make_result(vec![Offense::new(
            OffenseKind::SortVsSortBy,
            1,
        )]));
        assert!(!out.contains("autocorrectable"));
    }

    #[test]
    fn statistics_reports_unparsable_files_only_when_present() {
        let none = format_statistics(&make_result(vec![]));
        assert!(!none.contains("unparsable"));

        let some = format_statistics(&make_result_with_parse_errors(vec![], vec![parse_error()]));
        assert!(some.contains("1 unparsable file found"));
    }

    #[test]
    fn fix_statistics_reports_fixed_and_unfixable_counts() {
        let result = make_result(vec![
            Offense::new(OffenseKind::SortVsSortBy, 1),
            Offense::with_fix(OffenseKind::ForLoopVsEach, 2, Fix::single(0, 3, "x")),
        ]);
        let out = format_fix_statistics(&result, 1, 0);
        assert!(out.contains("1 offense fixed"));
        assert!(out.contains("1 offense cannot be auto-fixed"));
        assert!(!out.contains("skipped"));
    }

    #[test]
    fn fix_statistics_reports_skipped_files_when_writes_failed() {
        let result = make_result(vec![Offense::with_fix(
            OffenseKind::ForLoopVsEach,
            1,
            Fix::single(0, 3, "x"),
        )]);
        let out = format_fix_statistics(&result, 0, 2);
        assert!(out.contains("2 files skipped (syntax error after fix)"));
    }

    #[test]
    fn fix_statistics_without_errors_or_leftovers() {
        let result = make_result(vec![Offense::with_fix(
            OffenseKind::ForLoopVsEach,
            1,
            Fix::single(0, 3, "x"),
        )]);
        let out = format_fix_statistics(&result, 1, 0);
        assert!(out.contains("1 offense fixed"));
        assert!(!out.contains("cannot be auto-fixed"));
        assert!(!out.contains("skipped"));
    }

    /// Restores the global colour setting even if the test panics.
    struct ForcedColour;

    impl ForcedColour {
        fn on() -> Self {
            colored::control::set_override(true);
            Self
        }
    }

    impl Drop for ForcedColour {
        fn drop(&mut self) {
            colored::control::unset_override();
        }
    }

    /// Colour is the only difference between some branches, so assert on the codes.
    #[test]
    fn zero_offenses_and_fixes_are_coloured_differently_from_non_zero() {
        let _colour = ForcedColour::on();
        let green = "\u{1b}[32m";

        let clean = format_statistics(&make_result(vec![]));
        assert!(clean.contains(&format!("{}0 offenses detected", green)));

        let dirty = format_statistics(&make_result(vec![Offense::new(
            OffenseKind::SortVsSortBy,
            1,
        )]));
        assert!(!dirty.contains(&format!("{}1 offense detected", green)));

        let result = make_result(vec![Offense::with_fix(
            OffenseKind::ForLoopVsEach,
            1,
            Fix::single(0, 3, "x"),
        )]);
        assert!(
            format_fix_statistics(&result, 1, 0).contains(&format!("{}1 offense fixed", green))
        );
        assert!(
            !format_fix_statistics(&result, 0, 0).contains(&format!("{}0 offenses fixed", green))
        );

        colored::control::unset_override();
    }

    #[test]
    fn filter_unfixable_keeps_only_no_fix() {
        let offenses = vec![
            Offense::new(OffenseKind::GsubVsTr, 1),
            Offense::with_fix(OffenseKind::ForLoopVsEach, 2, Fix::single(0, 3, "x")),
            Offense::new(OffenseKind::SortVsSortBy, 3),
        ];
        let result = make_result(offenses);
        let filtered = filter_unfixable(&result);
        assert_eq!(filtered.results[0].offenses.len(), 2);
        assert!(filtered.results[0].offenses.iter().all(|o| o.fix.is_none()));
    }

    #[test]
    fn filter_unfixable_empty_when_all_fixable() {
        let offenses = vec![Offense::with_fix(
            OffenseKind::ForLoopVsEach,
            1,
            Fix::single(0, 3, "x"),
        )];
        let result = make_result(offenses);
        let filtered = filter_unfixable(&result);
        assert_eq!(filtered.results[0].offenses.len(), 0);
    }

    #[test]
    fn print_results_dispatches_all_formats() {
        let result = make_result(vec![Offense::new(OffenseKind::GsubVsTr, 1)]);
        print_results(&result, &OutputFormat::File);
        print_results(&result, &OutputFormat::Rule);
        print_results(&result, &OutputFormat::Plain);
    }

    #[test]
    fn print_results_with_parse_errors() {
        let result = TraversalResult {
            results: vec![AnalysisResult {
                path: "ok.rb".to_string(),
                offenses: vec![Offense::new(OffenseKind::GsubVsTr, 1)],
            }],
            parse_errors: vec![ParseError {
                path: "bad.rb".to_string(),
                message: "syntax error".to_string(),
            }],
            files_inspected: 2,
        };
        print_results(&result, &OutputFormat::File);
    }

    #[test]
    fn print_fix_results_with_parse_errors() {
        let result = TraversalResult {
            results: vec![AnalysisResult {
                path: "ok.rb".to_string(),
                offenses: vec![Offense::with_fix(
                    OffenseKind::ForLoopVsEach,
                    1,
                    Fix::single(0, 3, "x"),
                )],
            }],
            parse_errors: vec![ParseError {
                path: "bad.rb".to_string(),
                message: "syntax error".to_string(),
            }],
            files_inspected: 2,
        };
        print_fix_results(&result, 1, 0, &OutputFormat::File);
    }

    #[test]
    fn print_fix_results_unfixable_remaining() {
        let offenses = vec![
            Offense::new(OffenseKind::GsubVsTr, 1),
            Offense::with_fix(OffenseKind::ForLoopVsEach, 2, Fix::single(0, 3, "x")),
        ];
        let result = make_result(offenses);
        print_fix_results(&result, 1, 0, &OutputFormat::Rule);
        print_fix_results(&result, 1, 0, &OutputFormat::Plain);
    }
}
