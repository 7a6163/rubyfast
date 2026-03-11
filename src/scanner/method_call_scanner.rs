use lib_ruby_parser::Node;
use lib_ruby_parser::nodes::{Block, Send};

use crate::ast_helpers::*;
use crate::fix::Fix;
use crate::offense::{Offense, OffenseKind};

/// Scan a method call (Send node) that is NOT inside a Block.
pub fn scan_send(send: &Send) -> Vec<Offense> {
    let mut offenses = Vec::new();

    check_shuffle_first(send, &mut offenses);
    check_reverse_each(send, &mut offenses);
    check_keys_each(send, &mut offenses);
    check_each_with_index(send, &mut offenses);
    check_include_vs_cover(send, &mut offenses);
    check_gsub_vs_tr(send, &mut offenses);
    check_fetch_with_argument(send, &mut offenses);
    check_hash_merge_bang(send, &mut offenses);
    check_map_flatten(send, &mut offenses);
    check_select_first(send, &mut offenses);
    check_select_last(send, &mut offenses);
    check_module_eval_send(send, &mut offenses);

    offenses
}

/// Scan a Block node (method call + block).
pub fn scan_block(block: &Block) -> Vec<Offense> {
    let mut offenses = Vec::new();

    let send = match block_call_as_send(block) {
        Some(s) => s,
        None => return offenses,
    };

    // Checks that only apply when a block is present
    check_sort_vs_sort_by(send, &mut offenses);
    check_module_eval_send(send, &mut offenses);
    check_block_vs_symbol_to_proc(send, block, &mut offenses);

    // Chain checks where receiver might be a block call
    // e.g., .select{}.first — the .first Send wraps the Block
    // These are actually checked on the outer Send whose receiver is this Block.
    // But we also run the send-level checks on the call inside the block.
    check_shuffle_first(send, &mut offenses);
    check_reverse_each(send, &mut offenses);
    check_keys_each(send, &mut offenses);
    check_each_with_index(send, &mut offenses);
    check_include_vs_cover(send, &mut offenses);
    check_gsub_vs_tr(send, &mut offenses);
    // NOTE: check_fetch_with_argument is intentionally excluded here.
    // If fetch already has a block, the rule doesn't apply.
    check_hash_merge_bang(send, &mut offenses);

    offenses
}

/// Scan a Send whose receiver is a Block node.
/// This handles chains like `.select { }.first` where .first's receiver is a Block.
pub fn scan_send_on_block(send: &Send, recv_block: &Block) -> Vec<Offense> {
    let mut offenses = Vec::new();

    let recv_send = match block_call_as_send(recv_block) {
        Some(s) => s,
        None => return offenses,
    };

    // .select{}.first → .detect{}
    if send.method_name == "first"
        && recv_send.method_name == "select"
        && arg_count_without_block_pass(&send.args) == 0
    {
        let offense = match (recv_send.selector_l.as_ref(), send.dot_l.as_ref()) {
            (Some(sel_l), Some(dot_l)) => {
                let fix = Fix::two(
                    sel_l.begin,
                    sel_l.end,
                    "detect",
                    dot_l.begin,
                    send.expression_l.end,
                    "",
                );
                Offense::with_fix(
                    OffenseKind::SelectFirstVsDetect,
                    send.expression_l.begin,
                    fix,
                )
            }
            _ => Offense::new(OffenseKind::SelectFirstVsDetect, send.expression_l.begin),
        };
        offenses.push(offense);
    }

    // .select{}.last (no auto-fix — transform to .reverse.detect is too risky)
    if send.method_name == "last"
        && recv_send.method_name == "select"
        && arg_count_without_block_pass(&send.args) == 0
    {
        offenses.push(Offense::new(
            OffenseKind::SelectLastVsReverseDetect,
            send.expression_l.begin,
        ));
    }

    // .map{}.flatten(1) → .flat_map{}
    if send.method_name == "flatten"
        && recv_send.method_name == "map"
        && send.args.len() == 1
        && is_int_one(&send.args[0])
    {
        let offense = match (recv_send.selector_l.as_ref(), send.dot_l.as_ref()) {
            (Some(sel_l), Some(dot_l)) => {
                let fix = Fix::two(
                    sel_l.begin,
                    sel_l.end,
                    "flat_map",
                    dot_l.begin,
                    send.expression_l.end,
                    "",
                );
                Offense::with_fix(
                    OffenseKind::MapFlattenVsFlatMap,
                    send.expression_l.begin,
                    fix,
                )
            }
            _ => Offense::new(OffenseKind::MapFlattenVsFlatMap, send.expression_l.begin),
        };
        offenses.push(offense);
    }

    offenses
}

// --- Individual offense checks ---

