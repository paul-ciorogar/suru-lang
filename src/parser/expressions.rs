use super::helpers::get_precedence;
use super::{ParseError, Parser};
use crate::ast::{AstNode, NodeType};
use crate::lexer::TokenKind;

// Recursive expression parsing methods
impl<'a> Parser<'a> {
    /// Parse an expression recursively with precedence climbing
    /// Returns the AST node index of the parsed expression
    pub(super) fn parse_expression(
        &mut self,
        depth: usize,
        min_precedence: u8,
    ) -> Result<usize, ParseError> {
        self.check_depth(depth)?;
        self.parse_expression_internal(depth, min_precedence)
    }

    /// Internal expression parser using precedence climbing
    fn parse_expression_internal(
        &mut self,
        depth: usize,
        min_precedence: u8,
    ) -> Result<usize, ParseError> {
        // Parse left side (primary or unary)
        let mut left_idx = self.parse_primary_or_unary(depth)?;

        // POSTFIX PHASE: Handle function calls and method calls
        loop {
            match self.current_token().kind {
                // Function call: identifier(...)
                TokenKind::LParen if self.ast.nodes[left_idx].node_type == NodeType::Identifier => {
                    left_idx = self.parse_function_call(depth, left_idx)?;
                }
                // Method call: expr.method(...) or expr.property
                TokenKind::Dot => {
                    left_idx = self.parse_method_call(depth, left_idx)?;
                }
                _ => break,
            }
        }

        // Handle binary operators with precedence climbing
        loop {
            let token = self.current_token();

            // Check if we have a binary operator
            let op_precedence = match get_precedence(&token.kind) {
                Some(p) if p >= min_precedence => p,
                _ => break, // No more operators at this precedence level
            };

            // Consume the operator
            let op_kind = token.kind.clone();
            self.advance();

            // Parse right side with higher precedence for left-associativity
            let right_idx = self.parse_expression(depth + 1, op_precedence + 1)?;

            // Create binary operator node
            let op_node_type = match op_kind {
                TokenKind::And => NodeType::And,
                TokenKind::Or => NodeType::Or,
                _ => unreachable!(),
            };

            let op_node = AstNode::new(op_node_type);
            let op_node_idx = self.ast.add_node(op_node);

            // Add children: left then right
            self.ast.add_child(op_node_idx, left_idx);
            self.ast.add_child(op_node_idx, right_idx);

            // The operator node becomes the new left side
            left_idx = op_node_idx;
        }

        Ok(left_idx)
    }

    /// Parse primary expression (literals) or unary operator (not)
    fn parse_primary_or_unary(&mut self, depth: usize) -> Result<usize, ParseError> {
        let token = self.current_token();

        match &token.kind {
            // Unary not operator
            TokenKind::Not => {
                self.advance(); // Consume 'not'

                // Parse the operand recursively with 'not' precedence
                let operand_idx = self.parse_expression(depth + 1, 3)?; // 3 is precedence of 'not'

                // Create not node
                let not_node = AstNode::new(NodeType::Not);
                let not_node_idx = self.ast.add_node(not_node);

                // Add operand as child
                self.ast.add_child(not_node_idx, operand_idx);

                Ok(not_node_idx)
            }

            // Primary expressions: literals
            TokenKind::True | TokenKind::False => {
                let literal_node = AstNode::new_terminal(NodeType::LiteralBoolean, self.current);
                let literal_node_idx = self.ast.add_node(literal_node);
                self.advance();
                Ok(literal_node_idx)
            }

            TokenKind::Number(_) => {
                let literal_node = AstNode::new_terminal(NodeType::LiteralNumber, self.current);
                let literal_node_idx = self.ast.add_node(literal_node);
                self.advance();
                Ok(literal_node_idx)
            }

            TokenKind::String(_) => {
                let literal_node = AstNode::new_terminal(NodeType::LiteralString, self.current);
                let literal_node_idx = self.ast.add_node(literal_node);
                self.advance();
                Ok(literal_node_idx)
            }

            // Identifiers (for function calls and variable references)
            TokenKind::Identifier => {
                let ident_node = AstNode::new_terminal(NodeType::Identifier, self.current);
                let ident_node_idx = self.ast.add_node(ident_node);
                self.advance();
                Ok(ident_node_idx)
            }

            _ => Err(ParseError::unexpected_token(
                "expression (literal, identifier, or 'not')",
                token,
                self.current,
                self.source,
            )),
        }
    }

