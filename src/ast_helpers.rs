use ruby_prism::Node;

/// Convert a byte offset to a 1-based line number using pre-computed newline positions.
pub fn byte_offset_to_line(newline_positions: &[usize], byte_offset: usize) -> usize {
    match newline_positions.binary_search(&byte_offset) {
        Ok(idx) => idx + 1,
        Err(idx) => idx + 1,
    }
}

/// Check if a node is a CallNode with the given method name.
pub fn receiver_is_call_with_name(recv: &Option<Node<'_>>, name: &[u8]) -> bool {
    match recv {
        Some(node) => {
            if let Some(call) = node.as_call_node() {
                call.name().as_slice() == name
            } else {
                false
            }
        }
        None => false,
    }
}

/// Extract the inner CallNode from a receiver, if it is one.
pub fn receiver_as_call<'pr>(recv: &'pr Option<Node<'pr>>) -> Option<ruby_prism::CallNode<'pr>> {
    match recv {
        Some(node) => node.as_call_node(),
        None => None,
    }
}

/// Check if a CallNode has a BlockArgumentNode in its block field.
pub fn has_block_pass(call: &ruby_prism::CallNode<'_>) -> bool {
    matches!(call.block(), Some(Node::BlockArgumentNode { .. }))
}

/// Check if a CallNode has a full BlockNode (not just a BlockArgumentNode).
pub fn has_full_block(call: &ruby_prism::CallNode<'_>) -> bool {
    matches!(call.block(), Some(Node::BlockNode { .. }))
}

/// Count arguments from a CallNode's arguments (excluding block argument which is in block field).
pub fn arg_count(call: &ruby_prism::CallNode<'_>) -> usize {
    match call.arguments() {
        Some(args) => args.arguments().iter().count(),
        None => 0,
    }
}

/// Get the first argument from a CallNode (without allocating a Vec).
pub fn first_call_arg<'pr>(call: &ruby_prism::CallNode<'pr>) -> Option<Node<'pr>> {
    call.arguments()
        .and_then(|args| args.arguments().iter().next())
}

/// Get the first two arguments from a CallNode as a tuple (without collecting all).
pub fn call_args_pair<'pr>(call: &ruby_prism::CallNode<'pr>) -> Option<(Node<'pr>, Node<'pr>)> {
    let args = call.arguments()?;
    let mut iter = args.arguments().iter();
    let first = iter.next()?;
    let second = iter.next()?;
    if iter.next().is_some() {
        return None; // more than 2 args
    }
    Some((first, second))
}

/// Check if a node is a single-character string literal.
pub fn is_single_char_string(node: &Node<'_>) -> bool {
    match node.as_string_node() {
        Some(s) => s.unescaped().len() == 1,
        None => false,
    }
}

/// Check if the receiver is a range (RangeNode, inclusive or exclusive).
/// Also handles parenthesized ranges: `(1..10)` parses as `ParenthesesNode(RangeNode)`.
pub fn receiver_is_range(recv: &Option<Node<'_>>) -> bool {
    let Some(node) = recv else { return false };
    if node.as_range_node().is_some() {
        return true;
    }
    let Some(paren) = node.as_parentheses_node() else {
        return false;
    };
    let Some(body) = paren.body() else {
        return false;
    };
    body_single_expression(Some(body)).is_some_and(|expr| expr.as_range_node().is_some())
}

/// Check if a node is a literal/primitive (not a variable reference or method call).
pub fn is_primitive(node: &Node<'_>) -> bool {
    matches!(
        node,
        Node::IntegerNode { .. }
            | Node::FloatNode { .. }
            | Node::StringNode { .. }
            | Node::SymbolNode { .. }
            | Node::TrueNode { .. }
            | Node::FalseNode { .. }
            | Node::NilNode { .. }
            | Node::ArrayNode { .. }
            | Node::HashNode { .. }
            | Node::RangeNode { .. }
            | Node::RationalNode { .. }
            | Node::ImaginaryNode { .. }
    )
}

