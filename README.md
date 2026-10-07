# rubyfast

[![CI](https://github.com/7a6163/rubyfast/actions/workflows/ci.yml/badge.svg)](https://github.com/7a6163/rubyfast/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/7a6163/rubyfast/graph/badge.svg)](https://codecov.io/gh/7a6163/rubyfast)
[![Crates.io](https://img.shields.io/crates/v/rubyfast)](https://crates.io/crates/rubyfast)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Ruby performance linter rewritten in Rust. Detects 19 common performance anti-patterns in Ruby code.

Rust rewrite of [fasterer](https://github.com/DamirSvrtan/fasterer) — same detection rules, but faster execution, parallel file scanning, and zero Ruby runtime dependency.

## Installation

### Homebrew

```bash
brew install 7a6163/tap/rubyfast
```

### Cargo

```bash
cargo install rubyfast
```

### From source

```bash
cargo install --path .
```

## Usage

```bash
# Scan current directory
rubyfast

# Scan a specific file
rubyfast path/to/file.rb

# Scan a specific directory
rubyfast path/to/ruby/project

# Auto-fix safe offenses in-place
rubyfast --fix path/to/ruby/project

# Choose output format
rubyfast --format file path/to/project   # group by file (default)
rubyfast --format rule path/to/project   # group by rule
rubyfast --format plain path/to/project  # one per line (for CI/grep/reviewdog)
```

Exit code `0` if no offenses, `1` if any offenses found.

## Output Formats

### `--format file` (default)

Groups offenses by file path — compact and easy to scan:

```
app/controllers/concerns/lottery_common.rb
  L13  Hash#fetch with second argument is slower than Hash#fetch with block
  L94  Hash#fetch with second argument is slower than Hash#fetch with block

app/controllers/api/v1/health_articles_controller.rb
  L11  Hash#fetch with second argument is slower than Hash#fetch with block
```

Offenses that support `--fix` are tagged with `(fixable)`, and the summary line shows how many can be auto-fixed:

```
tests/fixtures/19_for_loop.rb
  L1  For loop is slower than using each (fixable)

22 files inspected, 41 offenses detected, 21 offenses autocorrectable (run rubyfast --fix)
```

### `--format rule`

Groups offenses by rule — useful for understanding which patterns are most common:

```
Hash#fetch with second argument is slower than Hash#fetch with block. (3 offenses)
  app/controllers/api/v1/health_articles_controller.rb:11
  app/controllers/concerns/lottery_common.rb:13
  app/controllers/concerns/lottery_common.rb:94
```

### `--format plain`

One offense per line — suitable for grep, reviewdog, and CI pipelines:

```
app/controllers/api/v1/health_articles_controller.rb:11 Hash#fetch with second argument is slower than Hash#fetch with block.
```

## Auto-fix

`rubyfast --fix` automatically corrects 8 safe offenses in-place:

| # | Pattern | Fix |
|---|---------|-----|
| 1 | `.shuffle.first` | `.sample` |
| 2 | `.select{}.first` | `.detect{}` |
| 4 | `.reverse.each` | `.reverse_each` |
| 5 | `.keys.each` | `.each_key` |
| 6 | `.map{}.flatten(1)` | `.flat_map{}` |
| 7 | `.gsub("x","y")` | `.tr("x","y")` |
| 13 | `(1..10).include?` | `.cover?` |
| 19 | `for x in arr` | `arr.each do \|x\|` |

Fixes are applied in reverse byte order with syntax verification — if a fix would produce invalid Ruby, the file is left unchanged.

## Detected Offenses

| # | Pattern | Suggestion | Auto-fix |
|---|---------|------------|----------|
| 1 | `.shuffle.first` | `.sample` | Yes |
| 2 | `.select{}.first` | `.detect{}` | Yes |
| 3 | `.select{}.last` | `.reverse.detect{}` | No |
| 4 | `.reverse.each` | `.reverse_each` | Yes |
| 5 | `.keys.each` | `.each_key` | Yes |
| 6 | `.map{}.flatten(1)` | `.flat_map{}` | Yes |
| 7 | `.gsub("x","y")` (single chars) | `.tr("x","y")` | Yes |
| 8 | `.sort { \|a,b\| ... }` | `.sort_by` | No |
| 9 | `.fetch(k, v)` — constructed default | `.fetch(k) { v }` | No |
| 10 | `.merge!({k: v})` | `h[k] = v` | No |
| 11 | `.map { \|x\| x.foo }` | `.map(&:foo)` | No |
| 12 | `.each_with_index` | `while` loop | No |
| 13 | `(1..10).include?` | `.cover?` | Numeric ranges |
| 14 | `.module_eval("def ...")` | `define_method` | No |
| 15 | `rescue NoMethodError` | `respond_to?` | No |
| 16 | `def foo(&block); block.call; end` | `yield` | No |
| 17 | `def x; @x; end` | `attr_reader` | No |
| 18 | `def x=(v); @x=v; end` | `attr_writer` | No |
| 19 | `for x in arr` | `arr.each` | When scope-safe |

Rule 9 only fires when the default has to be constructed (`fetch(k, [])`,
`fetch(k, "s")`, `fetch(k, Time.now)`). A cheap default — `nil`, a number, a symbol,
`true`/`false`, a constant, or a variable read — is left alone, because the block's
invocation cost makes the block form the slower one there. This matches the caveat fast-ruby
documents next to the benchmark this rule comes from.

Some fixes are only applied when they can't change behavior. Rule 13 is fixed only for
numeric-literal ranges: on other ranges such as `('a'..'z')`, `include?` checks membership
while `cover?` compares bounds. Rule 19 is fixed only when the loop body assigns no locals
and the loop variable isn't used outside the loop, because a `for` loop shares the
enclosing scope and a block doesn't. In `--fix` mode, any offense whose fix wasn't applied
is still reported and makes the run exit 1.

## Inline Disable

Suppress specific offenses with inline comments (similar to RuboCop):

```ruby
# Disable on the same line
x = [].shuffle.first # rubyfast:disable shuffle_first_vs_sample

# Disable the next line
# rubyfast:disable-next-line shuffle_first_vs_sample
x = [].shuffle.first

# Block disable/enable
# rubyfast:disable for_loop_vs_each
for i in arr
  puts i
end
# rubyfast:enable for_loop_vs_each

# Disable all rules
code # rubyfast:disable all

# Multiple rules
code # rubyfast:disable shuffle_first_vs_sample, for_loop_vs_each

# Backwards compatible with fasterer
code # fasterer:disable shuffle_first_vs_sample
```

## Configuration

Create a `.rubyfast.yml` (or `.fasterer.yml` for backwards compatibility) in your project root to disable specific checks or exclude files:

```yaml
speedups:
  shuffle_first_vs_sample: true
  for_loop_vs_each: false          # disable this check

exclude_paths:
  - vendor/**/*.rb
  - tmp/**/*.rb
```

The config file is auto-discovered by walking up from the scan directory. `.rubyfast.yml` takes precedence if both exist.

## CI/CD Integration

### GitHub Action

```yaml
- uses: 7a6163/rubyfast-action@v1
```

With [reviewdog](https://github.com/reviewdog/reviewdog) inline PR comments (uses `--format plain` internally):

```yaml
rubyfast:
  runs-on: ubuntu-latest
  permissions:
    contents: read
    checks: write
    pull-requests: write
  steps:
    - uses: actions/checkout@v4
    - uses: 7a6163/rubyfast-action@v1
      with:
        reviewdog: "true"
        github-token: ${{ secrets.GITHUB_TOKEN }}
        reviewdog-reporter: github-pr-review
```

See [rubyfast-action](https://github.com/7a6163/rubyfast-action) for all options.

### GitLab CI

```yaml
rubyfast:
  image:
    name: ghcr.io/7a6163/rubyfast:latest
    entrypoint: [""]
  script:
    - rubyfast .
```

### Docker

```bash
docker run --rm -v $(pwd):/workspace ghcr.io/7a6163/rubyfast:latest .
```

## Benchmark

Compared against [fasterer](https://github.com/DamirSvrtan/fasterer) (v0.11.0, Ruby + ruby_parser) and a [prism-based fork](https://github.com/mattmenefee/fasterer/tree/native-prism-migration) (Ruby + prism, unreleased).

Corpus: [rubygems.org](https://github.com/rubygems/rubygems.org) at `3c8ea0d4c` — 1,338 Ruby files. Measured with [hyperfine](https://github.com/sharkdp/hyperfine) on Apple Silicon (macOS 26, Ruby 4.0.5); each tool run from the project root, since `fasterer` only scans the current directory.

| Tool | Parser | Time | Relative |
|:---|:---|---:|---:|
| **rubyfast v1.4.0** | Rust + ruby-prism | **64.7 ms** | **1x** |
| fasterer (prism fork) | Ruby + prism | 546 ms | 8.4x slower |
| fasterer v0.11.0 | Ruby + ruby_parser | 4.38 s | 68x slower |

**rubyfast is 68x faster** than the original fasterer and **8.4x faster** than the prism-based Ruby fork.

On this corpus rubyfast reports 33 offenses to fasterer's 74. Every rule matches
exactly except `fetch` (15 vs 56); all 41 of the difference are cheap defaults
(`fetch(:k, nil)`, `fetch("PORT", 3000)`, `fetch(:otp, "")` in a frozen-string-literal
file) that fasterer reports and rubyfast deliberately does not — see rule 9 above.
The 15 that remain all build something: `[]`, `{}`, a method call, or an interpolated
string.

## Development

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo llvm-cov              # line coverage (100%)
cargo mutants               # mutation testing
```

## License

MIT
