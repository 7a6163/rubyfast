# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

rubyfast is a Rust rewrite of the Ruby linter [fasterer](https://github.com/DamirSvrtan/fasterer): it parses Ruby with `ruby-prism` and detects 19 performance anti-patterns, 8 of which it can auto-fix (`--fix`).

## Commands

CI (`.github/workflows/ci.yml`) runs all of these, and each must pass:

```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test
cargo audit
cargo mutants --no-shuffle -j 2          # CI runs it with --in-diff on PRs; config in .cargo/mutants.toml
cargo llvm-cov --lcov --output-path lcov.info
```

- One test: `cargo test <name_substring>`, e.g. `cargo test overlapping_skipped`
- Integration tests only: `cargo test --test integration_tests`
- Run against a file: `cargo run -- tests/fixtures/19_for_loop.rb [--fix] [--format file|rule|plain]`

Exit code is 1 when offenses are found. With `--fix`, it is 1 only when offenses remain (unfixable, or their fix was not applied).

## Architecture

The pipeline, in order:

1. `main.rs` loads `Config` (walks up from the scan dir looking for `.rubyfast.yml`, falling back to `.fasterer.yml`), then calls `file_traverser::traverse_and_analyze`.
2. `file_traverser` collects `.rb` files, drops anything matching `exclude_paths`, and analyzes files in parallel with rayon.
3. `analyzer::analyze_file` parses the file. If the parse has errors, it returns no offenses. Otherwise `scan_tree` runs a `Walker` (prism `Visit` impl) that sends `ForNode`/`DefNode`/`RescueNode`/`CallNode` to the scanners in `src/scanner/`. Each override must still call `ruby_prism::visit_*` so traversal continues into child nodes.
4. Scanners return `Offense`s whose `line` field holds a **byte offset** at that point. `analyze_file` converts it to a line number, then filters by config and by inline `# rubyfast:disable` / `fasterer:disable` directives (`comment_directives`).
5. `output` prints the results. In `--fix` mode, `fix::apply_fixes_to_file` applies each offense's `Fix` (byte-range `Replacement`s).

Key invariants:
- A fix is all-or-nothing. It is skipped if it overlaps another fix or is out of bounds, and the file stays unchanged if the result doesn't re-parse (`verify_syntax`). `main.rs` clears `o.fix` for each fix that wasn't applied, so that offense is still reported.
- A fix must never change program behavior. Some rules are only fixed when that's provably safe: `include?` → `cover?` only on numeric-literal ranges, and `for` → `each` only when the body assigns no locals and the loop variable isn't used after the loop. Rule 9 (`fetch` with a default argument) only fires when the default has to be constructed. Literals count as constructed unless the file has `# frozen_string_literal: true`.
- `OffenseKind` (`offense.rs`) is the single registry for each rule: its config key, its message, whether it's fixable, and `all()`. A new rule goes there plus a scanner.

## Tests

- `tests/fixtures/NN_<rule>.rb` has one fixture per rule. `tests/integration/fixture_tests.rs` lists the **exact** set and count of offenses each fixture must produce, so update that table whenever you change a fixture or a rule.
- `tests/integration/cli_tests.rs` runs the built binary via `CARGO_BIN_EXE_rubyfast` (not `cargo run`) so coverage instrumentation applies.
- Mutation testing gates CI, so new logic needs tests that kill its mutants. `src/main.rs` is excluded from mutation testing. Add any equivalent mutant to `exclude_re` in `.cargo/mutants.toml` with a comment explaining why no test can catch it.
- Removing or changing public library items is a breaking change (see the 2.0.0 entry in `CHANGELOG.md`).
