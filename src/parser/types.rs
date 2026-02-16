use super::{ParseError, Parser};
use crate::ast::{AstNode, NodeType};
use crate::lexer::TokenKind;

// Type declaration parsing
impl<'a> Parser<'a> {
    /// Parse a type declaration: type Name [<Params>] [: Body]
    /// Handles all type forms: unit, alias, union, struct, intersection, function, generic
    pub(super) fn parse_type_decl(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume 'type' keyword
        self.consume(TokenKind::Type, "'type'")?;

        // Parse type name
        let name_token = self.clone_current_token();
        let type_name_node = AstNode::new_terminal(NodeType::TypeName, name_token);
        let type_name_idx = self.ast.add_node(type_name_node);
        self.consume(TokenKind::Identifier, "type name")?;

        // Create TypeDecl node
        let type_decl_node = AstNode::new(NodeType::TypeDecl);
        let type_decl_idx = self.ast.add_node(type_decl_node);

        // Add type name as first child
        self.ast.add_child(type_decl_idx, type_name_idx);

        // Check for generic parameters
        if self.current_token().kind == TokenKind::Lt {
            let params_idx = self.parse_type_params(depth + 1)?;
            self.ast.add_child(type_decl_idx, params_idx);
        }

        // Parse type body (unit or other forms)
        // NOTE: Don't skip newlines here - parse_type_body needs to see them for unit types
        let type_body_idx = self.parse_type_body(depth + 1)?;
        self.ast.add_child(type_decl_idx, type_body_idx);

        Ok(type_decl_idx)
    }

    /// Parse type body - determines which type form to parse
    /// Returns TypeBody node containing the appropriate child nodes
    fn parse_type_body(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Create TypeBody container
        let type_body_node = AstNode::new(NodeType::TypeBody);
        let type_body_idx = self.ast.add_node(type_body_node);

        match self.current_token().kind {
            TokenKind::Newline | TokenKind::Eof => {
                // Unit type - no body
                Ok(type_body_idx)
            }
            TokenKind::Colon => {
                // Type has a body - consume colon and determine what kind
                self.advance(); // Consume ':'
                self.skip_newlines();

                match self.peek_kind() {
                    TokenKind::Identifier => {
                        // Parse type expression (alias, union, or intersection)
                        let type_expr_idx = self.parse_type_expression(depth + 1)?;
                        self.ast.add_child(type_body_idx, type_expr_idx);
                        Ok(type_body_idx)
                    }
                    TokenKind::LBrace => {
                        // Struct type
                        let struct_idx = self.parse_struct_body(depth + 1)?;
                        self.ast.add_child(type_body_idx, struct_idx);
                        Ok(type_body_idx)
                    }
                    TokenKind::LParen => {
                        // Function type
                        let func_type_idx = self.parse_function_type(depth + 1)?;
                        self.ast.add_child(type_body_idx, func_type_idx);
                        Ok(type_body_idx)
                    }
                    _ => Err(self.new_unexpected_token("type body (identifier, '{', or '(')")),
                }
            }
            _ => Err(self.new_unexpected_token("':' or newline after type name")),
        }
    }

    /// Parse generic type parameters: <T, K, V> or <T: Constraint>
    /// Returns TypeParams node containing TypeParam children
    pub(super) fn parse_type_params(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume '<'
        self.consume(TokenKind::Lt, "'<'")?;

        // Create TypeParams node
        let params_node = AstNode::new(NodeType::TypeParams);
        let params_idx = self.ast.add_node(params_node);

        // Parse parameter list
        loop {
            self.skip_newlines();

            // Parse parameter name
            let param_name_token = self.clone_current_token();
            self.consume(TokenKind::Identifier, "type parameter name")?;

            // Create TypeParam node
            let type_param_node = AstNode::new(NodeType::TypeParam);
            let type_param_idx = self.ast.add_node(type_param_node);

            // Add parameter name as identifier child
            let name_node = AstNode::new_terminal(NodeType::Identifier, param_name_token);
            let name_idx = self.ast.add_node(name_node);
            self.ast.add_child(type_param_idx, name_idx);

            // Check for constraint (: Constraint)
            if self.current_token().kind == TokenKind::Colon {
                self.advance(); // Consume ':'
                self.skip_newlines();

                // Parse constraint type
                if self.peek_kind() != TokenKind::Identifier {
                    return Err(self.new_unexpected_token("type constraint name"));
                }

                let constraint_node =
                    AstNode::new_terminal(NodeType::TypeConstraint, self.clone_current_token());
                let constraint_idx = self.ast.add_node(constraint_node);
                self.advance(); // Consume constraint
                self.ast.add_child(type_param_idx, constraint_idx);
            }

            // Add parameter to params list
            self.ast.add_child(params_idx, type_param_idx);

            // Check for comma or closing angle bracket
            self.skip_newlines();
            match self.current_token().kind {
                TokenKind::Comma => {
                    self.advance(); // Consume comma
                }
                TokenKind::Gt => {
                    self.advance(); // Consume '>'
                    break;
                }
                _ => {
                    return Err(self.new_unexpected_token("',' or '>'"));
                }
            }
        }

        Ok(params_idx)
    }

