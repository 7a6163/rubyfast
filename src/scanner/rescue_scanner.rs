use ruby_prism::Node;

use crate::offense::{Offense, OffenseKind};

/// Fires when a rescue clause catches `NoMethodError`.
pub fn scan(node: &ruby_prism::RescueNode<'_>) -> Vec<Offense> {
    if rescues_no_method_error(node) {
        vec![Offense::new(
            OffenseKind::RescueVsRespondTo,
            node.keyword_loc().start_offset(),
        )]
    } else {
        vec![]
    }
}

fn rescues_no_method_error(rb: &ruby_prism::RescueNode<'_>) -> bool {
    let exceptions: Vec<Node<'_>> = rb.exceptions().iter().collect();
    if exceptions.is_empty() {
        return false;
    }
    exceptions.iter().any(|exc| is_no_method_error_const(exc))
}

fn is_no_method_error_const(node: &Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        c.name().as_slice() == b"NoMethodError"
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_helpers::test_helpers::leak_parse;
    use crate::ast_visitor::for_each_node;

    fn parse_and_find_rescue_bodies(source: &[u8]) -> Vec<Offense> {
        let result = leak_parse(source);
        let mut offenses = Vec::new();
        collect_rescue_offenses(&result.node(), &mut offenses);
        offenses
    }

    fn collect_rescue_offenses(node: &Node<'_>, offenses: &mut Vec<Offense>) {
        // RescueNode is reached through a typed visit, so start from its BeginNode.
        for_each_node(node, |n| {
            if let Some(rescue) = n.as_begin_node().and_then(|b| b.rescue_clause()) {
                collect_from_rescue_chain(&rescue, offenses);
            }
        });
    }

    fn collect_from_rescue_chain(rescue: &ruby_prism::RescueNode<'_>, offenses: &mut Vec<Offense>) {
        offenses.extend(scan(rescue));
        if let Some(subsequent) = rescue.subsequent() {
            collect_from_rescue_chain(&subsequent, offenses);
        }
    }

    #[test]
    fn rescue_chain_scans_subsequent_clauses() {
        let offenses =
            parse_and_find_rescue_bodies(b"begin; rescue ArgumentError; rescue NoMethodError; end");
        assert_eq!(offenses.len(), 1);
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
