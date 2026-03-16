use crate::ast_helpers::*;
use crate::fix::Fix;
use crate::offense::{Offense, OffenseKind};

/// Scan a method call (CallNode) that does NOT have a block.
pub fn scan_call(call: &ruby_prism::CallNode<'_>) -> Vec<Offense> {
    let mut offenses = Vec::new();

    check_shuffle_first(call, &mut offenses);
    check_reverse_each(call, &mut offenses);
    check_keys_each(call, &mut offenses);
    check_each_with_index(call, &mut offenses);
    check_include_vs_cover(call, &mut offenses);
    check_gsub_vs_tr(call, &mut offenses);
    check_fetch_with_argument(call, &mut offenses);
    check_hash_merge_bang(call, &mut offenses);
    check_map_flatten(call, &mut offenses);
    check_select_first(call, &mut offenses);
    check_select_last(call, &mut offenses);
    check_module_eval_call(call, &mut offenses);

    offenses
}

/// Scan a CallNode that has a BlockNode (method call + block).
pub fn scan_call_with_block(
    call: &ruby_prism::CallNode<'_>,
    block: &ruby_prism::BlockNode<'_>,
) -> Vec<Offense> {
    let mut offenses = Vec::new();

    // Checks that only apply when a block is present
    check_sort_vs_sort_by(call, &mut offenses);
    check_module_eval_call(call, &mut offenses);
    check_block_vs_symbol_to_proc(call, block, &mut offenses);

    // Chain checks on the call inside the block
    check_shuffle_first(call, &mut offenses);
    check_reverse_each(call, &mut offenses);
    check_keys_each(call, &mut offenses);
    check_each_with_index(call, &mut offenses);
    check_include_vs_cover(call, &mut offenses);
    check_gsub_vs_tr(call, &mut offenses);
    // NOTE: check_fetch_with_argument excluded — if fetch already has a block, rule doesn't apply.
    check_hash_merge_bang(call, &mut offenses);

    offenses
}

/// Scan a CallNode whose receiver is another CallNode that has a block.
/// This handles chains like `.select { }.first` where .first's receiver is a call-with-block.
pub fn scan_call_on_block_call(
    outer: &ruby_prism::CallNode<'_>,
    recv_call: &ruby_prism::CallNode<'_>,
) -> Vec<Offense> {
    let mut offenses = Vec::new();

    let outer_name = outer.name().as_slice();
    let recv_name = recv_call.name().as_slice();

    // .select{}.first → .detect{}
    if outer_name == b"first" && recv_name == b"select" && arg_count(outer) == 0 {
        let offense = match (recv_call.message_loc(), outer.call_operator_loc()) {
            (Some(sel_l), Some(dot_l)) => {
                let fix = Fix::two(
                    sel_l.start_offset(),
                    sel_l.end_offset(),
                    "detect",
                    dot_l.start_offset(),
                    outer.location().end_offset(),
                    "",
                );
                Offense::with_fix(
                    OffenseKind::SelectFirstVsDetect,
                    outer.location().start_offset(),
                    fix,
                )
            }
            _ => Offense::new(
                OffenseKind::SelectFirstVsDetect,
                outer.location().start_offset(),
            ),
        };
        offenses.push(offense);
    }

    // .select{}.last (no auto-fix)
    if outer_name == b"last" && recv_name == b"select" && arg_count(outer) == 0 {
        offenses.push(Offense::new(
            OffenseKind::SelectLastVsReverseDetect,
            outer.location().start_offset(),
        ));
    }

    // .map{}.flatten(1) → .flat_map{}
    if outer_name == b"flatten" && recv_name == b"map" {
        let args = call_args(outer);
        if args.len() == 1 && is_int_one(&args[0]) {
            let offense = match (recv_call.message_loc(), outer.call_operator_loc()) {
                (Some(sel_l), Some(dot_l)) => {
                    let fix = Fix::two(
                        sel_l.start_offset(),
                        sel_l.end_offset(),
                        "flat_map",
                        dot_l.start_offset(),
                        outer.location().end_offset(),
                        "",
                    );
                    Offense::with_fix(
                        OffenseKind::MapFlattenVsFlatMap,
                        outer.location().start_offset(),
                        fix,
                    )
                }
                _ => Offense::new(
                    OffenseKind::MapFlattenVsFlatMap,
                    outer.location().start_offset(),
                ),
            };
            offenses.push(offense);
        }
    }

    offenses
}

