use super::{ParseError, Parser};
use crate::ast::{AstNode, NodeType};
use crate::lexer::TokenKind;

// Recursive statement parsing methods
impl<'a> Parser<'a> {
    /// Parse all statements in the program
    pub(super) fn parse_statements(&mut self, depth: usize) -> Result<(), ParseError> {
        loop {
            self.skip_newlines();

            if self.current_token().kind == TokenKind::Eof {
                break;
            }

            if let Some(stmt_idx) = self.parse_statement(depth + 1)? {
                if let Some(root_idx) = self.ast.root {
                    self.ast.add_child(root_idx, stmt_idx);
                }
            }
        }
        Ok(())
    }

    /// Parse a single statement
    pub(super) fn parse_statement(&mut self, depth: usize) -> Result<Option<usize>, ParseError> {
        self.check_depth(depth)?;
        self.skip_newlines();

        let token = self.current_token();
        match &token.kind {
            TokenKind::Return => {
                // Return statement
                return Ok(Some(self.parse_return_stmt(depth + 1)?));
            }
            TokenKind::Type => {
                // Type declaration
                return Ok(Some(self.parse_type_decl(depth + 1)?));
            }
            TokenKind::Identifier => {
                // Lookahead to distinguish function/variable/call
                match self.peek_statement_type()? {
                    "function" => Ok(Some(self.parse_function_decl(depth + 1)?)),
                    "variable" => Ok(Some(self.parse_var_decl(depth + 1)?)),
                    "call" => self.parse_standalone_call(depth + 1),
                    _ => unreachable!(),
                }
            }
            TokenKind::RBrace => Ok(None), // End of block
            TokenKind::Eof => Ok(None),
            _ => Err(ParseError::unexpected_token(
                "statement",
                token,
                self.current,
                self.source,
            )),
        }
    }

