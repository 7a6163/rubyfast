use std::collections::BTreeMap;

use colored::Colorize;

use crate::analyzer::ParseError;
use crate::cli::OutputFormat;
use crate::file_traverser::TraversalResult;
use crate::offense::OffenseKind;

/// Print analysis results using the selected output format.
pub fn print_results(result: &TraversalResult, format: &OutputFormat) {
    match format {
        OutputFormat::File => print_results_by_file(result),
        OutputFormat::Rule => print_results_by_rule(result),
        OutputFormat::Plain => print_results_plain(result),
    }

    if !result.parse_errors.is_empty() {
        print_parse_errors(&result.parse_errors);
    }

    print_statistics(result);
}

/// `--format file` — group offenses by file path.
///
/// ```text
/// app/controllers/concerns/lottery_common.rb
///   L13  Hash#fetch with second argument is slower than Hash#fetch with block [fetch_with_argument_vs_block]
///   L94  Hash#fetch with second argument is slower than Hash#fetch with block [fetch_with_argument_vs_block]
/// ```
fn print_results_by_file(result: &TraversalResult) {
    for analysis in &result.results {
        if analysis.offenses.is_empty() {
            continue;
        }
        println!("{}", analysis.path.bold());
        for offense in &analysis.offenses {
            println!(
                "  {}  {} [{}]",
                format!("L{}", offense.line).cyan(),
                offense.kind.explanation(),
                offense.kind.config_key().dimmed()
            );
        }
        println!();
    }
}

/// `--format rule` — group offenses by rule kind.
///
/// ```text
/// Hash#fetch with second argument is slower than Hash#fetch with block. (5 offenses)
///   app/controllers/api/v1/health_articles_controller.rb:11
///   app/controllers/concerns/lottery_common.rb:13
/// ```
fn print_results_by_rule(result: &TraversalResult) {
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
        println!(
            "{} ({} {})",
            kind.explanation().yellow(),
            count,
            pluralize("offense", count)
        );
        for (path, line) in locations {
            println!("  {}:{}", path, line);
        }
        println!();
    }
}

/// `--format plain` — one offense per line (original format, for grep/reviewdog).
///
/// ```text
/// app/controllers/api/v1/health_articles_controller.rb:11 Hash#fetch with second argument ...
/// ```
fn print_results_plain(result: &TraversalResult) {
    for analysis in &result.results {
        if analysis.offenses.is_empty() {
            continue;
        }
        for offense in &analysis.offenses {
            let location = format!("{}:{}", analysis.path, offense.line);
            println!("{} {}.", location.red(), offense.kind.explanation());
        }
        println!();
    }
}

fn print_parse_errors(errors: &[ParseError]) {
    println!(
        "rubyfast was unable to process some files because the\n\
         internal parser is not able to read some characters or\n\
         has timed out. Unprocessable files were:"
    );
    println!("-----------------------------------------------------");
    for err in errors {
        println!("{} - {}", err.path, err.message);
    }
    println!();
}

fn print_statistics(result: &TraversalResult) {
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

    if parse_errors > 0 {
        let errors_str = format!(
            "{} unparsable {} found",
            parse_errors,
            pluralize("file", parse_errors)
        );
        println!(
            "{}, {}, {}",
            files_str.green(),
            colored_offenses,
            errors_str.red()
        );
    } else {
        println!("{}, {}", files_str.green(), colored_offenses);
    }
}

