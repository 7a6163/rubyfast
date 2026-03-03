# rubyfast

A Ruby performance linter rewritten in Rust. Detects 19 common performance anti-patterns in Ruby code.

Rust rewrite of [fasterer](https://github.com/DamirSvrtan/fasterer) — same detection rules, but faster execution, parallel file scanning, and zero Ruby runtime dependency.

## Installation

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
```

Exit code `0` if no offenses, `1` if any offenses found.

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
| 9 | `.fetch(k, v)` | `.fetch(k) { v }` | No |
| 10 | `.merge!({k: v})` | `h[k] = v` | No |
| 11 | `.map { \|x\| x.foo }` | `.map(&:foo)` | No |
| 12 | `.each_with_index` | `while` loop | No |
| 13 | `(1..10).include?` | `.cover?` | Yes |
| 14 | `.module_eval("def ...")` | `define_method` | No |
| 15 | `rescue NoMethodError` | `respond_to?` | No |
| 16 | `def foo(&block); block.call; end` | `yield` | No |
| 17 | `def x; @x; end` | `attr_reader` | No |
| 18 | `def x=(v); @x=v; end` | `attr_writer` | No |
| 19 | `for x in arr` | `arr.each` | Yes |

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

With [reviewdog](https://github.com/reviewdog/reviewdog) inline PR comments:

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

Compared against the original [fasterer](https://github.com/DamirSvrtan/fasterer) Ruby gem (v0.11.0) using [hyperfine](https://github.com/sharkdp/hyperfine). Measured on Apple Silicon, macOS.

### Large project: Discourse (9,055 Ruby files)

| Command | Mean | Min | Max | Relative |
|:---|---:|---:|---:|---:|
| `rubyfast` | 585.7 ms | 557.6 ms | 624.4 ms | **1.00** |
| `fasterer` | 58,218.0 ms | 55,535.9 ms | 65,065.4 ms | 99.40x slower |

### Small project: 21 test fixtures

| Command | Mean | Min | Max | Relative |
|:---|---:|---:|---:|---:|
| `rubyfast` | 4.8 ms | 3.2 ms | 71.5 ms | **1.00** |
| `fasterer` | 353.7 ms | 347.1 ms | 366.2 ms | 73.21x slower |

**~100x faster** on real-world codebases. The Rust implementation processes 9,000+ Ruby files in under a second, while the Ruby gem takes nearly a minute.

## Development

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

## License

MIT
