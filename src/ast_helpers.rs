use lib_ruby_parser::Node;
use lib_ruby_parser::nodes::*;

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

/// Collect all direct child nodes of a given node.
/// Prefer `for_each_child` in hot paths to avoid Vec allocation.
pub fn node_children(node: &Node) -> Vec<&Node> {
    let mut children = Vec::new();
    for_each_child(node, |child| children.push(child));
    children
}

/// Visit each direct child of a node via callback — zero allocation.
#[inline]
pub fn for_each_child<'a>(node: &'a Node, mut f: impl FnMut(&'a Node)) {
    visit_children(node, &mut f);
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

    fn count_children(node: &Node) -> usize {
        let mut count = 0;
        for_each_child(node, |_| count += 1);
        count
    }

    /// Recursively count all nodes in the AST.
    fn count_all_nodes(node: &Node) -> usize {
        let mut count = 1;
        for_each_child(node, |child| count += count_all_nodes(child));
        count
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
    fn node_children_matches_for_each_child() {
        let ast = parse(b"a + b").unwrap();
        let children = node_children(&ast);
        let mut count = 0;
        for_each_child(&ast, |_| count += 1);
        assert_eq!(children.len(), count);
    }

    // Exercise many AST node types through visit_children for coverage
    #[test]
    fn visit_children_comprehensive() {
        let sources: &[&[u8]] = &[
            // Alias
            b"alias new_method old_method",
            // And, Or
            b"a && b || c",
            // AndAsgn, OrAsgn
            b"x &&= 1; y ||= 2",
            // Array, ArrayPattern
            b"[1, 2, 3]",
            // Begin, Break, Next, Return
            b"begin; break 1; end",
            b"loop { next }",
            // Case, When
            b"case x; when 1; 'a'; when 2; 'b'; else 'c'; end",
            // Casgn
            b"FOO = 1",
            // Class
            b"class Foo < Bar; end",
            // Const
            b"Foo::Bar",
            // CSend
            b"x&.foo(1)",
            // Cvasgn, Cvar
            b"@@x = 1; @@x",
            // Def, Defs
            b"def foo(a); end",
            b"def self.bar; end",
            // Defined
            b"defined?(x)",
            // Dstr, Dsym
            b"\"hello #{world}\"",
            b":\"sym_#{x}\"",
            // EFlipFlop, IFlipFlop
            // Ensure
            b"begin; 1; ensure; 2; end",
            // Erange, Irange
            b"1...10; 1..10",
            // For
            b"for x in [1]; end",
            // Gvasgn, Gvar
            b"$x = 1; $x",
            // Hash, Pair
            b"{a: 1, b: 2}",
            // Heredoc
            b"<<~HERE\nhello\nHERE\n",
            // If, IfMod, IfTernary
            b"if true; 1; else; 2; end",
            b"x = 1 if true",
            b"true ? 1 : 2",
            // Index, IndexAsgn
            b"a[0]; a[0] = 1",
            // Ivasgn, Ivar
            b"@x = 1; @x",
            // Kwargs, Kwsplat
            b"foo(a: 1, **opts)",
            // KwBegin
            b"begin; 1; rescue; 2; end",
            // Kwoptarg, Kwrestarg
            b"def foo(a: 1, **rest); end",
            // Lvasgn
            b"x = 42",
            // Masgn, Mlhs
            b"a, b = 1, 2",
            // Module
            b"module Foo; end",
            // Next, Return
            b"def foo; return 1; end",
            // Numblock
            b"arr.map { _1.to_s }",
            // OpAsgn
            b"x += 1",
            // Optarg
            b"def foo(a = 1); end",
            // Pin (pattern matching)
            b"case x; in ^y; end",
            // Postexe, Preexe
            b"END { 1 }",
            b"BEGIN { 1 }",
            // Procarg0 (block with single destructured arg)
            b"arr.each { |(a)| a }",
            // Regexp, RegOpt
            b"/foo/i",
            // Rescue, RescueBody
            b"begin; rescue StandardError => e; end",
            // SClass
            b"class << self; end",
            // Send
            b"foo.bar(1, 2)",
            // Splat
            b"foo(*args)",
            // Str
            b"'hello'",
            // Super
            b"def foo; super(1); end",
            // Undef
            b"undef :foo",
            // Until, While
            b"until false; end",
            b"while true; break; end",
            // Yield
            b"def foo; yield 1; end",
            // Xstr, XHeredoc
            b"`echo hi`",
            // MatchCurrentLine
            b"if /pattern/; end",
            // Block, BlockPass
            b"arr.select(&:odd?)",
            // FindPattern, InPattern
            b"case x; in [1, *rest, 2]; end",
            // MatchAlt, MatchAs
            b"case x; in 1 | 2 => y; end",
            // MatchPattern, MatchPatternP
            b"x in [1, 2]",
            b"x in [1, 2] rescue false",
            // MatchNilPattern, MatchVar
            b"case x; in **nil; end",
            b"case x; in {a:}; end",
            // MatchWithLvasgn
            b"/(?<name>.)/ =~ str",
            // HashPattern, ConstPattern
            b"case x; in Foo[a:]; end",
            // MatchRest
            b"case x; in [*, 1]; end",
            // WhilePost
            b"begin; 1; end while true",
            // UntilPost
            b"begin; 1; end until true",
            // UnlessGuard
            b"case x; in 1 unless false; end",
            // EFlipFlop (exclusive)
            b"if (a == 1)...(b == 2); end",
            // IFlipFlop (inclusive)
            b"if (a == 1)..(b == 2); end",
            // Rational, Complex
            b"1r",
            b"1i",
            // BackRef, NthRef
            b"$~ ; $1",
            // Redo, Retry
            b"begin; retry; rescue; end",
            // Self
            b"self",
            // ZSuper
            b"def foo; super; end",
            // Lambda
            b"-> { 1 }",
            // Encoding, File, Line
            b"__ENCODING__",
            b"__FILE__",
            b"__LINE__",
            // XHeredoc
            b"<<~`CMD`\necho hi\nCMD\n",
            // ArrayPatternWithTail
            b"case x; in [1, 2,]; end",
            // ForwardArg, ForwardedArgs
            b"def foo(...); bar(...); end",
            // Kwarg
            b"def foo(a:); end",
            // Kwnilarg
            b"def foo(**nil); end",
            // Shadowarg
            b"arr.each { |x; y| y }",
            // Restarg
            b"def foo(*args); end",
        ];

        for source in sources {
            if let Some(ast) = parse(source) {
                let total = count_all_nodes(&ast);
                assert!(
                    total > 0,
                    "No nodes in AST for {:?}",
                    std::str::from_utf8(source)
                );
            }
        }
    }

    #[test]
    fn visit_children_pattern_matching() {
        // These exercise pattern matching AST nodes specifically
        let sources: &[&[u8]] = &[
            // MatchPattern (in operator)
            b"1 in Integer",
            // MatchPatternP (case/in with guard)
            b"case 1; in Integer if true; end",
            // IfGuard
            b"case 1; in x if x > 0; end",
            // UnlessGuard
            b"case 1; in x unless x < 0; end",
            // FindPattern
            b"case [1,2,3]; in [*, 2, *]; end",
            // HashPattern
            b"case {a: 1}; in {a: Integer}; end",
            // ConstPattern
            b"case x; in Foo(1); end",
            // MatchNilPattern
            b"case {a: 1}; in **nil; end",
            // MatchVar
            b"case 1; in x; end",
            // MatchRest
            b"case [1,2]; in [Integer, *rest]; end",
            // MatchAlt
            b"case 1; in 1 | 2; end",
            // MatchAs
            b"case 1; in Integer => x; end",
            // MatchWithLvasgn (regex named capture)
            b"/(?<name>.)/ =~ 'x'",
            // Pin
            b"x = 1; case 2; in ^x; end",
        ];
        for source in sources {
            if let Some(ast) = parse(source) {
                let total = count_all_nodes(&ast);
                assert!(total > 0, "No nodes for {:?}", std::str::from_utf8(source));
            }
        }
    }

    #[test]
    fn visit_children_leaf_nodes() {
        // Leaf nodes should have 0 children
        let leaf_sources: &[&[u8]] = &[
            b"42",    // Int
            b"3.14",  // Float
            b"'s'",   // Str
            b":sym",  // Sym
            b"true",  // True
            b"false", // False
            b"nil",   // Nil
            b"x",     // Lvar
        ];

        for source in leaf_sources {
            let ast = parse(source).unwrap();
            assert_eq!(
                count_children(&ast),
                0,
                "Expected 0 children for {:?}",
                std::str::from_utf8(source)
            );
        }
    }
}

fn visit_opt<'a>(opt: &'a Option<Box<Node>>, f: &mut impl FnMut(&'a Node)) {
    if let Some(n) = opt.as_deref() {
        f(n);
    }
}

fn visit_vec<'a>(v: &'a [Node], f: &mut impl FnMut(&'a Node)) {
    for n in v {
        f(n);
    }
}

fn visit_children<'a>(node: &'a Node, f: &mut impl FnMut(&'a Node)) {
    match node {
        Node::Alias(n) => {
            f(&n.to);
            f(&n.from);
        }
        Node::And(n) => {
            f(&n.lhs);
            f(&n.rhs);
        }
        Node::AndAsgn(n) => {
            f(&n.recv);
            f(&n.value);
        }
        Node::Arg(_)
        | Node::BackRef(_)
        | Node::Blockarg(_)
        | Node::Cbase(_)
        | Node::Complex(_)
        | Node::Cvar(_)
        | Node::EmptyElse(_)
        | Node::Encoding(_)
        | Node::False(_)
        | Node::File(_)
        | Node::Float(_)
        | Node::ForwardArg(_)
        | Node::ForwardedArgs(_)
        | Node::Gvar(_)
        | Node::Int(_)
        | Node::Ivar(_)
        | Node::Kwarg(_)
        | Node::Kwnilarg(_)
        | Node::Lambda(_)
        | Node::Line(_)
        | Node::Lvar(_)
        | Node::Nil(_)
        | Node::Rational(_)
        | Node::Redo(_)
        | Node::Restarg(_)
        | Node::Retry(_)
        | Node::Self_(_)
        | Node::Shadowarg(_)
        | Node::Sym(_)
        | Node::True(_)
        | Node::ZSuper(_)
        | Node::NthRef(_)
        | Node::RegOpt(_) => {}
        Node::Args(n) => visit_vec(&n.args, f),
        Node::Array(n) => visit_vec(&n.elements, f),
        Node::ArrayPattern(n) => visit_vec(&n.elements, f),
        Node::ArrayPatternWithTail(n) => visit_vec(&n.elements, f),
        Node::Begin(n) => visit_vec(&n.statements, f),
        Node::Block(n) => {
            f(&n.call);
            visit_opt(&n.args, f);
            visit_opt(&n.body, f);
        }
        Node::BlockPass(n) => visit_opt(&n.value, f),
        Node::Break(n) => visit_vec(&n.args, f),
        Node::Case(n) => {
            visit_opt(&n.expr, f);
            visit_vec(&n.when_bodies, f);
            visit_opt(&n.else_body, f);
        }
        Node::CaseMatch(n) => {
            f(&n.expr);
            visit_vec(&n.in_bodies, f);
            visit_opt(&n.else_body, f);
        }
        Node::Casgn(n) => {
            visit_opt(&n.scope, f);
            visit_opt(&n.value, f);
        }
        Node::Class(n) => {
            f(&n.name);
            visit_opt(&n.superclass, f);
            visit_opt(&n.body, f);
        }
        Node::Const(n) => visit_opt(&n.scope, f),
        Node::ConstPattern(n) => {
            f(&n.const_);
            f(&n.pattern);
        }
        Node::CSend(n) => {
            f(&n.recv);
            visit_vec(&n.args, f);
        }
        Node::Cvasgn(n) => visit_opt(&n.value, f),
        Node::Def(n) => {
            visit_opt(&n.args, f);
            visit_opt(&n.body, f);
        }
        Node::Defined(n) => f(&n.value),
        Node::Defs(n) => {
            f(&n.definee);
            visit_opt(&n.args, f);
            visit_opt(&n.body, f);
        }
        Node::Dstr(n) => visit_vec(&n.parts, f),
        Node::Dsym(n) => visit_vec(&n.parts, f),
        Node::EFlipFlop(n) => {
            visit_opt(&n.left, f);
            visit_opt(&n.right, f);
        }
        Node::Ensure(n) => {
            visit_opt(&n.body, f);
            visit_opt(&n.ensure, f);
        }
        Node::Erange(n) => {
            visit_opt(&n.left, f);
            visit_opt(&n.right, f);
        }
        Node::FindPattern(n) => visit_vec(&n.elements, f),
        Node::For(n) => {
            f(&n.iterator);
            f(&n.iteratee);
            visit_opt(&n.body, f);
        }
        Node::Gvasgn(n) => visit_opt(&n.value, f),
        Node::Hash(n) => visit_vec(&n.pairs, f),
        Node::HashPattern(n) => visit_vec(&n.elements, f),
        Node::Heredoc(n) => visit_vec(&n.parts, f),
        Node::If(n) => {
            f(&n.cond);
            visit_opt(&n.if_true, f);
            visit_opt(&n.if_false, f);
        }
        Node::IfGuard(n) => f(&n.cond),
        Node::IFlipFlop(n) => {
            visit_opt(&n.left, f);
            visit_opt(&n.right, f);
        }
        Node::IfMod(n) => {
            f(&n.cond);
            visit_opt(&n.if_true, f);
            visit_opt(&n.if_false, f);
        }
        Node::IfTernary(n) => {
            f(&n.cond);
            f(&n.if_true);
            f(&n.if_false);
        }
        Node::Index(n) => {
            f(&n.recv);
            visit_vec(&n.indexes, f);
        }
        Node::IndexAsgn(n) => {
            f(&n.recv);
            visit_vec(&n.indexes, f);
            visit_opt(&n.value, f);
        }
        Node::InPattern(n) => {
            f(&n.pattern);
            visit_opt(&n.guard, f);
            visit_opt(&n.body, f);
        }
        Node::Irange(n) => {
            visit_opt(&n.left, f);
            visit_opt(&n.right, f);
        }
        Node::Ivasgn(n) => visit_opt(&n.value, f),
        Node::Kwargs(n) => visit_vec(&n.pairs, f),
        Node::KwBegin(n) => visit_vec(&n.statements, f),
        Node::Kwoptarg(n) => f(&n.default),
        Node::Kwrestarg(_) => {}
        Node::Kwsplat(n) => f(&n.value),
        Node::Lvasgn(n) => visit_opt(&n.value, f),
        Node::Masgn(n) => {
            f(&n.lhs);
            f(&n.rhs);
        }
        Node::MatchAlt(n) => {
            f(&n.lhs);
            f(&n.rhs);
        }
        Node::MatchAs(n) => {
            f(&n.value);
            f(&n.as_);
        }
        Node::MatchCurrentLine(n) => f(&n.re),
        Node::MatchNilPattern(_) => {}
        Node::MatchPattern(n) => {
            f(&n.value);
            f(&n.pattern);
        }
        Node::MatchPatternP(n) => {
            f(&n.value);
            f(&n.pattern);
        }
        Node::MatchRest(n) => visit_opt(&n.name, f),
        Node::MatchVar(_) => {}
        Node::MatchWithLvasgn(n) => {
            f(&n.re);
            f(&n.value);
        }
        Node::Mlhs(n) => visit_vec(&n.items, f),
        Node::Module(n) => {
            f(&n.name);
            visit_opt(&n.body, f);
        }
        Node::Next(n) => visit_vec(&n.args, f),
        Node::Numblock(n) => {
            f(&n.call);
            f(&n.body);
        }
        Node::OpAsgn(n) => {
            f(&n.recv);
            f(&n.value);
        }
        Node::Optarg(n) => f(&n.default),
        Node::Or(n) => {
            f(&n.lhs);
            f(&n.rhs);
        }
        Node::OrAsgn(n) => {
            f(&n.recv);
            f(&n.value);
        }
        Node::Pair(n) => {
            f(&n.key);
            f(&n.value);
        }
        Node::Pin(n) => f(&n.var),
        Node::Postexe(n) => visit_opt(&n.body, f),
        Node::Preexe(n) => visit_opt(&n.body, f),
        Node::Procarg0(n) => visit_vec(&n.args, f),
        Node::Regexp(n) => visit_vec(&n.parts, f),
        Node::Rescue(n) => {
            visit_opt(&n.body, f);
            visit_vec(&n.rescue_bodies, f);
            visit_opt(&n.else_, f);
        }
        Node::RescueBody(n) => {
            visit_opt(&n.exc_list, f);
            visit_opt(&n.exc_var, f);
            visit_opt(&n.body, f);
        }
        Node::Return(n) => visit_vec(&n.args, f),
        Node::SClass(n) => {
            f(&n.expr);
            visit_opt(&n.body, f);
        }
        Node::Send(n) => {
            visit_opt(&n.recv, f);
            visit_vec(&n.args, f);
        }
        Node::Splat(n) => visit_opt(&n.value, f),
        Node::Str(_) => {}
        Node::Super(n) => visit_vec(&n.args, f),
        Node::Undef(n) => visit_vec(&n.names, f),
        Node::UnlessGuard(n) => f(&n.cond),
        Node::Until(n) => {
            f(&n.cond);
            visit_opt(&n.body, f);
        }
        Node::UntilPost(n) => {
            f(&n.cond);
            f(&n.body);
        }
        Node::When(n) => {
            visit_vec(&n.patterns, f);
            visit_opt(&n.body, f);
        }
        Node::While(n) => {
            f(&n.cond);
            visit_opt(&n.body, f);
        }
        Node::WhilePost(n) => {
            f(&n.cond);
            f(&n.body);
        }
        Node::XHeredoc(n) => visit_vec(&n.parts, f),
        Node::Xstr(n) => visit_vec(&n.parts, f),
        Node::Yield(n) => visit_vec(&n.args, f),
    }
}
