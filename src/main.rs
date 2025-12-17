pub mod ast;
pub mod codegen;
pub mod lexer;
pub mod limits;
pub mod parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load compiler limits from project.toml or use defaults
    let limits = match limits::CompilerLimits::from_project_toml("project.toml") {
        Ok(l) => {
            l.validate()?;
            println!("✓ Loaded limits from project.toml");
            l
        }
        Err(e) => {
            println!("Using default compiler limits ({})", e.message);
            limits::CompilerLimits::default()
        }
    };

    // Demo parser
    let source = "x: true or false and true\n";
    println!("\nParsing source:\n{}\n", source);

    // Use new limits-aware API
    let tokens = lexer::lex_with_limits(source, limits.clone())?;
    let tokens_for_print = tokens.clone();
    let ast = parser::parse_with_limits(source, tokens, limits)?;

    println!("Parsed {} nodes in AST", ast.nodes.len());
    println!("Root node index: {:?}\n", ast.root);

    // Print tree structure
    if let Some(root_idx) = ast.root {
        print_tree(&ast, &tokens_for_print, root_idx, 0);
    }

    Ok(())
}

// Helper to print tree recursively (just for demo)
fn print_tree(ast: &ast::Ast, tokens: &[lexer::Token], node_idx: usize, depth: usize) {
    let node = &ast.nodes[node_idx];
    let indent = "  ".repeat(depth);

    let text = ast
        .node_text(node_idx, tokens)
        .map(|s| format!(" \"{}\"", s))
        .unwrap_or_default();

    println!("{}{:?}{}", indent, node.node_type, text);

    // Print children
    if let Some(child_idx) = node.first_child {
        let mut current = child_idx;
        loop {
            print_tree(ast, tokens, current, depth + 1);
            if let Some(next) = ast.nodes[current].next_sibling {
                current = next;
            } else {
                break;
            }
        }
    }
}
