use lib_ruby_parser::nodes::*;
use lib_ruby_parser::Node;

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

/// Check if a Block node's call is a Send with a given method name.
pub fn block_call_method_name(block: &Block) -> Option<&str> {
    match block.call.as_ref() {
        Node::Send(s) => Some(&s.method_name),
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
/// This replaces the private `inner_ref()` method.
pub fn node_children(node: &Node) -> Vec<&Node> {
    let mut children = Vec::new();
    push_children(node, &mut children);
    children
}

fn push_opt<'a>(opt: &'a Option<Box<Node>>, out: &mut Vec<&'a Node>) {
    if let Some(n) = opt.as_deref() {
        out.push(n);
    }
}

fn push_vec<'a>(v: &'a [Node], out: &mut Vec<&'a Node>) {
    for n in v {
        out.push(n);
    }
}

fn push_children<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
    match node {
        Node::Alias(n) => {
            out.push(&n.to);
            out.push(&n.from);
        }
        Node::And(n) => {
            out.push(&n.lhs);
            out.push(&n.rhs);
        }
        Node::AndAsgn(n) => {
            out.push(&n.recv);
            out.push(&n.value);
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
        Node::Args(n) => push_vec(&n.args, out),
        Node::Array(n) => push_vec(&n.elements, out),
        Node::ArrayPattern(n) => push_vec(&n.elements, out),
        Node::ArrayPatternWithTail(n) => push_vec(&n.elements, out),
        Node::Begin(n) => push_vec(&n.statements, out),
        Node::Block(n) => {
            out.push(&n.call);
            push_opt(&n.args, out);
            push_opt(&n.body, out);
        }
        Node::BlockPass(n) => push_opt(&n.value, out),
        Node::Break(n) => push_vec(&n.args, out),
        Node::Case(n) => {
            push_opt(&n.expr, out);
            push_vec(&n.when_bodies, out);
            push_opt(&n.else_body, out);
        }
        Node::CaseMatch(n) => {
            out.push(&n.expr);
            push_vec(&n.in_bodies, out);
            push_opt(&n.else_body, out);
        }
        Node::Casgn(n) => {
            push_opt(&n.scope, out);
            push_opt(&n.value, out);
        }
        Node::Class(n) => {
            out.push(&n.name);
            push_opt(&n.superclass, out);
            push_opt(&n.body, out);
        }
        Node::Const(n) => push_opt(&n.scope, out),
        Node::ConstPattern(n) => {
            out.push(&n.const_);
            out.push(&n.pattern);
        }
        Node::CSend(n) => {
            out.push(&n.recv);
            push_vec(&n.args, out);
        }
        Node::Cvasgn(n) => push_opt(&n.value, out),
        Node::Def(n) => {
            push_opt(&n.args, out);
            push_opt(&n.body, out);
        }
        Node::Defined(n) => out.push(&n.value),
        Node::Defs(n) => {
            out.push(&n.definee);
            push_opt(&n.args, out);
            push_opt(&n.body, out);
        }
        Node::Dstr(n) => push_vec(&n.parts, out),
        Node::Dsym(n) => push_vec(&n.parts, out),
        Node::EFlipFlop(n) => {
            push_opt(&n.left, out);
            push_opt(&n.right, out);
        }
        Node::Ensure(n) => {
            push_opt(&n.body, out);
            push_opt(&n.ensure, out);
        }
        Node::Erange(n) => {
            push_opt(&n.left, out);
            push_opt(&n.right, out);
        }
        Node::FindPattern(n) => push_vec(&n.elements, out),
        Node::For(n) => {
            out.push(&n.iterator);
            out.push(&n.iteratee);
            push_opt(&n.body, out);
        }
        Node::Gvasgn(n) => push_opt(&n.value, out),
        Node::Hash(n) => push_vec(&n.pairs, out),
        Node::HashPattern(n) => push_vec(&n.elements, out),
        Node::Heredoc(n) => push_vec(&n.parts, out),
        Node::If(n) => {
            out.push(&n.cond);
            push_opt(&n.if_true, out);
            push_opt(&n.if_false, out);
        }
        Node::IfGuard(n) => out.push(&n.cond),
        Node::IFlipFlop(n) => {
            push_opt(&n.left, out);
            push_opt(&n.right, out);
        }
        Node::IfMod(n) => {
            out.push(&n.cond);
            push_opt(&n.if_true, out);
            push_opt(&n.if_false, out);
        }
        Node::IfTernary(n) => {
            out.push(&n.cond);
            out.push(&n.if_true);
            out.push(&n.if_false);
        }
        Node::Index(n) => {
            out.push(&n.recv);
            push_vec(&n.indexes, out);
        }
        Node::IndexAsgn(n) => {
            out.push(&n.recv);
            push_vec(&n.indexes, out);
            push_opt(&n.value, out);
        }
        Node::InPattern(n) => {
            out.push(&n.pattern);
            push_opt(&n.guard, out);
            push_opt(&n.body, out);
        }
        Node::Irange(n) => {
            push_opt(&n.left, out);
            push_opt(&n.right, out);
        }
        Node::Ivasgn(n) => push_opt(&n.value, out),
        Node::Kwargs(n) => push_vec(&n.pairs, out),
        Node::KwBegin(n) => push_vec(&n.statements, out),
        Node::Kwoptarg(n) => out.push(&n.default),
        Node::Kwrestarg(_) => {}
        Node::Kwsplat(n) => out.push(&n.value),
        Node::Lvasgn(n) => push_opt(&n.value, out),
        Node::Masgn(n) => {
            out.push(&n.lhs);
            out.push(&n.rhs);
        }
        Node::MatchAlt(n) => {
            out.push(&n.lhs);
            out.push(&n.rhs);
        }
        Node::MatchAs(n) => {
            out.push(&n.value);
            out.push(&n.as_);
        }
        Node::MatchCurrentLine(n) => out.push(&n.re),
        Node::MatchNilPattern(_) => {}
        Node::MatchPattern(n) => {
            out.push(&n.value);
            out.push(&n.pattern);
        }
        Node::MatchPatternP(n) => {
            out.push(&n.value);
            out.push(&n.pattern);
        }
        Node::MatchRest(n) => push_opt(&n.name, out),
        Node::MatchVar(_) => {}
        Node::MatchWithLvasgn(n) => {
            out.push(&n.re);
            out.push(&n.value);
        }
        Node::Mlhs(n) => push_vec(&n.items, out),
        Node::Module(n) => {
            out.push(&n.name);
            push_opt(&n.body, out);
        }
        Node::Next(n) => push_vec(&n.args, out),
        Node::Numblock(n) => {
            out.push(&n.call);
            out.push(&n.body);
        }
        Node::OpAsgn(n) => {
            out.push(&n.recv);
            out.push(&n.value);
        }
        Node::Optarg(n) => out.push(&n.default),
        Node::Or(n) => {
            out.push(&n.lhs);
            out.push(&n.rhs);
        }
        Node::OrAsgn(n) => {
            out.push(&n.recv);
            out.push(&n.value);
        }
        Node::Pair(n) => {
            out.push(&n.key);
            out.push(&n.value);
        }
        Node::Pin(n) => out.push(&n.var),
        Node::Postexe(n) => push_opt(&n.body, out),
        Node::Preexe(n) => push_opt(&n.body, out),
        Node::Procarg0(n) => push_vec(&n.args, out),
        Node::Regexp(n) => push_vec(&n.parts, out),
        Node::Rescue(n) => {
            push_opt(&n.body, out);
            push_vec(&n.rescue_bodies, out);
            push_opt(&n.else_, out);
        }
        Node::RescueBody(n) => {
            push_opt(&n.exc_list, out);
            push_opt(&n.exc_var, out);
            push_opt(&n.body, out);
        }
        Node::Return(n) => push_vec(&n.args, out),
        Node::SClass(n) => {
            out.push(&n.expr);
            push_opt(&n.body, out);
        }
        Node::Send(n) => {
            push_opt(&n.recv, out);
            push_vec(&n.args, out);
        }
        Node::Splat(n) => push_opt(&n.value, out),
        Node::Str(_) => {}
        Node::Super(n) => push_vec(&n.args, out),
        Node::Undef(n) => push_vec(&n.names, out),
        Node::UnlessGuard(n) => out.push(&n.cond),
        Node::Until(n) => {
            out.push(&n.cond);
            push_opt(&n.body, out);
        }
        Node::UntilPost(n) => {
            out.push(&n.cond);
            out.push(&n.body);
        }
        Node::When(n) => {
            push_vec(&n.patterns, out);
            push_opt(&n.body, out);
        }
        Node::While(n) => {
            out.push(&n.cond);
            push_opt(&n.body, out);
        }
        Node::WhilePost(n) => {
            out.push(&n.cond);
            out.push(&n.body);
        }
        Node::XHeredoc(n) => push_vec(&n.parts, out),
        Node::Xstr(n) => push_vec(&n.parts, out),
        Node::Yield(n) => push_vec(&n.args, out),
    }
}