// --- Individual offense checks ---

/// `.shuffle.first` → `.sample`
fn check_shuffle_first(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"first"
        || !receiver_is_call_with_name(&call.receiver(), b"shuffle")
    {
        return;
    }
    let offense = match receiver_as_call(&call.receiver()).and_then(|rs| rs.call_operator_loc()) {
        Some(dot_l) => {
            let fix = Fix::single(
                dot_l.start_offset(),
                call.location().end_offset(),
                ".sample",
            );
            Offense::with_fix(
                OffenseKind::ShuffleFirstVsSample,
                call.location().start_offset(),
                fix,
            )
        }
        None => Offense::new(
            OffenseKind::ShuffleFirstVsSample,
            call.location().start_offset(),
        ),
    };
    offenses.push(offense);
}

/// `.reverse.each` → `.reverse_each`
fn check_reverse_each(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"each"
        || !receiver_is_call_with_name(&call.receiver(), b"reverse")
    {
        return;
    }
    let offense = match (
        receiver_as_call(&call.receiver()).and_then(|rs| rs.call_operator_loc()),
        call.message_loc(),
    ) {
        (Some(dot_l), Some(sel_l)) => {
            let fix = Fix::single(dot_l.start_offset(), sel_l.end_offset(), ".reverse_each");
            Offense::with_fix(
                OffenseKind::ReverseEachVsReverseEach,
                call.location().start_offset(),
                fix,
            )
        }
        _ => Offense::new(
            OffenseKind::ReverseEachVsReverseEach,
            call.location().start_offset(),
        ),
    };
    offenses.push(offense);
}

/// `.keys.each` → `.each_key` (keys must have 0 args)
fn check_keys_each(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"each" {
        return;
    }
    if let Some(recv_call) = receiver_as_call(&call.receiver())
        && recv_call.name().as_slice() == b"keys"
        && arg_count(&recv_call) == 0
    {
        let offense = match (recv_call.call_operator_loc(), call.message_loc()) {
            (Some(dot_l), Some(sel_l)) => {
                let fix = Fix::single(dot_l.start_offset(), sel_l.end_offset(), ".each_key");
                Offense::with_fix(
                    OffenseKind::KeysEachVsEachKey,
                    call.location().start_offset(),
                    fix,
                )
            }
            _ => Offense::new(
                OffenseKind::KeysEachVsEachKey,
                call.location().start_offset(),
            ),
        };
        offenses.push(offense);
    }
}

/// `.select{}.first` → `.detect{}` (when receiver is a plain call with block_pass, not block)
fn check_select_first(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"first" || arg_count(call) != 0 {
        return;
    }
    if let Some(recv_call) = receiver_as_call(&call.receiver())
        && recv_call.name().as_slice() == b"select"
        && has_block_pass(&recv_call)
    {
        let offense = match (recv_call.message_loc(), call.call_operator_loc()) {
            (Some(sel_l), Some(dot_l)) => {
                let fix = Fix::two(
                    sel_l.start_offset(),
                    sel_l.end_offset(),
                    "detect",
                    dot_l.start_offset(),
                    call.location().end_offset(),
                    "",
                );
                Offense::with_fix(
                    OffenseKind::SelectFirstVsDetect,
                    call.location().start_offset(),
                    fix,
                )
            }
            _ => Offense::new(
                OffenseKind::SelectFirstVsDetect,
                call.location().start_offset(),
            ),
        };
        offenses.push(offense);
    }
}

/// `.select{}.last` → `.reverse.detect{}` (when receiver is a plain call with block_pass)
fn check_select_last(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"last" || arg_count(call) != 0 {
        return;
    }
    if let Some(recv_call) = receiver_as_call(&call.receiver())
        && recv_call.name().as_slice() == b"select"
        && has_block_pass(&recv_call)
    {
        offenses.push(Offense::new(
            OffenseKind::SelectLastVsReverseDetect,
            call.location().start_offset(),
        ));
    }
}

