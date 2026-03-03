# Changelog

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
