use lib_ruby_parser::nodes::RescueBody;
use lib_ruby_parser::Node;

use crate::offense::{Offense, OffenseKind};

/// Fires when a rescue clause catches `NoMethodError`.
pub fn scan(node: &RescueBody) -> Vec<Offense> {
    if rescues_no_method_error(node) {
        vec![Offense::new(
            OffenseKind::RescueVsRespondTo,
            node.keyword_l.begin,
        )]
    } else {
        vec![]
    }
}

fn rescues_no_method_error(rb: &RescueBody) -> bool {
    let exc_list = match rb.exc_list.as_deref() {
        Some(node) => node,
        None => return false,
    };

    match exc_list {
        Node::Array(arr) => arr.elements.iter().any(is_no_method_error_const),
        node => is_no_method_error_const(node),
    }
}

fn is_no_method_error_const(node: &Node) -> bool {
    match node {
        Node::Const(c) => c.name == "NoMethodError" && c.scope.is_none(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast_helpers::node_children;

    fn parse_and_find_rescue_bodies(source: &[u8]) -> Vec<Offense> {
        let result = lib_ruby_parser::Parser::new(source.to_vec(), Default::default()).do_parse();
        let mut offenses = Vec::new();
        if let Some(ast) = result.ast {
            collect_rescue_offenses(&ast, &mut offenses);
        }
        offenses
    }

    fn collect_rescue_offenses(node: &Node, offenses: &mut Vec<Offense>) {
        if let Node::RescueBody(rb) = node {
            offenses.extend(scan(rb));
        }
        for child in node_children(node) {
            collect_rescue_offenses(child, offenses);
        }
    }

    #[test]
    fn rescue_no_method_error_fires() {
        let offenses = parse_and_find_rescue_bodies(b"begin; rescue NoMethodError; end");
        assert_eq!(offenses.len(), 1);
    }

    #[test]
    fn rescue_standard_error_does_not_fire() {
        let offenses = parse_and_find_rescue_bodies(b"begin; rescue StandardError; end");
        assert_eq!(offenses.len(), 0);
    }

    #[test]
    fn bare_rescue_does_not_fire() {
        let offenses = parse_and_find_rescue_bodies(b"begin; rescue; end");
        assert_eq!(offenses.len(), 0);
    }

    #[test]
    fn multiple_exceptions_including_no_method_error() {
        let offenses =
            parse_and_find_rescue_bodies(b"begin; rescue ArgumentError, NoMethodError; end");
        assert_eq!(offenses.len(), 1);
    }

    #[test]
    fn scoped_no_method_error_no_fire() {
        let offenses =
            parse_and_find_rescue_bodies(b"begin; rescue SomeModule::NoMethodError; end");
        assert_eq!(offenses.len(), 0);
    }

    #[test]
    fn rescue_other_error_no_fire() {
        let offenses = parse_and_find_rescue_bodies(b"begin; rescue TypeError; end");
        assert_eq!(offenses.len(), 0);
    }
}
