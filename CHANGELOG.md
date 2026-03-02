# Changelog

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
