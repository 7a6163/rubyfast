use ruby_prism::{Node, Visit};

use crate::fix::Fix;
use crate::offense::{Offense, OffenseKind};

/// Any `for` loop emits an offense — prefer `.each`.
pub fn scan(node: &ruby_prism::ForNode<'_>, source: &[u8]) -> Vec<Offense> {
    let fix = build_fix(node, source);
    vec![Offense::with_optional_fix(
        OffenseKind::ForLoopVsEach,
        node.for_keyword_loc().start_offset(),
        fix,
    )]
}

/// Build a fix that transforms `for x in arr` → `arr.each do |x|`.
///
/// Only offered when the rewrite keeps semantics: `for` shares the enclosing scope while a
/// block opens a new one, so the fix is skipped if the body assigns any local or the loop
/// variable is mentioned anywhere outside the loop.
fn build_fix(node: &ruby_prism::ForNode<'_>, source: &[u8]) -> Option<Fix> {
    let names = loop_var_names(&node.index())?;
    let loop_loc = node.location();
    if names
        .iter()
        .any(|n| mentioned_outside(source, n, loop_loc.start_offset(), loop_loc.end_offset()))
    {
        return None;
    }
    if let Some(body) = node.statements() {
        let mut v = LocalWriteFinder(false);
        v.visit(&body.as_node());
        if v.0 {
            return None;
        }
    }

    let iterator = names
        .iter()
        .map(|n| String::from_utf8_lossy(n))
        .collect::<Vec<_>>()
        .join(", ");
    let collection = node.collection();
    let coll_loc = collection.location();
    let coll_text = extract_trimmed(source, coll_loc.start_offset(), coll_loc.end_offset())?;
    let iteratee = if is_safe_receiver(&collection) {
        coll_text
    } else {
        format!("({})", coll_text)
    };

    // Replace through `do`, or through a `;` right after the collection; otherwise stop at
    // the collection so trailing comments stay put.
    let header_end = match node.do_keyword_loc() {
        Some(do_loc) => do_loc.end_offset(),
        None => {
            let end = coll_loc.end_offset()
                + source[coll_loc.end_offset()..]
                    .iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count();
            if source.get(end) == Some(&b';') {
                end + 1
            } else {
                coll_loc.end_offset()
            }
        }
    };

    let new_header = format!("{}.each do |{}|", iteratee, iterator);
    Some(Fix::single(
        node.for_keyword_loc().start_offset(),
        header_end,
        new_header,
    ))
}

/// Names bound by the loop index, if every target is a plain local (`x` or `a, b`).
fn loop_var_names(index: &Node<'_>) -> Option<Vec<Vec<u8>>> {
    if let Some(t) = index.as_local_variable_target_node() {
        return Some(vec![t.name().as_slice().to_vec()]);
    }
    let multi = index.as_multi_target_node()?;
    if multi.rest().is_some() || multi.rights().iter().next().is_some() {
        return None;
    }
    multi
        .lefts()
        .iter()
        .map(|n| {
            n.as_local_variable_target_node()
                .map(|t| t.name().as_slice().to_vec())
        })
        .collect()
}

/// Whether `name` appears as a whole identifier outside `start..end`. Purely textual, so it
/// errs toward "yes" (comments, strings, method names) — that only costs a skipped fix.
fn mentioned_outside(source: &[u8], name: &[u8], start: usize, end: usize) -> bool {
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80;
    source.windows(name.len()).enumerate().any(|(i, w)| {
        w == name
            && !(start..end).contains(&i)
            && (i == 0 || !is_ident(source[i - 1]))
            && source.get(i + name.len()).is_none_or(|&b| !is_ident(b))
    })
}

/// Expressions that can take `.each` without parentheses changing what it binds to.
fn is_safe_receiver(node: &Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let identifier_name = call
            .name()
            .as_slice()
            .first()
            .is_some_and(|&b| b.is_ascii_alphabetic() || b == b'_' || b >= 0x80);
        return identifier_name && (call.arguments().is_none() || call.opening_loc().is_some());
    }
    node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
        || node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
        || node.as_array_node().is_some()
        || node.as_parentheses_node().is_some()
        || node.as_self_node().is_some()
}

/// Flags any assignment to a local variable.
struct LocalWriteFinder(bool);

