// Composition operator tests
// The composition operator (+) is parsed as a binary operator in expressions.rs
// This file contains comprehensive tests for composition behavior.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::lexer::lex;

    fn to_ast(source: &str) -> Result<Ast, ParseError> {
        let limits = crate::limits::CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        parse(tokens, &limits)
    }

    // ========== Category 1: Basic Composition ==========

    #[test]
    fn test_simple_compose() {
        let ast = to_ast("x: base + extension").unwrap();
        let root = ast.root.unwrap();

        // Program -> VarDecl
        let var_decl = ast.nodes[root].first_child.unwrap();
        assert_eq!(ast.nodes[var_decl].node_type, NodeType::VarDecl);

        // VarDecl -> Identifier, Compose
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[identifier].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> Identifier(base), Identifier(extension)
        let left = ast.nodes[compose].first_child.unwrap();
        let right = ast.nodes[left].next_sibling.unwrap();

        assert_eq!(ast.nodes[left].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[right].node_type, NodeType::Identifier);
    }

    #[test]
    fn test_compose_with_struct_literals() {
        let ast = to_ast("x: {a: 1} + {b: 2}").unwrap();
        let root = ast.root.unwrap();

        // Program -> VarDecl -> Identifier, Compose
        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> StructInit, StructInit
        let left = ast.nodes[compose].first_child.unwrap();
        let right = ast.nodes[left].next_sibling.unwrap();

        assert_eq!(ast.nodes[left].node_type, NodeType::StructInit);
        assert_eq!(ast.nodes[right].node_type, NodeType::StructInit);
    }

    #[test]
    fn test_compose_with_identifiers() {
        let ast = to_ast("result: obj1 + obj2").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);
    }

    #[test]
    fn test_compose_with_literals() {
        let ast = to_ast("x: 1 + 2").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> LiteralNumber, LiteralNumber
        let left = ast.nodes[compose].first_child.unwrap();
        let right = ast.nodes[left].next_sibling.unwrap();

        assert_eq!(ast.nodes[left].node_type, NodeType::LiteralNumber);
        assert_eq!(ast.nodes[right].node_type, NodeType::LiteralNumber);
    }

    // ========== Category 2: Composition Chaining ==========

    #[test]
    fn test_compose_chaining_two() {
        // a + b + c should parse as ((a + b) + c) - left-associative
        let ast = to_ast("x: a + b + c").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let outer_compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[outer_compose].node_type, NodeType::Compose);

        // Outer Compose -> inner Compose, Identifier(c)
        let inner_compose = ast.nodes[outer_compose].first_child.unwrap();
        let c = ast.nodes[inner_compose].next_sibling.unwrap();

        assert_eq!(ast.nodes[inner_compose].node_type, NodeType::Compose);
        assert_eq!(ast.nodes[c].node_type, NodeType::Identifier);

        // Inner Compose -> Identifier(a), Identifier(b)
        let a = ast.nodes[inner_compose].first_child.unwrap();
        let b = ast.nodes[a].next_sibling.unwrap();

        assert_eq!(ast.nodes[a].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[b].node_type, NodeType::Identifier);
    }

    #[test]
    fn test_compose_chaining_three() {
        // Verify left-associativity with 4 terms: a + b + c + d
        let ast = to_ast("x: a + b + c + d").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let outer_compose = ast.nodes[identifier].next_sibling.unwrap();

        // Should be: (((a + b) + c) + d)
        assert_eq!(ast.nodes[outer_compose].node_type, NodeType::Compose);

        // The left child should be another Compose
        let middle_compose = ast.nodes[outer_compose].first_child.unwrap();
        assert_eq!(ast.nodes[middle_compose].node_type, NodeType::Compose);
    }

    #[test]
    fn test_compose_with_method_composition() {
        // Method composition in struct literal
        let ast = to_ast("obj: base + { method: getValue() }").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> Identifier(base), StructInit
        let base = ast.nodes[compose].first_child.unwrap();
        let struct_init = ast.nodes[base].next_sibling.unwrap();

        assert_eq!(ast.nodes[base].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[struct_init].node_type, NodeType::StructInit);
    }

    // ========== Category 3: Precedence Interactions ==========

    #[test]
    fn test_compose_precedence_with_and() {
        // a + b and c should parse as a + (b and c)
        // because 'and' has higher precedence (2) than + (1)
        let ast = to_ast("x: a + b and c").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> Identifier(a), And
        let a = ast.nodes[compose].first_child.unwrap();
        let and_node = ast.nodes[a].next_sibling.unwrap();

        assert_eq!(ast.nodes[a].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[and_node].node_type, NodeType::And);

        // And -> Identifier(b), Identifier(c)
        let b = ast.nodes[and_node].first_child.unwrap();
        let c = ast.nodes[b].next_sibling.unwrap();

        assert_eq!(ast.nodes[b].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[c].node_type, NodeType::Identifier);
    }

    #[test]
    fn test_compose_precedence_with_or() {
        // a or b + c should parse as (a or b) + c
        // because 'or' and '+' have same precedence (1), left-associative
        let ast = to_ast("x: a or b + c").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> Or, Identifier(c)
        let or_node = ast.nodes[compose].first_child.unwrap();
        let c = ast.nodes[or_node].next_sibling.unwrap();

        assert_eq!(ast.nodes[or_node].node_type, NodeType::Or);
        assert_eq!(ast.nodes[c].node_type, NodeType::Identifier);
    }

    #[test]
    fn test_compose_precedence_with_pipe() {
        // a | b + c should parse as (a | b) + c
        // because '|' and '+' have same precedence (1), left-associative
        let ast = to_ast("x: a | b + c").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> Pipe, Identifier(c)
        let pipe = ast.nodes[compose].first_child.unwrap();
        let c = ast.nodes[pipe].next_sibling.unwrap();

        assert_eq!(ast.nodes[pipe].node_type, NodeType::Pipe);
        assert_eq!(ast.nodes[c].node_type, NodeType::Identifier);
    }

    #[test]
    fn test_compose_with_not() {
        // not a + b should parse as (not a) + b
        // because 'not' has higher precedence (3) than + (1)
        let ast = to_ast("x: not a + b").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> Not, Identifier(b)
        let not_node = ast.nodes[compose].first_child.unwrap();
        let b = ast.nodes[not_node].next_sibling.unwrap();

        assert_eq!(ast.nodes[not_node].node_type, NodeType::Not);
        assert_eq!(ast.nodes[b].node_type, NodeType::Identifier);
    }

    #[test]
    fn test_compose_vs_intersectiontype() {
        // Verify that + in type context creates IntersectionType,
        // but + in expression context creates Compose

        // Type context
        let ast = to_ast("type T: A + B").unwrap();
        let root = ast.root.unwrap();
        let type_decl = ast.nodes[root].first_child.unwrap();
        assert_eq!(ast.nodes[type_decl].node_type, NodeType::TypeDecl);

        // Find the IntersectionType node
        let type_name = ast.nodes[type_decl].first_child.unwrap();
        let type_body = ast.nodes[type_name].next_sibling.unwrap();
        let intersection = ast.nodes[type_body].first_child.unwrap();
        assert_eq!(ast.nodes[intersection].node_type, NodeType::IntersectionType);

        // Expression context
        let ast2 = to_ast("x: a + b").unwrap();
        let root2 = ast2.root.unwrap();
        let var_decl = ast2.nodes[root2].first_child.unwrap();
        let identifier = ast2.nodes[var_decl].first_child.unwrap();
        let compose = ast2.nodes[identifier].next_sibling.unwrap();
        assert_eq!(ast2.nodes[compose].node_type, NodeType::Compose);
    }

    // ========== Category 4: Complex Expressions ==========

    #[test]
    fn test_compose_in_variable_declaration() {
        // Full struct composition
        let ast = to_ast("user: { name: \"Paul\" } + { age: 30 }").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        let left_struct = ast.nodes[compose].first_child.unwrap();
        let right_struct = ast.nodes[left_struct].next_sibling.unwrap();

        assert_eq!(ast.nodes[left_struct].node_type, NodeType::StructInit);
        assert_eq!(ast.nodes[right_struct].node_type, NodeType::StructInit);
    }

    #[test]
    fn test_compose_with_function_calls() {
        let ast = to_ast("x: getData() + enhance()").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> FunctionCall, FunctionCall
        let left_call = ast.nodes[compose].first_child.unwrap();
        let right_call = ast.nodes[left_call].next_sibling.unwrap();

        assert_eq!(ast.nodes[left_call].node_type, NodeType::FunctionCall);
        assert_eq!(ast.nodes[right_call].node_type, NodeType::FunctionCall);
    }

    #[test]
    fn test_compose_with_pipes_and_compose() {
        // base | transform + extra should parse as (base | transform) + extra
        let ast = to_ast("x: base | transform + extra").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Compose -> Pipe, Identifier(extra)
        let pipe = ast.nodes[compose].first_child.unwrap();
        let extra = ast.nodes[pipe].next_sibling.unwrap();

        assert_eq!(ast.nodes[pipe].node_type, NodeType::Pipe);
        assert_eq!(ast.nodes[extra].node_type, NodeType::Identifier);
    }

    #[test]
    fn test_compose_with_method_calls() {
        let ast = to_ast("x: obj.getData() + other.getExtra()").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let compose = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);

        // Both sides should be MethodCall nodes
        let left_method = ast.nodes[compose].first_child.unwrap();
        let right_method = ast.nodes[left_method].next_sibling.unwrap();

        assert_eq!(ast.nodes[left_method].node_type, NodeType::MethodCall);
        assert_eq!(ast.nodes[right_method].node_type, NodeType::MethodCall);
    }

    // ========== Category 5: Edge Cases ==========

    #[test]
    fn test_compose_in_function_argument() {
        // Compose can be used in function arguments
        let ast = to_ast("result: process(a + b)").unwrap();
        let root = ast.root.unwrap();

        let var_decl = ast.nodes[root].first_child.unwrap();
        let identifier = ast.nodes[var_decl].first_child.unwrap();
        let func_call = ast.nodes[identifier].next_sibling.unwrap();

        assert_eq!(ast.nodes[func_call].node_type, NodeType::FunctionCall);

        // FunctionCall -> Identifier(process), ArgList
        let process_id = ast.nodes[func_call].first_child.unwrap();
        let arg_list = ast.nodes[process_id].next_sibling.unwrap();

        assert_eq!(ast.nodes[arg_list].node_type, NodeType::ArgList);

        // ArgList -> Compose
        let compose = ast.nodes[arg_list].first_child.unwrap();
        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);
    }

    #[test]
    fn test_compose_in_return_statement() {
        // Compose can be used in return statements
        let ast = to_ast("f: () { return base + extension }").unwrap();
        let root = ast.root.unwrap();

        // Program -> FunctionDecl (functions are FunctionDecl, not VarDecl)
        let func_decl = ast.nodes[root].first_child.unwrap();
        assert_eq!(ast.nodes[func_decl].node_type, NodeType::FunctionDecl);

        // FunctionDecl -> Identifier, ParamList, Block
        let identifier = ast.nodes[func_decl].first_child.unwrap();
        let param_list = ast.nodes[identifier].next_sibling.unwrap();
        let block = ast.nodes[param_list].next_sibling.unwrap();

        assert_eq!(ast.nodes[identifier].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[param_list].node_type, NodeType::ParamList);
        assert_eq!(ast.nodes[block].node_type, NodeType::Block);

        // Block -> ReturnStmt
        let return_stmt = ast.nodes[block].first_child.unwrap();
        assert_eq!(ast.nodes[return_stmt].node_type, NodeType::ReturnStmt);

        // ReturnStmt -> Compose
        let compose = ast.nodes[return_stmt].first_child.unwrap();
        assert_eq!(ast.nodes[compose].node_type, NodeType::Compose);
    }
}