/// `.shuffle.first` → `.sample`
fn check_shuffle_first(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "first" || !receiver_is_send_with_name(&send.recv, "shuffle") {
        return;
    }
    let offense = match receiver_as_send(&send.recv).and_then(|rs| rs.dot_l.as_ref()) {
        Some(dot_l) => {
            let fix = Fix::single(dot_l.begin, send.expression_l.end, ".sample");
            Offense::with_fix(
                OffenseKind::ShuffleFirstVsSample,
                send.expression_l.begin,
                fix,
            )
        }
        None => Offense::new(OffenseKind::ShuffleFirstVsSample, send.expression_l.begin),
    };
    offenses.push(offense);
}

/// `.reverse.each` → `.reverse_each`
fn check_reverse_each(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "each" || !receiver_is_send_with_name(&send.recv, "reverse") {
        return;
    }
    let offense = match (
        receiver_as_send(&send.recv).and_then(|rs| rs.dot_l.as_ref()),
        send.selector_l.as_ref(),
    ) {
        (Some(dot_l), Some(sel_l)) => {
            let fix = Fix::single(dot_l.begin, sel_l.end, ".reverse_each");
            Offense::with_fix(
                OffenseKind::ReverseEachVsReverseEach,
                send.expression_l.begin,
                fix,
            )
        }
        _ => Offense::new(
            OffenseKind::ReverseEachVsReverseEach,
            send.expression_l.begin,
        ),
    };
    offenses.push(offense);
}

/// `.keys.each` → `.each_key` (keys must have 0 args)
fn check_keys_each(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "each" {
        return;
    }
    if let Some(recv_send) = receiver_as_send(&send.recv)
        && recv_send.method_name == "keys"
        && recv_send.args.is_empty()
    {
        let offense = match (recv_send.dot_l.as_ref(), send.selector_l.as_ref()) {
            (Some(dot_l), Some(sel_l)) => {
                let fix = Fix::single(dot_l.begin, sel_l.end, ".each_key");
                Offense::with_fix(OffenseKind::KeysEachVsEachKey, send.expression_l.begin, fix)
            }
            _ => Offense::new(OffenseKind::KeysEachVsEachKey, send.expression_l.begin),
        };
        offenses.push(offense);
    }
}

/// `.select{}.first` → `.detect{}` (when receiver is a plain Send, not Block)
fn check_select_first(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "first" || arg_count_without_block_pass(&send.args) != 0 {
        return;
    }
    if let Some(recv_send) = receiver_as_send(&send.recv)
        && recv_send.method_name == "select"
        && has_block_pass(&recv_send.args)
    {
        let offense = match (recv_send.selector_l.as_ref(), send.dot_l.as_ref()) {
            (Some(sel_l), Some(dot_l)) => {
                let fix = Fix::two(
                    sel_l.begin,
                    sel_l.end,
                    "detect",
                    dot_l.begin,
                    send.expression_l.end,
                    "",
                );
                Offense::with_fix(
                    OffenseKind::SelectFirstVsDetect,
                    send.expression_l.begin,
                    fix,
                )
            }
            _ => Offense::new(OffenseKind::SelectFirstVsDetect, send.expression_l.begin),
        };
        offenses.push(offense);
    }
}

/// `.select{}.last` → `.reverse.detect{}` (when receiver is a plain Send)
fn check_select_last(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "last" || arg_count_without_block_pass(&send.args) != 0 {
        return;
    }
    if let Some(recv_send) = receiver_as_send(&send.recv)
        && recv_send.method_name == "select"
        && has_block_pass(&recv_send.args)
    {
        offenses.push(Offense::new(
            OffenseKind::SelectLastVsReverseDetect,
            send.expression_l.begin,
        ));
    }
}

/// `.map{}.flatten(1)` → `.flat_map{}` (when receiver is a plain Send)
fn check_map_flatten(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "flatten" || send.args.len() != 1 || !is_int_one(&send.args[0]) {
        return;
    }
    if receiver_is_send_with_name(&send.recv, "map") {
        offenses.push(Offense::new(
            OffenseKind::MapFlattenVsFlatMap,
            send.expression_l.begin,
        ));
    }
}

/// `.each_with_index` → while loop
fn check_each_with_index(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name == "each_with_index" {
        offenses.push(Offense::new(
            OffenseKind::EachWithIndexVsWhile,
            send.expression_l.begin,
        ));
    }
}

/// `(1..10).include?` → `.cover?`
fn check_include_vs_cover(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "include?" || !receiver_is_range(&send.recv) {
        return;
    }
    let offense = match send.selector_l.as_ref() {
        Some(sel_l) => {
            let fix = Fix::single(sel_l.begin, sel_l.end, "cover?");
            Offense::with_fix(
                OffenseKind::IncludeVsCoverOnRange,
                send.expression_l.begin,
                fix,
            )
        }
        None => Offense::new(OffenseKind::IncludeVsCoverOnRange, send.expression_l.begin),
    };
    offenses.push(offense);
}

