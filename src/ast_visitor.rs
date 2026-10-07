use ruby_prism::{Node, Visit};

/// Call `f` on `node` and every node below it, in pre-order.
///
/// Built on prism's generated visitor, so every node kind is descended into. Wrapper nodes
/// prism visits through typed calls (`StatementsNode`, `ElseNode`, `RescueNode`, …) are
/// descended into but not passed to `f`.
pub fn for_each_node<'pr>(node: &Node<'pr>, f: impl FnMut(&Node<'pr>)) {
    struct Each<F>(F);

    impl<'pr, F: FnMut(&Node<'pr>)> Visit<'pr> for Each<F> {
        fn visit_branch_node_enter(&mut self, node: Node<'pr>) {
            (self.0)(&node);
        }

        fn visit_leaf_node_enter(&mut self, node: Node<'pr>) {
            (self.0)(&node);
        }
    }

    Each(f).visit(node);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visits_root_and_nested_nodes_in_preorder() {
        let result = ruby_prism::parse(b"x = a.b rescue c");
        let mut names = Vec::new();
        for_each_node(&result.node(), |n| {
            if let Some(call) = n.as_call_node() {
                names.push(String::from_utf8_lossy(call.name().as_slice()).into_owned());
            }
        });
        assert_eq!(names, ["b", "a", "c"]);
    }

    #[test]
    fn visits_leaf_nodes() {
        let result = ruby_prism::parse(b"[1, [2]]");
        let mut ints = 0;
        for_each_node(&result.node(), |n| {
            ints += usize::from(n.as_integer_node().is_some())
        });
        assert_eq!(ints, 2);
    }
}