    /// Helper: Determine statement type by looking ahead after identifier
    /// Returns 'function', 'variable', or 'call'
    fn peek_statement_type(&self) -> Result<&'static str, ParseError> {
        let next_idx = self.current + 1;
        if next_idx < self.tokens.len() {
            let next_token = &self.tokens[next_idx];

            match &next_token.kind {
                TokenKind::Colon => {
                    // Check token after colon (skip newlines)
                    let mut after_colon_idx = next_idx + 1;
                    while after_colon_idx < self.tokens.len()
                        && self.tokens[after_colon_idx].kind == TokenKind::Newline
                    {
                        after_colon_idx += 1;
                    }

                    if after_colon_idx < self.tokens.len() {
                        if self.tokens[after_colon_idx].kind == TokenKind::LParen {
                            // Function declaration: ident : ()
                            Ok("function")
                        } else {
                            // Variable declaration: ident : expr
                            Ok("variable")
                        }
                    } else {
                        Err(ParseError::unexpected_token(
                            "expression or '('",
                            self.current_token(),
                            self.current,
                            self.source,
                        ))
                    }
                }
                TokenKind::LParen => {
                    // Standalone function call: ident()
                    Ok("call")
                }
                _ => Err(ParseError::unexpected_token(
                    "':' or '('",
                    next_token,
                    next_idx,
                    self.source,
                )),
            }
        } else {
            Err(ParseError::unexpected_token(
                "':' or '('",
                self.current_token(),
                self.current,
                self.source,
            ))
        }
    }

    /// Helper: Parse standalone function call as expression statement
    fn parse_standalone_call(&mut self, depth: usize) -> Result<Option<usize>, ParseError> {
        // Parse the function call expression
        let expr_idx = self.parse_expression(depth + 1, 0)?;

        // Wrap in ExprStmt
        let expr_stmt = AstNode::new(NodeType::ExprStmt);
        let expr_stmt_idx = self.ast.add_node(expr_stmt);
        self.ast.add_child(expr_stmt_idx, expr_idx);

        // Expect newline, EOF, or RBrace (end of block)
        match &self.current_token().kind {
            TokenKind::Newline => self.advance(),
            TokenKind::Eof | TokenKind::RBrace => {
                // Let caller handle RBrace
            }
            _ => {
                return Err(ParseError::unexpected_token(
                    "newline, '}', or end of file",
                    self.current_token(),
                    self.current,
                    self.source,
                ));
            }
        }

        Ok(Some(expr_stmt_idx))
    }

    /// Parse a variable declaration: identifier : expression
    fn parse_var_decl(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // 1. Parse identifier token
        let ident_token = self.current_token();
        if ident_token.kind != TokenKind::Identifier {
            return Err(ParseError::unexpected_token(
                "identifier",
                ident_token,
                self.current,
                self.source,
            ));
        }

        // 2. Create nodes
        let var_decl_node = AstNode::new(NodeType::VarDecl);
        let var_decl_idx = self.ast.add_node(var_decl_node);

        let ident_node = AstNode::new_terminal(NodeType::Identifier, self.current);
        let ident_idx = self.ast.add_node(ident_node);
        self.ast.add_child(var_decl_idx, ident_idx);

        self.advance(); // consume identifier

        // 3. Expect colon
        self.consume(TokenKind::Colon, "':'")?;

        // 4. Parse expression
        let expr_idx = self.parse_expression(depth + 1, 0)?;
        self.ast.add_child(var_decl_idx, expr_idx);

        // 5. Expect newline, EOF, or RBrace (end of block)
        let token = self.current_token();
        match &token.kind {
            TokenKind::Newline => {
                self.advance();
            }
            TokenKind::Eof | TokenKind::RBrace => {
                // EOF or RBrace is fine, don't consume (let caller handle RBrace)
            }
            _ => {
                return Err(ParseError::unexpected_token(
                    "newline, '}', or end of file",
                    token,
                    self.current,
                    self.source,
                ));
            }
        }

        Ok(var_decl_idx)
    }

    /// Parse a return statement: return or return expr
    fn parse_return_stmt(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume 'return' keyword
        self.consume(TokenKind::Return, "return keyword")?;

        // Create ReturnStmt node
        let return_stmt_node = AstNode::new(NodeType::ReturnStmt);
        let return_stmt_idx = self.ast.add_node(return_stmt_node);

        // Check if there's a return value
        self.skip_newlines();
        let token = self.current_token();
        match &token.kind {
            TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace => {
                // No return value (void return) - don't consume, let caller handle
                Ok(return_stmt_idx)
            }
            _ => {
                // Parse return expression
                self.parse_return_expression(depth, return_stmt_idx)
            }
        }
    }

    fn parse_return_expression(
        &mut self,
        depth: usize,
        return_stmt_idx: usize,
    ) -> Result<usize, ParseError> {
        // Parse return expression
        let expr_idx = self.parse_expression(depth + 1, 0)?;
        self.ast.add_child(return_stmt_idx, expr_idx);

        // Expect newline, EOF, or RBrace
        let token = self.current_token();
        match &token.kind {
            TokenKind::Newline => {
                self.advance();
            }
            TokenKind::Eof | TokenKind::RBrace => {
                // EOF or RBrace is fine, don't consume (let caller handle RBrace)
            }
            _ => {
                return Err(ParseError::unexpected_token(
                    "newline, '}', or end of file after return statement",
                    token,
                    self.current,
                    self.source,
                ));
            }
        }

        Ok(return_stmt_idx)
    }

    /// Parse a block: { statements }
    fn parse_block(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume '{'
        self.consume(TokenKind::LBrace, "'{'")?;

        // Create Block node
        let block_node = AstNode::new(NodeType::Block);
        let block_idx = self.ast.add_node(block_node);

        // Parse statements until '}'
        loop {
            self.skip_newlines();

            // Check for closing brace
            if self.current_token().kind == TokenKind::RBrace {
                self.advance(); // Consume '}'
                break;
            }

            // Check for EOF (error case - unclosed block)
            if self.current_token().kind == TokenKind::Eof {
                return Err(ParseError::unexpected_token(
                    "'}'",
                    self.current_token(),
                    self.current,
                    self.source,
                ));
            }

            // Parse statement (variable decl or expression statement)
            if let Some(stmt_idx) = self.parse_statement(depth + 1)? {
                self.ast.add_child(block_idx, stmt_idx);
            }
        }

        Ok(block_idx)
    }

    /// Parse a single parameter: name or name Type
    fn parse_param(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Expect identifier (parameter name)
        let name_token = self.current_token();
        if name_token.kind != TokenKind::Identifier {
            return Err(ParseError::unexpected_token(
                "parameter name",
                name_token,
                self.current,
                self.source,
            ));
        }

        // Create Param node
        let param_node = AstNode::new(NodeType::Param);
        let param_idx = self.ast.add_node(param_node);

        // Create Identifier node for parameter name
        let ident_node = AstNode::new_terminal(NodeType::Identifier, self.current);
        let ident_idx = self.ast.add_node(ident_node);
        self.ast.add_child(param_idx, ident_idx);
        self.advance(); // Consume parameter name

        // Check for optional type annotation (identifier after parameter name)
        self.skip_newlines();
        if self.current_token().kind == TokenKind::Identifier {
            // This is a type annotation
            let type_node = AstNode::new_terminal(NodeType::TypeAnnotation, self.current);
            let type_idx = self.ast.add_node(type_node);
            self.ast.add_child(param_idx, type_idx);
            self.advance(); // Consume type name
        }

        Ok(param_idx)
    }

    /// Parse parameter list: () or (name, name) or (name Type, name Type)
    fn parse_param_list(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume '('
        self.consume(TokenKind::LParen, "'('")?;

        // Create ParamList node
        let param_list_node = AstNode::new(NodeType::ParamList);
        let param_list_idx = self.ast.add_node(param_list_node);

        // Skip newlines (allow formatting like `(\n)`)
        self.skip_newlines();

        // Check for empty parameter list
        if self.current_token().kind == TokenKind::RParen {
            self.advance(); // Consume ')'
            return Ok(param_list_idx);
        }

        // Parse parameters (comma-separated list)
        loop {
            // Parse parameter
            let param_idx = self.parse_param(depth + 1)?;
            self.ast.add_child(param_list_idx, param_idx);

            // Skip newlines after parameter
            self.skip_newlines();

            // Check for closing paren or comma
            let token = self.current_token();
            match token.kind {
                TokenKind::RParen => {
                    self.advance(); // Consume ')'
                    break;
                }
                TokenKind::Comma => {
                    self.advance(); // Consume comma, continue to next parameter
                    self.skip_newlines();

                    // Allow trailing comma (if next is ')')
                    if self.current_token().kind == TokenKind::RParen {
                        self.advance(); // Consume ')'
                        break;
                    }
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

        Ok(param_list_idx)
    }

    /// Parse function declaration: identifier : (params) ReturnType { statements }
    fn parse_function_decl(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Parse identifier
        let ident_token = self.current_token();
        if ident_token.kind != TokenKind::Identifier {
            return Err(ParseError::unexpected_token(
                "identifier",
                ident_token,
                self.current,
                self.source,
            ));
        }

        let ident_node = AstNode::new_terminal(NodeType::Identifier, self.current);
        let ident_idx = self.ast.add_node(ident_node);
        self.advance(); // Consume identifier

        // Expect colon
        self.consume(TokenKind::Colon, "':'")?;

        // Skip newlines and parse parameter list
        self.skip_newlines();
        let param_list_idx = self.parse_param_list(depth + 1)?;

        // Create FunctionDecl node (before adding children)
        let func_decl_node = AstNode::new(NodeType::FunctionDecl);
        let func_decl_idx = self.ast.add_node(func_decl_node);

        // Add children: identifier and params
        self.ast.add_child(func_decl_idx, ident_idx);
        self.ast.add_child(func_decl_idx, param_list_idx);

        // Check for optional return type annotation
        self.skip_newlines();
        if self.current_token().kind == TokenKind::Identifier {
            // This is a return type annotation
            let return_type_node = AstNode::new_terminal(NodeType::TypeAnnotation, self.current);
            let return_type_idx = self.ast.add_node(return_type_node);
            self.ast.add_child(func_decl_idx, return_type_idx);
            self.advance(); // Consume return type
        }

        // Skip newlines and parse block body
        self.skip_newlines();
        let block_idx = self.parse_block(depth + 1)?;

        // Add block as child
        self.ast.add_child(func_decl_idx, block_idx);

        Ok(func_decl_idx)
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

    // Variable declaration tests
    #[test]
    fn test_single_boolean_decl() {
        let ast = to_ast("flag: true\n").unwrap();

        // Should have: Program, VarDecl, Identifier, LiteralBoolean
        assert_eq!(ast.nodes.len(), 4);
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[2].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralBoolean);

        // Verify tree structure
        assert_eq!(ast.nodes[0].first_child, Some(1)); // Program -> VarDecl
        assert_eq!(ast.nodes[1].first_child, Some(2)); // VarDecl -> Identifier
        assert_eq!(ast.nodes[2].next_sibling, Some(3)); // Identifier -> Literal
    }

    #[test]
    fn test_multiple_decls() {
        let ast = to_ast("x: 42\ny: \"hello\"\nz: false\n").unwrap();

        // Should have: Program + 3 VarDecls (each with Identifier + Literal) = 1 + 3*3 = 10 nodes
        assert_eq!(ast.nodes.len(), 10);
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);

        // Verify first VarDecl
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[2].node_type, NodeType::Identifier);
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralNumber);

        // Verify sibling links between VarDecls
        assert_eq!(ast.nodes[1].next_sibling, Some(4));
        assert_eq!(ast.nodes[4].next_sibling, Some(7));
        assert_eq!(ast.nodes[7].next_sibling, None);
    }

    // Function declaration tests
    #[test]
    fn test_empty_function() {
        let ast = to_ast_string("f: () {}\n").unwrap();

        let expected = "\
Program
  FunctionDecl
    Identifier 'f'
    ParamList
    Block
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_with_single_call() {
        let ast = to_ast_string("main: () {\n    print('hello')\n}\n").unwrap();

        let expected = "\
Program
  FunctionDecl
    Identifier 'main'
    ParamList
    Block
      ExprStmt
        FunctionCall
          Identifier 'print'
          LiteralString 'hello'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_with_multiple_statements() {
        let ast = to_ast_string("main: () {\n    x: 42\n    print(x)\n}\n").unwrap();

        let expected = "\
Program
  FunctionDecl
    Identifier 'main'
    ParamList
    Block
      VarDecl
        Identifier 'x'
        LiteralNumber '42'
      ExprStmt
        FunctionCall
          Identifier 'print'
          Identifier 'x'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_with_return_type() {
        let ast = to_ast_string("getValue: () Number {\n    return 42\n}\n").unwrap();

        let expected = "\
Program
  FunctionDecl
    Identifier 'getValue'
    ParamList
    TypeAnnotation 'Number'
    Block
      ReturnStmt
        LiteralNumber '42'
";
        assert_eq!(ast, expected);
    }

    // Return statement tests
    #[test]
    fn test_return_void() {
        let ast = to_ast_string("f: () {\n    return\n}\n").unwrap();

        let expected = "\
Program
  FunctionDecl
    Identifier 'f'
    ParamList
    Block
      ReturnStmt
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_return_literal_number() {
        let ast = to_ast_string("f: () {\n    return 42\n}\n").unwrap();

        let expected = "\
Program
  FunctionDecl
    Identifier 'f'
    ParamList
    Block
      ReturnStmt
        LiteralNumber '42'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_return_variable() {
        let ast = to_ast_string("f: () {\n    return x\n}\n").unwrap();

        let expected = "\
Program
  FunctionDecl
    Identifier 'f'
    ParamList
    Block
      ReturnStmt
        Identifier 'x'
";
        assert_eq!(ast, expected);
    }

    // Standalone call tests
    #[test]
    fn test_standalone_function_call() {
        let ast = to_ast_string("print('hello')\n").unwrap();

        let expected = "\
Program
  ExprStmt
    FunctionCall
      Identifier 'print'
      LiteralString 'hello'
";
        assert_eq!(ast, expected);
    }

    // Parameter tests
    #[test]
    fn test_function_with_typed_params() {
        let ast = to_ast_string("add: (x Number, y Number) {}\n").unwrap();

        let expected = "\
Program
  FunctionDecl
    Identifier 'add'
    ParamList
      Param
        Identifier 'x'
        TypeAnnotation 'Number'
      Param
        Identifier 'y'
        TypeAnnotation 'Number'
    Block
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_with_inferred_params() {
        let ast = to_ast_string("identity: (value) {}\n").unwrap();

        let expected = "\
Program
  FunctionDecl
    Identifier 'identity'
    ParamList
      Param
        Identifier 'value'
    Block
";
        assert_eq!(ast, expected);
    }
}