/// `.gsub("x", "y")` → `.tr("x", "y")` when both args are single-char strings
fn check_gsub_vs_tr(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "gsub" || send.args.len() != 2 {
        return;
    }
    if is_single_char_string(&send.args[0]) && is_single_char_string(&send.args[1]) {
        let offense = match send.selector_l.as_ref() {
            Some(sel_l) => {
                let fix = Fix::single(sel_l.begin, sel_l.end, "tr");
                Offense::with_fix(OffenseKind::GsubVsTr, send.expression_l.begin, fix)
            }
            None => Offense::new(OffenseKind::GsubVsTr, send.expression_l.begin),
        };
        offenses.push(offense);
    }
}

/// `.sort { |a, b| ... }` → `.sort_by` (only fires when sort has a block)
fn check_sort_vs_sort_by(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name == "sort" {
        offenses.push(Offense::new(
            OffenseKind::SortVsSortBy,
            send.expression_l.begin,
        ));
    }
}

/// `.fetch(k, v)` → `.fetch(k) { v }`
fn check_fetch_with_argument(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name == "fetch"
        && arg_count_without_block_pass(&send.args) == 2
        && !has_block_pass(&send.args)
    {
        offenses.push(Offense::new(
            OffenseKind::FetchWithArgumentVsBlock,
            send.expression_l.begin,
        ));
    }
}

/// `.merge!({k: v})` → `h[k] = v` (single pair hash argument)
fn check_hash_merge_bang(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "merge!" || send.args.len() != 1 {
        return;
    }
    if first_arg_is_single_pair_hash(&send.args) {
        offenses.push(Offense::new(
            OffenseKind::HashMergeBangVsHashBrackets,
            send.expression_l.begin,
        ));
    }
}

/// `.module_eval("def ...")` → `define_method`
fn check_module_eval_send(send: &Send, offenses: &mut Vec<Offense>) {
    if send.method_name != "module_eval" {
        return;
    }
    if let Some(first_arg) = send.args.first()
        && str_contains_def(first_arg)
    {
        offenses.push(Offense::new(
            OffenseKind::ModuleEval,
            send.expression_l.begin,
        ));
    }
}

/// `.map { |x| x.foo }` → `.map(&:foo)`
fn check_block_vs_symbol_to_proc(send: &Send, block: &Block, offenses: &mut Vec<Offense>) {
    // Must not be a lambda literal
    if matches!(block.call.as_ref(), Node::Lambda(_)) {
        return;
    }

    // Outer method call must have 0 non-block-pass arguments
    if arg_count_without_block_pass(&send.args) != 0 {
        return;
    }

    // Block must have exactly 1 argument
    let arg_names = block_arg_names(&block.args);
    if arg_names.len() != 1 {
        return;
    }
    let block_arg_name = &arg_names[0];

    // Block body must be a single Send node
    let body = match block.body.as_deref() {
        Some(node) => node,
        None => return,
    };

    let inner_send = match body {
        Node::Send(s) => s,
        _ => return,
    };

    // Inner call must have 0 arguments and no block
    if !inner_send.args.is_empty() {
        return;
    }

    // Inner call must have a receiver
    let receiver = match inner_send.recv.as_deref() {
        Some(r) => r,
        None => return,
    };

    // Receiver must not be a primitive
    if is_primitive(receiver) {
        return;
    }

    // Receiver must be an Lvar matching the block argument name
    if let Node::Lvar(lv) = receiver
        && lv.name == *block_arg_name
    {
        offenses.push(Offense::new(
            OffenseKind::BlockVsSymbolToProc,
            send.expression_l.begin,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast_helpers::node_children;

    fn parse_and_collect(source: &[u8]) -> Vec<Offense> {
        let result = lib_ruby_parser::Parser::new(source.to_vec(), Default::default()).do_parse();
        let mut offenses = Vec::new();
        if let Some(ast) = result.ast {
            walk_for_offenses(&ast, &mut offenses);
        }
        offenses
    }

    /// Walk AST matching real analyzer behavior: Block's inner Send is NOT
    /// visited by scan_send (only scan_block handles it).
    fn walk_for_offenses(node: &Node, offenses: &mut Vec<Offense>) {
        match node {
            Node::Send(s) => {
                if let Some(Node::Block(recv_block)) = s.recv.as_deref() {
                    offenses.extend(scan_send_on_block(s, recv_block));
                }
                offenses.extend(scan_send(s));
                for child in node_children(node) {
                    walk_for_offenses(child, offenses);
                }
            }
            Node::Block(b) => {
                offenses.extend(scan_block(b));
                if let Node::Send(s) = b.call.as_ref() {
                    if let Some(recv) = &s.recv {
                        walk_for_offenses(recv, offenses);
                    }
                    for arg in &s.args {
                        walk_for_offenses(arg, offenses);
                    }
                }
                if let Some(args) = &b.args {
                    walk_for_offenses(args, offenses);
                }
                if let Some(body) = &b.body {
                    walk_for_offenses(body, offenses);
                }
            }
            _ => {
                for child in node_children(node) {
                    walk_for_offenses(child, offenses);
                }
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

    // --- Additional edge case tests ---

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
        // Empty block body
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
        // Single-paren range — parsed as Begin(Irange)
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
        // keys("x") is not Hash#keys — should not fire
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
