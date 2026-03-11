# Changelog

## [1.2.0] - 2026-03-11

### Changed

- Default `--format file` output now shows human-readable explanation with config key in brackets:
  `L73  Hash#fetch with second argument is slower than Hash#fetch with block [fetch_with_argument_vs_block]`
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
