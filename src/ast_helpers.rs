use lib_ruby_parser::Node;
use lib_ruby_parser::nodes::{Block, Def, Send};

/// Convert a byte offset to a 1-based line number using pre-computed newline positions.
pub fn byte_offset_to_line(newline_positions: &[usize], byte_offset: usize) -> usize {
    match newline_positions.binary_search(&byte_offset) {
        Ok(idx) => idx + 1,
        Err(idx) => idx + 1,
    }
}

/// Check if a Send node's receiver is itself a Send with a given method name.
pub fn receiver_is_send_with_name(recv: &Option<Box<Node>>, name: &str) -> bool {
    match recv.as_deref() {
        Some(Node::Send(s)) => s.method_name == name,
        _ => false,
    }
}

/// Extract the inner Send from a receiver, if it is one.
pub fn receiver_as_send(recv: &Option<Box<Node>>) -> Option<&Send> {
    match recv.as_deref() {
        Some(Node::Send(s)) => Some(s),
        _ => None,
    }
}

/// Extract the Send from a Block's call field.
pub fn block_call_as_send(block: &Block) -> Option<&Send> {
    match block.call.as_ref() {
        Node::Send(s) => Some(s),
        _ => None,
    }
}

/// Check if an argument list contains a BlockPass (e.g., `&:foo`).
pub fn has_block_pass(args: &[Node]) -> bool {
    args.iter().any(|a| matches!(a, Node::BlockPass(_)))
}

/// Count non-BlockPass arguments.
pub fn arg_count_without_block_pass(args: &[Node]) -> usize {
    args.iter()
        .filter(|a| !matches!(a, Node::BlockPass(_)))
        .count()
}

/// Check if a node is a single-character string literal.
pub fn is_single_char_string(node: &Node) -> bool {
    match node {
        Node::Str(s) => s.value.to_string_lossy().chars().count() == 1,
        _ => false,
    }
}

/// Check if the receiver is a range (Irange or Erange).
/// Also handles parenthesized ranges: `(1..10)` parses as `Begin(Irange(...))`.
pub fn receiver_is_range(recv: &Option<Box<Node>>) -> bool {
    match recv.as_deref() {
        Some(Node::Irange(_) | Node::Erange(_)) => true,
        Some(Node::Begin(b)) => {
            b.statements.len() == 1 && matches!(b.statements[0], Node::Irange(_) | Node::Erange(_))
        }
        _ => false,
    }
}

/// Check if a node is a literal/primitive (not a variable reference or method call).
pub fn is_primitive(node: &Node) -> bool {
    matches!(
        node,
        Node::Int(_)
            | Node::Float(_)
            | Node::Str(_)
            | Node::Sym(_)
            | Node::True(_)
            | Node::False(_)
            | Node::Nil(_)
            | Node::Array(_)
            | Node::Hash(_)
            | Node::Irange(_)
            | Node::Erange(_)
            | Node::Rational(_)
            | Node::Complex(_)
    )
}

/// Check if the first argument to a Send is a Hash/Kwargs node with exactly one key-value pair.
/// `h.merge!(item: 1)` parses as Kwargs, `h.merge!({item: 1})` parses as Hash.
pub fn first_arg_is_single_pair_hash(args: &[Node]) -> bool {
    match args.first() {
        Some(Node::Hash(h)) => h.pairs.len() == 1,
        Some(Node::Kwargs(k)) => k.pairs.len() == 1,
        _ => false,
    }
}

/// Check if a node is an Int with value 1.
pub fn is_int_one(node: &Node) -> bool {
    match node {
        Node::Int(i) => i.value == "1",
        _ => false,
    }
}

