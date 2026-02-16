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

        match self.peek_kind() {
            TokenKind::Module => {
                // Module declaration
                return Ok(Some(self.parse_module_decl(depth + 1)?));
            }
            TokenKind::Import => {
                // Import statement
                return Ok(Some(self.parse_import_stmt(depth + 1)?));
            }
            TokenKind::Export => {
                // Export statement
                return Ok(Some(self.parse_export_stmt(depth + 1)?));
            }
            TokenKind::Return => {
                // Return statement
                return Ok(Some(self.parse_return_stmt(depth + 1)?));
            }
            TokenKind::Type => {
                // Type declaration
                return Ok(Some(self.parse_type_decl(depth + 1)?));
            }
            TokenKind::Match => {
                // Match statement
                return Ok(Some(self.parse_match_stmt(depth + 1)?));
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
            _ => Err(self.new_unexpected_token("statement")),
        }
    }

    /// Helper: Determine statement type by looking ahead after identifier
    /// Returns 'function', 'variable', or 'call'
    /// Handles optional type annotations: ident [Type] : expr
    fn peek_statement_type(&self) -> Result<&'static str, ParseError> {
        match self.peek_next_kind(1) {
            TokenKind::Colon => {
                // Pattern: ident : ...
                match self.peek_next_kind(2) {
                    TokenKind::LParen => Ok("function"), // ident : ()
                    _ => Ok("variable"),                 // ident : expr
                }
            }
            TokenKind::Identifier => {
                // Could be type annotation: ident Type : ...
                // Check if there's a colon after the type
                match self.peek_next_kind(2) {
                    TokenKind::Colon => {
                        // Pattern: ident Type : ...
                        // Check what follows the colon
                        match self.peek_next_kind(3) {
                            TokenKind::LParen => Ok("function"), // ident Type : ()
                            _ => Ok("variable"),                 // ident Type : expr
                        }
                    }
                    _ => Err(self.new_unexpected_token("':' after type annotation")),
                }
            }
            TokenKind::Lt => {
                // Could be generic function: ident<T>: (params) { body }
                // Scan past <...> to find ':'
                let mut offset = 2;
                let mut angle_depth = 1;
                loop {
                    match self.peek_next_kind(offset) {
                        TokenKind::Lt => angle_depth += 1,
                        TokenKind::Gt => {
                            angle_depth -= 1;
                            if angle_depth == 0 {
                                offset += 1;
                                break;
                            }
                        }
                        TokenKind::Eof => {
                            return Err(self.new_unexpected_token("'>'"));
                        }
                        _ => {}
                    }
                    offset += 1;
                }
                // Token after '>' should be ':'
                match self.peek_next_kind(offset) {
                    TokenKind::Colon => match self.peek_next_kind(offset + 1) {
                        TokenKind::LParen => Ok("function"),
                        _ => Ok("variable"),
                    },
                    _ => Err(self.new_unexpected_token("':' after generic parameters")),
                }
            }
            TokenKind::LParen => {
                // Standalone function call: ident()
                Ok("call")
            }
            _ => Err(self.new_unexpected_token("':' or '('")),
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
        match self.peek_kind() {
            TokenKind::Newline => self.advance(),
            TokenKind::Eof | TokenKind::RBrace => {
                // Let caller handle RBrace
            }
            _ => {
                return Err(self.new_unexpected_token("newline, '}', or end of file"));
            }
        }

        Ok(Some(expr_stmt_idx))
    }

    /// Helper: Parse match statement (match wrapped as ExprStmt)
    fn parse_match_stmt(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume 'match' keyword
        self.consume(TokenKind::Match, "'match'")?;

        // Parse match expression (reuses existing implementation)
        let match_idx = self.parse_match_expression(depth + 1)?;

        // Wrap in ExprStmt
        let expr_stmt = AstNode::new(NodeType::ExprStmt);
        let expr_stmt_idx = self.ast.add_node(expr_stmt);
        self.ast.add_child(expr_stmt_idx, match_idx);

        // Expect newline, EOF, or RBrace (end of block)
        match self.peek_kind() {
            TokenKind::Newline => self.advance(),
            TokenKind::Eof | TokenKind::RBrace => {
                // Let caller handle RBrace
            }
            _ => {
                return Err(self.new_unexpected_token("newline, '}', or end of file"));
            }
        }

        Ok(expr_stmt_idx)
    }

    /// Parse a variable declaration: identifier [Type] : expression
    /// Supports optional type annotations and struct literals
    fn parse_var_decl(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Parse identifier token
        if self.peek_kind() != TokenKind::Identifier {
            return Err(self.new_unexpected_token("identifier"));
        }

        // Create nodes
        let var_decl_node = AstNode::new(NodeType::VarDecl);
        let var_decl_idx = self.ast.add_node(var_decl_node);

        let ident_node = AstNode::new_terminal(NodeType::Identifier, self.clone_current_token());
        let ident_idx = self.ast.add_node(ident_node);
        self.ast.add_child(var_decl_idx, ident_idx);

        self.advance(); // consume identifier

        // Check for optional type annotation
        self.skip_newlines();
        if self.peek_kind_is(TokenKind::Identifier) {
            // Could be a type annotation - need to look ahead for ':'
            let next_token_kind = self.peek_next_kind(1);
            if next_token_kind == TokenKind::Colon {
                // This is a type annotation: identifier TypeName : expr
                let type_node =
                    AstNode::new_terminal(NodeType::TypeAnnotation, self.clone_current_token());
                let type_idx = self.ast.add_node(type_node);
                self.ast.add_child(var_decl_idx, type_idx);
                self.advance(); // consume type name
            }
        }

        // Expect colon
        self.consume(TokenKind::Colon, "':'")?;

        // Skip newlines before expression
        self.skip_newlines();

        // Parse the expression (which can include struct literals)
        let expr_idx = self.parse_expression(depth + 1, 0)?;

        self.ast.add_child(var_decl_idx, expr_idx);

        // Expect newline, EOF, or RBrace (end of block)
        match self.peek_kind() {
            TokenKind::Newline => {
                self.advance();
            }
            TokenKind::Eof | TokenKind::RBrace => {
                // EOF or RBrace is fine, don't consume (let caller handle RBrace)
            }
            _ => {
                return Err(self.new_unexpected_token("newline, '}', or end of file"));
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
        match self.peek_kind() {
            TokenKind::Newline => {
                self.advance();
            }
            TokenKind::Eof | TokenKind::RBrace => {
                // EOF or RBrace is fine, don't consume (let caller handle RBrace)
            }
            _ => {
                return Err(self
                    .new_unexpected_token("newline, '}', or end of file after return statement"));
            }
        }

        Ok(return_stmt_idx)
    }

    /// Parse a block: { statements }
    pub(super) fn parse_block(&mut self, depth: usize) -> Result<usize, ParseError> {
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
            if self.peek_kind_is(TokenKind::Eof) {
                return Err(self.new_unexpected_token("'}'"));
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
        if self.peek_kind() != TokenKind::Identifier {
            return Err(self.new_unexpected_token("parameter name"));
        }

        // Create Param node
        let param_node = AstNode::new(NodeType::Param);
        let param_idx = self.ast.add_node(param_node);

        // Create Identifier node for parameter name
        let ident_node = AstNode::new_terminal(NodeType::Identifier, self.clone_current_token());
        let ident_idx = self.ast.add_node(ident_node);
        self.ast.add_child(param_idx, ident_idx);
        self.advance(); // Consume parameter name

        // Check for optional type annotation (identifier after parameter name)
        self.skip_newlines();
        if self.current_token().kind == TokenKind::Identifier {
            // This is a type annotation
            let type_node =
                AstNode::new_terminal(NodeType::TypeAnnotation, self.clone_current_token());
            let type_idx = self.ast.add_node(type_node);
            self.ast.add_child(param_idx, type_idx);
            self.advance(); // Consume type name
        }

        Ok(param_idx)
    }

    /// Parse parameter list: () or (name, name) or (name Type, name Type)
    pub(super) fn parse_param_list(&mut self, depth: usize) -> Result<usize, ParseError> {
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
            match self.peek_kind() {
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
                    return Err(self.new_unexpected_token("',' or ')'"));
                }
            }
        }

        Ok(param_list_idx)
    }

    /// Parse function declaration: identifier : (params) ReturnType { statements }
    fn parse_function_decl(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Parse identifier
        if self.peek_kind() != TokenKind::Identifier {
            return Err(self.new_unexpected_token("identifier"));
        }

        let ident_node = AstNode::new_terminal(NodeType::Identifier, self.clone_current_token());
        let ident_idx = self.ast.add_node(ident_node);
        self.advance(); // Consume identifier

        // Check for optional type parameters: ident<T, U>
        let type_params_idx = if self.peek_kind() == TokenKind::Lt {
            Some(self.parse_type_params(depth + 1)?)
        } else {
            None
        };

        // Expect colon
        self.consume(TokenKind::Colon, "':'")?;

        // Skip newlines and parse parameter list
        self.skip_newlines();
        let param_list_idx = self.parse_param_list(depth + 1)?;

        // Create FunctionDecl node (before adding children)
        let func_decl_node = AstNode::new(NodeType::FunctionDecl);
        let func_decl_idx = self.ast.add_node(func_decl_node);

        // Add children: identifier, optional type params, and params
        self.ast.add_child(func_decl_idx, ident_idx);
        if let Some(tp_idx) = type_params_idx {
            self.ast.add_child(func_decl_idx, tp_idx);
        }
        self.ast.add_child(func_decl_idx, param_list_idx);

        // Check for optional return type annotation
        self.skip_newlines();
        if self.current_token().kind == TokenKind::Identifier {
            // This is a return type annotation
            let return_type_node =
                AstNode::new_terminal(NodeType::TypeAnnotation, self.clone_current_token());
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
        let limits = crate::limits::CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        parse(tokens, &limits)
    }

    fn to_ast_string(source: &str) -> Result<String, ParseError> {
        let limits = crate::limits::CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits)?;
        Ok(ast.to_string())
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
          ArgList
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
          ArgList
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
      ArgList
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

    // Match statement test
    #[test]
    fn test_match_stmt_basic() {
        let ast = to_ast_string(
            r#"
            match status {
                Success: print("success")
                Error: exit()
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  ExprStmt
    Match
      MatchSubject
        Identifier 'status'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Success'
          FunctionCall
            Identifier 'print'
            ArgList
              LiteralString 'success'
        MatchArm
          MatchPattern
            Identifier 'Error'
          FunctionCall
            Identifier 'exit'
            ArgList
";
        assert_eq!(ast, expected);
    }

    // Tests for type annotations in var_decl

    #[test]
    fn test_var_decl_with_type_annotation_number() {
        let ast = to_ast_string("count Int16: 42\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'count'
    TypeAnnotation 'Int16'
    LiteralNumber '42'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_var_decl_with_type_annotation_function_call() {
        let ast = to_ast_string("name String: getName()\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'name'
    TypeAnnotation 'String'
    FunctionCall
      Identifier 'getName'
      ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_var_decl_with_type_annotation_expression() {
        let ast = to_ast_string("result Bool: x and y\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'result'
    TypeAnnotation 'Bool'
    And
      Identifier 'x'
      Identifier 'y'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_var_decl_with_type_and_struct_literal() {
        let ast = to_ast_string("user User: { name: \"Paul\" }\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'user'
    TypeAnnotation 'User'
    StructInit
      StructInitField
        Identifier 'name'
        LiteralString 'Paul'
";
        assert_eq!(ast, expected);
    }
}
