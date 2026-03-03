use clap::{Parser, ValueEnum};

/// Output format for displaying results.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Group offenses by file (default).
    #[default]
    File,
    /// Group offenses by rule.
    Rule,
    /// One offense per line, suitable for grep/reviewdog/CI.
    Plain,
}

/// A Ruby performance linter — detects common performance anti-patterns.
#[derive(Parser, Debug)]
#[command(name = "rubyfast", version, about)]
pub struct Cli {
    /// Path to a Ruby file or directory to scan (defaults to current directory).
    #[arg(default_value = ".")]
    pub path: String,

    /// Automatically fix safe offenses in-place.
    #[arg(long)]
    pub fix: bool,

    /// Output format: file (default), rule, or plain.
    #[arg(long, value_enum, default_value_t = OutputFormat::File)]
    pub format: OutputFormat,
}
