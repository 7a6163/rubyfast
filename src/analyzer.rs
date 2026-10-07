use std::path::Path;

use ruby_prism::{Node, Visit};

use crate::ast_helpers::{byte_offset_to_line, compute_newline_positions};
use crate::comment_directives::build_disabled_set;
use crate::config::Config;
use crate::offense::Offense;
use crate::scanner::{
    for_loop_scanner, method_call_scanner, method_definition_scanner, rescue_scanner,
};

/// Result of analyzing a single file.
#[derive(Debug)]
pub struct AnalysisResult {
    pub path: String,
    pub offenses: Vec<Offense>,
}

/// Result of a failed parse.
#[derive(Debug)]
pub struct ParseError {
    pub path: String,
    pub message: String,
}

/// Analyze a single Ruby file, returning detected offenses.
pub fn analyze_file(path: &Path, config: &Config) -> Result<AnalysisResult, ParseError> {
    let source = std::fs::read(path).map_err(|e| ParseError {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    // Pre-compute newline positions before handing source to the parser
    let newline_positions = compute_newline_positions(&source);

    let result = ruby_prism::parse(&source);

    // Check for parse errors
    let has_errors = result.errors().next().is_some();

    if has_errors {
        // Prism always produces an AST, but if there are errors, skip analysis
        // to avoid false positives (matching lib-ruby-parser behavior).
        return Ok(AnalysisResult {
            path: path.display().to_string(),
            offenses: vec![],
        });
    }

    let root = result.node();

    let disabled_set = build_disabled_set(&result, &source, &newline_positions);

    let offenses = scan_tree(&root, &source, has_frozen_string_literals(&result));

    // Resolve byte offsets to line numbers, then filter by config and inline directives
    let offenses = offenses
        .into_iter()
        .filter(|o| config.is_enabled(o.kind))
        .map(|o| {
            let line = byte_offset_to_line(&newline_positions, o.line);
            Offense {
                kind: o.kind,
                line,
                fix: o.fix,
            }
        })
        .filter(|o| !disabled_set.is_disabled(o.line, o.kind))
        .collect();

    Ok(AnalysisResult {
        path: path.display().to_string(),
        offenses,
    })
}

/// Per-file facts the walk needs beyond the node itself.
#[derive(Clone, Copy)]
struct FileContext<'a> {
    source: &'a [u8],
    /// The file carries `# frozen_string_literal: true`.
    frozen_string_literals: bool,
}

/// Whether the parsed file enables frozen string literals.
fn has_frozen_string_literals(result: &ruby_prism::ParseResult<'_>) -> bool {
    result
        .magic_comments()
        .any(|c| c.key() == b"frozen_string_literal" && c.value() == b"true")
}

/// Run every scanner over the tree under `root`, returning offenses with byte offsets.
pub(crate) fn scan_tree(
    root: &Node<'_>,
    source: &[u8],
    frozen_string_literals: bool,
) -> Vec<Offense> {
    let mut walker = Walker {
        offenses: Vec::new(),
        ctx: FileContext {
            source,
            frozen_string_literals,
        },
    };
    walker.visit(root);
    walker.offenses
}

/// Dispatches rule-carrying nodes to the scanners; prism's default visitors handle descent,
/// so every node kind is reached.
struct Walker<'a> {
    offenses: Vec<Offense>,
    ctx: FileContext<'a>,
}

