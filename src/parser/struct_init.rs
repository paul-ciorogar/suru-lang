use super::{ParseError, Parser};
use crate::ast::{AstNode, NodeType};
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    /// Parse struct initialization: { field: value, ... }
    /// Returns StructInit node index
    pub(super) fn parse_struct_init(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume '{'
        self.consume(TokenKind::LBrace, "'{'")?;

        // Create StructInit node
        let struct_init = AstNode::new(NodeType::StructInit);
        let struct_init_idx = self.ast.add_node(struct_init);

        // Parse fields/methods until '}'
        loop {
            self.skip_newlines();

            // Check for closing brace
            if self.current_token().kind == TokenKind::RBrace {
                self.advance();
                break;
            }

            // Check for EOF (error)
            if self.peek_kind_is(TokenKind::Eof) {
                return Err(self.new_unexpected_token("'}' to close struct literal"));
            }

            // Parse field or method
            let member_idx = self.parse_struct_init_member(depth + 1)?;
            self.ast.add_child(struct_init_idx, member_idx);

            // Skip newlines after member
            self.skip_newlines();

            // Check for comma (optional between members)
            if self.peek_kind_is(TokenKind::Comma) {
                self.advance(); // consume comma
                // Continue to next member
            } else if self.peek_kind_is(TokenKind::RBrace) {
                // End of struct - will be handled in next iteration
                continue;
            } else if !self.peek_kind_is(TokenKind::Newline) && !self.peek_kind_is(TokenKind::Eof) {
                // If not comma, newline, or closing brace, could be an error
                // But newlines already handled by skip_newlines, so continue
            }
        }

        Ok(struct_init_idx)
    }

    /// Parse struct initialization member (field or method)
    /// Handles privacy marker (_) prefix
    fn parse_struct_init_member(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Check for privacy marker
        let is_private = if self.peek_kind_is(TokenKind::Underscore) {
            self.advance(); // Consume '_'
            self.skip_newlines(); // Allow whitespace after _
            true
        } else {
            false
        };

        // Expect identifier (member name)
        if self.peek_kind() != TokenKind::Identifier {
            return Err(self.new_unexpected_token("field or method name"));
        }

        let name_token = self.clone_current_token();
        self.advance(); // Consume name

        // Expect ':'
        self.consume(TokenKind::Colon, "':' after member name")?;

        // Skip newlines before value
        self.skip_newlines();

        // Disambiguate field vs method by checking for '('
        let is_method = self.peek_kind_is(TokenKind::LParen);

        if is_method {
            // Parse method
            self.parse_struct_method(depth + 1, name_token, is_private)
        } else {
            // Parse field
            self.parse_struct_field(depth + 1, name_token, is_private)
        }
    }

    /// Parse struct field initialization: name: value
    fn parse_struct_field(
        &mut self,
        depth: usize,
        name_token: crate::lexer::Token,
        is_private: bool,
    ) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Create StructInitField node (with or without privacy flag)
        let field_node = if is_private {
            AstNode::new_private(NodeType::StructInitField)
        } else {
            AstNode::new(NodeType::StructInitField)
        };
        let field_idx = self.ast.add_node(field_node);

        // Add name as first child (with privacy flag if needed)
        let name_node = if is_private {
            AstNode::new_private_terminal(NodeType::Identifier, name_token)
        } else {
            AstNode::new_terminal(NodeType::Identifier, name_token)
        };
        let name_idx = self.ast.add_node(name_node);
        self.ast.add_child(field_idx, name_idx);

        // Parse field value expression
        let value_idx = self.parse_expression(depth + 1, 0)?;
        self.ast.add_child(field_idx, value_idx);

        Ok(field_idx)
    }

    /// Parse struct method initialization: name: (params) ReturnType { ... }
    fn parse_struct_method(
        &mut self,
        depth: usize,
        name_token: crate::lexer::Token,
        is_private: bool,
    ) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Create StructInitMethod node (with or without privacy flag)
        let method_node = if is_private {
            AstNode::new_private(NodeType::StructInitMethod)
        } else {
            AstNode::new(NodeType::StructInitMethod)
        };
        let method_idx = self.ast.add_node(method_node);

        // Clone name_token for use in FunctionDecl (needed for both StructInitMethod name and FunctionDecl name)
        let func_name_token = name_token.clone();

        // Add name as first child (with privacy flag if needed)
        let name_node = if is_private {
            AstNode::new_private_terminal(NodeType::Identifier, name_token)
        } else {
            AstNode::new_terminal(NodeType::Identifier, name_token)
        };
        let name_idx = self.ast.add_node(name_node);
        self.ast.add_child(method_idx, name_idx);

        // Parse parameter list
        self.skip_newlines();
        let param_list_idx = self.parse_param_list(depth + 1)?;

        // Create FunctionDecl node (reuse existing)
        let func_decl = AstNode::new(NodeType::FunctionDecl);
        let func_decl_idx = self.ast.add_node(func_decl);

        // Add function name as first child (for AST consistency with regular FunctionDecl)
        let func_name_node = AstNode::new_terminal(NodeType::Identifier, func_name_token);
        let func_name_idx = self.ast.add_node(func_name_node);
        self.ast.add_child(func_decl_idx, func_name_idx);

        // Add params as second child of function
        self.ast.add_child(func_decl_idx, param_list_idx);

        // Check for optional return type
        self.skip_newlines();
        if self.current_token().kind == TokenKind::Identifier {
            let return_type =
                AstNode::new_terminal(NodeType::TypeAnnotation, self.clone_current_token());
            let return_type_idx = self.ast.add_node(return_type);
            self.ast.add_child(func_decl_idx, return_type_idx);
            self.advance();
        }

        // Parse block
        self.skip_newlines();
        let block_idx = self.parse_block(depth + 1)?;
        self.ast.add_child(func_decl_idx, block_idx);

        // Add function decl as child of method node
        self.ast.add_child(method_idx, func_decl_idx);

        Ok(method_idx)
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex;
    use crate::parser::parse;

    fn to_ast_string(source: &str) -> Result<String, super::ParseError> {
        let limits = crate::limits::CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits)?;
        Ok(ast.to_string())
    }

    #[test]
    fn test_empty_struct_literal() {
        let ast = to_ast_string("user: {}\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'user'
    StructInit
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_init_simple_field() {
        let ast = to_ast_string("user: { username: \"Paul\" }\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'user'
    StructInit
      StructInitField
        Identifier 'username'
        LiteralString 'Paul'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_init_multiple_fields() {
        let ast = to_ast_string("user: { name: \"Paul\", age: 30 }\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'user'
    StructInit
      StructInitField
        Identifier 'name'
        LiteralString 'Paul'
      StructInitField
        Identifier 'age'
        LiteralNumber '30'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_init_with_method() {
        let ast = to_ast_string("user: { greet: () { return \"hello\" } }\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'user'
    StructInit
      StructInitMethod
        Identifier 'greet'
        FunctionDecl
          Identifier 'greet'
          ParamList
          Block
            ReturnStmt
              LiteralString 'hello'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_init_private_field() {
        let ast = to_ast_string("user: { _ secret: \"password\" }\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'user'
    StructInit
      StructInitField [private]
        Identifier 'secret' [private]
        LiteralString 'password'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_init_private_method() {
        let ast = to_ast_string("user: { _ internal: () { return true } }\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'user'
    StructInit
      StructInitMethod [private]
        Identifier 'internal' [private]
        FunctionDecl
          Identifier 'internal'
          ParamList
          Block
            ReturnStmt
              LiteralBoolean 'true'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_init_mixed_members() {
        let ast = to_ast_string(
            "user: { name: \"Paul\", _ secret: \"pw\", greet: () { return \"hi\" } }\n",
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'user'
    StructInit
      StructInitField
        Identifier 'name'
        LiteralString 'Paul'
      StructInitField [private]
        Identifier 'secret' [private]
        LiteralString 'pw'
      StructInitMethod
        Identifier 'greet'
        FunctionDecl
          Identifier 'greet'
          ParamList
          Block
            ReturnStmt
              LiteralString 'hi'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_error_unclosed_struct_literal() {
        let result = to_ast_string("user: { name: \"Paul\"\n");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("'}' to close struct literal"));
    }
}
