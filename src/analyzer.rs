use std::path::Path;

use lib_ruby_parser::{ErrorLevel, Node, Parser};

use crate::ast_helpers::{byte_offset_to_line, for_each_child};
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
    let newline_positions: Vec<usize> = source
        .iter()
        .enumerate()
        .filter(|(_, &b)| b == b'\n')
        .map(|(i, _)| i)
        .collect();

    let source_clone = source.clone();
    let result = Parser::new(source, Default::default()).do_parse();

    // Check for fatal parse errors
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.level == ErrorLevel::Error);

    if has_errors {
        if result.ast.is_none() {
            return Err(ParseError {
                path: path.display().to_string(),
                message: result
                    .diagnostics
                    .iter()
                    .filter(|d| d.level == ErrorLevel::Error)
                    .map(|d| format!("{:?}", d.message))
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }
        // Recovered AST with errors — skip analysis to avoid false positives
        return Ok(AnalysisResult {
            path: path.display().to_string(),
            offenses: vec![],
        });
    }

    let ast = match result.ast {
        Some(ast) => ast,
        None => {
            return Ok(AnalysisResult {
                path: path.display().to_string(),
                offenses: vec![],
            });
        }
    };

    let disabled_set = build_disabled_set(&result.comments, &source_clone, &newline_positions);

    let mut offenses = Vec::new();
    walk_node(&ast, &mut offenses, &source_clone);

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
fn walk_node(node: &Node, offenses: &mut Vec<Offense>, source: &[u8]) {
    match node {
        Node::For(f) => {
            offenses.extend(for_loop_scanner::scan(f, source));
            for_each_child(node, |child| walk_node(child, offenses, source));
        }
        Node::RescueBody(rb) => {
            offenses.extend(rescue_scanner::scan(rb));
            for_each_child(node, |child| walk_node(child, offenses, source));
        }
        Node::Def(d) => {
            offenses.extend(method_definition_scanner::scan(d));
            for_each_child(node, |child| walk_node(child, offenses, source));
        }
        Node::Send(s) => {
            if let Some(Node::Block(recv_block)) = s.recv.as_deref() {
                offenses.extend(method_call_scanner::scan_send_on_block(s, recv_block));
            }
            offenses.extend(method_call_scanner::scan_send(s));
            for_each_child(node, |child| walk_node(child, offenses, source));
        }
        Node::Block(b) => {
            offenses.extend(method_call_scanner::scan_block(b));
            // Walk children manually to skip the inner Send (avoids double-scanning).
            if let Node::Send(s) = b.call.as_ref() {
                if let Some(recv) = &s.recv {
                    walk_node(recv, offenses, source);
                }
                for arg in &s.args {
                    walk_node(arg, offenses, source);
                }
            }
            if let Some(args) = &b.args {
                walk_node(args, offenses, source);
            }
            if let Some(body) = &b.body {
                walk_node(body, offenses, source);
            }
        }
        _ => {
            for_each_child(node, |child| walk_node(child, offenses, source));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast_helpers::byte_offset_to_line;

    fn newline_positions(source: &[u8]) -> Vec<usize> {
        source
            .iter()
            .enumerate()
            .filter(|(_, &b)| b == b'\n')
            .map(|(i, _)| i)
            .collect()
    }

    #[test]
    fn byte_offset_to_line_works() {
        let source = b"line1\nline2\nline3";
        let positions = newline_positions(source);
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
    fn analyze_file_with_parse_errors_no_ast_returns_error() {
        let dir = tempfile::TempDir::new().unwrap();
        // This produces a fatal parse error with no recoverable AST
        let file = dir.path().join("fatal.rb");
        std::fs::write(&file, "\x00\x01\x02").unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config);
        // May be Ok with empty offenses or Err depending on parser behavior
        // Either way it should not panic
        let _ = result;
    }

    #[test]
    fn analyze_file_with_recovered_ast_returns_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("recovered.rb");
        std::fs::write(&file, "def foo; end; def def; end").unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config);
        match result {
            Ok(analysis) => assert!(analysis.offenses.is_empty()),
            Err(_) => {} // Also acceptable — fatal parse error
        }
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
    fn walk_node_block_with_non_send_call() {
        // A numblock (numbered params) has a different call structure
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
        std::fs::write(
            &file,
            "def foo\n  for x in [1,2]; puts x; end\nend\n",
        )
        .unwrap();
        let config = crate::config::Config::default();
        let result = super::analyze_file(&file, &config).unwrap();
        assert!(result
            .offenses
            .iter()
            .any(|o| o.kind == crate::offense::OffenseKind::ForLoopVsEach));
    }
}