impl<'pr> Visit<'pr> for Walker<'_> {
    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        self.offenses
            .extend(for_loop_scanner::scan(node, self.ctx.source));
        ruby_prism::visit_for_node(self, node);
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.offenses.extend(method_definition_scanner::scan(node));
        ruby_prism::visit_def_node(self, node);
    }

    // Also reached for each `subsequent` clause, which prism visits via this method.
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        self.offenses.extend(rescue_scanner::scan(node));
        ruby_prism::visit_rescue_node(self, node);
    }

    fn visit_call_node(&mut self, call: &ruby_prism::CallNode<'pr>) {
        // Chained on a block call (`.select{}.first`): prism hangs the block off the receiver.
        if let Some(recv) = call.receiver()
            && let Some(recv_call) = recv.as_call_node()
            && let Some(Node::BlockNode { .. }) = recv_call.block()
        {
            self.offenses
                .extend(method_call_scanner::scan_call_on_block_call(
                    call, &recv_call,
                ));
        }

        match call.block().and_then(|b| b.as_block_node()) {
            Some(block) => self
                .offenses
                .extend(method_call_scanner::scan_call_with_block(call, &block)),
            None => self.offenses.extend(method_call_scanner::scan_call(
                call,
                self.ctx.frozen_string_literals,
            )),
        }
        ruby_prism::visit_call_node(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::has_frozen_string_literals;
    use crate::ast_helpers::{byte_offset_to_line, compute_newline_positions};

    #[test]
    fn detects_the_frozen_string_literal_magic_comment() {
        let cases: &[(&[u8], bool)] = &[
            (b"# frozen_string_literal: true\nx = 1", true),
            (b"# frozen_string_literal: false\nx = 1", false),
            (b"# encoding: utf-8\nx = 1", false),
            (b"x = 1", false),
        ];
        for (source, expected) in cases {
            let result = ruby_prism::parse(source);
            assert_eq!(
                (source, has_frozen_string_literals(&result)),
                (source, *expected)
            );
        }
    }

    #[test]
    fn byte_offset_to_line_works() {
        let source = b"line1\nline2\nline3";
        let positions = compute_newline_positions(source);
        assert_eq!(byte_offset_to_line(&positions, 0), 1);
        assert_eq!(byte_offset_to_line(&positions, 5), 1);
        assert_eq!(byte_offset_to_line(&positions, 6), 2);
        assert_eq!(byte_offset_to_line(&positions, 12), 3);
    }

    #[test]
    fn walk_reaches_every_node_kind() {
        use crate::offense::OffenseKind;
        // Each of these used to be skipped by the hand-written traversal.
        let cases = [
            "x = arr.shuffle.first rescue nil",
            "self.foo ||= arr.shuffle.first",
            "self.foo &&= arr.shuffle.first",
            "self.foo += arr.shuffle.first",
            "END { arr.shuffle.first }",
            "BEGIN { arr.shuffle.first }",
            "def foo(x = arr.shuffle.first); end",
            "def foo(x: arr.shuffle.first); end",
            "def f; super do |x| x.shuffle.first end; end",
            "/(?<m>a)/ =~ arr.shuffle.first",
            "case x\nin Integer if arr.shuffle.first\nend",
        ];
        for src in cases {
            let result = ruby_prism::parse(src.as_bytes());
            let kinds: Vec<_> = super::scan_tree(&result.node(), src.as_bytes(), false)
                .into_iter()
                .map(|o| o.kind)
                .collect();
            assert_eq!(kinds, [OffenseKind::ShuffleFirstVsSample], "{src}");
        }
    }

    #[test]
    fn walk_scans_every_rescue_clause() {
        let source = b"begin; rescue ArgumentError; rescue NoMethodError; end";
        let result = ruby_prism::parse(source);
        assert_eq!(super::scan_tree(&result.node(), source, false).len(), 1);
    }

    #[test]
    fn analyze_nonexistent_file_returns_error() {
        let config = crate::config::Config::default();
        let result = super::analyze_file(std::path::Path::new("/nonexistent.rb"), &config);
        assert!(result.is_err());
    }

    #[test]
    fn analyze_file_with_parse_errors_returns_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("fatal.rb");
        std::fs::write(&file, "def def def").unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(result.offenses.is_empty());
    }

    #[test]
    fn analyze_empty_file_returns_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("empty.rb");
        std::fs::write(&file, "").unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(result.offenses.is_empty());
    }

    #[test]
    fn analyze_file_with_config_disabling_rule() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "for x in [1]; end").unwrap();
        let config =
            crate::config::Config::parse_yaml("speedups:\n  for_loop_vs_each: false\n").unwrap();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(result.offenses.is_empty());
    }

    #[test]
    fn analyze_file_with_inline_disable() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(
            &file,
            "for x in [1]; end # rubyfast:disable for_loop_vs_each\n",
        )
        .unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(result.offenses.is_empty());
    }

    #[test]
    fn walk_node_block_with_symbol_to_proc() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "arr.map { |x| x.to_s }").unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        // Should find block_vs_symbol_to_proc
        assert!(!result.offenses.is_empty());
    }

    #[test]
    fn walk_node_call_on_block_call_chain() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "arr.select { |x| x > 1 }.first").unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(
            result
                .offenses
                .iter()
                .any(|o| o.kind == crate::offense::OffenseKind::SelectFirstVsDetect)
        );
    }

    #[test]
    fn walk_node_rescue_clause() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(
            &file,
            "begin\n  foo\nrescue NoMethodError\n  bar\nrescue => e\n  baz\nend\n",
        )
        .unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(
            result
                .offenses
                .iter()
                .any(|o| o.kind == crate::offense::OffenseKind::RescueVsRespondTo)
        );
    }

    #[test]
    fn walk_node_begin_else_ensure() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(
            &file,
            "begin\n  for x in [1]; end\nrescue\n  1\nelse\n  for y in [2]; end\nensure\n  for z in [3]; end\nend\n",
        )
        .unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        // Should find for_loop offenses in the body, else, and ensure clauses
        let for_count = result
            .offenses
            .iter()
            .filter(|o| o.kind == crate::offense::OffenseKind::ForLoopVsEach)
            .count();
        assert!(
            for_count >= 2,
            "Expected at least 2 for_loop offenses in begin/else/ensure, got {}",
            for_count
        );
    }

    #[test]
    fn walk_node_call_without_block() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "h.fetch(:key, [])").unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(
            result
                .offenses
                .iter()
                .any(|o| o.kind == crate::offense::OffenseKind::FetchWithArgumentVsBlock)
        );
    }

    #[test]
    fn walk_node_call_with_block_argument() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "arr.select(&:odd?).first").unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(
            result
                .offenses
                .iter()
                .any(|o| o.kind == crate::offense::OffenseKind::SelectFirstVsDetect)
        );
    }

    #[test]
    fn walk_node_standalone_rescue_node() {
        // Test inline rescue which creates RescueNode not inside BeginNode
        // `def foo; bar rescue NoMethodError; end` should hit the RescueNode branch
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "def foo\n  bar rescue NoMethodError\nend\n").unwrap();
        let config = crate::config::Config::default();
        let _result = super::analyze_file(&file, &config).unwrap();
        // Just ensuring the code path doesn't panic
    }

    #[test]
    fn walk_node_deeply_nested() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(
            &file,
            "class Foo\n  module Bar\n    def baz\n      if true\n        for x in [1]; end\n      end\n    end\n  end\nend\n",
        )
        .unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(
            result
                .offenses
                .iter()
                .any(|o| o.kind == crate::offense::OffenseKind::ForLoopVsEach)
        );
    }

    #[test]
    fn walk_node_nested_for_inside_method() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("test.rb");
        std::fs::write(&file, "def foo\n  for x in [1,2]; puts x; end\nend\n").unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(
            result
                .offenses
                .iter()
                .any(|o| o.kind == crate::offense::OffenseKind::ForLoopVsEach)
        );
    }
}
