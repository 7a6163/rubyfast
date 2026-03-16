//! Spike: Verify ruby-prism API for the migration from lib-ruby-parser.
//!
//! Run with: cargo run --example prism_spike

use ruby_prism::*;

fn main() {
    println!("=== 1. Block/Call structure ===");
    spike_block_call();

    println!("\n=== 2. Send on block (chained call) ===");
    spike_send_on_block();

    println!("\n=== 3. For loop locations ===");
    spike_for_loop();

    println!("\n=== 4. Method definition ===");
    spike_def();

    println!("\n=== 5. Rescue body ===");
    spike_rescue();

    println!("\n=== 6. Range types ===");
    spike_range();

    println!("\n=== 7. String / Integer values ===");
    spike_values();

    println!("\n=== 8. Comments ===");
    spike_comments();

    println!("\n=== 9. Error tolerance ===");
    spike_errors();

    println!("\n=== 10. ASCII encoding ===");
    spike_encoding();

    println!("\n=== 11. Block pass (symbol to proc) ===");
    spike_block_pass();

    println!("\n=== 12. Child nodes / visitor ===");
    spike_visitor();

    println!("\n=== 13. ERB template (expected error) ===");
    spike_erb();

    println!("\n=== 14. Node enum pattern matching ===");
    spike_pattern_match();
}

fn spike_block_call() {
    // arr.map { |x| x.to_s }
    let source = b"arr.map { |x| x.to_s }";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    println!("Root: {:#?}", result.node());
}

fn spike_send_on_block() {
    // arr.select { |x| x > 1 }.first
    let source = b"arr.select { |x| x > 1 }.first";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    println!("Root: {:#?}", result.node());
}

fn spike_for_loop() {
    let source = b"for x in [1, 2, 3]\n  puts x\nend";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    println!("Root: {:#?}", result.node());
}

fn spike_def() {
    let source = b"def foo(a, &block); block.call; end";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    println!("Root: {:#?}", result.node());
}

fn spike_rescue() {
    let source = b"begin; x; rescue NoMethodError => e; retry; end";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    println!("Root: {:#?}", result.node());
}

fn spike_range() {
    let source = b"(1..10).include?(5); (1...10).cover?(5)";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    println!("Root: {:#?}", result.node());
}

fn spike_values() {
    let source = b"'x'; 42; 1; :sym";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    println!("Root: {:#?}", result.node());
}

fn spike_comments() {
    let source = b"x = 1 # rubyfast:disable shuffle_first_vs_sample\ny = 2\n";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    for comment in result.comments() {
        let loc = comment.location();
        println!(
            "  Comment at {}..{}: {:?}",
            loc.start_offset(),
            loc.end_offset(),
            std::str::from_utf8(loc.as_slice()).unwrap_or("<invalid utf8>")
        );
    }
}

fn spike_errors() {
    let source = b"def foo; end; def def; end";
    let result = parse(source);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    let error_count: usize = result.errors().count();
    println!("Errors: {}", error_count);
    for err in result.errors() {
        println!("  Error: {:?}", err.message());
    }
    println!("Has AST: true (prism always produces one)");
    println!("Root: {:#?}", result.node());
}

fn spike_encoding() {
    let source = b"# encoding: us-ascii\nx = 1\n";
    let result = parse(source);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    let error_count: usize = result.errors().count();
    println!("Errors: {}", error_count);
    print_errors(&result);
    println!("Root: {:#?}", result.node());
}

fn spike_block_pass() {
    let source = b"arr.map(&:to_s)";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    println!("Root: {:#?}", result.node());
}

fn spike_visitor() {
    let source = b"arr.select { |x| x > 1 }.first";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());

    // Use the Visit trait
    struct CallCounter {
        count: usize,
    }
    impl<'pr> Visit<'pr> for CallCounter {
        fn visit_call_node(&mut self, node: &CallNode<'pr>) {
            println!(
                "  Found CallNode: name={:?}, has_receiver={}, has_block={}",
                std::str::from_utf8(node.name().as_slice()).unwrap_or("?"),
                node.receiver().is_some(),
                node.block().is_some()
            );
            self.count += 1;
            // Must call the default visitor to recurse into children
            visit_call_node(self, node);
        }
    }

    let mut counter = CallCounter { count: 0 };
    counter.visit(&result.node());
    println!("  Total CallNodes found: {}", counter.count);
}

fn spike_erb() {
    let source = b"class Foo < ActiveRecord::Migration<%= migration_version %>\nend";
    let result = parse(source);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());
    let error_count: usize = result.errors().count();
    println!("Errors: {}", error_count);
    for err in result.errors() {
        println!("  Error: {:?}", err.message());
    }
}

fn spike_pattern_match() {
    let source = b"arr.map { |x| x.to_s }";
    let result = parse(source);
    print_errors(&result);
    println!("Source: {:?}", std::str::from_utf8(source).unwrap());

    // Walk the top-level statements
    let program = result.node();
    if let Node::ProgramNode { .. } = &program {
        let prog = program.as_program_node().unwrap();
        let stmts = prog.statements();
        for node in stmts.body().iter() {
            println!("  Top-level node variant:");
            match &node {
                Node::CallNode { .. } => {
                    let call = node.as_call_node().unwrap();
                    println!(
                        "    CallNode: name={:?}, has_block={}",
                        std::str::from_utf8(call.name().as_slice()).unwrap_or("?"),
                        call.block().is_some()
                    );
                    if let Some(block) = call.block() {
                        println!("    Block: {:#?}", block);
                    }
                }
                other => println!("    {:?}", other),
            }
        }
    }
}

fn print_errors(result: &ParseResult) {
    for err in result.errors() {
        println!("  PARSE ERROR: {:?}", err.message());
    }
}
