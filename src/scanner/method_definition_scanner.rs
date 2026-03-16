use ruby_prism::Node;

use crate::ast_helpers::{
    body_expression_count, body_first_expression, def_block_arg_name, def_first_arg_name,
    def_regular_arg_count,
};
use crate::ast_visitor::for_each_descendant;
use crate::offense::{Offense, OffenseKind};

/// Scan a method definition for proc_call, getter, and setter offenses.
pub fn scan(def: &ruby_prism::DefNode<'_>) -> Vec<Offense> {
    let mut offenses = Vec::new();

    check_proc_call_vs_yield(def, &mut offenses);
    check_getter_vs_attr_reader(def, &mut offenses);
    check_setter_vs_attr_writer(def, &mut offenses);

    offenses
}

/// `def foo(&block); block.call; end` → use `yield` instead.
fn check_proc_call_vs_yield(def: &ruby_prism::DefNode<'_>, offenses: &mut Vec<Offense>) {
    let block_name = match def_block_arg_name(def) {
        Some(name) => name,
        None => return,
    };

    let body = def.body();
    if body_contains_block_call(&body, &block_name) {
        offenses.push(Offense::new(
            OffenseKind::ProcCallVsYield,
            def.def_keyword_loc().start_offset(),
        ));
    }
}

fn body_contains_block_call(body: &Option<Node<'_>>, block_name: &str) -> bool {
    match body {
        Some(node) => node_contains_block_call(node, block_name),
        None => false,
    }
}

fn node_contains_block_call(node: &Node<'_>, block_name: &str) -> bool {
    if let Some(call) = node.as_call_node()
        && call.name().as_slice() == b"call"
        && let Some(recv) = call.receiver()
        && let Some(lv) = recv.as_local_variable_read_node()
        && String::from_utf8_lossy(lv.name().as_slice()) == block_name
    {
        return true;
    }
    let mut found = false;
    for_each_descendant(node, &mut |child| {
        if !found && node_is_block_call(child, block_name) {
            found = true;
        }
    });
    found
}

fn node_is_block_call(node: &Node<'_>, block_name: &str) -> bool {
    if let Some(call) = node.as_call_node()
        && call.name().as_slice() == b"call"
        && let Some(recv) = call.receiver()
        && let Some(lv) = recv.as_local_variable_read_node()
    {
        return String::from_utf8_lossy(lv.name().as_slice()) == block_name;
    }
    false
}

/// `def name; @name; end` → use `attr_reader :name`.
fn check_getter_vs_attr_reader(def: &ruby_prism::DefNode<'_>, offenses: &mut Vec<Offense>) {
    let def_name = String::from_utf8_lossy(def.name().as_slice()).to_string();
    // Must not be a setter (name ends with =)
    if def_name.ends_with('=') {
        return;
    }
    // Must have 0 arguments
    if def_regular_arg_count(def) != 0 {
        return;
    }
    // Body must be a single ivar read matching @<method_name>
    let body = def.body();
    if body_expression_count(&body) != 1 {
        return;
    }
    let single = body_first_expression(&body).or(body);
    if let Some(iv) = single
        .as_ref()
        .and_then(|n| n.as_instance_variable_read_node())
    {
        let ivar_name = String::from_utf8_lossy(iv.name().as_slice()).to_string();
        let expected_ivar = format!("@{}", def_name);
        if ivar_name == expected_ivar {
            offenses.push(Offense::new(
                OffenseKind::GetterVsAttrReader,
                def.def_keyword_loc().start_offset(),
            ));
        }
    }
}

/// `def name=(value); @name = value; end` → use `attr_writer :name`.
fn check_setter_vs_attr_writer(def: &ruby_prism::DefNode<'_>, offenses: &mut Vec<Offense>) {
    let def_name = String::from_utf8_lossy(def.name().as_slice()).to_string();
    // Must be a setter
    let base_name = match def_name.strip_suffix('=') {
        Some(n) => n.to_string(),
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
    let body = def.body();
    if body_expression_count(&body) != 1 {
        return;
    }
    let single = body_first_expression(&body).or(body);
    if let Some(ia) = single
        .as_ref()
        .and_then(|n| n.as_instance_variable_write_node())
    {
        let ivar_name = String::from_utf8_lossy(ia.name().as_slice()).to_string();
        let expected_ivar = format!("@{}", base_name);
        if ivar_name != expected_ivar {
            return;
        }
        // The assigned value must be the argument
        if let Some(lv) = ia.value().as_local_variable_read_node()
            && String::from_utf8_lossy(lv.name().as_slice()) == arg_name
        {
            offenses.push(Offense::new(
                OffenseKind::SetterVsAttrWriter,
                def.def_keyword_loc().start_offset(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_visitor::for_each_direct_child;

    fn parse_and_scan(source: &[u8]) -> Vec<Offense> {
        let result = ruby_prism::parse(source);
        let result = Box::leak(Box::new(result));
        let mut offenses = Vec::new();
        collect_def_offenses(&result.node(), &mut offenses);
        offenses
    }

    fn collect_def_offenses<'pr>(node: &Node<'pr>, offenses: &mut Vec<Offense>) {
        if let Some(d) = node.as_def_node() {
            offenses.extend(scan(&d));
        }
        for_each_direct_child(node, &mut |child| {
            collect_def_offenses(child, offenses);
        });
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