/// Print results when --fix mode is active.
pub fn print_fix_results(
    result: &TraversalResult,
    total_fixed: usize,
    total_errors: usize,
    format: &OutputFormat,
) {
    // Print unfixable offenses using the selected format
    let unfixable_result = filter_unfixable(result);
    match format {
        OutputFormat::File => print_results_by_file(&unfixable_result),
        OutputFormat::Rule => print_results_by_rule(&unfixable_result),
        OutputFormat::Plain => print_results_plain(&unfixable_result),
    }

    if !result.parse_errors.is_empty() {
        print_parse_errors(&result.parse_errors);
    }

    print_fix_statistics(result, total_fixed, total_errors);
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

fn print_fix_statistics(result: &TraversalResult, total_fixed: usize, total_errors: usize) {
    let files = result.files_inspected;
    let offenses = result.total_offenses();
    let fixable: usize = result
        .results
        .iter()
        .flat_map(|r| &r.offenses)
        .filter(|o| o.fix.is_some())
        .count();

    let files_str = format!("{} {} inspected", files, pluralize("file", files));
    let offenses_str = format!("{} {} detected", offenses, pluralize("offense", offenses));
    let fixed_str = format!(
        "{} {} fixed",
        total_fixed,
        pluralize("offense", total_fixed)
    );

    let colored_offenses = if offenses == 0 {
        offenses_str.green().to_string()
    } else {
        offenses_str.red().to_string()
    };

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
        println!(
            "{}, {}, {}, {}",
            files_str.green(),
            colored_offenses,
            colored_fixed,
            err_str.yellow()
        );
    } else if unfixable > 0 {
        let unfixable_str = format!(
            "{} {} cannot be auto-fixed",
            unfixable,
            pluralize("offense", unfixable)
        );
        println!(
            "{}, {}, {}, {}",
            files_str.green(),
            colored_offenses,
            colored_fixed,
            unfixable_str.yellow()
        );
    } else {
        println!(
            "{}, {}, {}",
            files_str.green(),
            colored_offenses,
            colored_fixed
        );
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
    fn print_results_by_file_no_panic() {
        let result = make_result(vec![Offense::new(OffenseKind::GsubVsTr, 5)]);
        print_results_by_file(&result);
    }

    #[test]
    fn print_results_by_file_empty_no_panic() {
        let result = make_result(vec![]);
        print_results_by_file(&result);
    }

    #[test]
    fn print_results_by_rule_no_panic() {
        let result = make_result(vec![
            Offense::new(OffenseKind::GsubVsTr, 5),
            Offense::new(OffenseKind::GsubVsTr, 10),
        ]);
        print_results_by_rule(&result);
    }

    #[test]
    fn print_results_plain_no_panic() {
        let result = make_result(vec![Offense::new(OffenseKind::GsubVsTr, 5)]);
        print_results_plain(&result);
    }

    #[test]
    fn print_results_plain_empty_no_panic() {
        let result = make_result(vec![]);
        print_results_plain(&result);
    }

    #[test]
    fn print_statistics_no_offenses() {
        let result = make_result(vec![]);
        print_statistics(&result);
    }

    #[test]
    fn print_statistics_with_offenses() {
        let result = make_result(vec![Offense::new(OffenseKind::GsubVsTr, 5)]);
        print_statistics(&result);
    }

    #[test]
    fn print_statistics_with_parse_errors() {
        let result = TraversalResult {
            results: vec![],
            parse_errors: vec![ParseError {
                path: "bad.rb".to_string(),
                message: "syntax error".to_string(),
            }],
            files_inspected: 1,
        };
        print_statistics(&result);
    }

    #[test]
    fn print_parse_errors_no_panic() {
        let errors = vec![ParseError {
            path: "bad.rb".to_string(),
            message: "oops".to_string(),
        }];
        print_parse_errors(&errors);
    }

    #[test]
    fn print_results_dispatches_all_formats() {
        let result = make_result(vec![Offense::new(OffenseKind::GsubVsTr, 1)]);
        print_results(&result, &OutputFormat::File);
        print_results(&result, &OutputFormat::Rule);
        print_results(&result, &OutputFormat::Plain);
    }

    #[test]
    fn print_fix_results_no_panic() {
        let offenses = vec![
            Offense::new(OffenseKind::GsubVsTr, 1),
            Offense::with_fix(OffenseKind::ForLoopVsEach, 2, Fix::single(0, 3, "x")),
        ];
        let result = make_result(offenses);
        print_fix_results(&result, 1, 0, &OutputFormat::File);
    }

    #[test]
    fn print_fix_results_with_errors() {
        let offenses = vec![Offense::with_fix(
            OffenseKind::ForLoopVsEach,
            1,
            Fix::single(0, 3, "x"),
        )];
        let result = make_result(offenses);
        print_fix_results(&result, 0, 1, &OutputFormat::File);
    }

    #[test]
    fn print_fix_results_all_fixed() {
        let offenses = vec![Offense::with_fix(
            OffenseKind::ForLoopVsEach,
            1,
            Fix::single(0, 3, "x"),
        )];
        let result = make_result(offenses);
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