    /// Parse a function call: identifier(arg1, arg2, ...)
    /// ident_idx is the index of the already-parsed identifier node
    /// Returns the FunctionCall node index
    fn parse_function_call(&mut self, depth: usize, ident_idx: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Create FunctionCall node
        let call_node = AstNode::new(NodeType::FunctionCall);
        let call_node_idx = self.ast.add_node(call_node);

        // Add identifier as first child
        self.ast.add_child(call_node_idx, ident_idx);

        // Parse arguments and add ArgList as second child
        let arg_list_idx = self.parse_argument_list(depth)?;
        self.ast.add_child(call_node_idx, arg_list_idx);

        Ok(call_node_idx)
    }

    /// Parse a function call argument
    /// Uses parse_expression but prevents nested function calls for now
    fn parse_function_argument(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Parse the argument as an expression
        let arg_idx = self.parse_expression(depth + 1, 0)?;

        // Check if it's a function call or method call (nested calls not allowed)
        if self.ast.nodes[arg_idx].node_type == NodeType::FunctionCall
            || self.ast.nodes[arg_idx].node_type == NodeType::MethodCall
        {
            let token = self.current_token();
            return Err(ParseError::from_token(
                "Nested function/method calls are not supported".to_string(),
                token,
                self.current,
            ));
        }
        // Note: PropertyAccess is allowed in arguments since it's just a field read

        Ok(arg_idx)
    }

    /// Parse argument list: consumes '(' and ')', parses comma-separated arguments
    /// Creates an ArgList node and adds arguments as its children
    /// Returns the ArgList node index
    fn parse_argument_list(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume '('
        self.consume(TokenKind::LParen, "(")?;

        // Create ArgList node
        let arg_list_node = AstNode::new(NodeType::ArgList);
        let arg_list_idx = self.ast.add_node(arg_list_node);

        // Parse arguments (comma-separated list)
        loop {
            // Check for closing paren (empty args or end of list)
            if self.current_token().kind == TokenKind::RParen {
                self.advance(); // Consume ')'
                break;
            }

            // Parse argument
            let arg_idx = self.parse_function_argument(depth + 1)?;
            self.ast.add_child(arg_list_idx, arg_idx);

            // Check for comma or closing paren
            let token = self.current_token();
            match token.kind {
                TokenKind::Comma => {
                    self.advance(); // Consume comma, continue to next argument
                }
                TokenKind::RParen => {
                    self.advance(); // Consume ')'
                    break;
                }
                _ => {
                    return Err(ParseError::unexpected_token(
                        "',' or ')'",
                        token,
                        self.current,
                        self.source,
                    ));
                }
            }
        }

        Ok(arg_list_idx)
    }

