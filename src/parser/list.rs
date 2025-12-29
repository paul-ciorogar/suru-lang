use super::{ParseError, Parser};
use crate::ast::{AstNode, NodeType};
use crate::lexer::TokenKind;

impl<'a> Parser<'a> {
    /// Parse a list literal: [elem1, elem2, ...]
    /// Consumes '[' and ']', parses comma-separated elements
    /// Supports empty lists, trailing commas, and newlines
    /// Returns the List node index
    pub(super) fn parse_list(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume '['
        self.consume(TokenKind::LBracket, "[")?;

        // Create List node
        let list_node = AstNode::new(NodeType::List);
        let list_idx = self.ast.add_node(list_node);

        // Parse elements (comma-separated list)
        loop {
            // Skip any newlines (they are just whitespace in lists)
            while self.current_token().kind == TokenKind::Newline {
                self.advance();
            }

            // Check for closing bracket (empty list or end of list)
            if self.current_token().kind == TokenKind::RBracket {
                self.advance(); // Consume ']'
                break;
            }

            // Parse element as an expression
            let elem_idx = self.parse_expression(depth + 1, 0)?;
            self.ast.add_child(list_idx, elem_idx);

            // Skip any newlines after element
            while self.current_token().kind == TokenKind::Newline {
                self.advance();
            }

            // Check for comma or closing bracket
            match self.peek_kind() {
                TokenKind::Comma => {
                    self.advance(); // Consume comma
                    // Check for trailing comma
                    while self.current_token().kind == TokenKind::Newline {
                        self.advance();
                    }
                    if self.current_token().kind == TokenKind::RBracket {
                        self.advance(); // Consume ']'
                        break;
                    }
                }
                TokenKind::RBracket => {
                    self.advance(); // Consume ']'
                    break;
                }
                _ => {
                    return Err(self.new_unexpected_token("',' or ']'"));
                }
            }
        }

        Ok(list_idx)
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

    // List literal tests
    #[test]
    fn test_empty_list() {
        let ast = to_ast_string("x: []\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_single_element() {
        let ast = to_ast_string("x: [1]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      LiteralNumber '1'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_multiple_numbers() {
        let ast = to_ast_string("x: [1, 2, 3]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      LiteralNumber '1'
      LiteralNumber '2'
      LiteralNumber '3'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_strings() {
        let ast = to_ast_string("x: [\"Alice\", \"Bob\", \"Charlie\"]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      LiteralString 'Alice'
      LiteralString 'Bob'
      LiteralString 'Charlie'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_mixed_types() {
        let ast = to_ast_string("x: [1, \"text\", true]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      LiteralNumber '1'
      LiteralString 'text'
      LiteralBoolean 'true'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_with_trailing_comma() {
        let ast = to_ast_string("x: [1, 2, 3,]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      LiteralNumber '1'
      LiteralNumber '2'
      LiteralNumber '3'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_with_identifiers() {
        let ast = to_ast_string("x: [a, b, c]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      Identifier 'a'
      Identifier 'b'
      Identifier 'c'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_with_expressions() {
        let ast = to_ast_string("x: [a and b, not c]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      And
        Identifier 'a'
        Identifier 'b'
      Not
        Identifier 'c'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_with_function_calls() {
        let ast = to_ast_string("x: [getValue(), process(1)]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      FunctionCall
        Identifier 'getValue'
        ArgList
      FunctionCall
        Identifier 'process'
        ArgList
          LiteralNumber '1'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_with_method_calls() {
        let ast = to_ast_string("x: [obj.getValue(), data.process()]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      MethodCall
        Identifier 'obj'
        Identifier 'getValue'
        ArgList
      MethodCall
        Identifier 'data'
        Identifier 'process'
        ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_nested_list() {
        let ast = to_ast_string("x: [[1, 2], [3, 4]]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      List
        LiteralNumber '1'
        LiteralNumber '2'
      List
        LiteralNumber '3'
        LiteralNumber '4'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_with_pipe() {
        let ast = to_ast_string("x: [a | transform, b | process]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      Pipe
        Identifier 'a'
        Identifier 'transform'
      Pipe
        Identifier 'b'
        Identifier 'process'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_with_try() {
        let ast = to_ast_string("x: [try getValue(), try process()]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      Try
        FunctionCall
          Identifier 'getValue'
          ArgList
      Try
        FunctionCall
          Identifier 'process'
          ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_with_placeholder() {
        let ast = to_ast_string("x: [add(_, 5), multiply(_, 2)]\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    List
      FunctionCall
        Identifier 'add'
        ArgList
          Placeholder '_'
          LiteralNumber '5'
      FunctionCall
        Identifier 'multiply'
        ArgList
          Placeholder '_'
          LiteralNumber '2'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_in_function_call() {
        let ast = to_ast_string("x: process([1, 2, 3])\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    FunctionCall
      Identifier 'process'
      ArgList
        List
          LiteralNumber '1'
          LiteralNumber '2'
          LiteralNumber '3'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_method_call_on_list() {
        let ast = to_ast_string("x: [1, 2, 3].length()\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    MethodCall
      List
        LiteralNumber '1'
        LiteralNumber '2'
        LiteralNumber '3'
      Identifier 'length'
      ArgList
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_list_property_access() {
        let ast = to_ast_string("x: [1, 2, 3].length\n").unwrap();
        let expected = "\
Program
  VarDecl
    Identifier 'x'
    PropertyAccess
      List
        LiteralNumber '1'
        LiteralNumber '2'
        LiteralNumber '3'
      Identifier 'length'
";
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_error_list_missing_closing_bracket() {
        let result = to_ast("[1, 2\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_list_missing_comma() {
        let result = to_ast("[1 2 3]\n");
        assert!(result.is_err());
    }
}
