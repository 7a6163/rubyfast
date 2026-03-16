use ruby_prism::Node;

/// Recursively visit all descendant nodes of a given node, calling `f` for each.
/// This is used by scanners that need to search for patterns inside a subtree.
pub fn for_each_descendant<'pr>(node: &Node<'pr>, f: &mut impl FnMut(&Node<'pr>)) {
    for_each_direct_child(node, &mut |child: &Node<'pr>| {
        f(child);
        for_each_descendant(child, f);
    });
}

/// Iterate over direct children of a node, calling f for each.
/// This is the core traversal function for ruby-prism nodes.
///
/// Note: Some prism accessors return specific types (ElseNode, EnsureNode, etc.)
/// rather than Node. For those, we visit their inner statements directly.
pub fn for_each_direct_child<'pr>(node: &Node<'pr>, f: &mut impl FnMut(&Node<'pr>)) {
    match node {
        Node::ProgramNode { .. } => {
            let n = node.as_program_node().unwrap();
            for child in n.statements().body().iter() {
                f(&child);
            }
        }
        Node::StatementsNode { .. } => {
            let n = node.as_statements_node().unwrap();
            for child in n.body().iter() {
                f(&child);
            }
        }
        Node::CallNode { .. } => {
            let n = node.as_call_node().unwrap();
            if let Some(recv) = n.receiver() {
                f(&recv);
            }
            if let Some(args) = n.arguments() {
                for arg in args.arguments().iter() {
                    f(&arg);
                }
            }
            if let Some(block) = n.block() {
                f(&block);
            }
        }
        Node::BlockNode { .. } => {
            let n = node.as_block_node().unwrap();
            if let Some(params) = n.parameters() {
                f(&params);
            }
            if let Some(body) = n.body() {
                f(&body);
            }
        }
        Node::BlockArgumentNode { .. } => {
            let n = node.as_block_argument_node().unwrap();
            if let Some(expr) = n.expression() {
                f(&expr);
            }
        }
        Node::DefNode { .. } => {
            let n = node.as_def_node().unwrap();
            if let Some(params) = n.parameters() {
                f(&params.as_node());
            }
            if let Some(body) = n.body() {
                f(&body);
            }
        }
        Node::ForNode { .. } => {
            let n = node.as_for_node().unwrap();
            f(&n.index());
            f(&n.collection());
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::BeginNode { .. } => {
            let n = node.as_begin_node().unwrap();
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
            if let Some(rescue) = n.rescue_clause() {
                visit_rescue_children(&rescue, f);
            }
            if let Some(else_clause) = n.else_clause()
                && let Some(stmts) = else_clause.statements()
            {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
            if let Some(ensure) = n.ensure_clause()
                && let Some(stmts) = ensure.statements()
            {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::RescueNode { .. } => {
            let n = node.as_rescue_node().unwrap();
            visit_rescue_children(&n, f);
        }
        Node::EnsureNode { .. } => {
            let n = node.as_ensure_node().unwrap();
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::ElseNode { .. } => {
            let n = node.as_else_node().unwrap();
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::IfNode { .. } => {
            let n = node.as_if_node().unwrap();
            f(&n.predicate());
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
            if let Some(subsequent) = n.subsequent() {
                f(&subsequent);
            }
        }
        Node::UnlessNode { .. } => {
            let n = node.as_unless_node().unwrap();
            f(&n.predicate());
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
            if let Some(else_clause) = n.else_clause()
                && let Some(stmts) = else_clause.statements()
            {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::WhileNode { .. } => {
            let n = node.as_while_node().unwrap();
            f(&n.predicate());
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::UntilNode { .. } => {
            let n = node.as_until_node().unwrap();
            f(&n.predicate());
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::CaseNode { .. } => {
            let n = node.as_case_node().unwrap();
            if let Some(pred) = n.predicate() {
                f(&pred);
            }
            for condition in n.conditions().iter() {
                f(&condition);
            }
            if let Some(else_clause) = n.else_clause()
                && let Some(stmts) = else_clause.statements()
            {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::WhenNode { .. } => {
            let n = node.as_when_node().unwrap();
            for cond in n.conditions().iter() {
                f(&cond);
            }
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::ClassNode { .. } => {
            let n = node.as_class_node().unwrap();
            f(&n.constant_path());
            if let Some(superclass) = n.superclass() {
                f(&superclass);
            }
            if let Some(body) = n.body() {
                f(&body);
            }
        }
        Node::ModuleNode { .. } => {
            let n = node.as_module_node().unwrap();
            f(&n.constant_path());
            if let Some(body) = n.body() {
                f(&body);
            }
        }
        Node::SingletonClassNode { .. } => {
            let n = node.as_singleton_class_node().unwrap();
            f(&n.expression());
            if let Some(body) = n.body() {
                f(&body);
            }
        }
        Node::AndNode { .. } => {
            let n = node.as_and_node().unwrap();
            f(&n.left());
            f(&n.right());
        }
        Node::OrNode { .. } => {
            let n = node.as_or_node().unwrap();
            f(&n.left());
            f(&n.right());
        }
        Node::ArrayNode { .. } => {
            let n = node.as_array_node().unwrap();
            for elem in n.elements().iter() {
                f(&elem);
            }
        }
        Node::HashNode { .. } => {
            let n = node.as_hash_node().unwrap();
            for elem in n.elements().iter() {
                f(&elem);
            }
        }
        Node::KeywordHashNode { .. } => {
            let n = node.as_keyword_hash_node().unwrap();
            for elem in n.elements().iter() {
                f(&elem);
            }
        }
        Node::AssocNode { .. } => {
            let n = node.as_assoc_node().unwrap();
            f(&n.key());
            f(&n.value());
        }
        Node::AssocSplatNode { .. } => {
            let n = node.as_assoc_splat_node().unwrap();
            if let Some(value) = n.value() {
                f(&value);
            }
        }
        Node::RangeNode { .. } => {
            let n = node.as_range_node().unwrap();
            if let Some(left) = n.left() {
                f(&left);
            }
            if let Some(right) = n.right() {
                f(&right);
            }
        }
        Node::ParenthesesNode { .. } => {
            let n = node.as_parentheses_node().unwrap();
            if let Some(body) = n.body() {
                f(&body);
            }
        }
        Node::InterpolatedStringNode { .. } => {
            let n = node.as_interpolated_string_node().unwrap();
            for part in n.parts().iter() {
                f(&part);
            }
        }
        Node::InterpolatedSymbolNode { .. } => {
            let n = node.as_interpolated_symbol_node().unwrap();
            for part in n.parts().iter() {
                f(&part);
            }
        }
        Node::EmbeddedStatementsNode { .. } => {
            let n = node.as_embedded_statements_node().unwrap();
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::LocalVariableWriteNode { .. } => {
            let n = node.as_local_variable_write_node().unwrap();
            f(&n.value());
        }
        Node::InstanceVariableWriteNode { .. } => {
            let n = node.as_instance_variable_write_node().unwrap();
            f(&n.value());
        }
        Node::ClassVariableWriteNode { .. } => {
            let n = node.as_class_variable_write_node().unwrap();
            f(&n.value());
        }
        Node::GlobalVariableWriteNode { .. } => {
            let n = node.as_global_variable_write_node().unwrap();
            f(&n.value());
        }
        Node::ConstantWriteNode { .. } => {
            let n = node.as_constant_write_node().unwrap();
            f(&n.value());
        }
        Node::ConstantPathWriteNode { .. } => {
            let n = node.as_constant_path_write_node().unwrap();
            // target() returns ConstantPathNode, not Node — skip it
            f(&n.value());
        }
        Node::ConstantPathNode { .. } => {
            let n = node.as_constant_path_node().unwrap();
            if let Some(parent) = n.parent() {
                f(&parent);
            }
        }
        Node::MultiWriteNode { .. } => {
            let n = node.as_multi_write_node().unwrap();
            for target in n.lefts().iter() {
                f(&target);
            }
            if let Some(rest) = n.rest() {
                f(&rest);
            }
            for target in n.rights().iter() {
                f(&target);
            }
            f(&n.value());
        }
        Node::SplatNode { .. } => {
            let n = node.as_splat_node().unwrap();
            if let Some(expr) = n.expression() {
                f(&expr);
            }
        }
        Node::ReturnNode { .. } => {
            let n = node.as_return_node().unwrap();
            if let Some(args) = n.arguments() {
                for arg in args.arguments().iter() {
                    f(&arg);
                }
            }
        }
        Node::YieldNode { .. } => {
            let n = node.as_yield_node().unwrap();
            if let Some(args) = n.arguments() {
                for arg in args.arguments().iter() {
                    f(&arg);
                }
            }
        }
        Node::SuperNode { .. } => {
            let n = node.as_super_node().unwrap();
            if let Some(args) = n.arguments() {
                for arg in args.arguments().iter() {
                    f(&arg);
                }
            }
            if let Some(block) = n.block() {
                f(&block);
            }
        }
        Node::LambdaNode { .. } => {
            let n = node.as_lambda_node().unwrap();
            if let Some(params) = n.parameters() {
                f(&params);
            }
            if let Some(body) = n.body() {
                f(&body);
            }
        }
        Node::DefinedNode { .. } => {
            let n = node.as_defined_node().unwrap();
            f(&n.value());
        }
        Node::InterpolatedRegularExpressionNode { .. } => {
            let n = node.as_interpolated_regular_expression_node().unwrap();
            for part in n.parts().iter() {
                f(&part);
            }
        }
        Node::MatchPredicateNode { .. } => {
            let n = node.as_match_predicate_node().unwrap();
            f(&n.value());
            f(&n.pattern());
        }
        Node::MatchRequiredNode { .. } => {
            let n = node.as_match_required_node().unwrap();
            f(&n.value());
            f(&n.pattern());
        }
        Node::CaseMatchNode { .. } => {
            let n = node.as_case_match_node().unwrap();
            if let Some(pred) = n.predicate() {
                f(&pred);
            }
            for condition in n.conditions().iter() {
                f(&condition);
            }
            if let Some(else_clause) = n.else_clause()
                && let Some(stmts) = else_clause.statements()
            {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::InNode { .. } => {
            let n = node.as_in_node().unwrap();
            f(&n.pattern());
            if let Some(stmts) = n.statements() {
                for child in stmts.body().iter() {
                    f(&child);
                }
            }
        }
        Node::BreakNode { .. } => {
            let n = node.as_break_node().unwrap();
            if let Some(args) = n.arguments() {
                for arg in args.arguments().iter() {
                    f(&arg);
                }
            }
        }
        Node::NextNode { .. } => {
            let n = node.as_next_node().unwrap();
            if let Some(args) = n.arguments() {
                for arg in args.arguments().iter() {
                    f(&arg);
                }
            }
        }
        Node::AliasMethodNode { .. } => {
            let n = node.as_alias_method_node().unwrap();
            f(&n.new_name());
            f(&n.old_name());
        }
        Node::AliasGlobalVariableNode { .. } => {
            let n = node.as_alias_global_variable_node().unwrap();
            f(&n.new_name());
            f(&n.old_name());
        }
        Node::UndefNode { .. } => {
            let n = node.as_undef_node().unwrap();
            for name in n.names().iter() {
                f(&name);
            }
        }
        Node::LocalVariableOperatorWriteNode { .. } => {
            let n = node.as_local_variable_operator_write_node().unwrap();
            f(&n.value());
        }
        Node::LocalVariableAndWriteNode { .. } => {
            let n = node.as_local_variable_and_write_node().unwrap();
            f(&n.value());
        }
        Node::LocalVariableOrWriteNode { .. } => {
            let n = node.as_local_variable_or_write_node().unwrap();
            f(&n.value());
        }
        Node::InstanceVariableOperatorWriteNode { .. } => {
            let n = node.as_instance_variable_operator_write_node().unwrap();
            f(&n.value());
        }
        Node::InstanceVariableAndWriteNode { .. } => {
            let n = node.as_instance_variable_and_write_node().unwrap();
            f(&n.value());
        }
        Node::InstanceVariableOrWriteNode { .. } => {
            let n = node.as_instance_variable_or_write_node().unwrap();
            f(&n.value());
        }
        Node::ConstantOperatorWriteNode { .. } => {
            let n = node.as_constant_operator_write_node().unwrap();
            f(&n.value());
        }
        Node::ConstantAndWriteNode { .. } => {
            let n = node.as_constant_and_write_node().unwrap();
            f(&n.value());
        }
        Node::ConstantOrWriteNode { .. } => {
            let n = node.as_constant_or_write_node().unwrap();
            f(&n.value());
        }
        Node::ConstantPathOperatorWriteNode { .. } => {
            let n = node.as_constant_path_operator_write_node().unwrap();
            // target() returns ConstantPathNode, not Node - skip
            f(&n.value());
        }
        Node::ConstantPathAndWriteNode { .. } => {
            let n = node.as_constant_path_and_write_node().unwrap();
            // target() returns ConstantPathNode, not Node - skip
            f(&n.value());
        }
        Node::ConstantPathOrWriteNode { .. } => {
            let n = node.as_constant_path_or_write_node().unwrap();
            // target() returns ConstantPathNode, not Node - skip
            f(&n.value());
        }
        Node::ClassVariableOperatorWriteNode { .. } => {
            let n = node.as_class_variable_operator_write_node().unwrap();
            f(&n.value());
        }
        Node::ClassVariableAndWriteNode { .. } => {
            let n = node.as_class_variable_and_write_node().unwrap();
            f(&n.value());
        }
        Node::ClassVariableOrWriteNode { .. } => {
            let n = node.as_class_variable_or_write_node().unwrap();
            f(&n.value());
        }
        Node::GlobalVariableOperatorWriteNode { .. } => {
            let n = node.as_global_variable_operator_write_node().unwrap();
            f(&n.value());
        }
        Node::GlobalVariableAndWriteNode { .. } => {
            let n = node.as_global_variable_and_write_node().unwrap();
            f(&n.value());
        }
        Node::GlobalVariableOrWriteNode { .. } => {
            let n = node.as_global_variable_or_write_node().unwrap();
            f(&n.value());
        }
        Node::IndexOperatorWriteNode { .. } => {
            let n = node.as_index_operator_write_node().unwrap();
            if let Some(recv) = n.receiver() {
                f(&recv);
            }
            if let Some(args) = n.arguments() {
                for arg in args.arguments().iter() {
                    f(&arg);
                }
            }
            f(&n.value());
        }
        Node::IndexAndWriteNode { .. } => {
            let n = node.as_index_and_write_node().unwrap();
            if let Some(recv) = n.receiver() {
                f(&recv);
            }
            if let Some(args) = n.arguments() {
                for arg in args.arguments().iter() {
                    f(&arg);
                }
            }
            f(&n.value());
        }
        Node::IndexOrWriteNode { .. } => {
            let n = node.as_index_or_write_node().unwrap();
            if let Some(recv) = n.receiver() {
                f(&recv);
            }
            if let Some(args) = n.arguments() {
                for arg in args.arguments().iter() {
                    f(&arg);
                }
            }
            f(&n.value());
        }
        // Leaf nodes and remaining types — no children to visit
        _ => {}
    }
}

/// Visit children of a RescueNode and its chain of subsequent clauses.
fn visit_rescue_children<'pr>(
    rescue: &ruby_prism::RescueNode<'pr>,
    f: &mut impl FnMut(&Node<'pr>),
) {
    for exc in rescue.exceptions().iter() {
        f(&exc);
    }
    if let Some(reference) = rescue.reference() {
        f(&reference);
    }
    if let Some(stmts) = rescue.statements() {
        for child in stmts.body().iter() {
            f(&child);
        }
    }
    if let Some(subsequent) = rescue.subsequent() {
        visit_rescue_children(&subsequent, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_helpers::test_helpers::leak_parse;

    fn count_all_nodes(node: &Node<'_>) -> usize {
        let mut count = 1;
        for_each_descendant(node, &mut |_| count += 1);
        count
    }

    #[test]
    fn visitor_counts_nodes() {
        let result = leak_parse(b"a + b");
        let total = count_all_nodes(&result.node());
        assert!(total > 1, "Expected multiple nodes, got {}", total);
    }

    #[test]
    fn visitor_handles_many_node_types() {
        let sources: &[&[u8]] = &[
            b"alias new_method old_method",
            b"a && b || c",
            b"[1, 2, 3]",
            b"case x; when 1; 'a'; else 'c'; end",
            b"FOO = 1",
            b"class Foo < Bar; end",
            b"Foo::Bar",
            b"def foo(a); end",
            b"defined?(x)",
            b"\"hello #{world}\"",
            b"begin; 1; ensure; 2; end",
            b"1...10; 1..10",
            b"for x in [1]; end",
            b"{a: 1, b: 2}",
            b"if true; 1; else; 2; end",
            b"@x = 1; @x",
            b"x = 42",
            b"module Foo; end",
            b"def foo; return 1; end",
            b"x += 1",
            b"begin; rescue StandardError => e; end",
            b"class << self; end",
            b"foo.bar(1, 2)",
            b"'hello'",
            b"def foo; super(1); end",
            b"until false; end",
            b"while true; break; end",
            b"def foo; yield 1; end",
            b"arr.select(&:odd?)",
            b"arr.map { |x| x.to_s }",
            b"-> { 1 }",
        ];

        for source in sources {
            let result = leak_parse(source);
            let total = count_all_nodes(&result.node());
            assert!(
                total > 0,
                "No nodes in AST for {:?}",
                std::str::from_utf8(source)
            );
        }
    }

    #[test]
    fn visitor_handles_unless_node() {
        let result = leak_parse(b"unless false; 1; else; 2; end");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_until_node() {
        let result = leak_parse(b"until false; 1; end");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_case_match_node() {
        let result = leak_parse(b"case x; in Integer => i; 1; in String; 2; else; 3; end");
        assert!(count_all_nodes(&result.node()) > 4);
    }

    #[test]
    fn visitor_handles_match_predicate_node() {
        let result = leak_parse(b"x in [1, 2]");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_match_required_node() {
        let result = leak_parse(b"x => y");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_multi_write_node() {
        let result = leak_parse(b"a, b, *c, d = 1, 2, 3, 4");
        assert!(count_all_nodes(&result.node()) > 4);
    }

    #[test]
    fn visitor_handles_splat_node() {
        let result = leak_parse(b"a, *b = [1, 2, 3]");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_class_variable_write() {
        let result = leak_parse(b"@@x = 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_global_variable_write() {
        let result = leak_parse(b"$x = 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_constant_path_write() {
        let result = leak_parse(b"Foo::BAR = 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_constant_path_node() {
        let result = leak_parse(b"Foo::Bar::Baz");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_alias_global_variable() {
        let result = leak_parse(b"alias $new $old");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_undef_node() {
        let result = leak_parse(b"undef :foo, :bar");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_local_variable_and_write() {
        let result = leak_parse(b"x = 1; x &&= 2");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_local_variable_or_write() {
        let result = leak_parse(b"x = 1; x ||= 2");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_instance_variable_operator_write() {
        let result = leak_parse(b"@x += 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_instance_variable_and_write() {
        let result = leak_parse(b"@x &&= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_instance_variable_or_write() {
        let result = leak_parse(b"@x ||= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_constant_operator_write() {
        let result = leak_parse(b"FOO += 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_constant_and_write() {
        let result = leak_parse(b"FOO &&= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_constant_or_write() {
        let result = leak_parse(b"FOO ||= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_constant_path_operator_write() {
        let result = leak_parse(b"Foo::BAR += 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_constant_path_and_write() {
        let result = leak_parse(b"Foo::BAR &&= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_constant_path_or_write() {
        let result = leak_parse(b"Foo::BAR ||= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_class_variable_operator_write() {
        let result = leak_parse(b"@@x += 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_class_variable_and_write() {
        let result = leak_parse(b"@@x &&= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_class_variable_or_write() {
        let result = leak_parse(b"@@x ||= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_global_variable_operator_write() {
        let result = leak_parse(b"$x += 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_global_variable_and_write() {
        let result = leak_parse(b"$x &&= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_global_variable_or_write() {
        let result = leak_parse(b"$x ||= 1");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_index_operator_write() {
        let result = leak_parse(b"a[0] += 1");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_index_and_write() {
        let result = leak_parse(b"a[0] &&= 1");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_index_or_write() {
        let result = leak_parse(b"a[0] ||= 1");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_interpolated_symbol() {
        let result = leak_parse(b":\"foo#{bar}\"");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_interpolated_regex() {
        let result = leak_parse(b"/foo#{bar}/");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_embedded_statements() {
        let result = leak_parse(b"\"hello #{1 + 2} world\"");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_parentheses_node() {
        let result = leak_parse(b"(1 + 2)");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_keyword_hash_node() {
        let result = leak_parse(b"foo(a: 1, b: 2)");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_assoc_splat_node() {
        let result = leak_parse(b"{**opts}");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_next_node() {
        let result = leak_parse(b"loop { next 1 }");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_break_with_value() {
        let result = leak_parse(b"loop { break 42 }");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_super_with_block() {
        let result = leak_parse(b"def foo; super(1) { |x| x }; end");
        assert!(count_all_nodes(&result.node()) > 4);
    }

    #[test]
    fn visitor_handles_when_node() {
        let result = leak_parse(b"case x; when 1, 2; 'a'; when 3; 'b'; end");
        assert!(count_all_nodes(&result.node()) > 5);
    }

    #[test]
    fn visitor_handles_rescue_node_standalone() {
        let result = leak_parse(b"begin; 1; rescue => e; 2; end");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_else_node() {
        let result = leak_parse(b"if true; 1; else; 2; end");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_ensure_node() {
        let result = leak_parse(b"begin; 1; ensure; 2; end");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_block_argument_node() {
        let result = leak_parse(b"arr.map(&method(:puts))");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_begin_with_rescue_else_ensure() {
        let result = leak_parse(
            b"begin; 1; rescue StandardError => e; 2; else; 3; ensure; 4; end",
        );
        assert!(count_all_nodes(&result.node()) > 6);
    }

    #[test]
    fn visitor_handles_chained_rescue() {
        let result = leak_parse(
            b"begin; rescue TypeError; 1; rescue StandardError; 2; end",
        );
        assert!(count_all_nodes(&result.node()) > 4);
    }

    #[test]
    fn visitor_handles_class_with_superclass() {
        let result = leak_parse(b"class Foo < Bar; def x; end; end");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_module_with_body() {
        let result = leak_parse(b"module Foo; module Bar; end; end");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_singleton_class() {
        let result = leak_parse(b"class << self; def foo; end; end");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_range_no_left() {
        let result = leak_parse(b"..10");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_range_no_right() {
        let result = leak_parse(b"1..");
        assert!(count_all_nodes(&result.node()) > 1);
    }

    #[test]
    fn visitor_handles_yield_with_args() {
        let result = leak_parse(b"def foo; yield 1, 2; end");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_return_with_value() {
        let result = leak_parse(b"def foo; return 42; end");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_for_with_statements() {
        let result = leak_parse(b"for x in [1, 2]; puts x; end");
        assert!(count_all_nodes(&result.node()) > 4);
    }

    #[test]
    fn visitor_handles_case_with_else() {
        let result = leak_parse(b"case x; when 1; 'a'; else; 'default'; end");
        assert!(count_all_nodes(&result.node()) > 4);
    }

    #[test]
    fn visitor_handles_case_no_predicate() {
        let result = leak_parse(b"case; when true; 1; end");
        assert!(count_all_nodes(&result.node()) > 2);
    }

    #[test]
    fn visitor_handles_lambda_with_params() {
        let result = leak_parse(b"->(x, y) { x + y }");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_block_with_params_and_body() {
        let result = leak_parse(b"[1].each { |x| puts x }");
        assert!(count_all_nodes(&result.node()) > 4);
    }

    #[test]
    fn visitor_handles_def_with_params() {
        let result = leak_parse(b"def foo(a, b); a + b; end");
        assert!(count_all_nodes(&result.node()) > 4);
    }

    #[test]
    fn visitor_handles_if_with_subsequent() {
        let result = leak_parse(b"if a; 1; elsif b; 2; else; 3; end");
        assert!(count_all_nodes(&result.node()) > 5);
    }

    #[test]
    fn visitor_handles_while_with_statements() {
        let result = leak_parse(b"while x; puts x; x -= 1; end");
        assert!(count_all_nodes(&result.node()) > 3);
    }

    #[test]
    fn visitor_handles_assoc_node() {
        let result = leak_parse(b"{a: 1, b: 2, c: 3}");
        assert!(count_all_nodes(&result.node()) > 6);
    }

    #[test]
    fn visitor_handles_in_node_with_statements() {
        let result = leak_parse(b"case x; in [1, 2]; puts 'matched'; end");
        assert!(count_all_nodes(&result.node()) > 4);
    }

    /// Helper to count direct children of a parsed node using for_each_direct_child.
    fn count_direct_children(node: &Node<'_>) -> usize {
        let mut count = 0;
        for_each_direct_child(node, &mut |_| count += 1);
        count
    }

    /// Helper to get the first statement from a program.
    fn first_stmt<'a>(result: &'a ruby_prism::ParseResult<'a>) -> Node<'a> {
        let prog = result.node().as_program_node().unwrap();
        prog.statements().body().iter().next().unwrap()
    }

    #[test]
    fn direct_children_program_node() {
        let result = leak_parse(b"1; 2; 3");
        assert_eq!(count_direct_children(&result.node()), 3);
    }

    #[test]
    fn direct_children_call_node() {
        let result = leak_parse(b"foo(1, 2)");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_call_with_receiver_and_block() {
        let result = leak_parse(b"arr.map { |x| x }");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_block_node() {
        let result = leak_parse(b"arr.map { |x| x }");
        let call = first_stmt(result).as_call_node().unwrap();
        let block = call.block().unwrap();
        assert!(count_direct_children(&block) >= 1);
    }

    #[test]
    fn direct_children_block_argument_node() {
        let result = leak_parse(b"arr.map(&:to_s)");
        let call = first_stmt(result).as_call_node().unwrap();
        let block_arg = call.block().unwrap();
        assert!(count_direct_children(&block_arg) >= 1);
    }

    #[test]
    fn direct_children_def_node() {
        let result = leak_parse(b"def foo(a); a + 1; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_for_node() {
        let result = leak_parse(b"for x in [1]; puts x; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 3);
    }

    #[test]
    fn direct_children_begin_node() {
        let result = leak_parse(b"begin; 1; rescue => e; 2; else; 3; ensure; 4; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 3);
    }

    #[test]
    fn direct_children_ensure_node() {
        let result = leak_parse(b"begin; ensure; 1; 2; end");
        let begin = first_stmt(result).as_begin_node().unwrap();
        let ensure = begin.ensure_clause().unwrap();
        // Test that ensure's statements are visited
        assert!(ensure.statements().is_some());
    }

    #[test]
    fn direct_children_else_node() {
        let result = leak_parse(b"if true; 1; else; 2; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 3);
    }

    #[test]
    fn direct_children_if_node() {
        let result = leak_parse(b"if a; 1; elsif b; 2; else; 3; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 3);
    }

    #[test]
    fn direct_children_unless_node() {
        let result = leak_parse(b"unless x; 1; else; 2; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 3);
    }

    #[test]
    fn direct_children_while_node() {
        let result = leak_parse(b"while true; 1; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_until_node() {
        let result = leak_parse(b"until false; 1; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_case_node() {
        let result = leak_parse(b"case x; when 1; 'a'; else; 'b'; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 3);
    }

    #[test]
    fn direct_children_when_node() {
        let result = leak_parse(b"case x; when 1, 2; 'a'; end");
        let case = first_stmt(result).as_case_node().unwrap();
        let when = case.conditions().iter().next().unwrap();
        assert!(count_direct_children(&when) >= 2);
    }

    #[test]
    fn direct_children_class_node() {
        let result = leak_parse(b"class Foo < Bar; def x; end; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_module_node() {
        let result = leak_parse(b"module Foo; def x; end; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_singleton_class_node() {
        let result = leak_parse(b"class << self; def x; end; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_array_node() {
        let result = leak_parse(b"[1, 2, 3]");
        let node = first_stmt(result);
        assert_eq!(count_direct_children(&node), 3);
    }

    #[test]
    fn direct_children_hash_node() {
        let result = leak_parse(b"{a: 1, b: 2}");
        let node = first_stmt(result);
        assert_eq!(count_direct_children(&node), 2);
    }

    #[test]
    fn direct_children_keyword_hash_node() {
        let result = leak_parse(b"foo(a: 1, b: 2)");
        let call = first_stmt(result).as_call_node().unwrap();
        let args = call.arguments().unwrap();
        let kw_hash = args.arguments().iter().next().unwrap();
        assert_eq!(count_direct_children(&kw_hash), 2);
    }

    #[test]
    fn direct_children_assoc_splat_node() {
        let result = leak_parse(b"{**opts}");
        let hash = first_stmt(result).as_hash_node().unwrap();
        let splat = hash.elements().iter().next().unwrap();
        assert_eq!(count_direct_children(&splat), 1);
    }

    #[test]
    fn direct_children_range_node() {
        let result = leak_parse(b"1..10");
        let node = first_stmt(result);
        assert_eq!(count_direct_children(&node), 2);
    }

    #[test]
    fn direct_children_parentheses_node() {
        let result = leak_parse(b"(1 + 2)");
        let node = first_stmt(result);
        assert_eq!(count_direct_children(&node), 1);
    }

    #[test]
    fn direct_children_interpolated_string() {
        let result = leak_parse(b"\"hello #{x} world\"");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_interpolated_symbol() {
        let result = leak_parse(b":\"hello#{x}\"");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 1);
    }

    #[test]
    fn direct_children_embedded_statements() {
        let result = leak_parse(b"\"#{1 + 2}\"");
        let interp = first_stmt(result).as_interpolated_string_node().unwrap();
        let embedded = interp.parts().iter().next().unwrap();
        assert!(count_direct_children(&embedded) >= 1);
    }

    #[test]
    fn direct_children_constant_path_node() {
        let result = leak_parse(b"Foo::Bar");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 1);
    }

    #[test]
    fn direct_children_multi_write_node() {
        let result = leak_parse(b"a, *b, c = 1, 2, 3");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 3);
    }

    #[test]
    fn direct_children_splat_node() {
        let result = leak_parse(b"a, *b = [1, 2]");
        let mw = first_stmt(result).as_multi_write_node().unwrap();
        let rest = mw.rest().unwrap();
        assert!(count_direct_children(&rest) >= 1);
    }

    #[test]
    fn direct_children_return_node() {
        let result = leak_parse(b"def foo; return 1, 2; end");
        let def = first_stmt(result).as_def_node().unwrap();
        let body_stmts = def.body().unwrap().as_statements_node().unwrap();
        let ret = body_stmts.body().iter().next().unwrap();
        assert!(count_direct_children(&ret) >= 2);
    }

    #[test]
    fn direct_children_yield_node() {
        let result = leak_parse(b"def foo; yield 1, 2; end");
        let def = first_stmt(result).as_def_node().unwrap();
        let body_stmts = def.body().unwrap().as_statements_node().unwrap();
        let yld = body_stmts.body().iter().next().unwrap();
        assert!(count_direct_children(&yld) >= 2);
    }

    #[test]
    fn direct_children_super_node() {
        let result = leak_parse(b"def foo; super(1) { 2 }; end");
        let def = first_stmt(result).as_def_node().unwrap();
        let body_stmts = def.body().unwrap().as_statements_node().unwrap();
        let sup = body_stmts.body().iter().next().unwrap();
        assert!(count_direct_children(&sup) >= 2);
    }

    #[test]
    fn direct_children_lambda_node() {
        let result = leak_parse(b"->(x) { x + 1 }");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_interpolated_regex() {
        let result = leak_parse(b"/foo#{bar}/");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 1);
    }

    #[test]
    fn direct_children_case_match_node() {
        let result = leak_parse(b"case x; in Integer; 1; else; 2; end");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_in_node() {
        let result = leak_parse(b"case x; in Integer; 1; end");
        let cm = first_stmt(result).as_case_match_node().unwrap();
        let in_node = cm.conditions().iter().next().unwrap();
        assert!(count_direct_children(&in_node) >= 2);
    }

    #[test]
    fn direct_children_break_node() {
        let result = leak_parse(b"loop { break 42 }");
        let call = first_stmt(result).as_call_node().unwrap();
        let block = call.block().unwrap().as_block_node().unwrap();
        let body_stmts = block.body().unwrap().as_statements_node().unwrap();
        let brk = body_stmts.body().iter().next().unwrap();
        assert_eq!(count_direct_children(&brk), 1);
    }

    #[test]
    fn direct_children_next_node() {
        let result = leak_parse(b"loop { next 42 }");
        let call = first_stmt(result).as_call_node().unwrap();
        let block = call.block().unwrap().as_block_node().unwrap();
        let body_stmts = block.body().unwrap().as_statements_node().unwrap();
        let nxt = body_stmts.body().iter().next().unwrap();
        assert_eq!(count_direct_children(&nxt), 1);
    }

    #[test]
    fn direct_children_undef_node() {
        let result = leak_parse(b"undef :foo, :bar");
        let node = first_stmt(result);
        assert_eq!(count_direct_children(&node), 2);
    }

    #[test]
    fn direct_children_index_operator_write() {
        let result = leak_parse(b"a[0] += 1");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_index_and_write() {
        let result = leak_parse(b"a[0] &&= 1");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn direct_children_index_or_write() {
        let result = leak_parse(b"a[0] ||= 1");
        let node = first_stmt(result);
        assert!(count_direct_children(&node) >= 2);
    }

    #[test]
    fn leaf_nodes_have_no_extra_children() {
        let leaf_sources: &[&[u8]] = &[b"42", b"3.14", b"'s'", b":sym", b"true", b"false", b"nil"];

        for source in leaf_sources {
            let result = leak_parse(source);
            let prog = result.node().as_program_node().unwrap();
            let node = prog.statements().body().iter().next().unwrap();
            let mut child_count = 0;
            for_each_direct_child(&node, &mut |_| child_count += 1);
            assert_eq!(
                child_count,
                0,
                "Expected 0 children for {:?}",
                std::str::from_utf8(source)
            );
        }
    }
}
