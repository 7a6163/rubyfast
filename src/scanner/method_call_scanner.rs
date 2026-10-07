use crate::ast_helpers::*;
use crate::fix::Fix;
use crate::offense::{Offense, OffenseKind};

/// Scan a method call (CallNode) that does NOT have a block.
pub fn scan_call(call: &ruby_prism::CallNode<'_>, frozen_string_literals: bool) -> Vec<Offense> {
    let mut offenses = Vec::new();

    check_shuffle_first(call, &mut offenses);
    check_reverse_each(call, &mut offenses);
    check_keys_each(call, &mut offenses);
    check_each_with_index(call, &mut offenses);
    check_include_vs_cover(call, &mut offenses);
    check_gsub_vs_tr(call, &mut offenses);
    check_fetch_with_argument(call, frozen_string_literals, &mut offenses);
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
        let fix = recv_call
            .message_loc()
            .zip(outer.call_operator_loc())
            .map(|(sel_l, dot_l)| {
                Fix::two(
                    sel_l.start_offset(),
                    sel_l.end_offset(),
                    "detect",
                    dot_l.start_offset(),
                    outer.location().end_offset(),
                    "",
                )
            });
        offenses.push(Offense::with_optional_fix(
            OffenseKind::SelectFirstVsDetect,
            outer.location().start_offset(),
            fix,
        ));
    }

    // .select{}.last (no auto-fix)
    if outer_name == b"last" && recv_name == b"select" && arg_count(outer) == 0 {
        offenses.push(Offense::new(
            OffenseKind::SelectLastVsReverseDetect,
            outer.location().start_offset(),
        ));
    }

    // .map{}.flatten(1) → .flat_map{}
    if outer_name == b"flatten"
        && recv_name == b"map"
        && let Some(arg) = first_call_arg(outer)
        && arg_count(outer) == 1
        && is_int_one(&arg)
    {
        let fix = recv_call
            .message_loc()
            .zip(outer.call_operator_loc())
            .map(|(sel_l, dot_l)| {
                Fix::two(
                    sel_l.start_offset(),
                    sel_l.end_offset(),
                    "flat_map",
                    dot_l.start_offset(),
                    outer.location().end_offset(),
                    "",
                )
            });
        offenses.push(Offense::with_optional_fix(
            OffenseKind::MapFlattenVsFlatMap,
            outer.location().start_offset(),
            fix,
        ));
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
    let fix = receiver_as_call(&call.receiver())
        .and_then(|rs| rs.call_operator_loc())
        .map(|dot_l| {
            Fix::single(
                dot_l.start_offset(),
                call.location().end_offset(),
                ".sample",
            )
        });
    offenses.push(Offense::with_optional_fix(
        OffenseKind::ShuffleFirstVsSample,
        call.location().start_offset(),
        fix,
    ));
}

