use std::path::Path;
use std::process;

use clap::Parser;

use rubyfast::cli::Cli;
use rubyfast::config::Config;
use rubyfast::file_traverser::traverse_and_analyze;
use rubyfast::fix::apply_fixes_to_file;
use rubyfast::output::{print_fix_results, print_results};

fn main() {
    let cli = Cli::parse();
    let path = Path::new(&cli.path);

    if !path.exists() {
        eprintln!(
            "{}",
            colored::Colorize::red(format!("No such file or directory - {}", cli.path).as_str())
        );
        process::exit(1);
    }

    let base_dir = if path.is_file() {
        // `x.rb` has an empty parent, which can't be canonicalized or walked up from.
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
    } else {
        path
    };

    let config = match Config::load(base_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            process::exit(1);
        }
    };

    let mut result = traverse_and_analyze(path, &config);

    if cli.fix {
        let mut total_fixed = 0;
        let mut total_errors = 0;

        for analysis in &mut result.results {
            let fixes: Vec<_> = analysis
                .offenses
                .iter()
                .filter_map(|o| o.fix.clone())
                .collect();

            if fixes.is_empty() {
                continue;
            }

            let applied = match apply_fixes_to_file(Path::new(&analysis.path), &fixes) {
                Ok(applied) => applied,
                Err(e) => {
                    eprintln!("{}", colored::Colorize::yellow(e.as_str()));
                    total_errors += 1;
                    vec![false; fixes.len()]
                }
            };
            total_fixed += applied.iter().filter(|&&a| a).count();

            // Fixes that didn't land leave their offense in the source: drop the fix so the
            // offense is reported and counted as remaining below.
            let mut applied = applied.into_iter();
            for o in analysis.offenses.iter_mut().filter(|o| o.fix.is_some()) {
                if !applied.next().unwrap_or(false) {
                    o.fix = None;
                }
            }
        }

        print_fix_results(&result, total_fixed, total_errors, &cli.format);
    } else {
        print_results(&result, &cli.format);
    }

    if cli.fix {
        // In fix mode, only exit 1 if offenses remain (unfixable, or whose fix didn't apply)
        let unfixable = result
            .results
            .iter()
            .flat_map(|r| &r.offenses)
            .filter(|o| o.fix.is_none())
            .count();
        if unfixable > 0 {
            process::exit(1);
        }
    } else if result.has_offenses() {
        process::exit(1);
    }
}
