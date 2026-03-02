use clap::Parser;

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
}
