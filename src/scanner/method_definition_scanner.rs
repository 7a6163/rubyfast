use lib_ruby_parser::Node;
use lib_ruby_parser::nodes::Def;

use crate::ast_helpers::{
    body_expressions, def_block_arg_name, def_first_arg_name, def_regular_arg_count,
};
use crate::offense::{Offense, OffenseKind};

/// Scan a method definition for proc_call, getter, and setter offenses.
pub fn scan(def: &Def) -> Vec<Offense> {
    let mut offenses = Vec::new();

    check_proc_call_vs_yield(def, &mut offenses);
    check_getter_vs_attr_reader(def, &mut offenses);
    check_setter_vs_attr_writer(def, &mut offenses);

    offenses
}

/// `def foo(&block); block.call; end` → use `yield` instead.
fn check_proc_call_vs_yield(def: &Def, offenses: &mut Vec<Offense>) {
    let block_name = match def_block_arg_name(def) {
        Some(name) => name,
        None => return,
    };

    if body_contains_block_call(&def.body, &block_name) {
        offenses.push(Offense::new(
            OffenseKind::ProcCallVsYield,
            def.keyword_l.begin,
        ));
    }
}

fn body_contains_block_call(body: &Option<Box<Node>>, block_name: &str) -> bool {
    match body.as_deref() {
        Some(node) => node_contains_block_call(node, block_name),
        None => false,
    }
}

fn node_contains_block_call(node: &Node, block_name: &str) -> bool {
    if let Node::Send(s) = node
        && s.method_name == "call"
        && let Some(Node::Lvar(lv)) = s.recv.as_deref()
        && lv.name == block_name
    {
        return true;
    }
    let mut found = false;
    crate::ast_visitor::for_each_child(node, |child| {
        if !found && node_contains_block_call(child, block_name) {
            found = true;
        }
    });
    found
}

/// `def name; @name; end` → use `attr_reader :name`.
fn check_getter_vs_attr_reader(def: &Def, offenses: &mut Vec<Offense>) {
    // Must not be a setter (name ends with =)
    if def.name.ends_with('=') {
        return;
    }
    // Must have 0 arguments
    if def_regular_arg_count(def) != 0 {
        return;
    }
    // Body must be a single ivar read matching @<method_name>
    let exprs = body_expressions(&def.body);
    if exprs.len() != 1 {
        return;
    }
    if let Node::Ivar(iv) = exprs[0] {
        let expected_ivar = format!("@{}", def.name);
        if iv.name == expected_ivar {
            offenses.push(Offense::new(
                OffenseKind::GetterVsAttrReader,
                def.keyword_l.begin,
            ));
        }
    }
}

/// `def name=(value); @name = value; end` → use `attr_writer :name`.
fn check_setter_vs_attr_writer(def: &Def, offenses: &mut Vec<Offense>) {
    // Must be a setter
    let base_name = match def.name.strip_suffix('=') {
        Some(n) => n,
        None => return,
    };
    // Must have exactly 1 regular argument
    if def_regular_arg_count(def) != 1 {
        return;
    }
    let arg_name = match def_first_arg_name(def) {
        Some(name) => name,
        None => return,
    };
    // Body must be a single ivar assignment
    let exprs = body_expressions(&def.body);
    if exprs.len() != 1 {
        return;
    }
    if let Node::Ivasgn(ia) = exprs[0] {
        let expected_ivar = format!("@{}", base_name);
        if ia.name != expected_ivar {
            return;
        }
        // The assigned value must be the argument
        if let Some(Node::Lvar(lv)) = ia.value.as_deref()
            && lv.name == arg_name
        {
            offenses.push(Offense::new(
                OffenseKind::SetterVsAttrWriter,
                def.keyword_l.begin,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast_visitor::node_children;

    fn parse_and_scan(source: &[u8]) -> Vec<Offense> {
        let result = lib_ruby_parser::Parser::new(source.to_vec(), Default::default()).do_parse();
        let mut offenses = Vec::new();
        if let Some(ast) = result.ast {
            collect_def_offenses(&ast, &mut offenses);
        }
        offenses
    }

    fn collect_def_offenses(node: &Node, offenses: &mut Vec<Offense>) {
        if let Node::Def(d) = node {
            offenses.extend(scan(d));
        }
        for child in node_children(node) {
            collect_def_offenses(child, offenses);
        }
    }

    #[test]
    fn getter_fires() {
        let offenses = parse_and_scan(b"def name; @name; end");
        assert!(
            offenses
                .iter()
                .any(|o| o.kind == OffenseKind::GetterVsAttrReader)
        );
    }

    #[test]
    fn getter_with_assignment_does_not_fire() {
        let offenses = parse_and_scan(b"def name; @name = 1; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::GetterVsAttrReader)
        );
    }

    #[test]
    fn setter_fires() {
        let offenses = parse_and_scan(b"def name=(value); @name = value; end");
        assert!(
            offenses
                .iter()
                .any(|o| o.kind == OffenseKind::SetterVsAttrWriter)
        );
    }

    #[test]
    fn proc_call_fires() {
        let offenses = parse_and_scan(b"def foo(&block); block.call; end");
        assert!(
            offenses
                .iter()
                .any(|o| o.kind == OffenseKind::ProcCallVsYield)
        );
    }

    #[test]
    fn no_block_arg_no_proc_call() {
        let offenses = parse_and_scan(b"def foo; block.call; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::ProcCallVsYield)
        );
    }

    #[test]
    fn setter_wrong_ivar_name_no_fire() {
        let offenses = parse_and_scan(b"def name=(v); @other = v; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::SetterVsAttrWriter)
        );
    }

    #[test]
    fn setter_wrong_value_no_fire() {
        let offenses = parse_and_scan(b"def name=(v); @name = 42; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::SetterVsAttrWriter)
        );
    }

    #[test]
    fn setter_multiple_args_no_fire() {
        let offenses = parse_and_scan(b"def name=(a, b); @name = a; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::SetterVsAttrWriter)
        );
    }

    #[test]
    fn setter_no_body_no_fire() {
        let offenses = parse_and_scan(b"def name=(v); end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::SetterVsAttrWriter)
        );
    }

    #[test]
    fn getter_with_args_no_fire() {
        let offenses = parse_and_scan(b"def name(x); @name; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::GetterVsAttrReader)
        );
    }

    #[test]
    fn getter_multiple_body_stmts_no_fire() {
        let offenses = parse_and_scan(b"def name; puts 'x'; @name; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::GetterVsAttrReader)
        );
    }

    #[test]
    fn getter_wrong_ivar_no_fire() {
        let offenses = parse_and_scan(b"def name; @other; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::GetterVsAttrReader)
        );
    }

    #[test]
    fn getter_no_body_no_fire() {
        let offenses = parse_and_scan(b"def name; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::GetterVsAttrReader)
        );
    }

    #[test]
    fn proc_call_nested_in_body() {
        let offenses = parse_and_scan(b"def foo(&block); if true; block.call; end; end");
        assert!(
            offenses
                .iter()
                .any(|o| o.kind == OffenseKind::ProcCallVsYield)
        );
    }

    #[test]
    fn setter_name_method_is_not_getter() {
        // name= should not trigger getter check
        let offenses = parse_and_scan(b"def name=(v); @name = v; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::GetterVsAttrReader)
        );
    }

    #[test]
    fn setter_body_not_ivasgn_no_fire() {
        let offenses = parse_and_scan(b"def name=(v); puts v; end");
        assert!(
            !offenses
                .iter()
                .any(|o| o.kind == OffenseKind::SetterVsAttrWriter)
        );
    }
}
