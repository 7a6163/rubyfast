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
        path.parent().unwrap_or(Path::new("."))
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

    let result = traverse_and_analyze(path, &config);

    if cli.fix {
        let mut total_fixed = 0;
        let mut total_errors = 0;

        for analysis in &result.results {
            let fixes: Vec<_> = analysis
                .offenses
                .iter()
                .filter_map(|o| o.fix.as_ref())
                .cloned()
                .collect();

            if fixes.is_empty() {
                continue;
            }

            let file_path = Path::new(&analysis.path);
            match apply_fixes_to_file(file_path, &fixes) {
                Ok(count) => total_fixed += count,
                Err(e) => {
                    eprintln!("{}", colored::Colorize::yellow(e.as_str()));
                    total_errors += 1;
                }
            }
        }

        print_fix_results(&result, total_fixed, total_errors, &cli.format);
    } else {
        print_results(&result, &cli.format);
    }

    if cli.fix {
        // In fix mode, only exit 1 if there are unfixable offenses remaining
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