/// Check if the first argument is a Hash/KeywordHash node with exactly one key-value pair.
/// `h.merge!(item: 1)` parses as KeywordHashNode, `h.merge!({item: 1})` parses as HashNode.
/// Check if the first argument of a CallNode is a Hash/KeywordHash with exactly one pair.
pub fn first_arg_is_single_pair_hash(call: &ruby_prism::CallNode<'_>) -> bool {
    match first_call_arg(call) {
        Some(node) => {
            if let Some(h) = node.as_hash_node() {
                return h.elements().iter().count() == 1;
            }
            if let Some(k) = node.as_keyword_hash_node() {
                return k.elements().iter().count() == 1;
            }
            false
        }
        None => false,
    }
}

/// Check if a node is an IntegerNode with value 1.
pub fn is_int_one(node: &Node<'_>) -> bool {
    if let Some(i) = node.as_integer_node() {
        let text = i.location().as_slice();
        matches!(
            text,
            b"1" | b"0x1" | b"0X1" | b"0b1" | b"0B1" | b"0o1" | b"0O1"
        )
    } else {
        false
    }
}

/// Get block argument names from a BlockNode's parameters.
/// BlockNode.parameters() returns Option<Node> which is typically a BlockParametersNode.
pub fn block_arg_names(params: &Option<Node<'_>>) -> Vec<String> {
    match params {
        Some(node) => {
            if let Some(block_params) = node.as_block_parameters_node()
                && let Some(inner_params) = block_params.parameters()
            {
                return inner_params
                    .requireds()
                    .iter()
                    .filter_map(|p| {
                        p.as_required_parameter_node()
                            .map(|rp| String::from_utf8_lossy(rp.name().as_slice()).to_string())
                    })
                    .collect();
            }
            // Handle NumberedParametersNode or other cases
            Vec::new()
        }
        None => Vec::new(),
    }
}

/// Check if a DefNode has a block argument (&block), returning its name if so.
pub fn def_block_arg_name(def: &ruby_prism::DefNode<'_>) -> Option<String> {
    let params = def.parameters()?;
    let block_param = params.block()?;
    let name = block_param.name()?;
    Some(String::from_utf8_lossy(name.as_slice()).to_string())
}

/// Count regular (required) arguments in a DefNode.
pub fn def_regular_arg_count(def: &ruby_prism::DefNode<'_>) -> usize {
    match def.parameters() {
        Some(params) => params.requireds().iter().count(),
        None => 0,
    }
}

/// Get the first regular argument name from a DefNode.
pub fn def_first_arg_name(def: &ruby_prism::DefNode<'_>) -> Option<String> {
    let params = def.parameters()?;
    let first = params.requireds().iter().next()?;
    first
        .as_required_parameter_node()
        .map(|rp| String::from_utf8_lossy(rp.name().as_slice()).to_string())
}

/// Check if a string literal contains "def".
pub fn str_contains_def(node: &Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        return String::from_utf8_lossy(s.unescaped()).contains("def");
    }
    if let Some(interp) = node.as_interpolated_string_node() {
        return interp.parts().iter().any(|part| {
            if let Some(s) = part.as_string_node() {
                String::from_utf8_lossy(s.unescaped()).contains("def")
            } else {
                false
            }
        });
    }
    false
}

/// Count the number of top-level expressions in a body node.
pub fn body_expression_count(body: &Option<Node<'_>>) -> usize {
    match body {
        None => 0,
        Some(node) => {
            if let Some(stmts) = node.as_statements_node() {
                stmts.body().iter().count()
            } else {
                1
            }
        }
    }
}

/// Get the single expression from a body node, if the body has exactly one expression.
/// For StatementsNode bodies, returns the first (and only) statement.
/// For single-expression bodies (non-StatementsNode), returns the body node itself.
/// Returns None if body is empty or has multiple expressions.
pub fn body_single_expression(body: Option<Node<'_>>) -> Option<Node<'_>> {
    let node = body?;
    if let Some(stmts) = node.as_statements_node() {
        let mut iter = stmts.body().iter();
        let first = iter.next()?;
        if iter.next().is_some() {
            return None; // multiple expressions
        }
        Some(first)
    } else {
        // Single expression body — the node itself is the expression
        Some(node)
    }
}

/// Test-only helpers for parsing Ruby source with leaked lifetime.
/// `Box::leak` is intentional: test processes reclaim all memory at exit.
#[cfg(test)]
pub mod test_helpers {
    use ruby_prism::{Node, ParseResult};

    /// Parse source and leak the result to get a `'static` lifetime for tests.
    pub fn leak_parse(source: &[u8]) -> &'static ParseResult<'static> {
        let owned: Vec<u8> = source.to_vec();
        let static_source: &'static [u8] = Box::leak(owned.into_boxed_slice());
        Box::leak(Box::new(ruby_prism::parse(static_source)))
    }

    /// Parse source and return the first top-level statement node.
    pub fn parse_first_stmt(source: &[u8]) -> Node<'static> {
        let result = leak_parse(source);
        let program = result.node();
        let prog = program.as_program_node().unwrap();
        prog.statements().body().iter().next().unwrap()
    }
}

