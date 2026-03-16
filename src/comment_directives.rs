use std::collections::HashSet;

use crate::ast_helpers::byte_offset_to_line;
use crate::offense::OffenseKind;

/// Tracks which lines have which offense kinds disabled via inline comments.
#[derive(Debug)]
pub struct DisabledSet {
    /// Lines where all rules are disabled.
    all_disabled_lines: HashSet<usize>,
    /// Lines where specific rules are disabled.
    rule_disabled_lines: HashSet<(usize, OffenseKind)>,
}

impl DisabledSet {
    /// Check if a given offense kind is disabled on a given line.
    pub fn is_disabled(&self, line: usize, kind: OffenseKind) -> bool {
        self.all_disabled_lines.contains(&line) || self.rule_disabled_lines.contains(&(line, kind))
    }
}

/// Build a DisabledSet from a parse result, source bytes, and pre-computed newline positions.
///
/// Supports:
/// - `# rubyfast:disable rule` or `# fasterer:disable rule` — trailing (same line) or block start
/// - `# rubyfast:disable-next-line rule` — disables the next line
/// - `# rubyfast:enable rule` — ends a block disable
/// - `# rubyfast:disable all` — disable all rules
/// - `# rubyfast:disable rule1, rule2` — multiple rules
pub fn build_disabled_set(
    parse_result: &ruby_prism::ParseResult<'_>,
    source: &[u8],
    newline_positions: &[usize],
) -> DisabledSet {
    let total_lines = newline_positions.len() + 1;

    let mut all_disabled_lines = HashSet::new();
    let mut rule_disabled_lines = HashSet::new();

    // Track block disables: None = all, Some(kind) = specific
    let mut block_all_start: Option<usize> = None;
    let mut block_rule_starts: Vec<(OffenseKind, usize)> = Vec::new();

    for comment in parse_result.comments() {
        let loc = comment.location();
        let begin = loc.start_offset();
        let end = loc.end_offset();
        let comment_line = byte_offset_to_line(newline_positions, begin);
        let comment_text = &source[begin..end.min(source.len())];
        let is_trailing = is_trailing_comment(source, begin);

        // Use from_utf8 to avoid allocation for valid UTF-8 (the common case)
        let comment_str = match std::str::from_utf8(comment_text) {
            Ok(s) => s,
            Err(_) => continue, // skip non-UTF-8 comments
        };

        if let Some(directive) = parse_directive(comment_str) {
            match directive {
                Directive::Disable(targets) if is_trailing => {
                    // Same-line disable
                    apply_targets_to_line(
                        &targets,
                        comment_line,
                        &mut all_disabled_lines,
                        &mut rule_disabled_lines,
                    );
                }
                Directive::DisableNextLine(targets) => {
                    let next_line = comment_line + 1;
                    apply_targets_to_line(
                        &targets,
                        next_line,
                        &mut all_disabled_lines,
                        &mut rule_disabled_lines,
                    );
                }
                Directive::Disable(targets) => {
                    // Standalone comment — block start
                    for target in &targets {
                        match target {
                            Target::All => {
                                block_all_start = Some(comment_line + 1);
                            }
                            Target::Rule(kind) => {
                                block_rule_starts.push((*kind, comment_line + 1));
                            }
                        }
                    }
                }
                Directive::Enable(targets) => {
                    // Block end
                    let end_line = comment_line; // exclusive
                    for target in &targets {
                        match target {
                            Target::All => {
                                if let Some(start) = block_all_start.take() {
                                    for line in start..end_line {
                                        all_disabled_lines.insert(line);
                                    }
                                }
                            }
                            Target::Rule(kind) => {
                                let idx = block_rule_starts.iter().rposition(|(k, _)| k == kind);
                                if let Some(i) = idx {
                                    let (_, start) = block_rule_starts.remove(i);
                                    for line in start..end_line {
                                        rule_disabled_lines.insert((line, *kind));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Close unclosed block disables at end of file
    if let Some(start) = block_all_start {
        for line in start..=total_lines {
            all_disabled_lines.insert(line);
        }
    }
    for (kind, start) in &block_rule_starts {
        for line in *start..=total_lines {
            rule_disabled_lines.insert((line, *kind));
        }
    }

    DisabledSet {
        all_disabled_lines,
        rule_disabled_lines,
    }
}

#[derive(Debug)]
enum Target {
    All,
    Rule(OffenseKind),
}

#[derive(Debug)]
enum Directive {
    Disable(Vec<Target>),
    DisableNextLine(Vec<Target>),
    Enable(Vec<Target>),
}

/// Parse a comment string into a directive, if it matches.
fn parse_directive(comment: &str) -> Option<Directive> {
    // Strip leading `#` and whitespace
    let stripped = comment.trim_start_matches('#').trim();

    // Match `rubyfast:` or `fasterer:` prefix
    let rest = stripped
        .strip_prefix("rubyfast:")
        .or_else(|| stripped.strip_prefix("fasterer:"))?;

    let rest = rest.trim();

    if let Some(targets_str) = rest.strip_prefix("disable-next-line") {
        let targets = parse_targets(targets_str.trim());
        if targets.is_empty() {
            return None;
        }
        Some(Directive::DisableNextLine(targets))
    } else if let Some(targets_str) = rest.strip_prefix("disable") {
        let targets = parse_targets(targets_str.trim());
        if targets.is_empty() {
            return None;
        }
        Some(Directive::Disable(targets))
    } else if let Some(targets_str) = rest.strip_prefix("enable") {
        let targets = parse_targets(targets_str.trim());
        if targets.is_empty() {
            return None;
        }
        Some(Directive::Enable(targets))
    } else {
        None
    }
}

/// Parse comma-separated targets: "all" or "rule1, rule2"
fn parse_targets(s: &str) -> Vec<Target> {
    s.split(',')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .filter_map(|t| {
            if t == "all" {
                Some(Target::All)
            } else {
                OffenseKind::from_config_key(t).map(Target::Rule)
            }
        })
        .collect()
}

/// Check if a comment at `begin` byte offset is trailing (has code before it on the same line).
fn is_trailing_comment(source: &[u8], begin: usize) -> bool {
    // Walk backwards from begin to find the start of the line
    let line_start = source[..begin]
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|p| p + 1)
        .unwrap_or(0);

    // Check if there's non-whitespace before the comment on this line
    source[line_start..begin]
        .iter()
        .any(|&b| !b.is_ascii_whitespace())
}

fn apply_targets_to_line(
    targets: &[Target],
    line: usize,
    all_disabled_lines: &mut HashSet<usize>,
    rule_disabled_lines: &mut HashSet<(usize, OffenseKind)>,
) {
    for target in targets {
        match target {
            Target::All => {
                all_disabled_lines.insert(line);
            }
            Target::Rule(kind) => {
                rule_disabled_lines.insert((line, *kind));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_build(source: &str) -> DisabledSet {
        let bytes = source.as_bytes();
        let result = ruby_prism::parse(bytes);
        let newline_positions = crate::ast_helpers::compute_newline_positions(bytes);
        build_disabled_set(&result, bytes, &newline_positions)
    }

    #[test]
    fn trailing_disable_same_line() {
        let source = "x = [].shuffle.first # rubyfast:disable shuffle_first_vs_sample\ny = 1\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(1, OffenseKind::ShuffleFirstVsSample));
        assert!(!set.is_disabled(2, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn disable_next_line() {
        let source = "# rubyfast:disable-next-line shuffle_first_vs_sample\nx = [].shuffle.first\ny = [].shuffle.first\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(2, OffenseKind::ShuffleFirstVsSample));
        assert!(!set.is_disabled(3, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn block_disable_enable() {
        let source = "x = 1\n# rubyfast:disable for_loop_vs_each\nfor i in [1]; end\n# rubyfast:enable for_loop_vs_each\nfor j in [2]; end\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(3, OffenseKind::ForLoopVsEach));
        assert!(!set.is_disabled(5, OffenseKind::ForLoopVsEach));
    }

    #[test]
    fn disable_all() {
        let source = "x = 1 # rubyfast:disable all\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(1, OffenseKind::ShuffleFirstVsSample));
        assert!(set.is_disabled(1, OffenseKind::ForLoopVsEach));
    }

    #[test]
    fn multiple_rules() {
        let source = "x = 1 # rubyfast:disable shuffle_first_vs_sample, for_loop_vs_each\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(1, OffenseKind::ShuffleFirstVsSample));
        assert!(set.is_disabled(1, OffenseKind::ForLoopVsEach));
        assert!(!set.is_disabled(1, OffenseKind::GsubVsTr));
    }

    #[test]
    fn fasterer_compat() {
        let source = "x = 1 # fasterer:disable shuffle_first_vs_sample\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(1, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn unclosed_block_disable_extends_to_eof() {
        let source = "# rubyfast:disable for_loop_vs_each\nfor i in [1]; end\nfor j in [2]; end\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(2, OffenseKind::ForLoopVsEach));
        assert!(set.is_disabled(3, OffenseKind::ForLoopVsEach));
    }

    #[test]
    fn unknown_rule_ignored() {
        let source = "x = 1 # rubyfast:disable nonexistent_rule\n";
        let set = parse_and_build(source);
        assert!(!set.is_disabled(1, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn disable_next_line_all() {
        let source = "# rubyfast:disable-next-line all\nx = [].shuffle.first\ny = 1\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(2, OffenseKind::ShuffleFirstVsSample));
        assert!(set.is_disabled(2, OffenseKind::ForLoopVsEach));
        assert!(!set.is_disabled(3, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn block_disable_all_and_enable_all() {
        let source = "# rubyfast:disable all\nx = 1\ny = 2\n# rubyfast:enable all\nz = 3\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(2, OffenseKind::ShuffleFirstVsSample));
        assert!(set.is_disabled(3, OffenseKind::ForLoopVsEach));
        assert!(!set.is_disabled(5, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn multiple_rules_in_block_disable() {
        let source = "# rubyfast:disable shuffle_first_vs_sample, for_loop_vs_each\nx = 1\n# rubyfast:enable shuffle_first_vs_sample, for_loop_vs_each\ny = 2\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(2, OffenseKind::ShuffleFirstVsSample));
        assert!(set.is_disabled(2, OffenseKind::ForLoopVsEach));
        assert!(!set.is_disabled(4, OffenseKind::ShuffleFirstVsSample));
        assert!(!set.is_disabled(4, OffenseKind::ForLoopVsEach));
    }

    #[test]
    fn unclosed_block_disable_all_extends_to_eof() {
        let source = "# rubyfast:disable all\nx = 1\ny = 2\n";
        let set = parse_and_build(source);
        assert!(set.is_disabled(2, OffenseKind::ShuffleFirstVsSample));
        assert!(set.is_disabled(3, OffenseKind::GsubVsTr));
    }

    #[test]
    fn empty_disable_directive_ignored() {
        let source = "x = 1 # rubyfast:disable\n";
        let set = parse_and_build(source);
        assert!(!set.is_disabled(1, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn empty_disable_next_line_directive_ignored() {
        let source = "# rubyfast:disable-next-line\nx = [].shuffle.first\n";
        let set = parse_and_build(source);
        assert!(!set.is_disabled(2, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn empty_enable_directive_ignored() {
        let source = "# rubyfast:enable\n";
        let set = parse_and_build(source);
        assert!(!set.is_disabled(1, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn is_trailing_at_start_of_file() {
        let source = b"# comment\nx = 1\n";
        assert!(!is_trailing_comment(source, 0));
    }

    #[test]
    fn unrecognized_directive_action_ignored() {
        let source = "x = 1 # rubyfast:freeze all\n";
        let set = parse_and_build(source);
        assert!(!set.is_disabled(1, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn enable_without_matching_disable_is_noop() {
        let source = "# rubyfast:enable for_loop_vs_each\nfor x in [1]; end\n";
        let set = parse_and_build(source);
        assert!(!set.is_disabled(2, OffenseKind::ForLoopVsEach));
    }
}
