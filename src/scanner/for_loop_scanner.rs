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
fn build_fix(node: &ruby_prism::ForNode<'_>, source: &[u8]) -> Option<Fix> {
    let for_loc = node.for_keyword_loc();
    let in_loc = node.in_keyword_loc();

    // Extract iterator text from between "for" and "in"
    let iterator = extract_trimmed(source, for_loc.end_offset(), in_loc.start_offset())?;

    // do_keyword_loc is Option<Location> in prism — present only for `do` keyword, not `;`.
    let (iteratee, header_end) = if let Some(do_loc) = node.do_keyword_loc() {
        let text = extract_trimmed(source, in_loc.end_offset(), do_loc.start_offset())?;
        (text, do_loc.end_offset())
    } else {
        // No explicit `do` — look for `;` or newline as delimiter
        let search_start = in_loc.end_offset();
        let delimiter_pos = source[search_start..]
            .iter()
            .position(|&b| b == b'\n' || b == b';')
            .map(|p| search_start + p)
            .unwrap_or(source.len());
        let text = extract_trimmed(source, search_start, delimiter_pos)?;
        // If delimiter is `;`, include it in header_end so it gets replaced
        let header_end = if source.get(delimiter_pos) == Some(&b';') {
            delimiter_pos + 1
        } else {
            delimiter_pos
        };
        (text, header_end)
    };

    if iterator.is_empty() || iteratee.is_empty() {
        return None;
    }

    let new_header = format!("{}.each do |{}|", iteratee, iterator);
    Some(Fix::single(for_loc.start_offset(), header_end, new_header))
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
        let stmts: Vec<_> = prog.statements().body().iter().collect();
        stmts[0].as_for_node().unwrap()
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
        let fixed_str = String::from_utf8(fixed).unwrap();
        assert!(fixed_str.starts_with("[1,2,3].each do |x|"));
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
