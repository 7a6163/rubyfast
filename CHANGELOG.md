# Changelog

## [1.3.2] - 2026-03-16

### Fixed

- `apply_fixes` now returns actual replacement count instead of fix object count
- `is_int_one` handles `0x1`, `0b1`, `0o1` integer literal forms

### Changed

- Replace `serde_yaml` (deprecated) with `serde-saphyr` (YAML 1.2, pure Rust)
- Use `HashSet` for disabled offenses config (O(1) lookup)
- Replace `call_args()` Vec allocation with zero-alloc `first_call_arg()` / `call_args_pair()`
- Replace `body_first_expression` API with safer `body_single_expression`
- Extract `walk_call_children` to deduplicate CallNode traversal in analyzer
- Use `std::str::from_utf8` instead of `String::from_utf8_lossy` in comment directives
- Extract shared `leak_parse` / `parse_first_stmt` test helpers
- Update benchmark: 3-way comparison (rubyfast vs fasterer vs fasterer-prism fork)

## [1.3.1] - 2026-03-16

### Fixed

- Add `clang` dependency to Dockerfile for `ruby-prism` bindgen compilation

## [1.3.0] - 2026-03-16

### Changed

- Migrate parser from `lib-ruby-parser` (Ruby 3.1.2) to `ruby-prism` (Ruby 3.3+)
  - Support Ruby 3.2, 3.3, 3.4+ syntax
  - Native handling of all encodings (ASCII, US-ASCII, etc.) — no custom decoder needed
  - Error-tolerant parsing: always produces an AST, even with syntax errors
  - Prism is Ruby's official default parser since 3.3

### Removed

- `lib-ruby-parser` dependency replaced by `ruby-prism`

## [1.2.5] - 2026-03-16

### Fixed

- Handle `# encoding: ASCII` and `# encoding: us-ascii` magic comments in Ruby files
  that previously caused `UnsupportedEncoding` parse errors

### Changed

- Upgrade `colored` dependency from 2.x to 3.x

## [1.2.4] - 2026-03-16

### Refactored

- Extract AST visitor into dedicated `ast_visitor` module (`ast_helpers.rs`: 976 → ~370 lines)
- Add `compute_newline_positions()` utility to eliminate 3 duplicated newline computation blocks
- Consolidate output statistics formatting with shared `StatsParts` builder

## [1.2.3] - 2026-03-13

### Added

- Summary line now shows autocorrectable count when fixable offenses are detected:
  `22 files inspected, 41 offenses detected, 21 offenses autocorrectable (run rubyfast --fix)`
- CI: `cargo audit` security check for dependency vulnerabilities

## [1.2.2] - 2026-03-11

### Changed

- Upgrade to Rust edition 2024
- Update crate description

## [1.2.1] - 2026-03-11

### Changed

- Offenses that support `--fix` are now tagged with `(fixable)` in output
- Shortened verbose explanation messages for better readability
- Updated benchmark results (17,091 Ruby files: 3.4s vs 150s)

## [1.2.0] - 2026-03-11

### Changed

- Default `--format file` output now shows human-readable explanation instead of config key
- Performance: use `HashSet` instead of `Vec` for file exclusion lookup (O(1) vs O(n))

### Internal

- Test coverage increased from 61.87% to 93.10% (242 tests)
- Upgrade GitHub Actions to Node.js 24 compatible versions (checkout@v6, upload-artifact@v7, download-artifact@v8)

## [1.1.1] - 2026-03-11

### Changed

- Upgrade `codecov/codecov-action` from v4 to v5 with `fail_ci_if_error`
- Add crates.io publish step to release workflow
- Add CI, Codecov, Crates.io, and License badges to README

## [1.1.0] - 2026-03-03

### Added

- `--format` flag to control output grouping: `file` (default), `rule`, or `plain`
  - `--format file` groups offenses under file paths with line numbers
  - `--format rule` groups offenses by rule kind with offense counts
  - `--format plain` outputs one offense per line (original format, for grep/reviewdog/CI)

### Fixed

- False positive: `fetch` calls with an existing block (e.g. `Rails.cache.fetch(key, opts) { ... }`) no longer trigger `fetch_with_argument_vs_block`

## [1.0.0] - 2026-03-03

### Changed

- Bump to stable 1.0.0 release

## [0.1.0] - 2026-03-03

### Added

- 19 Ruby performance anti-pattern detections (ported from fasterer gem)
- Auto-fix support for 8 safe offenses (`--fix` flag)
- Inline disable comments (`rubyfast:disable`, `rubyfast:disable-next-line`)
- Backwards compatible with `fasterer:disable` comments
- Configuration via `.rubyfast.yml` (or `.fasterer.yml` for compatibility)
- Exclude paths support in configuration
- Parallel file scanning with rayon
- Colored terminal output
- Exit code 1 when offenses found
- ~100x faster than the original Ruby gem on real-world codebases