/// Compute byte positions of all newline characters in source.
pub fn compute_newline_positions(source: &[u8]) -> Vec<usize> {
    source
        .iter()
        .enumerate()
        .filter(|&(_, &b)| b == b'\n')
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast_helpers::test_helpers::parse_first_stmt;

    #[test]
    fn byte_offset_to_line_basic() {
        let positions = vec![5, 11];
        assert_eq!(byte_offset_to_line(&positions, 0), 1);
        assert_eq!(byte_offset_to_line(&positions, 5), 1); // exact match
        assert_eq!(byte_offset_to_line(&positions, 6), 2);
        assert_eq!(byte_offset_to_line(&positions, 12), 3);
    }

    #[test]
    fn byte_offset_to_line_empty() {
        assert_eq!(byte_offset_to_line(&[], 0), 1);
        assert_eq!(byte_offset_to_line(&[], 100), 1);
    }

    #[test]
    fn receiver_is_call_with_name_works() {
        let node = parse_first_stmt(b"a.foo.bar");
        let call = node.as_call_node().unwrap();
        assert!(receiver_is_call_with_name(&call.receiver(), b"foo"));
        assert!(!receiver_is_call_with_name(&call.receiver(), b"baz"));
    }

    #[test]
    fn receiver_is_call_with_name_none() {
        assert!(!receiver_is_call_with_name(&None, b"foo"));
    }

    #[test]
    fn receiver_as_call_works() {
        let node = parse_first_stmt(b"a.foo.bar");
        let call = node.as_call_node().unwrap();
        let recv = call.receiver();
        let inner = receiver_as_call(&recv).unwrap();
        assert_eq!(inner.name().as_slice(), b"foo");
    }

    #[test]
    fn receiver_as_call_not_call() {
        assert!(receiver_as_call(&None).is_none());
    }

    #[test]
    fn has_block_pass_works() {
        let node = parse_first_stmt(b"arr.map(&:to_s)");
        let call = node.as_call_node().unwrap();
        assert!(has_block_pass(&call));
    }

    #[test]
    fn has_block_pass_without() {
        let node = parse_first_stmt(b"arr.map(1)");
        let call = node.as_call_node().unwrap();
        assert!(!has_block_pass(&call));
    }

    #[test]
    fn arg_count_works() {
        let node = parse_first_stmt(b"arr.select(1, 2)");
        let call = node.as_call_node().unwrap();
        assert_eq!(arg_count(&call), 2);
    }

    #[test]
    fn is_single_char_string_works() {
        let node = parse_first_stmt(b"'x'");
        assert!(is_single_char_string(&node));
        let node2 = parse_first_stmt(b"'xy'");
        assert!(!is_single_char_string(&node2));
    }

    #[test]
    fn is_single_char_string_not_string() {
        let node = parse_first_stmt(b"42");
        assert!(!is_single_char_string(&node));
    }

    #[test]
    fn receiver_is_range_inclusive() {
        let node = parse_first_stmt(b"(1..10).include?(5)");
        let call = node.as_call_node().unwrap();
        assert!(receiver_is_range(&call.receiver()));
    }

    #[test]
    fn receiver_is_range_exclusive() {
        let node = parse_first_stmt(b"(1...10).include?(5)");
        let call = node.as_call_node().unwrap();
        assert!(receiver_is_range(&call.receiver()));
    }

    #[test]
    fn receiver_is_range_not_range() {
        let node = parse_first_stmt(b"[1].include?(5)");
        let call = node.as_call_node().unwrap();
        assert!(!receiver_is_range(&call.receiver()));
    }

    #[test]
    fn is_primitive_covers_types() {
        assert!(is_primitive(&parse_first_stmt(b"42")));
        assert!(is_primitive(&parse_first_stmt(b"3.14")));
        assert!(is_primitive(&parse_first_stmt(b"'s'")));
        assert!(is_primitive(&parse_first_stmt(b":sym")));
        assert!(is_primitive(&parse_first_stmt(b"true")));
        assert!(is_primitive(&parse_first_stmt(b"false")));
        assert!(is_primitive(&parse_first_stmt(b"nil")));
        assert!(is_primitive(&parse_first_stmt(b"[]")));
        assert!(is_primitive(&parse_first_stmt(b"{}")));
        assert!(is_primitive(&parse_first_stmt(b"1..5")));
        assert!(is_primitive(&parse_first_stmt(b"1...5")));
        assert!(!is_primitive(&parse_first_stmt(b"x")));
    }

    #[test]
    fn first_arg_is_single_pair_hash_kwargs() {
        let node = parse_first_stmt(b"h.merge!(a: 1)");
        let call = node.as_call_node().unwrap();
        assert!(first_arg_is_single_pair_hash(&call));
    }

    #[test]
    fn first_arg_is_single_pair_hash_explicit() {
        let node = parse_first_stmt(b"h.merge!({a: 1})");
        let call = node.as_call_node().unwrap();
        assert!(first_arg_is_single_pair_hash(&call));
    }

    #[test]
    fn first_arg_is_single_pair_hash_multi() {
        let node = parse_first_stmt(b"h.merge!(a: 1, b: 2)");
        let call = node.as_call_node().unwrap();
        assert!(!first_arg_is_single_pair_hash(&call));
    }

    #[test]
    fn first_arg_is_single_pair_hash_not_hash() {
        let node = parse_first_stmt(b"h.merge!(x)");
        let call = node.as_call_node().unwrap();
        assert!(!first_arg_is_single_pair_hash(&call));
    }

    #[test]
    fn is_int_one_works() {
        assert!(is_int_one(&parse_first_stmt(b"1")));
        assert!(!is_int_one(&parse_first_stmt(b"2")));
        assert!(!is_int_one(&parse_first_stmt(b"'1'")));
    }

    #[test]
    fn block_arg_names_single() {
        let node = parse_first_stmt(b"arr.map { |x| x }");
        let call = node.as_call_node().unwrap();
        if let Some(Node::BlockNode { .. }) = call.block() {
            let block = call.block().unwrap().as_block_node().unwrap();
            let names = block_arg_names(&block.parameters());
            assert_eq!(names, vec!["x".to_string()]);
        } else {
            panic!("Expected BlockNode");
        }
    }

    #[test]
    fn block_arg_names_none() {
        let names = block_arg_names(&None);
        assert!(names.is_empty());
    }

    #[test]
    fn def_block_arg_name_present() {
        let node = parse_first_stmt(b"def foo(&block); end");
        let def = node.as_def_node().unwrap();
        assert_eq!(def_block_arg_name(&def), Some("block".to_string()));
    }

    #[test]
    fn def_block_arg_name_absent() {
        let node = parse_first_stmt(b"def foo(x); end");
        let def = node.as_def_node().unwrap();
        assert_eq!(def_block_arg_name(&def), None);
    }

    #[test]
    fn def_regular_arg_count_works() {
        let node = parse_first_stmt(b"def foo(a, b); end");
        let def = node.as_def_node().unwrap();
        assert_eq!(def_regular_arg_count(&def), 2);
    }

    #[test]
    fn def_regular_arg_count_no_args() {
        let node = parse_first_stmt(b"def foo; end");
        let def = node.as_def_node().unwrap();
        assert_eq!(def_regular_arg_count(&def), 0);
    }

    #[test]
    fn def_first_arg_name_works() {
        let node = parse_first_stmt(b"def foo(bar); end");
        let def = node.as_def_node().unwrap();
        assert_eq!(def_first_arg_name(&def), Some("bar".to_string()));
    }

    #[test]
    fn def_first_arg_name_no_args() {
        let node = parse_first_stmt(b"def foo; end");
        let def = node.as_def_node().unwrap();
        assert_eq!(def_first_arg_name(&def), None);
    }

    #[test]
    fn str_contains_def_in_string() {
        let node = parse_first_stmt(b"\"def foo\"");
        assert!(str_contains_def(&node));
    }

    #[test]
    fn str_contains_def_no_def() {
        let node = parse_first_stmt(b"\"hello\"");
        assert!(!str_contains_def(&node));
    }

    #[test]
    fn str_contains_def_not_string() {
        let node = parse_first_stmt(b"42");
        assert!(!str_contains_def(&node));
    }

    #[test]
    fn str_contains_def_heredoc() {
        let node = parse_first_stmt(b"<<~RUBY\ndef foo\nRUBY\n");
        assert!(str_contains_def(&node));
    }

    #[test]
    fn body_expression_count_none() {
        assert_eq!(body_expression_count(&None), 0);
    }

    #[test]
    fn body_expression_count_single() {
        let node = parse_first_stmt(b"def foo; 42; end");
        let def = node.as_def_node().unwrap();
        assert_eq!(body_expression_count(&def.body()), 1);
    }

    #[test]
    fn body_expression_count_multiple() {
        let node = parse_first_stmt(b"def foo; 1; 2; 3; end");
        let def = node.as_def_node().unwrap();
        assert_eq!(body_expression_count(&def.body()), 3);
    }

    #[test]
    fn body_single_expression_works() {
        let node = parse_first_stmt(b"def foo; 42; end");
        let def = node.as_def_node().unwrap();
        let single = body_single_expression(def.body());
        assert!(single.is_some());
        assert!(single.unwrap().as_integer_node().is_some());
    }

    #[test]
    fn body_single_expression_none_for_multiple() {
        let node = parse_first_stmt(b"def foo; 1; 2; end");
        let def = node.as_def_node().unwrap();
        assert!(body_single_expression(def.body()).is_none());
    }

    #[test]
    fn body_single_expression_none_for_empty() {
        assert!(body_single_expression(None).is_none());
    }

    #[test]
    fn has_full_block_works() {
        let node = parse_first_stmt(b"arr.map { |x| x }");
        let call = node.as_call_node().unwrap();
        assert!(has_full_block(&call));
    }

    #[test]
    fn has_full_block_without() {
        let node = parse_first_stmt(b"arr.map(&:to_s)");
        let call = node.as_call_node().unwrap();
        assert!(!has_full_block(&call));
    }

    #[test]
    fn first_call_arg_some() {
        let node = parse_first_stmt(b"foo(42)");
        let call = node.as_call_node().unwrap();
        let arg = first_call_arg(&call);
        assert!(arg.is_some());
    }

    #[test]
    fn first_call_arg_none() {
        let node = parse_first_stmt(b"foo()");
        let call = node.as_call_node().unwrap();
        let arg = first_call_arg(&call);
        assert!(arg.is_none());
    }

    #[test]
    fn call_args_pair_exactly_two() {
        let node = parse_first_stmt(b"foo(1, 2)");
        let call = node.as_call_node().unwrap();
        let pair = call_args_pair(&call);
        assert!(pair.is_some());
    }

    #[test]
    fn call_args_pair_one_arg() {
        let node = parse_first_stmt(b"foo(1)");
        let call = node.as_call_node().unwrap();
        assert!(call_args_pair(&call).is_none());
    }

    #[test]
    fn call_args_pair_three_args() {
        let node = parse_first_stmt(b"foo(1, 2, 3)");
        let call = node.as_call_node().unwrap();
        assert!(call_args_pair(&call).is_none());
    }

    #[test]
    fn call_args_pair_no_args() {
        let node = parse_first_stmt(b"foo()");
        let call = node.as_call_node().unwrap();
        assert!(call_args_pair(&call).is_none());
    }

    #[test]
    fn receiver_is_range_none() {
        assert!(!receiver_is_range(&None));
    }

    #[test]
    fn is_primitive_rational() {
        assert!(is_primitive(&parse_first_stmt(b"3r")));
    }

    #[test]
    fn is_primitive_imaginary() {
        assert!(is_primitive(&parse_first_stmt(b"1i")));
    }

    #[test]
    fn first_arg_is_single_pair_hash_no_arg() {
        let node = parse_first_stmt(b"h.merge!()");
        let call = node.as_call_node().unwrap();
        assert!(!first_arg_is_single_pair_hash(&call));
    }

    #[test]
    fn is_int_one_hex() {
        assert!(is_int_one(&parse_first_stmt(b"0x1")));
    }

    #[test]
    fn is_int_one_binary() {
        assert!(is_int_one(&parse_first_stmt(b"0b1")));
    }

    #[test]
    fn is_int_one_octal() {
        assert!(is_int_one(&parse_first_stmt(b"0o1")));
    }

    #[test]
    fn block_arg_names_multiple() {
        let node = parse_first_stmt(b"arr.each_with_object([]) { |x, acc| x }");
        let call = node.as_call_node().unwrap();
        if let Some(ruby_prism::Node::BlockNode { .. }) = call.block() {
            let block = call.block().unwrap().as_block_node().unwrap();
            let names = block_arg_names(&block.parameters());
            assert_eq!(names.len(), 2);
        }
    }

    #[test]
    fn block_arg_names_numbered_params() {
        // Numbered parameters (_1) produce NumberedParametersNode, not BlockParametersNode
        let node = parse_first_stmt(b"arr.map { _1.to_s }");
        let call = node.as_call_node().unwrap();
        if let Some(ruby_prism::Node::BlockNode { .. }) = call.block() {
            let block = call.block().unwrap().as_block_node().unwrap();
            let names = block_arg_names(&block.parameters());
            assert!(names.is_empty());
        }
    }

    #[test]
    fn str_contains_def_in_interpolated_string() {
        let node = parse_first_stmt(b"\"prefix def foo #{x} end\"");
        assert!(str_contains_def(&node));
    }

    #[test]
    fn str_contains_def_interpolated_no_def() {
        let node = parse_first_stmt(b"\"prefix #{x} suffix\"");
        assert!(!str_contains_def(&node));
    }

    #[test]
    fn arg_count_no_args() {
        let node = parse_first_stmt(b"arr.map");
        let call = node.as_call_node().unwrap();
        assert_eq!(arg_count(&call), 0);
    }

    #[test]
    fn receiver_as_call_non_call_recv() {
        let node = parse_first_stmt(b"42.to_s");
        let call = node.as_call_node().unwrap();
        let recv = call.receiver();
        assert!(receiver_as_call(&recv).is_none());
    }

    #[test]
    fn body_expression_count_endless_method() {
        // Endless method: `def foo = 42` — body is a single IntegerNode, not StatementsNode
        let node = parse_first_stmt(b"def foo = 42");
        let def = node.as_def_node().unwrap();
        assert_eq!(body_expression_count(&def.body()), 1);
    }

    #[test]
    fn body_single_expression_endless_method() {
        // Endless method body is not a StatementsNode
        let node = parse_first_stmt(b"def foo = 42");
        let def = node.as_def_node().unwrap();
        let single = body_single_expression(def.body());
        assert!(single.is_some());
        assert!(single.unwrap().as_integer_node().is_some());
    }

    #[test]
    fn compute_newline_positions_works() {
        let source = b"line1\nline2\nline3";
        let positions = compute_newline_positions(source);
        assert_eq!(positions, vec![5, 11]);
    }

    #[test]
    fn compute_newline_positions_empty() {
        assert!(compute_newline_positions(b"").is_empty());
    }

    #[test]
    fn compute_newline_positions_no_newlines() {
        assert!(compute_newline_positions(b"hello").is_empty());
    }

    #[test]
    fn prism_handles_ascii_encoding() {
        let source = b"# encoding: ASCII\nx = 1\n";
        let result = ruby_prism::parse(source);
        assert!(result.errors().next().is_none());
    }

    #[test]
    fn prism_handles_us_ascii_encoding() {
        let source = b"# encoding: us-ascii\nx = 1\n";
        let result = ruby_prism::parse(source);
        assert!(result.errors().next().is_none());
    }
}