impl LocalWriteFinder {
    fn check(&mut self, node: &Node<'_>) {
        self.0 |= node.as_local_variable_write_node().is_some()
            || node.as_local_variable_target_node().is_some()
            || node.as_local_variable_operator_write_node().is_some()
            || node.as_local_variable_or_write_node().is_some()
            || node.as_local_variable_and_write_node().is_some();
    }
}

impl<'pr> Visit<'pr> for LocalWriteFinder {
    fn visit_branch_node_enter(&mut self, node: Node<'pr>) {
        self.check(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: Node<'pr>) {
        self.check(&node);
    }
}

/// Extract a trimmed UTF-8 string from a byte range. Returns None if not valid UTF-8.
fn extract_trimmed(source: &[u8], start: usize, end: usize) -> Option<String> {
    if start >= end || end > source.len() {
        return None;
    }
    String::from_utf8(source[start..end].to_vec())
        .ok()
        .map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_helpers::test_helpers::leak_parse;

    fn parse_first_for(source: &'static [u8]) -> ruby_prism::ForNode<'static> {
        let result = leak_parse(source);
        let program = result.node();
        let prog = program.as_program_node().unwrap();
        prog.statements()
            .body()
            .iter()
            .find_map(|n| n.as_for_node())
            .unwrap()
    }

    #[test]
    fn for_loop_always_fires() {
        let source = b"for x in [1,2,3]; end";
        let f = parse_first_for(source);
        let offenses = scan(&f, source);
        assert_eq!(offenses.len(), 1);
        assert_eq!(offenses[0].kind, OffenseKind::ForLoopVsEach);
        assert!(offenses[0].fix.is_some());
    }

    fn fixed(source: &'static [u8]) -> Option<String> {
        let f = parse_first_for(source);
        let fix = build_fix(&f, source)?;
        let (out, _) = crate::fix::apply_fixes(source, &[fix]);
        Some(String::from_utf8(out).unwrap())
    }

    #[test]
    fn fix_collection_on_the_next_line() {
        assert_eq!(
            fixed(b"for x in  \n  arr\n  puts x\nend").unwrap(),
            "arr.each do |x|\n  puts x\nend"
        );
    }

    #[test]
    fn fix_parenthesizes_range_collection() {
        assert_eq!(
            fixed(b"for x in 1..3 do\n  puts x\nend").unwrap(),
            "(1..3).each do |x|\n  puts x\nend"
        );
    }

    #[test]
    fn fix_parenthesizes_operator_and_unparenthesized_args() {
        assert_eq!(
            fixed(b"for x in a || b; puts x; end").unwrap(),
            "(a || b).each do |x| puts x; end"
        );
        assert_eq!(
            fixed(b"for x in foo bar do puts x end").unwrap(),
            "(foo bar).each do |x| puts x end"
        );
        assert_eq!(
            fixed(b"for x in -a; puts x; end").unwrap(),
            "(-a).each do |x| puts x; end"
        );
    }

    #[test]
    fn fix_keeps_simple_receivers_bare() {
        assert_eq!(
            fixed(b"for x in a.b(1); puts x; end").unwrap(),
            "a.b(1).each do |x| puts x; end"
        );
        assert_eq!(
            fixed(b"for x in @items; puts x; end").unwrap(),
            "@items.each do |x| puts x; end"
        );
        assert_eq!(
            fixed(b"for x in Foo::BAR; puts x; end").unwrap(),
            "Foo::BAR.each do |x| puts x; end"
        );
        for recv in [
            "items", "_items", "a.b", "@@items", "$items", "self", "Items", "[1, 2]", "(a + b)",
        ] {
            let src: &'static [u8] = format!("for x in {recv}; puts x; end").leak().as_bytes();
            assert_eq!(
                fixed(src).unwrap(),
                format!("{recv}.each do |x| puts x; end"),
                "receiver {recv}"
            );
        }
    }

    #[test]
    fn fix_consumes_spaces_before_semicolon() {
        assert_eq!(
            fixed(b"for x in arr \t ; puts x; end").unwrap(),
            "arr.each do |x| puts x; end"
        );
    }

    #[test]
    fn fix_leaves_trailing_comment_in_place() {
        assert_eq!(
            fixed(b"for x in arr # loop\n  puts x\nend").unwrap(),
            "arr.each do |x| # loop\n  puts x\nend"
        );
    }

    #[test]
    fn fix_multiple_loop_vars() {
        assert_eq!(
            fixed(b"for k, v in h; puts k, v; end").unwrap(),
            "h.each do |k, v| puts k, v; end"
        );
        assert_eq!(
            fixed(b"for (k, v) in h; puts k, v; end").unwrap(),
            "h.each do |k, v| puts k, v; end"
        );
    }

    #[test]
    fn no_fix_when_body_assigns_a_local() {
        // `last` would become block-local and vanish after the loop.
        assert!(fixed(b"for x in arr; last = x; end").is_none());
        assert!(fixed(b"for x in arr; n += x; end").is_none());
        assert!(fixed(b"for x in arr; m ||= x; end").is_none());
        assert!(fixed(b"for x in arr; m &&= x; end").is_none());
        assert!(fixed(b"for x in arr; a, b = x; end").is_none());
    }

    #[test]
    fn no_fix_when_loop_var_is_used_outside() {
        assert!(fixed(b"for x in arr; puts x; end\nputs x").is_none());
        assert!(fixed(b"x = 0\nfor x in arr; puts x; end").is_none());
    }

    #[test]
    fn loop_var_substring_elsewhere_does_not_block_fix() {
        assert!(fixed(b"for x in arr; puts x; end\nputs xs, ax, x_1").is_some());
    }

    #[test]
    fn no_fix_for_non_local_or_splat_index() {
        assert!(fixed(b"for @x in arr; end").is_none());
        assert!(fixed(b"for a, *b in arr; end").is_none());
        assert!(fixed(b"for *a, b in arr; end").is_none());
        assert!(fixed(b"for a, @b in arr; end").is_none());
    }

    #[test]
    fn offense_still_fires_without_fix() {
        let source = b"for x in arr; last = x; end";
        let f = parse_first_for(source);
        let offenses = scan(&f, source);
        assert_eq!(offenses.len(), 1);
        assert!(offenses[0].fix.is_none());
    }

    #[test]
    fn fix_for_loop_with_do() {
        let source = b"for x in arr do\n  puts x\nend";
        let f = parse_first_for(source);
        let fix = build_fix(&f, source).unwrap();
        let (fixed, _) = crate::fix::apply_fixes(source, &[fix]);
        assert_eq!(
            String::from_utf8(fixed).unwrap(),
            "arr.each do |x|\n  puts x\nend"
        );
    }

    #[test]
    fn fix_for_loop_with_semicolon() {
        let source = b"for x in [1,2,3]; puts x; end";
        let f = parse_first_for(source);
        let fix = build_fix(&f, source).unwrap();
        let (fixed, _) = crate::fix::apply_fixes(source, &[fix]);
        assert_eq!(
            String::from_utf8(fixed).unwrap(),
            "[1,2,3].each do |x| puts x; end"
        );
    }

    #[test]
    fn fix_for_loop_newline_only() {
        let source = b"for x in arr\n  puts x\nend";
        let f = parse_first_for(source);
        let fix = build_fix(&f, source).unwrap();
        let (fixed, _) = crate::fix::apply_fixes(source, &[fix]);
        assert_eq!(
            String::from_utf8(fixed).unwrap(),
            "arr.each do |x|\n  puts x\nend"
        );
    }

    #[test]
    fn extract_trimmed_valid() {
        let source = b"  hello  ";
        let result = extract_trimmed(source, 0, 9);
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn extract_trimmed_start_ge_end() {
        assert_eq!(extract_trimmed(b"hello", 5, 3), None);
        assert_eq!(extract_trimmed(b"hello", 3, 3), None);
    }

    #[test]
    fn extract_trimmed_end_gt_len() {
        assert_eq!(extract_trimmed(b"hi", 0, 10), None);
    }

    #[test]
    fn extract_trimmed_empty_after_trim() {
        let result = extract_trimmed(b"   ", 0, 3);
        assert_eq!(result, Some("".to_string()));
    }

    #[test]
    fn scan_always_returns_offense() {
        let source = b"for x in arr; end";
        let f = parse_first_for(source);
        let offenses = scan(&f, source);
        assert_eq!(offenses.len(), 1);
        assert_eq!(offenses[0].kind, OffenseKind::ForLoopVsEach);
    }
}