/// Get block argument names from Args node.
pub fn block_arg_names(args: &Option<Box<Node>>) -> Vec<String> {
    match args.as_deref() {
        Some(Node::Args(a)) => a
            .args
            .iter()
            .filter_map(|arg| match arg {
                Node::Arg(a) => Some(a.name.clone()),
                Node::Procarg0(p) => match p.args.as_slice() {
                    [Node::Arg(a)] => Some(a.name.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Check if a Def node has a block argument (&block), returning its name if so.
pub fn def_block_arg_name(def: &Def) -> Option<String> {
    let args_node = def.args.as_deref()?;
    if let Node::Args(args) = args_node {
        for arg in &args.args {
            if let Node::Blockarg(ba) = arg {
                return ba.name.clone();
            }
        }
    }
    None
}

/// Count regular (non-optional, non-keyword, non-rest, non-block) arguments in a Def.
pub fn def_regular_arg_count(def: &Def) -> usize {
    match def.args.as_deref() {
        Some(Node::Args(args)) => args
            .args
            .iter()
            .filter(|a| matches!(a, Node::Arg(_)))
            .count(),
        _ => 0,
    }
}

/// Get the first regular argument name from a Def.
pub fn def_first_arg_name(def: &Def) -> Option<String> {
    match def.args.as_deref() {
        Some(Node::Args(args)) => args.args.iter().find_map(|a| match a {
            Node::Arg(arg) => Some(arg.name.clone()),
            _ => None,
        }),
        _ => None,
    }
}

/// Check if a string literal contains "def".
pub fn str_contains_def(node: &Node) -> bool {
    match node {
        Node::Str(s) => s.value.to_string_lossy().contains("def"),
        Node::Heredoc(h) => h.parts.iter().any(|part| match part {
            Node::Str(s) => s.value.to_string_lossy().contains("def"),
            _ => false,
        }),
        _ => false,
    }
}

/// Get expressions from a body node. If it's a Begin, return its statements.
/// Otherwise return a single-element slice-like iterator.
pub fn body_expressions(body: &Option<Box<Node>>) -> Vec<&Node> {
    match body.as_deref() {
        None => vec![],
        Some(Node::Begin(b)) => b.statements.iter().collect(),
        Some(node) => vec![node],
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
    use lib_ruby_parser::Parser;

    fn parse(source: &[u8]) -> Option<Box<Node>> {
        Parser::new(source.to_vec(), Default::default())
            .do_parse()
            .ast
    }

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
    fn receiver_is_send_with_name_works() {
        let ast = parse(b"a.foo.bar").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(receiver_is_send_with_name(&s.recv, "foo"));
            assert!(!receiver_is_send_with_name(&s.recv, "baz"));
        }
    }

    #[test]
    fn receiver_is_send_with_name_none() {
        assert!(!receiver_is_send_with_name(&None, "foo"));
    }

    #[test]
    fn receiver_as_send_works() {
        let ast = parse(b"a.foo.bar").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            let inner = receiver_as_send(&s.recv).unwrap();
            assert_eq!(inner.method_name, "foo");
        }
    }

    #[test]
    fn receiver_as_send_not_send() {
        assert!(receiver_as_send(&None).is_none());
    }

    #[test]
    fn block_call_as_send_works() {
        let ast = parse(b"arr.map { |x| x }").unwrap();
        if let Node::Block(b) = ast.as_ref() {
            let send = block_call_as_send(b).unwrap();
            assert_eq!(send.method_name, "map");
        }
    }

    #[test]
    fn has_block_pass_works() {
        let ast = parse(b"arr.map(&:to_s)").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(has_block_pass(&s.args));
        }
    }

    #[test]
    fn has_block_pass_without() {
        let ast = parse(b"arr.map(1)").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(!has_block_pass(&s.args));
        }
    }

    #[test]
    fn arg_count_without_block_pass_works() {
        let ast = parse(b"arr.select(&:odd?).first").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert_eq!(arg_count_without_block_pass(&s.args), 0);
        }
    }

    #[test]
    fn is_single_char_string_works() {
        let ast = parse(b"'x'").unwrap();
        assert!(is_single_char_string(&ast));
        let ast2 = parse(b"'xy'").unwrap();
        assert!(!is_single_char_string(&ast2));
    }

    #[test]
    fn is_single_char_string_not_string() {
        let ast = parse(b"42").unwrap();
        assert!(!is_single_char_string(&ast));
    }

    #[test]
    fn receiver_is_range_irange() {
        let ast = parse(b"(1..10).include?(5)").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(receiver_is_range(&s.recv));
        }
    }

    #[test]
    fn receiver_is_range_erange() {
        let ast = parse(b"(1...10).include?(5)").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(receiver_is_range(&s.recv));
        }
    }

    #[test]
    fn receiver_is_range_not_range() {
        let ast = parse(b"[1].include?(5)").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(!receiver_is_range(&s.recv));
        }
    }

    #[test]
    fn is_primitive_covers_types() {
        assert!(is_primitive(&parse(b"42").unwrap()));
        assert!(is_primitive(&parse(b"3.14").unwrap()));
        assert!(is_primitive(&parse(b"'s'").unwrap()));
        assert!(is_primitive(&parse(b":sym").unwrap()));
        assert!(is_primitive(&parse(b"true").unwrap()));
        assert!(is_primitive(&parse(b"false").unwrap()));
        assert!(is_primitive(&parse(b"nil").unwrap()));
        assert!(is_primitive(&parse(b"[]").unwrap()));
        assert!(is_primitive(&parse(b"{}").unwrap()));
        assert!(is_primitive(&parse(b"1..5").unwrap()));
        assert!(is_primitive(&parse(b"1...5").unwrap()));
        assert!(!is_primitive(&parse(b"x").unwrap()));
    }

    #[test]
    fn first_arg_is_single_pair_hash_kwargs() {
        let ast = parse(b"h.merge!(a: 1)").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(first_arg_is_single_pair_hash(&s.args));
        }
    }

    #[test]
    fn first_arg_is_single_pair_hash_explicit() {
        let ast = parse(b"h.merge!({a: 1})").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(first_arg_is_single_pair_hash(&s.args));
        }
    }

    #[test]
    fn first_arg_is_single_pair_hash_multi() {
        let ast = parse(b"h.merge!(a: 1, b: 2)").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(!first_arg_is_single_pair_hash(&s.args));
        }
    }

    #[test]
    fn first_arg_is_single_pair_hash_not_hash() {
        let ast = parse(b"h.merge!(x)").unwrap();
        if let Node::Send(s) = ast.as_ref() {
            assert!(!first_arg_is_single_pair_hash(&s.args));
        }
    }

    #[test]
    fn is_int_one_works() {
        assert!(is_int_one(&parse(b"1").unwrap()));
        assert!(!is_int_one(&parse(b"2").unwrap()));
        assert!(!is_int_one(&parse(b"'1'").unwrap()));
    }

    #[test]
    fn block_arg_names_single() {
        let ast = parse(b"arr.map { |x| x }").unwrap();
        if let Node::Block(b) = ast.as_ref() {
            let names = block_arg_names(&b.args);
            assert_eq!(names, vec!["x".to_string()]);
        }
    }

    #[test]
    fn block_arg_names_none() {
        let names = block_arg_names(&None);
        assert!(names.is_empty());
    }

    #[test]
    fn def_block_arg_name_present() {
        let ast = parse(b"def foo(&block); end").unwrap();
        if let Node::Def(d) = ast.as_ref() {
            assert_eq!(def_block_arg_name(d), Some("block".to_string()));
        }
    }

    #[test]
    fn def_block_arg_name_absent() {
        let ast = parse(b"def foo(x); end").unwrap();
        if let Node::Def(d) = ast.as_ref() {
            assert_eq!(def_block_arg_name(d), None);
        }
    }

    #[test]
    fn def_regular_arg_count_works() {
        let ast = parse(b"def foo(a, b); end").unwrap();
        if let Node::Def(d) = ast.as_ref() {
            assert_eq!(def_regular_arg_count(d), 2);
        }
    }

    #[test]
    fn def_regular_arg_count_no_args() {
        let ast = parse(b"def foo; end").unwrap();
        if let Node::Def(d) = ast.as_ref() {
            assert_eq!(def_regular_arg_count(d), 0);
        }
    }

    #[test]
    fn def_first_arg_name_works() {
        let ast = parse(b"def foo(bar); end").unwrap();
        if let Node::Def(d) = ast.as_ref() {
            assert_eq!(def_first_arg_name(d), Some("bar".to_string()));
        }
    }

    #[test]
    fn def_first_arg_name_no_args() {
        let ast = parse(b"def foo; end").unwrap();
        if let Node::Def(d) = ast.as_ref() {
            assert_eq!(def_first_arg_name(d), None);
        }
    }

    #[test]
    fn str_contains_def_in_string() {
        let ast = parse(b"\"def foo\"").unwrap();
        assert!(str_contains_def(&ast));
    }

    #[test]
    fn str_contains_def_no_def() {
        let ast = parse(b"\"hello\"").unwrap();
        assert!(!str_contains_def(&ast));
    }

    #[test]
    fn str_contains_def_not_string() {
        let ast = parse(b"42").unwrap();
        assert!(!str_contains_def(&ast));
    }

    #[test]
    fn str_contains_def_heredoc() {
        let ast = parse(b"<<~RUBY\ndef foo\nRUBY\n").unwrap();
        assert!(str_contains_def(&ast));
    }

    #[test]
    fn body_expressions_none() {
        assert!(body_expressions(&None).is_empty());
    }

    #[test]
    fn body_expressions_single() {
        let ast = parse(b"def foo; 42; end").unwrap();
        if let Node::Def(d) = ast.as_ref() {
            let exprs = body_expressions(&d.body);
            assert_eq!(exprs.len(), 1);
        }
    }

    #[test]
    fn body_expressions_begin() {
        let ast = parse(b"def foo; 1; 2; 3; end").unwrap();
        if let Node::Def(d) = ast.as_ref() {
            let exprs = body_expressions(&d.body);
            assert_eq!(exprs.len(), 3);
        }
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
}
