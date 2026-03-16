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
    use crate::ast_visitor::for_each_direct_child;

    fn parse_and_find_rescue_bodies(source: &[u8]) -> Vec<Offense> {
        let result = leak_parse(source);
        let mut offenses = Vec::new();
        collect_rescue_offenses(&result.node(), &mut offenses);
        offenses
    }

    fn collect_rescue_offenses<'pr>(node: &Node<'pr>, offenses: &mut Vec<Offense>) {
        // For BeginNode, we need to access the rescue clause specially
        if let Some(begin) = node.as_begin_node() {
            if let Some(rescue) = begin.rescue_clause() {
                collect_from_rescue_chain(&rescue, offenses);
            }
        }
        for_each_direct_child(node, &mut |child| {
            collect_rescue_offenses(child, offenses);
        });
    }

    fn collect_from_rescue_chain(rescue: &ruby_prism::RescueNode<'_>, offenses: &mut Vec<Offense>) {
        offenses.extend(scan(rescue));
        if let Some(subsequent) = rescue.subsequent() {
            collect_from_rescue_chain(&subsequent, offenses);
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
