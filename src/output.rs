use colored::Colorize;

use crate::analyzer::ParseError;
use crate::file_traverser::TraversalResult;

/// Print analysis results to stdout, matching original fasterer format.
pub fn print_results(result: &TraversalResult) {
    // Print offenses grouped by file
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

    // Print parse errors if any
    if !result.parse_errors.is_empty() {
        print_parse_errors(&result.parse_errors);
    }

    // Print statistics
    print_statistics(result);
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
pub fn print_fix_results(result: &TraversalResult, total_fixed: usize, total_errors: usize) {
    // Print unfixable offenses (those without fixes)
    for analysis in &result.results {
        for offense in &analysis.offenses {
            if offense.fix.is_none() {
                let location = format!("{}:{}", analysis.path, offense.line);
                println!("{} {}.", location.red(), offense.kind.explanation());
            }
        }
    }

    if !result.parse_errors.is_empty() {
        print_parse_errors(&result.parse_errors);
    }

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

    let unfixable = offenses - fixable;
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
