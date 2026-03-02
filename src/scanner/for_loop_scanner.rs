use lib_ruby_parser::nodes::For;

use crate::fix::Fix;
use crate::offense::{Offense, OffenseKind};

/// Any `for` loop emits an offense — prefer `.each`.
pub fn scan(node: &For, source: &[u8]) -> Vec<Offense> {
    match build_fix(node, source) {
        Some(fix) => vec![Offense::with_fix(
            OffenseKind::ForLoopVsEach,
            node.keyword_l.begin,
            fix,
        )],
        None => vec![Offense::new(
            OffenseKind::ForLoopVsEach,
            node.keyword_l.begin,
        )],
    }
}

/// Build a fix that transforms `for x in arr` → `arr.each do |x|`.
fn build_fix(node: &For, source: &[u8]) -> Option<Fix> {
    // Extract iterator text from between "for" and "in"
    let iterator = extract_trimmed(source, node.keyword_l.end, node.operator_l.begin)?;

    // begin_l is Loc (not Option). It points to `do`, `;`, or the newline.
    // Only use it as a real delimiter if it points to `do` or `;` (not a newline).
    let begin_char = source.get(node.begin_l.begin).copied().unwrap_or(0);
    let has_explicit_begin = !node.begin_l.is_empty() && begin_char != b'\n';

    // Determine where the iteratee ends and the header ends
    let (iteratee, header_end) = if has_explicit_begin {
        let text = extract_trimmed(source, node.operator_l.end, node.begin_l.begin)?;
        (text, node.begin_l.end)
    } else {
        // No explicit `do` or `;` — header ends at the newline
        let search_start = node.operator_l.end;
        let line_end = source[search_start..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| search_start + p)
            .unwrap_or(source.len());
        let text = extract_trimmed(source, search_start, line_end)?;
        (text, line_end)
    };

    if iterator.is_empty() || iteratee.is_empty() {
        return None;
    }

    let new_header = format!("{}.each do |{}|", iteratee, iterator);
    Some(Fix::single(node.keyword_l.begin, header_end, new_header))
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

    #[test]
    fn for_loop_always_fires() {
        let source = b"for x in [1,2,3]; end";
        let result = lib_ruby_parser::Parser::new(source.to_vec(), Default::default()).do_parse();
        let ast = result.ast.unwrap();
        if let lib_ruby_parser::Node::For(f) = ast.as_ref() {
            let offenses = scan(f, source);
            assert_eq!(offenses.len(), 1);
            assert_eq!(offenses[0].kind, OffenseKind::ForLoopVsEach);
            assert!(offenses[0].fix.is_some());
        } else {
            panic!("Expected For node");
        }
    }

    #[test]
    fn fix_for_loop_with_do() {
        let source = b"for x in arr do\n  puts x\nend";
        let result = lib_ruby_parser::Parser::new(source.to_vec(), Default::default()).do_parse();
        let ast = result.ast.unwrap();
        if let lib_ruby_parser::Node::For(f) = ast.as_ref() {
            let fix = build_fix(f, source).unwrap();
            let fixed = crate::fix::apply_fixes(source, &[fix]);
            assert_eq!(
                String::from_utf8(fixed).unwrap(),
                "arr.each do |x|\n  puts x\nend"
            );
        } else {
            panic!("Expected For node");
        }
    }

    #[test]
    fn fix_for_loop_with_semicolon() {
        let source = b"for x in [1,2,3]; puts x; end";
        let result = lib_ruby_parser::Parser::new(source.to_vec(), Default::default()).do_parse();
        let ast = result.ast.unwrap();
        if let lib_ruby_parser::Node::For(f) = ast.as_ref() {
            let fix = build_fix(f, source).unwrap();
            let fixed = crate::fix::apply_fixes(source, &[fix]);
            let fixed_str = String::from_utf8(fixed).unwrap();
            assert!(fixed_str.starts_with("[1,2,3].each do |x|"));
        } else {
            panic!("Expected For node");
        }
    }

    #[test]
    fn fix_for_loop_newline_only() {
        let source = b"for x in arr\n  puts x\nend";
        let result = lib_ruby_parser::Parser::new(source.to_vec(), Default::default()).do_parse();
        let ast = result.ast.unwrap();
        if let lib_ruby_parser::Node::For(f) = ast.as_ref() {
            let fix = build_fix(f, source).unwrap();
            let fixed = crate::fix::apply_fixes(source, &[fix]);
            assert_eq!(
                String::from_utf8(fixed).unwrap(),
                "arr.each do |x|\n  puts x\nend"
            );
        } else {
            panic!("Expected For node");
        }
    }
}