/// `.map{}.flatten(1)` → `.flat_map{}` (when receiver is a plain call with block_pass, not full block)
fn check_map_flatten(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"flatten" {
        return;
    }
    let args = call_args(call);
    if args.len() != 1 || !is_int_one(&args[0]) {
        return;
    }
    // Only match when receiver is map WITHOUT a full block (block_pass is ok).
    // Full block cases are handled by scan_call_on_block_call.
    if let Some(recv_call) = receiver_as_call(&call.receiver())
        && recv_call.name().as_slice() == b"map"
        && !has_full_block(&recv_call)
    {
        offenses.push(Offense::new(
            OffenseKind::MapFlattenVsFlatMap,
            call.location().start_offset(),
        ));
    }
}

/// `.each_with_index` → while loop
fn check_each_with_index(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() == b"each_with_index" {
        offenses.push(Offense::new(
            OffenseKind::EachWithIndexVsWhile,
            call.location().start_offset(),
        ));
    }
}

/// `(1..10).include?` → `.cover?`
fn check_include_vs_cover(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"include?" || !receiver_is_range(&call.receiver()) {
        return;
    }
    let offense = match call.message_loc() {
        Some(sel_l) => {
            let fix = Fix::single(sel_l.start_offset(), sel_l.end_offset(), "cover?");
            Offense::with_fix(
                OffenseKind::IncludeVsCoverOnRange,
                call.location().start_offset(),
                fix,
            )
        }
        None => Offense::new(
            OffenseKind::IncludeVsCoverOnRange,
            call.location().start_offset(),
        ),
    };
    offenses.push(offense);
}

/// `.gsub("x", "y")` → `.tr("x", "y")` when both args are single-char strings
fn check_gsub_vs_tr(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"gsub" {
        return;
    }
    let args = call_args(call);
    if args.len() != 2 {
        return;
    }
    if is_single_char_string(&args[0]) && is_single_char_string(&args[1]) {
        let offense = match call.message_loc() {
            Some(sel_l) => {
                let fix = Fix::single(sel_l.start_offset(), sel_l.end_offset(), "tr");
                Offense::with_fix(OffenseKind::GsubVsTr, call.location().start_offset(), fix)
            }
            None => Offense::new(OffenseKind::GsubVsTr, call.location().start_offset()),
        };
        offenses.push(offense);
    }
}

/// `.sort { |a, b| ... }` → `.sort_by` (only fires when sort has a block)
fn check_sort_vs_sort_by(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() == b"sort" {
        offenses.push(Offense::new(
            OffenseKind::SortVsSortBy,
            call.location().start_offset(),
        ));
    }
}

/// `.fetch(k, v)` → `.fetch(k) { v }`
fn check_fetch_with_argument(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() == b"fetch" && arg_count(call) == 2 && !has_block_pass(call) {
        offenses.push(Offense::new(
            OffenseKind::FetchWithArgumentVsBlock,
            call.location().start_offset(),
        ));
    }
}

/// `.merge!({k: v})` → `h[k] = v` (single pair hash argument)
fn check_hash_merge_bang(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"merge!" {
        return;
    }
    let args = call_args(call);
    if args.len() != 1 {
        return;
    }
    if first_arg_is_single_pair_hash(&args) {
        offenses.push(Offense::new(
            OffenseKind::HashMergeBangVsHashBrackets,
            call.location().start_offset(),
        ));
    }
}

/// `.module_eval("def ...")` → `define_method`
fn check_module_eval_call(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"module_eval" {
        return;
    }
    let args = call_args(call);
    if let Some(first_arg) = args.first()
        && str_contains_def(first_arg)
    {
        offenses.push(Offense::new(
            OffenseKind::ModuleEval,
            call.location().start_offset(),
        ));
    }
}

