use std::path::Path;

use ruby_prism::Node;

use crate::ast_helpers::{byte_offset_to_line, compute_newline_positions};
use crate::ast_visitor::for_each_direct_child;
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

    let mut offenses = Vec::new();
    walk_node(&root, &mut offenses, &source);

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

/// Recursively walk the AST, dispatching to scanners.
fn walk_node(node: &Node<'_>, offenses: &mut Vec<Offense>, source: &[u8]) {
    match node {
        Node::ProgramNode { .. } => {
            let prog = node.as_program_node().unwrap();
            for child in prog.statements().body().iter() {
                walk_node(&child, offenses, source);
            }
        }
        Node::ForNode { .. } => {
            let f = node.as_for_node().unwrap();
            offenses.extend(for_loop_scanner::scan(&f, source));
            for_each_direct_child(node, &mut |child| walk_node(child, offenses, source));
        }
        Node::BeginNode { .. } => {
            let begin = node.as_begin_node().unwrap();
            // Visit statements
            if let Some(stmts) = begin.statements() {
                for child in stmts.body().iter() {
                    walk_node(&child, offenses, source);
                }
            }
            // Visit rescue clauses
            if let Some(rescue) = begin.rescue_clause() {
                walk_rescue_node(&rescue, offenses, source);
            }
            // Visit else clause
            if let Some(else_clause) = begin.else_clause()
                && let Some(stmts) = else_clause.statements()
            {
                for child in stmts.body().iter() {
                    walk_node(&child, offenses, source);
                }
            }
            // Visit ensure clause
            if let Some(ensure) = begin.ensure_clause()
                && let Some(stmts) = ensure.statements()
            {
                for child in stmts.body().iter() {
                    walk_node(&child, offenses, source);
                }
            }
        }
        Node::RescueNode { .. } => {
            let rn = node.as_rescue_node().unwrap();
            walk_rescue_node(&rn, offenses, source);
        }
        Node::DefNode { .. } => {
            let d = node.as_def_node().unwrap();
            offenses.extend(method_definition_scanner::scan(&d));
            // Walk the body
            if let Some(body) = d.body() {
                walk_node(&body, offenses, source);
            }
        }
        Node::CallNode { .. } => {
            let call = node.as_call_node().unwrap();

            // Check if receiver is a CallNode with a BlockNode (chained: .select{}.first)
            if let Some(recv) = call.receiver()
                && let Some(recv_call) = recv.as_call_node()
                && let Some(Node::BlockNode { .. }) = recv_call.block()
            {
                offenses.extend(method_call_scanner::scan_call_on_block_call(
                    &call, &recv_call,
                ));
            }

            // Check if this call has a block (CallNode owns BlockNode in prism)
            match call.block() {
                Some(Node::BlockNode { .. }) => {
                    let block = call.block().unwrap().as_block_node().unwrap();
                    offenses.extend(method_call_scanner::scan_call_with_block(&call, &block));

                    // Walk receiver and arguments (skip the block's call — we already scanned it)
                    if let Some(recv) = call.receiver() {
                        walk_node(&recv, offenses, source);
                    }
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            walk_node(&arg, offenses, source);
                        }
                    }
                    // Walk block body
                    if let Some(body) = block.body() {
                        walk_node(&body, offenses, source);
                    }
                }
                _ => {
                    // No block or block argument — scan as plain send
                    offenses.extend(method_call_scanner::scan_call(&call));
                    // Walk all children
                    if let Some(recv) = call.receiver() {
                        walk_node(&recv, offenses, source);
                    }
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            walk_node(&arg, offenses, source);
                        }
                    }
                    if let Some(block) = call.block() {
                        walk_node(&block, offenses, source);
                    }
                }
            }
        }
        _ => {
            for_each_direct_child(node, &mut |child| walk_node(child, offenses, source));
        }
    }
}

/// Walk a RescueNode and its chain of subsequent rescue clauses.
fn walk_rescue_node(
    rescue: &ruby_prism::RescueNode<'_>,
    offenses: &mut Vec<Offense>,
    source: &[u8],
) {
    offenses.extend(rescue_scanner::scan(rescue));

    // Walk exception list
    for exc in rescue.exceptions().iter() {
        walk_node(&exc, offenses, source);
    }
    // Walk reference
    if let Some(reference) = rescue.reference() {
        walk_node(&reference, offenses, source);
    }
    // Walk statements
    if let Some(stmts) = rescue.statements() {
        for child in stmts.body().iter() {
            walk_node(&child, offenses, source);
        }
    }
    // Walk subsequent rescue clauses
    if let Some(subsequent) = rescue.subsequent() {
        walk_rescue_node(&subsequent, offenses, source);
    }
}

#[cfg(test)]
mod tests {
    use crate::ast_helpers::{byte_offset_to_line, compute_newline_positions};

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