    /// Parse a method call: receiver.method(args) or receiver.property
    /// receiver_idx is the index of the already-parsed receiver expression
    /// Returns the MethodCall or PropertyAccess node index
    fn parse_method_call(&mut self, depth: usize, receiver_idx: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume '.'
        self.consume(TokenKind::Dot, ".")?;

        // Parse method/property name (must be identifier)
        let token = self.current_token();
        if token.kind != TokenKind::Identifier {
            return Err(ParseError::unexpected_token(
                "method or property name (identifier)",
                token,
                self.current,
                self.source,
            ));
        }

        let name_node = AstNode::new_terminal(NodeType::Identifier, self.current);
        let name_idx = self.ast.add_node(name_node);
        self.advance();

        // Check if this is a method call (has '(') or property access (no '(')
        if self.current_token().kind == TokenKind::LParen {
            // METHOD CALL: receiver.method(args)
            let call_node = AstNode::new(NodeType::MethodCall);
            let call_node_idx = self.ast.add_node(call_node);

            // Add receiver and method name as first two children
            self.ast.add_child(call_node_idx, receiver_idx);
            self.ast.add_child(call_node_idx, name_idx);

            // Parse arguments and add ArgList as third child
            let arg_list_idx = self.parse_argument_list(depth)?;
            self.ast.add_child(call_node_idx, arg_list_idx);

            Ok(call_node_idx)
        } else {
            // PROPERTY ACCESS: receiver.property
            let access_node = AstNode::new(NodeType::PropertyAccess);
            let access_node_idx = self.ast.add_node(access_node);

            // Add receiver and property name as children
            self.ast.add_child(access_node_idx, receiver_idx);
            self.ast.add_child(access_node_idx, name_idx);

            Ok(access_node_idx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::lexer::lex;

    fn to_ast(source: &str) -> Result<Ast, ParseError> {
        let tokens = lex(source).unwrap();
        let limits = crate::limits::CompilerLimits::default();
        parse(source, &tokens, limits)
    }

    fn to_ast_string(source: &str) -> Result<String, ParseError> {
        let tokens = lex(source).unwrap();
        let limits = crate::limits::CompilerLimits::default();
        let ast = parse(source, &tokens, limits)?;
        Ok(ast.to_string(&tokens))
    }

    // Boolean operator tests
    #[test]
    fn test_not_operator() {
        let ast = to_ast("x: not true\n").unwrap();

        // Should have: Program, VarDecl, Identifier, LiteralBoolean, Not
        assert_eq!(ast.nodes.len(), 5);
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[2].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralBoolean);
        assert_eq!(ast.nodes[4].node_type, NodeType::Not);

        // Verify tree: VarDecl -> [Identifier, Not], Not -> LiteralBoolean
        assert_eq!(ast.nodes[1].first_child, Some(2)); // VarDecl -> Identifier
        assert_eq!(ast.nodes[2].next_sibling, Some(4)); // Identifier -> Not
        assert_eq!(ast.nodes[4].first_child, Some(3)); // Not -> LiteralBoolean
    }

    #[test]
    fn test_and_operator() {
        let ast = to_ast("x: true and false\n").unwrap();

        // Nodes: Program, VarDecl, Identifier, LiteralBoolean(true), LiteralBoolean(false), And
        assert_eq!(ast.nodes.len(), 6);
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[2].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralBoolean); // left: true
        assert_eq!(ast.nodes[4].node_type, NodeType::LiteralBoolean); // right: false
        assert_eq!(ast.nodes[5].node_type, NodeType::And);

        // Verify tree: VarDecl -> [Identifier, And], And -> [true, false]
        assert_eq!(ast.nodes[1].first_child, Some(2)); // VarDecl -> Identifier
        assert_eq!(ast.nodes[2].next_sibling, Some(5)); // Identifier -> And
        assert_eq!(ast.nodes[5].first_child, Some(3)); // And -> true
        assert_eq!(ast.nodes[3].next_sibling, Some(4)); // true -> false
    }

    #[test]
    fn test_or_operator() {
        let ast = to_ast("x: true or false\n").unwrap();

        // Nodes: Program, VarDecl, Identifier, LiteralBoolean(true), Or, LiteralBoolean(false)
        assert_eq!(ast.nodes.len(), 6);

        // Find the Or node and verify structure
        let var_decl_idx = ast.nodes[0].first_child.unwrap();
        let ident_idx = ast.nodes[var_decl_idx].first_child.unwrap();
        let expr_idx = ast.nodes[ident_idx].next_sibling.unwrap();

        assert_eq!(ast.nodes[expr_idx].node_type, NodeType::Or);
    }

    #[test]
    fn test_precedence_and_over_or() {
        // "true or false and true" should parse as "true or (false and true)"
        let ast = to_ast("x: true or false and true\n").unwrap();

        // Just verify it parses and has correct number of nodes
        assert_eq!(ast.nodes.len(), 8);

        // Find the expression and verify Or is at top level (lowest precedence)
        let var_decl_idx = ast.nodes[0].first_child.unwrap();
        let ident_idx = ast.nodes[var_decl_idx].first_child.unwrap();
        let expr_idx = ast.nodes[ident_idx].next_sibling.unwrap();

        // Top level should be Or (lowest precedence)
        assert_eq!(ast.nodes[expr_idx].node_type, NodeType::Or);

        // Or's right child should be And (higher precedence)
        let or_left = ast.nodes[expr_idx].first_child.unwrap();
        let or_right = ast.nodes[or_left].next_sibling.unwrap();
        assert_eq!(ast.nodes[or_right].node_type, NodeType::And);
    }

    #[test]
    fn test_precedence_not_over_and() {
        // "not true and false" should parse as "(not true) and false"
        let ast = to_ast("x: not true and false\n").unwrap();

        // Just verify it parses and has correct number of nodes
        assert_eq!(ast.nodes.len(), 7);

        // Find expression
        let var_decl_idx = ast.nodes[0].first_child.unwrap();
        let ident_idx = ast.nodes[var_decl_idx].first_child.unwrap();
        let expr_idx = ast.nodes[ident_idx].next_sibling.unwrap();

        // Top should be And
        assert_eq!(ast.nodes[expr_idx].node_type, NodeType::And);

        // And's left child should be Not
        let and_left = ast.nodes[expr_idx].first_child.unwrap();
        assert_eq!(ast.nodes[and_left].node_type, NodeType::Not);
    }

    // Function call tests
    #[test]
    fn test_simple_function_call_no_args() {
        let ast = to_ast_string("x: print()\n").unwrap();

        let expected = "\
Program
  VarDecl
    Identifier 'x'
    FunctionCall
      Identifier 'print'
      ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_call_single_arg() {
        let ast = to_ast_string("x: print(42)\n").unwrap();

        let expected = "\
Program
  VarDecl
    Identifier 'x'
    FunctionCall
      Identifier 'print'
      ArgList
        LiteralNumber '42'
";

        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_call_multiple_args() {
        let ast = to_ast_string("x: add(1, 2, 3)\n").unwrap();

        let expected = "\
Program
  VarDecl
    Identifier 'x'
    FunctionCall
      Identifier 'add'
      ArgList
        LiteralNumber '1'
        LiteralNumber '2'
        LiteralNumber '3'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_call_string_arg() {
        let ast = to_ast_string("x: print('hello')\n").unwrap();

        let expected = "\
Program
  VarDecl
    Identifier 'x'
    FunctionCall
      Identifier 'print'
      ArgList
        LiteralString 'hello'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_call_boolean_args() {
        let ast = to_ast_string("x: test(true, false)\n").unwrap();

        let expected = "\
Program
  VarDecl
    Identifier 'x'
    FunctionCall
      Identifier 'test'
      ArgList
        LiteralBoolean 'true'
        LiteralBoolean 'false'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_error_nested_function_call() {
        let result = to_ast("x: outer(inner())\n");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Nested"));
    }

    #[test]
    fn test_recursion_depth_limit() {
        // Create deeply nested not expressions to test depth limit
        let limits = crate::limits::CompilerLimits {
            max_expr_depth: 5,
            ..Default::default()
        };

        let source = "x: not not not not not not true\n"; // 6 nots (depth 6+)
        let tokens = lex(source).unwrap();
        let result = parse(source, &tokens, limits);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("too deep"));
    }

    // ========== METHOD CALL TESTS ==========

    // Basic method calls
    #[test]
    fn test_simple_method_call_no_args() {
        let ast = to_ast_string("x: person.greet()\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      Identifier 'person'
      Identifier 'greet'
      ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_simple_method_call_with_args() {
        let ast = to_ast_string("x: person.greet('Alice', 42)\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      Identifier 'person'
      Identifier 'greet'
      ArgList
        LiteralString 'Alice'
        LiteralNumber '42'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_property_access() {
        let ast = to_ast_string("x: person.name\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    PropertyAccess
      Identifier 'person'
      Identifier 'name'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_method_on_literal_string() {
        let ast = to_ast_string("x: 'hello'.toUpper()\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      LiteralString 'hello'
      Identifier 'toUpper'
      ArgList
";
        assert_eq!(ast, expected);
    }

    // Method chaining
    #[test]
    fn test_method_chaining_two_calls() {
        let ast = to_ast_string("x: numbers.add(6).set(0)\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      MethodCall
        Identifier 'numbers'
        Identifier 'add'
        ArgList
          LiteralNumber '6'
      Identifier 'set'
      ArgList
        LiteralNumber '0'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_method_chaining_three_calls() {
        let ast = to_ast_string("x: numbers.add(6).add(7).set(0, 0)\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      MethodCall
        MethodCall
          Identifier 'numbers'
          Identifier 'add'
          ArgList
            LiteralNumber '6'
        Identifier 'add'
        ArgList
          LiteralNumber '7'
      Identifier 'set'
      ArgList
        LiteralNumber '0'
        LiteralNumber '0'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_property_then_method() {
        let ast = to_ast_string("x: template.metadata.toString()\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      PropertyAccess
        Identifier 'template'
        Identifier 'metadata'
      Identifier 'toString'
      ArgList
";
        assert_eq!(ast, expected);
    }

    // Precedence & integration
    #[test]
    fn test_method_call_in_boolean_expression() {
        let ast_struct = to_ast("x: value.isValid() and other.check()\n").unwrap();
        let var_decl_idx = ast_struct.nodes[0].first_child.unwrap();
        let ident_idx = ast_struct.nodes[var_decl_idx].first_child.unwrap();
        let expr_idx = ast_struct.nodes[ident_idx].next_sibling.unwrap();

        // Top level should be And
        assert_eq!(ast_struct.nodes[expr_idx].node_type, NodeType::And);

        // Both children should be MethodCall
        let left_idx = ast_struct.nodes[expr_idx].first_child.unwrap();
        let right_idx = ast_struct.nodes[left_idx].next_sibling.unwrap();
        assert_eq!(ast_struct.nodes[left_idx].node_type, NodeType::MethodCall);
        assert_eq!(ast_struct.nodes[right_idx].node_type, NodeType::MethodCall);
    }

    #[test]
    fn test_method_call_with_not_operator() {
        let ast_struct = to_ast("x: not value.isValid()\n").unwrap();
        let var_decl_idx = ast_struct.nodes[0].first_child.unwrap();
        let ident_idx = ast_struct.nodes[var_decl_idx].first_child.unwrap();
        let expr_idx = ast_struct.nodes[ident_idx].next_sibling.unwrap();

        // Top level should be Not
        assert_eq!(ast_struct.nodes[expr_idx].node_type, NodeType::Not);

        // Child should be MethodCall
        let operand_idx = ast_struct.nodes[expr_idx].first_child.unwrap();
        assert_eq!(ast_struct.nodes[operand_idx].node_type, NodeType::MethodCall);
    }

    #[test]
    fn test_function_call_then_method_call() {
        let ast = to_ast_string("x: getUser().getName()\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      FunctionCall
        Identifier 'getUser'
        ArgList
      Identifier 'getName'
      ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_method_on_number_literal() {
        let ast = to_ast_string("x: 42.toString()\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      LiteralNumber '42'
      Identifier 'toString'
      ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_method_on_boolean_literal() {
        let ast = to_ast_string("x: true.toString()\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      LiteralBoolean 'true'
      Identifier 'toString'
      ArgList
";
        assert_eq!(ast, expected);
    }

    // Error cases
    #[test]
    fn test_error_method_call_without_name() {
        let result = to_ast("x: person.()\n");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("method or property name"));
    }

    #[test]
    fn test_error_nested_method_call_in_args() {
        let result = to_ast("x: obj.method(inner.call())\n");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Nested"));
    }
}