/// `.map { |x| x.foo }` → `.map(&:foo)`
fn check_block_vs_symbol_to_proc(
    call: &ruby_prism::CallNode<'_>,
    block: &ruby_prism::BlockNode<'_>,
    offenses: &mut Vec<Offense>,
) {
    // Outer method call must have 0 arguments
    if arg_count(call) != 0 {
        return;
    }

    // Block must have exactly 1 argument
    let arg_names = block_arg_names(&block.parameters());
    if arg_names.len() != 1 {
        return;
    }
    let block_arg_name = &arg_names[0];

    // Block body must be a single CallNode
    let body = match block.body() {
        Some(node) => node,
        None => return,
    };

    // If body is a StatementsNode with a single statement, unwrap it
    let inner_node = if let Some(stmts) = body.as_statements_node() {
        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
        }
        body_nodes.into_iter().next().unwrap()
    } else {
        body
    };

    let inner_call = match inner_node.as_call_node() {
        Some(c) => c,
        None => return,
    };

    // Inner call must have 0 arguments and no block
    if arg_count(&inner_call) != 0 || inner_call.block().is_some() {
        return;
    }

    // Inner call must have a receiver
    let receiver = match inner_call.receiver() {
        Some(r) => r,
        None => return,
    };

    // Receiver must not be a primitive
    if is_primitive(&receiver) {
        return;
    }

    // Receiver must be a LocalVariableReadNode matching the block argument name
    if let Some(lv) = receiver.as_local_variable_read_node()
        && String::from_utf8_lossy(lv.name().as_slice()) == *block_arg_name
    {
        offenses.push(Offense::new(
            OffenseKind::BlockVsSymbolToProc,
            call.location().start_offset(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_visitor::for_each_direct_child;
    use ruby_prism::Node;

    fn parse_and_collect(source: &[u8]) -> Vec<Offense> {
        let result = ruby_prism::parse(source);
        let result = Box::leak(Box::new(result));
        let mut offenses = Vec::new();
        let root = result.node();
        walk_for_offenses(&root, &mut offenses);
        offenses
    }

    /// Walk AST matching real analyzer behavior.
    fn walk_for_offenses<'pr>(node: &Node<'pr>, offenses: &mut Vec<Offense>) {
        match node {
            Node::CallNode { .. } => {
                let call = node.as_call_node().unwrap();

                // Check receiver-is-block-call chains
                if let Some(recv) = call.receiver() {
                    if let Some(recv_call) = recv.as_call_node() {
                        if let Some(Node::BlockNode { .. }) = recv_call.block() {
                            offenses.extend(scan_call_on_block_call(&call, &recv_call));
                        }
                    }
                }

                match call.block() {
                    Some(Node::BlockNode { .. }) => {
                        let block = call.block().unwrap().as_block_node().unwrap();
                        offenses.extend(scan_call_with_block(&call, &block));
                        // Walk receiver and arguments
                        if let Some(recv) = call.receiver() {
                            walk_for_offenses(&recv, offenses);
                        }
                        if let Some(args) = call.arguments() {
                            for arg in args.arguments().iter() {
                                walk_for_offenses(&arg, offenses);
                            }
                        }
                        // Walk block body
                        if let Some(body) = block.body() {
                            walk_for_offenses(&body, offenses);
                        }
                    }
                    _ => {
                        offenses.extend(scan_call(&call));
                        for_each_direct_child(node, &mut |child| {
                            walk_for_offenses(child, offenses);
                        });
                    }
                }
            }
            _ => {
                for_each_direct_child(node, &mut |child| {
                    walk_for_offenses(child, offenses);
                });
            }
        }
    }

    #[test]
    fn shuffle_first() {
        let o = parse_and_collect(b"[].shuffle.first");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::ShuffleFirstVsSample)
        );
    }

    #[test]
    fn reverse_each() {
        let o = parse_and_collect(b"arr.reverse.each { |x| x }");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::ReverseEachVsReverseEach)
        );
    }

    #[test]
    fn keys_each() {
        let o = parse_and_collect(b"h.keys.each { |k| k }");
        assert!(o.iter().any(|x| x.kind == OffenseKind::KeysEachVsEachKey));
    }

    #[test]
    fn keys_with_arg_each_no_fire() {
        let o = parse_and_collect(b"redis.keys('queue:*').each { |q| q }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::KeysEachVsEachKey));
    }

    #[test]
    fn gsub_single_chars() {
        let o = parse_and_collect(b"s.gsub('r', 'k')");
        assert!(o.iter().any(|x| x.kind == OffenseKind::GsubVsTr));
    }

    #[test]
    fn gsub_multi_char_no_fire() {
        let o = parse_and_collect(b"s.gsub('pet', 'fat')");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::GsubVsTr));
    }

    #[test]
    fn fetch_two_args() {
        let o = parse_and_collect(b"h.fetch(:key, [])");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::FetchWithArgumentVsBlock)
        );
    }

    #[test]
    fn fetch_with_block_no_fire() {
        let o = parse_and_collect(b"Rails.cache.fetch('key', expires_in: 1.hour) { compute }");
        assert!(
            !o.iter()
                .any(|x| x.kind == OffenseKind::FetchWithArgumentVsBlock)
        );
    }

    #[test]
    fn merge_bang_single_pair() {
        let o = parse_and_collect(b"h.merge!(item: 1)");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::HashMergeBangVsHashBrackets)
        );
    }

    #[test]
    fn merge_bang_explicit_hash() {
        let o = parse_and_collect(b"h.merge!({item: 1})");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::HashMergeBangVsHashBrackets)
        );
    }

    #[test]
    fn merge_bang_two_pairs_no_fire() {
        let o = parse_and_collect(b"h.merge!(a: 1, b: 2)");
        assert!(
            !o.iter()
                .any(|x| x.kind == OffenseKind::HashMergeBangVsHashBrackets)
        );
    }

    #[test]
    fn each_with_index() {
        let o = parse_and_collect(b"arr.each_with_index { |x, i| x }");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::EachWithIndexVsWhile)
        );
    }

    #[test]
    fn include_on_range() {
        let o = parse_and_collect(b"(1..10).include?(5)");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::IncludeVsCoverOnRange)
        );
    }

    #[test]
    fn sort_with_block() {
        let o = parse_and_collect(b"arr.sort { |a, b| a <=> b }");
        assert!(o.iter().any(|x| x.kind == OffenseKind::SortVsSortBy));
    }

    #[test]
    fn select_first_with_block() {
        let o = parse_and_collect(b"arr.select { |x| x > 1 }.first");
        assert!(o.iter().any(|x| x.kind == OffenseKind::SelectFirstVsDetect));
    }

    #[test]
    fn select_last_with_block() {
        let o = parse_and_collect(b"arr.select { |x| x > 1 }.last");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::SelectLastVsReverseDetect)
        );
    }

    #[test]
    fn map_flatten_one() {
        let o = parse_and_collect(b"arr.map { |e| [e, e] }.flatten(1)");
        assert!(o.iter().any(|x| x.kind == OffenseKind::MapFlattenVsFlatMap));
    }

    #[test]
    fn map_flatten_no_arg_no_fire() {
        let o = parse_and_collect(b"arr.map { |e| [e, e] }.flatten");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::MapFlattenVsFlatMap));
    }

    #[test]
    fn block_vs_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| x.to_s }");
        assert!(o.iter().any(|x| x.kind == OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_with_args_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| x.to_s(16) }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn lambda_no_symbol_to_proc() {
        let o = parse_and_collect(b"->(x) { x.to_s }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn first_not_on_shuffle_no_fire() {
        let o = parse_and_collect(b"arr.first");
        assert!(
            !o.iter()
                .any(|x| x.kind == OffenseKind::ShuffleFirstVsSample)
        );
    }

    #[test]
    fn reverse_not_each_no_fire() {
        let o = parse_and_collect(b"arr.reverse.map { |x| x }");
        assert!(
            !o.iter()
                .any(|x| x.kind == OffenseKind::ReverseEachVsReverseEach)
        );
    }

    #[test]
    fn select_first_with_block_pass() {
        let o = parse_and_collect(b"arr.select(&:odd?).first");
        assert!(o.iter().any(|x| x.kind == OffenseKind::SelectFirstVsDetect));
    }

    #[test]
    fn select_last_with_block_pass() {
        let o = parse_and_collect(b"arr.select(&:odd?).last");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::SelectLastVsReverseDetect)
        );
    }

    #[test]
    fn map_flatten_with_arg_2_no_fire() {
        let o = parse_and_collect(b"arr.map { |e| [e] }.flatten(2)");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::MapFlattenVsFlatMap));
    }

    #[test]
    fn select_first_with_args_no_fire() {
        let o = parse_and_collect(b"arr.select { |x| x > 1 }.first(3)");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::SelectFirstVsDetect));
    }

    #[test]
    fn select_last_with_args_no_fire() {
        let o = parse_and_collect(b"arr.select { |x| x > 1 }.last(3)");
        assert!(
            !o.iter()
                .any(|x| x.kind == OffenseKind::SelectLastVsReverseDetect)
        );
    }

    #[test]
    fn module_eval_with_def_string() {
        let o = parse_and_collect(b"klass.module_eval(\"def foo; end\")");
        assert!(o.iter().any(|x| x.kind == OffenseKind::ModuleEval));
    }

    #[test]
    fn module_eval_without_def_no_fire() {
        let o = parse_and_collect(b"klass.module_eval(\"puts 1\")");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::ModuleEval));
    }

    #[test]
    fn module_eval_non_string_no_fire() {
        let o = parse_and_collect(b"klass.module_eval(some_var)");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::ModuleEval));
    }

    #[test]
    fn module_eval_with_block() {
        let o = parse_and_collect(b"klass.module_eval { define_method(:foo) {} }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::ModuleEval));
    }

    #[test]
    fn block_multiple_args_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.each_with_object([]) { |x, acc| x.to_s }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_no_body_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_receiver_not_lvar_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| @y.to_s }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_receiver_is_primitive_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| 42.to_s }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn hash_merge_bang_no_args_no_fire() {
        let o = parse_and_collect(b"h.merge!");
        assert!(
            !o.iter()
                .any(|x| x.kind == OffenseKind::HashMergeBangVsHashBrackets)
        );
    }

    #[test]
    fn gsub_one_arg_no_fire() {
        let o = parse_and_collect(b"s.gsub('x')");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::GsubVsTr));
    }

    #[test]
    fn fetch_one_arg_no_fire() {
        let o = parse_and_collect(b"h.fetch(:key)");
        assert!(
            !o.iter()
                .any(|x| x.kind == OffenseKind::FetchWithArgumentVsBlock)
        );
    }

    #[test]
    fn include_not_on_range_no_fire() {
        let o = parse_and_collect(b"[1,2,3].include?(5)");
        assert!(
            !o.iter()
                .any(|x| x.kind == OffenseKind::IncludeVsCoverOnRange)
        );
    }

    #[test]
    fn include_on_exclusive_range() {
        let o = parse_and_collect(b"(1...10).include?(5)");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::IncludeVsCoverOnRange)
        );
    }

    #[test]
    fn include_on_parenthesized_range() {
        let o = parse_and_collect(b"(1..10).include?(5)");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::IncludeVsCoverOnRange)
        );
    }

    #[test]
    fn sort_without_block_no_fire() {
        let o = parse_and_collect(b"arr.sort");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::SortVsSortBy));
    }

    #[test]
    fn block_wrong_lvar_name_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| y.to_s }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_with_args_on_outer_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.inject(0) { |x| x.to_s }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn module_eval_with_heredoc_containing_def() {
        let o = parse_and_collect(b"klass.module_eval(<<~RUBY)\n  def foo\n    42\n  end\nRUBY\n");
        assert!(o.iter().any(|x| x.kind == OffenseKind::ModuleEval));
    }

    #[test]
    fn keys_each_with_keys_having_args_no_fire() {
        let o = parse_and_collect(b"h.keys(\"x\").each { |k| k }");
        assert!(!o.iter().any(|x| x.kind == OffenseKind::KeysEachVsEachKey));
    }

    #[test]
    fn each_with_index_without_block_still_fires() {
        let o = parse_and_collect(b"arr.each_with_index");
        assert!(
            o.iter()
                .any(|x| x.kind == OffenseKind::EachWithIndexVsWhile)
        );
    }

    #[test]
    fn fetch_with_block_pass_no_fire() {
        let o = parse_and_collect(b"h.fetch(:key, &block)");
        assert!(
            !o.iter()
                .any(|x| x.kind == OffenseKind::FetchWithArgumentVsBlock)
        );
    }
}