/// `.reverse.each` → `.reverse_each`
fn check_reverse_each(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"each"
        || !receiver_is_call_with_name(&call.receiver(), b"reverse")
    {
        return;
    }
    let fix = receiver_as_call(&call.receiver())
        .and_then(|rs| rs.call_operator_loc())
        .zip(call.message_loc())
        .map(|(dot_l, sel_l)| {
            Fix::single(dot_l.start_offset(), sel_l.end_offset(), ".reverse_each")
        });
    offenses.push(Offense::with_optional_fix(
        OffenseKind::ReverseEachVsReverseEach,
        call.location().start_offset(),
        fix,
    ));
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
        let fix = recv_call
            .call_operator_loc()
            .zip(call.message_loc())
            .map(|(dot_l, sel_l)| {
                Fix::single(dot_l.start_offset(), sel_l.end_offset(), ".each_key")
            });
        offenses.push(Offense::with_optional_fix(
            OffenseKind::KeysEachVsEachKey,
            call.location().start_offset(),
            fix,
        ));
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
        let fix = recv_call
            .message_loc()
            .zip(call.call_operator_loc())
            .map(|(sel_l, dot_l)| {
                Fix::two(
                    sel_l.start_offset(),
                    sel_l.end_offset(),
                    "detect",
                    dot_l.start_offset(),
                    call.location().end_offset(),
                    "",
                )
            });
        offenses.push(Offense::with_optional_fix(
            OffenseKind::SelectFirstVsDetect,
            call.location().start_offset(),
            fix,
        ));
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
    if arg_count(call) != 1 {
        return;
    }
    if !first_call_arg(call).is_some_and(|a| is_int_one(&a)) {
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
///
/// Only numeric-literal ranges get a fix: on other ranges (e.g. `'a'..'z'`) `include?` tests
/// membership while `cover?` compares bounds, so swapping them changes results.
fn check_include_vs_cover(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"include?" {
        return;
    }
    let Some(range) = receiver_range(&call.receiver()) else {
        return;
    };
    let is_number = |n: &ruby_prism::Node<'_>| {
        n.as_integer_node().is_some()
            || n.as_float_node().is_some()
            || n.as_rational_node().is_some()
    };
    let ends = [range.left(), range.right()];
    let numeric = ends.iter().any(Option::is_some) && ends.iter().flatten().all(is_number);
    let fix = call
        .message_loc()
        .filter(|_| numeric)
        .map(|sel_l| Fix::single(sel_l.start_offset(), sel_l.end_offset(), "cover?"));
    offenses.push(Offense::with_optional_fix(
        OffenseKind::IncludeVsCoverOnRange,
        call.location().start_offset(),
        fix,
    ));
}

/// `.gsub("x", "y")` → `.tr("x", "y")` when both args are single-char strings
fn check_gsub_vs_tr(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"gsub" {
        return;
    }
    let Some((first, second)) = call_args_pair(call) else {
        return;
    };
    if is_single_char_string(&first) && is_single_char_string(&second) {
        let fix = call
            .message_loc()
            .map(|sel_l| Fix::single(sel_l.start_offset(), sel_l.end_offset(), "tr"));
        offenses.push(Offense::with_optional_fix(
            OffenseKind::GsubVsTr,
            call.location().start_offset(),
            fix,
        ));
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
fn check_fetch_with_argument(
    call: &ruby_prism::CallNode<'_>,
    frozen_string_literals: bool,
    offenses: &mut Vec<Offense>,
) {
    if call.name().as_slice() != b"fetch" || arg_count(call) != 2 || has_block_pass(call) {
        return;
    }
    // The block form only wins when the default has to be constructed. With a cheap
    // default (nil, a number, a symbol, a constant) the block's invocation cost makes
    // it the slower option — fast-ruby documents this exemption next to the benchmark.
    if let Some((_, default)) = call_args_pair(call)
        && is_cheap_value(&default, frozen_string_literals)
    {
        return;
    }
    offenses.push(Offense::new(
        OffenseKind::FetchWithArgumentVsBlock,
        call.location().start_offset(),
    ));
}

/// `.merge!({k: v})` → `h[k] = v` (single pair hash argument)
fn check_hash_merge_bang(call: &ruby_prism::CallNode<'_>, offenses: &mut Vec<Offense>) {
    if call.name().as_slice() != b"merge!" {
        return;
    }
    if arg_count(call) != 1 {
        return;
    }
    if first_arg_is_single_pair_hash(call) {
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
    if let Some(first_arg) = first_call_arg(call)
        && str_contains_def(&first_arg)
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

    // Block must take exactly one plain argument
    let Some(block_arg_name) = sole_block_arg_name(&block.parameters()) else {
        return;
    };

    // Block body must be a single expression
    let inner_node = match crate::ast_helpers::body_single_expression(block.body()) {
        Some(node) => node,
        None => return,
    };

    let inner_call = match inner_node.as_call_node() {
        Some(c) => c,
        None => return,
    };

    // Inner call must have 0 arguments, no block, and no `&.` (which `&:foo` can't express)
    if arg_count(&inner_call) != 0
        || inner_call.block().is_some()
        || inner_call.is_safe_navigation()
    {
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
    use crate::ast_helpers::test_helpers::leak_parse;
    use crate::offense::has_kind;

    fn parse_and_collect(source: &[u8]) -> Vec<Offense> {
        let result = leak_parse(source);
        let frozen = result
            .magic_comments()
            .any(|c| c.key() == b"frozen_string_literal" && c.value() == b"true");
        crate::analyzer::scan_tree(&result.node(), source, frozen)
    }

    #[test]
    fn shuffle_first() {
        let o = parse_and_collect(b"[].shuffle.first");
        assert!(has_kind(&o, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn reverse_each() {
        let o = parse_and_collect(b"arr.reverse.each { |x| x }");
        assert!(has_kind(&o, OffenseKind::ReverseEachVsReverseEach));
    }

    #[test]
    fn keys_each() {
        let o = parse_and_collect(b"h.keys.each { |k| k }");
        assert!(has_kind(&o, OffenseKind::KeysEachVsEachKey));
    }

    #[test]
    fn keys_with_arg_each_no_fire() {
        let o = parse_and_collect(b"redis.keys('queue:*').each { |q| q }");
        assert!(!has_kind(&o, OffenseKind::KeysEachVsEachKey));
    }

    #[test]
    fn gsub_single_chars() {
        let o = parse_and_collect(b"s.gsub('r', 'k')");
        assert!(has_kind(&o, OffenseKind::GsubVsTr));
    }

    #[test]
    fn gsub_multi_char_no_fire() {
        let o = parse_and_collect(b"s.gsub('pet', 'fat')");
        assert!(!has_kind(&o, OffenseKind::GsubVsTr));
    }

    #[test]
    fn fetch_two_args() {
        let o = parse_and_collect(b"h.fetch(:key, [])");
        assert!(has_kind(&o, OffenseKind::FetchWithArgumentVsBlock));
    }

    #[test]
    fn select_first_and_last_with_args_no_fire() {
        // `first`/`last` taking an argument returns a slice, not one element.
        let o = parse_and_collect(b"arr.select(&:even?).first(2)");
        assert!(!has_kind(&o, OffenseKind::SelectFirstVsDetect));
        let o = parse_and_collect(b"arr.select(&:even?).last(2)");
        assert!(!has_kind(&o, OffenseKind::SelectLastVsReverseDetect));
    }

    #[test]
    fn fetch_cheap_defaults_no_fire() {
        for source in [
            &b"ENV.fetch(\"TOKEN\", nil)"[..],
            b"h.fetch(:k, 0)",
            b"h.fetch(:k, 1.5)",
            b"h.fetch(:k, :missing)",
            b"h.fetch(:k, true)",
            b"h.fetch(:k, false)",
            b"h.fetch(:k, DEFAULT)",
            b"h.fetch(:k, Foo::DEFAULT)",
            b"x = compute\nh.fetch(:k, x)",
            b"h.fetch(:k, @default)",
            b"h.fetch(:k, @@default)",
            b"h.fetch(:k, $default)",
        ] {
            let o = parse_and_collect(source);
            assert!(
                !has_kind(&o, OffenseKind::FetchWithArgumentVsBlock),
                "cheap default should not fire"
            );
        }
    }

    #[test]
    fn fetch_string_default_fires_without_the_frozen_magic_comment() {
        let o = parse_and_collect(b"h.fetch(:k, \"fallback\")");
        assert!(has_kind(&o, OffenseKind::FetchWithArgumentVsBlock));
    }

    #[test]
    fn fetch_string_default_no_fire_under_frozen_string_literal() {
        // A plain literal is a frozen, deduplicated value here — nothing to build.
        let o = parse_and_collect(b"# frozen_string_literal: true\nENV.fetch(\"PORT\", \"3000\")");
        assert!(!has_kind(&o, OffenseKind::FetchWithArgumentVsBlock));
    }

    #[test]
    fn fetch_interpolated_default_still_fires_under_frozen_string_literal() {
        // Interpolation builds a new string on every call, magic comment or not.
        let o = parse_and_collect(b"# frozen_string_literal: true\nh.fetch(:k, \"sum+#{name}\")");
        assert!(has_kind(&o, OffenseKind::FetchWithArgumentVsBlock));
    }

    #[test]
    fn fetch_collection_default_still_fires_under_frozen_string_literal() {
        let o = parse_and_collect(b"# frozen_string_literal: true\nh.fetch(:k, [])");
        assert!(has_kind(&o, OffenseKind::FetchWithArgumentVsBlock));
    }

    #[test]
    fn fetch_constructed_defaults_fire() {
        for source in [
            &b"h.fetch(:k, \"str\")"[..],
            b"h.fetch(:k, [])",
            b"h.fetch(:k, {})",
            b"h.fetch(:k, Time.now)",
            b"h.fetch(:k, compute_default)",
            b"h.fetch(:k, 1..10)",
        ] {
            let o = parse_and_collect(source);
            assert!(
                has_kind(&o, OffenseKind::FetchWithArgumentVsBlock),
                "constructed default should fire"
            );
        }
    }

    #[test]
    fn fetch_with_block_no_fire() {
        let o = parse_and_collect(b"Rails.cache.fetch('key', expires_in: 1.hour) { compute }");
        assert!(!has_kind(&o, OffenseKind::FetchWithArgumentVsBlock));
    }

    #[test]
    fn merge_bang_single_pair() {
        let o = parse_and_collect(b"h.merge!(item: 1)");
        assert!(has_kind(&o, OffenseKind::HashMergeBangVsHashBrackets));
    }

    #[test]
    fn merge_bang_explicit_hash() {
        let o = parse_and_collect(b"h.merge!({item: 1})");
        assert!(has_kind(&o, OffenseKind::HashMergeBangVsHashBrackets));
    }

    #[test]
    fn merge_bang_two_pairs_no_fire() {
        let o = parse_and_collect(b"h.merge!(a: 1, b: 2)");
        assert!(!has_kind(&o, OffenseKind::HashMergeBangVsHashBrackets));
    }

    #[test]
    fn each_with_index() {
        let o = parse_and_collect(b"arr.each_with_index { |x, i| x }");
        assert!(has_kind(&o, OffenseKind::EachWithIndexVsWhile));
    }

    #[test]
    fn include_on_range() {
        let o = parse_and_collect(b"(1..10).include?(5)");
        assert!(has_kind(&o, OffenseKind::IncludeVsCoverOnRange));
    }

    #[test]
    fn sort_with_block() {
        let o = parse_and_collect(b"arr.sort { |a, b| a <=> b }");
        assert!(has_kind(&o, OffenseKind::SortVsSortBy));
    }

    #[test]
    fn select_first_with_block() {
        let o = parse_and_collect(b"arr.select { |x| x > 1 }.first");
        assert!(has_kind(&o, OffenseKind::SelectFirstVsDetect));
    }

    #[test]
    fn select_last_with_block() {
        let o = parse_and_collect(b"arr.select { |x| x > 1 }.last");
        assert!(has_kind(&o, OffenseKind::SelectLastVsReverseDetect));
    }

    #[test]
    fn map_flatten_one() {
        let o = parse_and_collect(b"arr.map { |e| [e, e] }.flatten(1)");
        assert!(has_kind(&o, OffenseKind::MapFlattenVsFlatMap));
    }

    #[test]
    fn map_flatten_no_arg_no_fire() {
        let o = parse_and_collect(b"arr.map { |e| [e, e] }.flatten");
        assert!(!has_kind(&o, OffenseKind::MapFlattenVsFlatMap));
    }

    #[test]
    fn block_vs_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| x.to_s }");
        assert!(has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_with_args_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| x.to_s(16) }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn lambda_no_symbol_to_proc() {
        let o = parse_and_collect(b"->(x) { x.to_s }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn first_not_on_shuffle_no_fire() {
        let o = parse_and_collect(b"arr.first");
        assert!(!has_kind(&o, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn reverse_not_each_no_fire() {
        let o = parse_and_collect(b"arr.reverse.map { |x| x }");
        assert!(!has_kind(&o, OffenseKind::ReverseEachVsReverseEach));
    }

    #[test]
    fn select_first_with_block_pass() {
        let o = parse_and_collect(b"arr.select(&:odd?).first");
        assert!(has_kind(&o, OffenseKind::SelectFirstVsDetect));
    }

    #[test]
    fn select_last_with_block_pass() {
        let o = parse_and_collect(b"arr.select(&:odd?).last");
        assert!(has_kind(&o, OffenseKind::SelectLastVsReverseDetect));
    }

    #[test]
    fn map_flatten_with_arg_2_no_fire() {
        let o = parse_and_collect(b"arr.map { |e| [e] }.flatten(2)");
        assert!(!has_kind(&o, OffenseKind::MapFlattenVsFlatMap));
    }

    #[test]
    fn select_first_with_args_no_fire() {
        let o = parse_and_collect(b"arr.select { |x| x > 1 }.first(3)");
        assert!(!has_kind(&o, OffenseKind::SelectFirstVsDetect));
    }

    #[test]
    fn select_last_with_args_no_fire() {
        let o = parse_and_collect(b"arr.select { |x| x > 1 }.last(3)");
        assert!(!has_kind(&o, OffenseKind::SelectLastVsReverseDetect));
    }

    #[test]
    fn module_eval_with_def_string() {
        let o = parse_and_collect(b"klass.module_eval(\"def foo; end\")");
        assert!(has_kind(&o, OffenseKind::ModuleEval));
    }

    #[test]
    fn module_eval_without_def_no_fire() {
        let o = parse_and_collect(b"klass.module_eval(\"puts 1\")");
        assert!(!has_kind(&o, OffenseKind::ModuleEval));
    }

    #[test]
    fn module_eval_non_string_no_fire() {
        let o = parse_and_collect(b"klass.module_eval(some_var)");
        assert!(!has_kind(&o, OffenseKind::ModuleEval));
    }

    #[test]
    fn module_eval_with_block() {
        let o = parse_and_collect(b"klass.module_eval { define_method(:foo) {} }");
        assert!(!has_kind(&o, OffenseKind::ModuleEval));
    }

    #[test]
    fn block_multiple_args_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.each_with_object([]) { |x, acc| x.to_s }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_no_body_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_receiver_not_lvar_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| @y.to_s }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_receiver_is_primitive_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| 42.to_s }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn hash_merge_bang_no_args_no_fire() {
        let o = parse_and_collect(b"h.merge!");
        assert!(!has_kind(&o, OffenseKind::HashMergeBangVsHashBrackets));
    }

    #[test]
    fn gsub_one_arg_no_fire() {
        let o = parse_and_collect(b"s.gsub('x')");
        assert!(!has_kind(&o, OffenseKind::GsubVsTr));
    }

    #[test]
    fn fetch_one_arg_no_fire() {
        let o = parse_and_collect(b"h.fetch(:key)");
        assert!(!has_kind(&o, OffenseKind::FetchWithArgumentVsBlock));
    }

    #[test]
    fn include_not_on_range_no_fire() {
        let o = parse_and_collect(b"[1,2,3].include?(5)");
        assert!(!has_kind(&o, OffenseKind::IncludeVsCoverOnRange));
    }

    #[test]
    fn include_on_exclusive_range() {
        let o = parse_and_collect(b"(1...10).include?(5)");
        assert!(has_kind(&o, OffenseKind::IncludeVsCoverOnRange));
    }

    fn include_fix(src: &[u8]) -> Option<bool> {
        parse_and_collect(src)
            .iter()
            .find(|o| o.kind == OffenseKind::IncludeVsCoverOnRange)
            .map(|o| o.fix.is_some())
    }

    #[test]
    fn include_on_numeric_range_is_fixable() {
        assert_eq!(include_fix(b"(1..10).include?(5)"), Some(true));
        assert_eq!(include_fix(b"(1.0...2.5).include?(x)"), Some(true));
        assert_eq!(include_fix(b"(-1..1r).include?(x)"), Some(true));
        assert_eq!(include_fix(b"(1..).include?(x)"), Some(true));
        assert_eq!(include_fix(b"(..10).include?(x)"), Some(true));
    }

    #[test]
    fn include_on_non_numeric_range_fires_without_fix() {
        // ('a'..'z').include?('bb') is false but cover?('bb') is true.
        assert_eq!(include_fix(b"('a'..'z').include?('bb')"), Some(false));
        assert_eq!(include_fix(b"(a..b).include?(x)"), Some(false));
        assert_eq!(include_fix(b"(1..n).include?(x)"), Some(false));
        assert_eq!(include_fix(b"(nil..nil).include?(x)"), Some(false));
    }

    #[test]
    fn include_on_parenthesized_range() {
        let o = parse_and_collect(b"(1..10).include?(5)");
        assert!(has_kind(&o, OffenseKind::IncludeVsCoverOnRange));
    }

    #[test]
    fn sort_without_block_no_fire() {
        let o = parse_and_collect(b"arr.sort");
        assert!(!has_kind(&o, OffenseKind::SortVsSortBy));
    }

    #[test]
    fn block_with_extra_params_no_symbol_to_proc() {
        // `|x,|` destructures; the others take more than one argument.
        for src in [
            "pairs.map { |x,| x.foo }",
            "arr.map { |x, *r| x.foo }",
            "arr.map { |x, y = 1| x.foo }",
            "arr.map { |x, &b| x.foo }",
        ] {
            let o = parse_and_collect(src.as_bytes());
            assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc), "{src}");
        }
    }

    #[test]
    fn safe_navigation_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| x&.foo }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_wrong_lvar_name_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| y.to_s }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_with_args_on_outer_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.inject(0) { |x| x.to_s }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn module_eval_with_heredoc_containing_def() {
        let o = parse_and_collect(b"klass.module_eval(<<~RUBY)\n  def foo\n    42\n  end\nRUBY\n");
        assert!(has_kind(&o, OffenseKind::ModuleEval));
    }

    #[test]
    fn keys_each_with_keys_having_args_no_fire() {
        let o = parse_and_collect(b"h.keys(\"x\").each { |k| k }");
        assert!(!has_kind(&o, OffenseKind::KeysEachVsEachKey));
    }

    #[test]
    fn each_with_index_without_block_still_fires() {
        let o = parse_and_collect(b"arr.each_with_index");
        assert!(has_kind(&o, OffenseKind::EachWithIndexVsWhile));
    }

    #[test]
    fn map_flatten_with_block_pass() {
        let o = parse_and_collect(b"arr.map(&:to_a).flatten(1)");
        assert!(has_kind(&o, OffenseKind::MapFlattenVsFlatMap));
    }

    #[test]
    fn map_flatten_with_full_block_fires_via_chain() {
        let o = parse_and_collect(b"arr.map { |x| [x] }.flatten(1)");
        assert!(has_kind(&o, OffenseKind::MapFlattenVsFlatMap));
    }

    #[test]
    fn map_flatten_no_flatten_arg_via_chain_no_fire() {
        let o = parse_and_collect(b"arr.map { |x| [x] }.flatten");
        assert!(!has_kind(&o, OffenseKind::MapFlattenVsFlatMap));
    }

    #[test]
    fn reverse_each_with_block() {
        let o = parse_and_collect(b"arr.reverse.each { |x| puts x }");
        assert!(has_kind(&o, OffenseKind::ReverseEachVsReverseEach));
    }

    #[test]
    fn gsub_with_block_single_chars() {
        let o = parse_and_collect(b"s.gsub('r', 'k') { |m| m }");
        // gsub with single chars still fires even with block (scan_call_with_block calls check_gsub_vs_tr)
        assert!(has_kind(&o, OffenseKind::GsubVsTr));
    }

    #[test]
    fn each_with_index_with_block() {
        let o = parse_and_collect(b"arr.each_with_index { |item, idx| puts idx }");
        assert!(has_kind(&o, OffenseKind::EachWithIndexVsWhile));
    }

    #[test]
    fn include_on_range_with_block() {
        // include? on range fires via scan_call_with_block path too
        let o = parse_and_collect(b"(1..10).include?(5)");
        assert!(has_kind(&o, OffenseKind::IncludeVsCoverOnRange));
    }

    #[test]
    fn shuffle_first_with_block() {
        let o = parse_and_collect(b"[].shuffle.first { 0 }");
        assert!(has_kind(&o, OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn keys_each_with_block() {
        let o = parse_and_collect(b"h.keys.each { |k| puts k }");
        assert!(has_kind(&o, OffenseKind::KeysEachVsEachKey));
    }

    #[test]
    fn hash_merge_bang_with_block() {
        let o = parse_and_collect(b"h.merge!(item: 1) { |k, v1, v2| v1 }");
        assert!(has_kind(&o, OffenseKind::HashMergeBangVsHashBrackets));
    }

    #[test]
    fn module_eval_with_block_and_def_string() {
        // module_eval with a def string arg AND a block should fire (scan_call_with_block checks module_eval)
        let o = parse_and_collect(b"klass.module_eval(\"def foo; end\") { }");
        assert!(has_kind(&o, OffenseKind::ModuleEval));
    }

    #[test]
    fn block_multiple_body_stmts_no_symbol_to_proc() {
        // Block with 1 arg but multiple statements in body → early return
        let o = parse_and_collect(b"arr.map { |x| puts x; x.to_s }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_no_params_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { 42 }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_body_not_call_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| x }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_inner_call_has_block_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| x.foo { 1 } }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn block_inner_call_no_receiver_no_symbol_to_proc() {
        let o = parse_and_collect(b"arr.map { |x| puts }");
        assert!(!has_kind(&o, OffenseKind::BlockVsSymbolToProc));
    }

    #[test]
    fn select_last_via_chain_with_args_no_fire() {
        let o = parse_and_collect(b"arr.select { |x| x }.last(3)");
        assert!(!has_kind(&o, OffenseKind::SelectLastVsReverseDetect));
    }

    #[test]
    fn fetch_with_block_pass_no_fire() {
        let o = parse_and_collect(b"h.fetch(:key, &block)");
        assert!(!has_kind(&o, OffenseKind::FetchWithArgumentVsBlock));
    }
}