    /// Parse struct body: { fields and methods }
    /// Fields: name Type
    /// Methods: name: FunctionType
    fn parse_struct_body(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume '{'
        self.consume(TokenKind::LBrace, "'{'")?;

        // Create StructBody node
        let struct_body_node = AstNode::new(NodeType::StructBody);
        let struct_body_idx = self.ast.add_node(struct_body_node);

        // Parse members (fields and methods)
        loop {
            self.skip_newlines();

            // Check for closing brace
            if self.current_token().kind == TokenKind::RBrace {
                self.advance(); // Consume '}'
                break;
            }

            // Check for EOF (error case - unclosed struct)
            if self.current_token().kind == TokenKind::Eof {
                return Err(self.new_unexpected_token("'}'"));
            }

            // Parse member name
            let member_name_token = self.clone_current_token();
            self.consume(TokenKind::Identifier, "field or method name")?;

            // Lookahead: field or method?
            match self.current_token().kind {
                TokenKind::Identifier => {
                    // Field: name Type
                    let field_node = AstNode::new(NodeType::StructField);
                    let field_idx = self.ast.add_node(field_node);

                    // Add field name as identifier child
                    let name_node = AstNode::new_terminal(NodeType::Identifier, member_name_token);
                    let name_idx = self.ast.add_node(name_node);
                    self.ast.add_child(field_idx, name_idx);

                    // Add type annotation child
                    let type_node =
                        AstNode::new_terminal(NodeType::TypeAnnotation, self.clone_current_token());
                    let type_idx = self.ast.add_node(type_node);
                    self.advance(); // Consume type
                    self.ast.add_child(field_idx, type_idx);

                    // Add field to struct body
                    self.ast.add_child(struct_body_idx, field_idx);
                }
                TokenKind::Colon => {
                    // Method: name: FunctionType
                    self.advance(); // Consume ':'

                    // Parse function type
                    if self.peek_kind() != TokenKind::LParen {
                        return Err(self.new_unexpected_token("'(' for method function type"));
                    }

                    let func_type_idx = self.parse_function_type(depth + 1)?;

                    // Create StructMethod node
                    let method_node = AstNode::new(NodeType::StructMethod);
                    let method_idx = self.ast.add_node(method_node);

                    // Add method name as identifier child
                    let name_node = AstNode::new_terminal(NodeType::Identifier, member_name_token);
                    let name_idx = self.ast.add_node(name_node);
                    self.ast.add_child(method_idx, name_idx);

                    // Add function type as second child
                    self.ast.add_child(method_idx, func_type_idx);

                    // Add method to struct body
                    self.ast.add_child(struct_body_idx, method_idx);
                }
                _ => {
                    return Err(
                        self.new_unexpected_token("type annotation or ':' after field/method name")
                    );
                }
            }

            // Skip optional comma between members
            self.skip_newlines();
            if self.current_token().kind == TokenKind::Comma {
                self.advance();
            }
        }

        Ok(struct_body_idx)
    }

    /// Parse function type signature: (a Type, b Type) ReturnType
    /// Used for type declarations like: type AddFn: (x Number, y Number) Number
    fn parse_function_type(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume '('
        self.consume(TokenKind::LParen, "'('")?;

        // Create FunctionType node
        let func_type_node = AstNode::new(NodeType::FunctionType);
        let func_type_idx = self.ast.add_node(func_type_node);

        // Create FunctionTypeParams node
        let params_node = AstNode::new(NodeType::FunctionTypeParams);
        let params_idx = self.ast.add_node(params_node);
        self.ast.add_child(func_type_idx, params_idx);

        // Parse parameters (all must have types)
        loop {
            self.skip_newlines();

            if self.current_token().kind == TokenKind::RParen {
                self.advance(); // Consume ')'
                break;
            }

            // Parse: name Type
            let param_name_token = self.clone_current_token();
            self.consume(TokenKind::Identifier, "parameter name")?;

            // Function type parameters must have types
            if self.current_token().kind != TokenKind::Identifier {
                return Err(
                    self.new_unexpected_token("type annotation for function type parameter")
                );
            }

            let param_type_token = self.clone_current_token();
            self.advance(); // Consume type

            // Create StructField node for parameter (reuse pattern)
            let field_node = AstNode::new(NodeType::StructField);
            let field_idx = self.ast.add_node(field_node);

            // Add parameter name as identifier child
            let name_node = AstNode::new_terminal(NodeType::Identifier, param_name_token);
            let name_idx = self.ast.add_node(name_node);
            self.ast.add_child(field_idx, name_idx);

            // Add type annotation child
            let type_node = AstNode::new_terminal(NodeType::TypeAnnotation, param_type_token);
            let type_idx = self.ast.add_node(type_node);
            self.ast.add_child(field_idx, type_idx);

            // Add parameter to params list
            self.ast.add_child(params_idx, field_idx);

            // Check for comma or closing paren
            self.skip_newlines();
            match self.current_token().kind {
                TokenKind::Comma => {
                    self.advance(); // Consume comma
                }
                TokenKind::RParen => {
                    self.advance(); // Consume ')'
                    break;
                }
                _ => {
                    return Err(self.new_unexpected_token("',' or ')'"));
                }
            }
        }

        // Parse return type
        if self.current_token().kind != TokenKind::Identifier {
            return Err(self.new_unexpected_token("type idetifier or 'void'"));
        }
        let return_type_node =
            AstNode::new_terminal(NodeType::TypeAnnotation, self.clone_current_token());
        let return_type_idx = self.ast.add_node(return_type_node);
        self.advance(); // Consume return type
        self.ast.add_child(func_type_idx, return_type_idx);

        Ok(func_type_idx)
    }

