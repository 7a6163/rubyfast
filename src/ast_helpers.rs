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
