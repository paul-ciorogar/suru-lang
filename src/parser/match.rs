use super::{ParseError, Parser};
use crate::ast::{AstNode, NodeType};
use crate::lexer::TokenKind;

// Match expression parsing methods
impl<'a> Parser<'a> {
    /// Parse match expression: match subject { pattern: result, ... }
    /// Returns the Match node index
    pub(super) fn parse_match_expression(
        &mut self,
        depth: usize,
    ) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Create Match node
        let match_node = AstNode::new(NodeType::Match);
        let match_idx = self.ast.add_node(match_node);

        // Parse subject expression
        let subject_idx = self.parse_match_subject(depth + 1)?;
        self.ast.add_child(match_idx, subject_idx);

        // Skip newlines before '{'
        self.skip_newlines();

        // Consume '{'
        self.consume(TokenKind::LBrace, "'{' after match subject expression")?;

        // Parse match arms
        let arms_idx = self.parse_match_arms(depth + 1)?;
        self.ast.add_child(match_idx, arms_idx);

        Ok(match_idx)
    }

    /// Parse match subject expression wrapped in MatchSubject node
    fn parse_match_subject(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Create MatchSubject wrapper node
        let subject_node = AstNode::new(NodeType::MatchSubject);
        let subject_idx = self.ast.add_node(subject_node);

        // Parse the expression being matched
        let expr_idx = self.parse_expression(depth + 1, 0)?;
        self.ast.add_child(subject_idx, expr_idx);

        Ok(subject_idx)
    }

    /// Parse all match arms until '}'
    fn parse_match_arms(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Create MatchArms container
        let arms_node = AstNode::new(NodeType::MatchArms);
        let arms_idx = self.ast.add_node(arms_node);

        let mut arm_count = 0;

        // Parse arms until '}'
        loop {
            self.skip_newlines();

            // Check for closing brace
            if self.current_token().kind == TokenKind::RBrace {
                // Check if we have at least one arm
                if arm_count == 0 {
                    return Err(self.new_unexpected_token(
                        "at least one match arm (pattern : result)",
                    ));
                }
                self.advance(); // Consume '}'
                break;
            }

            // Check for EOF (error - unclosed match)
            if self.peek_kind_is(TokenKind::Eof) {
                return Err(self.new_unexpected_token("'}' to close match expression"));
            }

            // Parse single arm
            let arm_idx = self.parse_match_arm(depth + 1)?;
            self.ast.add_child(arms_idx, arm_idx);
            arm_count += 1;
        }

        Ok(arms_idx)
    }

    /// Parse a single match arm: pattern : result
    fn parse_match_arm(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Create MatchArm node
        let arm_node = AstNode::new(NodeType::MatchArm);
        let arm_idx = self.ast.add_node(arm_node);

        // 1. Parse pattern (wrapped in MatchPattern)
        let pattern_idx = self.parse_match_pattern(depth + 1)?;
        self.ast.add_child(arm_idx, pattern_idx);

        // 2. Consume ':'
        self.consume(TokenKind::Colon, "':' after match pattern")?;

        // 3. Skip newlines before result expression
        self.skip_newlines();

        // 4. Parse result expression
        let result_idx = self.parse_expression(depth + 1, 0)?;
        self.ast.add_child(arm_idx, result_idx);

        Ok(arm_idx)
    }

    /// Parse a match pattern
    /// Supports: identifiers, literals (number/string/boolean), wildcards (_)
    fn parse_match_pattern(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Create MatchPattern wrapper
        let pattern_node = AstNode::new(NodeType::MatchPattern);
        let pattern_idx = self.ast.add_node(pattern_node);

        let token = self.current_token();
        let pattern_expr_idx = match &token.kind {
            // Wildcard pattern: _
            TokenKind::Underscore => {
                let placeholder_node =
                    AstNode::new_terminal(NodeType::Placeholder, self.clone_current_token());
                let placeholder_idx = self.ast.add_node(placeholder_node);
                self.advance();
                placeholder_idx
            }

            // Literal number pattern
            TokenKind::Number(_) => {
                let literal_node =
                    AstNode::new_terminal(NodeType::LiteralNumber, self.clone_current_token());
                let literal_idx = self.ast.add_node(literal_node);
                self.advance();
                literal_idx
            }

            // Literal string pattern
            TokenKind::String(_) => {
                let literal_node =
                    AstNode::new_terminal(NodeType::LiteralString, self.clone_current_token());
                let literal_idx = self.ast.add_node(literal_node);
                self.advance();
                literal_idx
            }

            // Literal boolean pattern
            TokenKind::True | TokenKind::False => {
                let literal_node =
                    AstNode::new_terminal(NodeType::LiteralBoolean, self.clone_current_token());
                let literal_idx = self.ast.add_node(literal_node);
                self.advance();
                literal_idx
            }

            // Identifier pattern (type or value name)
            TokenKind::Identifier => {
                let ident_node =
                    AstNode::new_terminal(NodeType::Identifier, self.clone_current_token());
                let ident_idx = self.ast.add_node(ident_node);
                self.advance();
                ident_idx
            }

            _ => {
                return Err(self.new_unexpected_token(
                    "match pattern (identifier, literal, or '_')",
                ));
            }
        };

        // Add pattern expression as child of MatchPattern wrapper
        self.ast.add_child(pattern_idx, pattern_expr_idx);

        Ok(pattern_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    fn to_ast(source: &str) -> Result<crate::ast::Ast, ParseError> {
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

    // ========== BASIC MATCH TESTS ==========

    #[test]
    fn test_match_simple_types() {
        let ast = to_ast_string(
            r#"
            x: match status {
                Success: "ok"
                Error: "fail"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'status'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Success'
          LiteralString 'ok'
        MatchArm
          MatchPattern
            Identifier 'Error'
          LiteralString 'fail'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_numbers() {
        let ast = to_ast_string(
            r#"
            x: match n {
                0: "zero"
                1: "one"
                2: "two"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'n'
      MatchArms
        MatchArm
          MatchPattern
            LiteralNumber '0'
          LiteralString 'zero'
        MatchArm
          MatchPattern
            LiteralNumber '1'
          LiteralString 'one'
        MatchArm
          MatchPattern
            LiteralNumber '2'
          LiteralString 'two'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_strings() {
        let ast = to_ast_string(
            r#"
            x: match key {
                "admin": "admin user"
                "guest": "guest user"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'key'
      MatchArms
        MatchArm
          MatchPattern
            LiteralString 'admin'
          LiteralString 'admin user'
        MatchArm
          MatchPattern
            LiteralString 'guest'
          LiteralString 'guest user'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_booleans() {
        let ast = to_ast_string(
            r#"
            x: match flag {
                true: "yes"
                false: "no"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'flag'
      MatchArms
        MatchArm
          MatchPattern
            LiteralBoolean 'true'
          LiteralString 'yes'
        MatchArm
          MatchPattern
            LiteralBoolean 'false'
          LiteralString 'no'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_wildcard() {
        let ast = to_ast_string(
            r#"
            x: match value {
                0: "zero"
                _: "other"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'value'
      MatchArms
        MatchArm
          MatchPattern
            LiteralNumber '0'
          LiteralString 'zero'
        MatchArm
          MatchPattern
            Placeholder '_'
          LiteralString 'other'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_empty_arms() {
        // Match with no arms should error
        let result = to_ast(
            r#"
            x: match value {
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_match_no_closing_brace() {
        // Unclosed match should error
        let result = to_ast(
            r#"
            x: match value {
                0: "zero"
        "#,
        );
        assert!(result.is_err());
    }

    // ========== COMPLEX SUBJECTS ==========

    #[test]
    fn test_match_function_call_subject() {
        let ast = to_ast_string(
            r#"
            x: match getStatus() {
                Success: "ok"
                _: "fail"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        FunctionCall
          Identifier 'getStatus'
          ArgList
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Success'
          LiteralString 'ok'
        MatchArm
          MatchPattern
            Placeholder '_'
          LiteralString 'fail'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_method_call_subject() {
        let ast = to_ast_string(
            r#"
            x: match user.getStatus() {
                Active: "active"
                _: "inactive"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        MethodCall
          Identifier 'user'
          Identifier 'getStatus'
          ArgList
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Active'
          LiteralString 'active'
        MatchArm
          MatchPattern
            Placeholder '_'
          LiteralString 'inactive'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_property_access_subject() {
        let ast = to_ast_string(
            r#"
            x: match user.status {
                Active: "active"
                _: "inactive"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        PropertyAccess
          Identifier 'user'
          Identifier 'status'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Active'
          LiteralString 'active'
        MatchArm
          MatchPattern
            Placeholder '_'
          LiteralString 'inactive'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_pipe_subject() {
        let ast = to_ast_string(
            r#"
            x: match data | transform {
                Success: "ok"
                _: "fail"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Pipe
          Identifier 'data'
          Identifier 'transform'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Success'
          LiteralString 'ok'
        MatchArm
          MatchPattern
            Placeholder '_'
          LiteralString 'fail'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_complex_subject() {
        let ast = to_ast_string(
            r#"
            x: match data | transform | validate {
                Valid: "ok"
                Invalid: "fail"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Pipe
          Pipe
            Identifier 'data'
            Identifier 'transform'
          Identifier 'validate'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Valid'
          LiteralString 'ok'
        MatchArm
          MatchPattern
            Identifier 'Invalid'
          LiteralString 'fail'
";
        assert_eq!(ast, expected);
    }

    // ========== RESULT EXPRESSIONS ==========

    #[test]
    fn test_match_function_call_result() {
        let ast = to_ast_string(
            r#"
            x: match status {
                Error: logError()
                _: doNothing()
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'status'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Error'
          FunctionCall
            Identifier 'logError'
            ArgList
        MatchArm
          MatchPattern
            Placeholder '_'
          FunctionCall
            Identifier 'doNothing'
            ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_method_call_result() {
        let ast = to_ast_string(
            r#"
            x: match status {
                Error: logger.error()
                _: logger.info()
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'status'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Error'
          MethodCall
            Identifier 'logger'
            Identifier 'error'
            ArgList
        MatchArm
          MatchPattern
            Placeholder '_'
          MethodCall
            Identifier 'logger'
            Identifier 'info'
            ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_boolean_expression_result() {
        let ast = to_ast_string(
            r#"
            x: match value {
                0: true and false
                _: true or false
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'value'
      MatchArms
        MatchArm
          MatchPattern
            LiteralNumber '0'
          And
            LiteralBoolean 'true'
            LiteralBoolean 'false'
        MatchArm
          MatchPattern
            Placeholder '_'
          Or
            LiteralBoolean 'true'
            LiteralBoolean 'false'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_pipe_result() {
        let ast = to_ast_string(
            r#"
            x: match status {
                Error: msg | logError
                _: msg | logInfo
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'status'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Error'
          Pipe
            Identifier 'msg'
            Identifier 'logError'
        MatchArm
          MatchPattern
            Placeholder '_'
          Pipe
            Identifier 'msg'
            Identifier 'logInfo'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_multiline_result() {
        let ast = to_ast_string(
            r#"
            x: match status {
                Success: 42
                Error: 0
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'status'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Success'
          LiteralNumber '42'
        MatchArm
          MatchPattern
            Identifier 'Error'
          LiteralNumber '0'
";
        assert_eq!(ast, expected);
    }

    // ========== NESTED MATCH ==========

    #[test]
    fn test_match_nested_simple() {
        let ast = to_ast_string(
            r#"
            x: match outer {
                Ok: match inner {
                    TypeA: "A"
                    _: "other"
                }
                Error: "error"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'outer'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Ok'
          Match
            MatchSubject
              Identifier 'inner'
            MatchArms
              MatchArm
                MatchPattern
                  Identifier 'TypeA'
                LiteralString 'A'
              MatchArm
                MatchPattern
                  Placeholder '_'
                LiteralString 'other'
        MatchArm
          MatchPattern
            Identifier 'Error'
          LiteralString 'error'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_nested_deep() {
        let ast = to_ast_string(
            r#"
            x: match a {
                1: match b {
                    2: match c {
                        3: "deep"
                        _: "c-other"
                    }
                    _: "b-other"
                }
                _: "a-other"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'a'
      MatchArms
        MatchArm
          MatchPattern
            LiteralNumber '1'
          Match
            MatchSubject
              Identifier 'b'
            MatchArms
              MatchArm
                MatchPattern
                  LiteralNumber '2'
                Match
                  MatchSubject
                    Identifier 'c'
                  MatchArms
                    MatchArm
                      MatchPattern
                        LiteralNumber '3'
                      LiteralString 'deep'
                    MatchArm
                      MatchPattern
                        Placeholder '_'
                      LiteralString 'c-other'
              MatchArm
                MatchPattern
                  Placeholder '_'
                LiteralString 'b-other'
        MatchArm
          MatchPattern
            Placeholder '_'
          LiteralString 'a-other'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_nested_in_multiple_arms() {
        let ast = to_ast_string(
            r#"
            x: match status {
                Ok: match detail {
                    High: "ok-high"
                    _: "ok-other"
                }
                Error: match detail {
                    Critical: "err-critical"
                    _: "err-other"
                }
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'status'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Ok'
          Match
            MatchSubject
              Identifier 'detail'
            MatchArms
              MatchArm
                MatchPattern
                  Identifier 'High'
                LiteralString 'ok-high'
              MatchArm
                MatchPattern
                  Placeholder '_'
                LiteralString 'ok-other'
        MatchArm
          MatchPattern
            Identifier 'Error'
          Match
            MatchSubject
              Identifier 'detail'
            MatchArms
              MatchArm
                MatchPattern
                  Identifier 'Critical'
                LiteralString 'err-critical'
              MatchArm
                MatchPattern
                  Placeholder '_'
                LiteralString 'err-other'
";
        assert_eq!(ast, expected);
    }

    // ========== MATCH AS EXPRESSION ==========

    #[test]
    fn test_match_in_variable_declaration() {
        let ast = to_ast_string(
            r#"
            status: match result {
                Success: "ok"
                _: "fail"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'status'
    Match
      MatchSubject
        Identifier 'result'
      MatchArms
        MatchArm
          MatchPattern
            Identifier 'Success'
          LiteralString 'ok'
        MatchArm
          MatchPattern
            Placeholder '_'
          LiteralString 'fail'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_in_return_statement() {
        let ast = to_ast_string(
            r#"
            f: () {
                return match x {
                    0: "zero"
                    _: "other"
                }
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  FunctionDecl
    Identifier 'f'
    ParamList
    Block
      ReturnStmt
        Match
          MatchSubject
            Identifier 'x'
          MatchArms
            MatchArm
              MatchPattern
                LiteralNumber '0'
              LiteralString 'zero'
            MatchArm
              MatchPattern
                Placeholder '_'
              LiteralString 'other'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_match_in_pipe_error() {
        // Match in pipe without subject should error
        let result = to_ast(
            r#"
            x: getData() | match {
                Success: processSuccess
                _: handleError
            } | finalize
        "#,
        );
        assert!(result.is_err());
    }

    // ========== ERRORS & EDGE CASES ==========

    #[test]
    fn test_error_match_no_subject() {
        // Match with no subject should error
        let result = to_ast(
            r#"
            x: match {
                Success: "ok"
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_error_match_no_lbrace() {
        // Match without opening brace should error
        let result = to_ast(
            r#"
            x: match status
                Success: "ok"
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_error_match_no_colon_in_arm() {
        // Match arm without colon should error
        let result = to_ast(
            r#"
            x: match status {
                Success "ok"
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_error_match_no_result_expression() {
        // Match arm without result expression should error
        let result = to_ast(
            r#"
            x: match status {
                Success:
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_error_match_invalid_pattern() {
        // Invalid pattern token should error
        let result = to_ast(
            r#"
            x: match status {
                +: "ok"
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_match_multiple_wildcards() {
        // Multiple wildcards are syntactically valid (semantics will validate)
        let ast = to_ast_string(
            r#"
            x: match value {
                _: "first"
                _: "second"
            }
        "#,
        )
        .unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    Match
      MatchSubject
        Identifier 'value'
      MatchArms
        MatchArm
          MatchPattern
            Placeholder '_'
          LiteralString 'first'
        MatchArm
          MatchPattern
            Placeholder '_'
          LiteralString 'second'
";
        assert_eq!(ast, expected);
    }
}