    /// Parse type primary - a single type component
    /// Can be TypeAnnotation (identifier) or inline StructBody
    fn parse_type_primary(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        match self.current_token().kind {
            TokenKind::Identifier => {
                // Type reference
                let type_node =
                    AstNode::new_terminal(NodeType::TypeAnnotation, self.clone_current_token());
                let type_idx = self.ast.add_node(type_node);
                self.advance(); // Consume identifier
                Ok(type_idx)
            }
            TokenKind::LBrace => {
                // Inline struct body (for intersections like: Person + { salary Int64 })
                self.parse_struct_body(depth + 1)
            }
            _ => Err(self.new_unexpected_token("type name or '{'")),
        }
    }

    /// Parse type expression - handles aliases, unions, and intersections
    /// Returns TypeAnnotation for simple alias, UnionTypeList for unions, IntersectionType for intersections
    fn parse_type_expression(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Parse first type component
        let mut result_idx = self.parse_type_primary(depth + 1)?;

        // Check for comma (union type) - binds tighter than plus
        if self.current_token().kind == TokenKind::Comma {
            // Create UnionTypeList and add first type as child
            let union_list_node = AstNode::new(NodeType::UnionTypeList);
            let union_list_idx = self.ast.add_node(union_list_node);
            self.ast.add_child(union_list_idx, result_idx);

            // Parse remaining types in union
            while self.current_token().kind == TokenKind::Comma {
                self.advance(); // Consume comma
                self.skip_newlines(); // Allow newlines after comma

                // Parse next type
                let type_idx = self.parse_type_primary(depth + 1)?;
                self.ast.add_child(union_list_idx, type_idx);
            }

            result_idx = union_list_idx;
        }

        // Check for plus (intersection type)
        // Support chaining: A + B + C (left-associative)
        while self.current_token().kind == TokenKind::Plus {
            self.advance(); // Consume '+'
            self.skip_newlines(); // Allow newlines after plus

            // Parse right side
            let right_idx = self.parse_type_primary(depth + 1)?;

            // Create IntersectionType node
            let intersection_node = AstNode::new(NodeType::IntersectionType);
            let intersection_idx = self.ast.add_node(intersection_node);

            // Add left and right as children
            self.ast.add_child(intersection_idx, result_idx);
            self.ast.add_child(intersection_idx, right_idx);

            // The intersection becomes the new left side (for chaining)
            result_idx = intersection_idx;
        }

        Ok(result_idx)
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

    // Unit type tests
    #[test]
    fn test_unit_type() {
        let ast = to_ast_string("type Success\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Success'
    TypeBody
";
        assert_eq!(ast, expected);
    }

    // Type alias tests
    #[test]
    fn test_type_alias() {
        let ast = to_ast_string("type UserId: Number\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'UserId'
    TypeBody
      TypeAnnotation 'Number'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_type_alias_string() {
        let ast = to_ast_string("type Username: String\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Username'
    TypeBody
      TypeAnnotation 'String'
";
        assert_eq!(ast, expected);
    }

    // Union type tests
    #[test]
    fn test_union_type_two() {
        let ast = to_ast_string("type Status: Success, Error\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Status'
    TypeBody
      UnionTypeList
        TypeAnnotation 'Success'
        TypeAnnotation 'Error'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_union_type_three() {
        let ast = to_ast_string("type Value: Int64, String, Bool\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Value'
    TypeBody
      UnionTypeList
        TypeAnnotation 'Int64'
        TypeAnnotation 'String'
        TypeAnnotation 'Bool'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_union_type_tree_structure() {
        let ast = to_ast("type Status: Success, Error\n").unwrap();

        let program_idx = ast.root.unwrap();
        let type_decl_idx = ast.nodes[program_idx].first_child.unwrap();
        let type_name_idx = ast.nodes[type_decl_idx].first_child.unwrap();
        let type_body_idx = ast.nodes[type_name_idx].next_sibling.unwrap();
        let union_list_idx = ast.nodes[type_body_idx].first_child.unwrap();

        assert_eq!(ast.nodes[union_list_idx].node_type, NodeType::UnionTypeList);

        // Check first type
        let first_type_idx = ast.nodes[union_list_idx].first_child.unwrap();
        assert_eq!(
            ast.nodes[first_type_idx].node_type,
            NodeType::TypeAnnotation
        );

        // Check second type
        let second_type_idx = ast.nodes[first_type_idx].next_sibling.unwrap();
        assert_eq!(
            ast.nodes[second_type_idx].node_type,
            NodeType::TypeAnnotation
        );
    }

    // Struct type tests
    #[test]
    fn test_struct_fields_only() {
        let ast = to_ast_string("type Person: {\n    name String\n    age Number\n}\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Person'
    TypeBody
      StructBody
        StructField
          Identifier 'name'
          TypeAnnotation 'String'
        StructField
          Identifier 'age'
          TypeAnnotation 'Number'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_methods_only() {
        let ast =
            to_ast_string("type Calculator: {\n    add: (x Number, y Number) Number\n}\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Calculator'
    TypeBody
      StructBody
        StructMethod
          Identifier 'add'
          FunctionType
            FunctionTypeParams
              StructField
                Identifier 'x'
                TypeAnnotation 'Number'
              StructField
                Identifier 'y'
                TypeAnnotation 'Number'
            TypeAnnotation 'Number'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_empty() {
        let ast = to_ast_string("type Empty: {}\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Empty'
    TypeBody
      StructBody
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_fields_comma_separated() {
        let ast = to_ast_string("type Point: {x Number, y Number}\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Point'
    TypeBody
      StructBody
        StructField
          Identifier 'x'
          TypeAnnotation 'Number'
        StructField
          Identifier 'y'
          TypeAnnotation 'Number'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_fields_mixed_separators() {
        let ast =
            to_ast_string("type Person: {\n    name String,\n    age Number\n}\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Person'
    TypeBody
      StructBody
        StructField
          Identifier 'name'
          TypeAnnotation 'String'
        StructField
          Identifier 'age'
          TypeAnnotation 'Number'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_struct_mixed_members_comma_separated() {
        let ast = to_ast_string(
            "type Calculator: {value Number, add: (x Number, y Number) Number}\n",
        )
        .unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Calculator'
    TypeBody
      StructBody
        StructField
          Identifier 'value'
          TypeAnnotation 'Number'
        StructMethod
          Identifier 'add'
          FunctionType
            FunctionTypeParams
              StructField
                Identifier 'x'
                TypeAnnotation 'Number'
              StructField
                Identifier 'y'
                TypeAnnotation 'Number'
            TypeAnnotation 'Number'
";
        assert_eq!(ast, expected);
    }

    // Intersection type tests
    #[test]
    fn test_intersection_two_types() {
        let ast = to_ast_string("type Admin: Person + Manager\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Admin'
    TypeBody
      IntersectionType
        TypeAnnotation 'Person'
        TypeAnnotation 'Manager'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_intersection_chained() {
        let ast = to_ast_string("type SuperAdmin: Person + Manager + Executive\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'SuperAdmin'
    TypeBody
      IntersectionType
        IntersectionType
          TypeAnnotation 'Person'
          TypeAnnotation 'Manager'
        TypeAnnotation 'Executive'
";
        assert_eq!(ast, expected);
    }

    // Generic type tests
    #[test]
    fn test_generic_single_param() {
        let ast = to_ast_string("type List<T>: { items Array }\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'List'
    TypeParams
      TypeParam
        Identifier 'T'
    TypeBody
      StructBody
        StructField
          Identifier 'items'
          TypeAnnotation 'Array'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_generic_multiple_params() {
        let ast = to_ast_string("type Map<K, V>: { size Number }\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Map'
    TypeParams
      TypeParam
        Identifier 'K'
      TypeParam
        Identifier 'V'
    TypeBody
      StructBody
        StructField
          Identifier 'size'
          TypeAnnotation 'Number'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_generic_with_constraint() {
        let ast = to_ast_string("type Container<T: Sizeable>: { value T }\n").unwrap();

        let expected = "\
Program
  TypeDecl
    TypeName 'Container'
    TypeParams
      TypeParam
        Identifier 'T'
        TypeConstraint 'Sizeable'
    TypeBody
      StructBody
        StructField
          Identifier 'value'
          TypeAnnotation 'T'
";
        assert_eq!(ast, expected);
    }
}
