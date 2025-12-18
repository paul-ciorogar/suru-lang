use crate::ast::{Ast, AstNode, NodeType};
use crate::lexer::{Token, TokenKind};

// Parse error
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub token_idx: usize,
}

impl ParseError {
    fn from_token(message: String, token: &Token, token_idx: usize) -> Self {
        Self {
            message,
            line: token.line,
            column: token.column,
            token_idx,
        }
    }

    fn unexpected_token(expected: &str, token: &Token, token_idx: usize, source: &str) -> Self {
        let found = match &token.kind {
            TokenKind::Eof => "end of file".to_string(),
            TokenKind::Newline => "newline".to_string(),
            TokenKind::Ident => format!("identifier '{}'", token.text(source)),
            _ => format!("{:?}", token.kind),
        };

        Self::from_token(
            format!("Expected {}, found {}", expected, found),
            token,
            token_idx,
        )
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Parse error at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}

// Operator precedence levels
fn get_precedence(token_kind: &TokenKind) -> Option<u8> {
    match token_kind {
        TokenKind::Or => Some(1),
        TokenKind::And => Some(2),
        TokenKind::Not => Some(3), // Unary operator
        _ => None,
    }
}

// Parser structure
pub struct Parser<'src> {
    source: &'src str,
    tokens: Vec<Token>,
    current: usize,
    ast: Ast,
    limits: crate::limits::CompilerLimits,
}

impl<'src> Parser<'src> {
    pub fn new(
        source: &'src str,
        tokens: Vec<Token>,
        limits: crate::limits::CompilerLimits,
    ) -> Self {
        let mut ast = Ast::new_with_limits(source.to_string(), limits.clone());

        // Create the Program root node
        let program_node = AstNode::new(NodeType::Program);
        let root_idx = ast.add_node(program_node);
        ast.root = Some(root_idx);

        Self {
            source,
            tokens,
            current: 0,
            ast,
            limits,
        }
    }

    // Helper: Check recursion depth limit
    fn check_depth(&self, depth: usize) -> Result<(), ParseError> {
        if depth >= self.limits.max_expr_depth {
            return Err(ParseError::from_token(
                format!(
                    "Parsing nesting too deep: {} levels (max {}). Consider simplifying.",
                    depth, self.limits.max_expr_depth
                ),
                self.current_token(),
                self.current,
            ));
        }
        Ok(())
    }

    // Main parsing entry point
    pub fn parse(mut self) -> Result<Ast, ParseError> {
        self.parse_statements(0)?;
        Ok(self.ast)
    }

    // Recursive statement parsing methods

    /// Parse all statements in the program
    fn parse_statements(&mut self, depth: usize) -> Result<(), ParseError> {
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
    fn parse_statement(&mut self, depth: usize) -> Result<Option<usize>, ParseError> {
        self.check_depth(depth)?;
        self.skip_newlines();

        let token = self.current_token();
        match &token.kind {
            TokenKind::Ident => {
                let stmt_idx = self.parse_var_decl(depth + 1)?;
                Ok(Some(stmt_idx))
            }
            TokenKind::Eof => Ok(None),
            _ => Err(ParseError::unexpected_token(
                "statement (identifier or end of file)",
                token,
                self.current,
                self.source,
            )),
        }
    }

    /// Parse a variable declaration: identifier : expression
    fn parse_var_decl(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // 1. Parse identifier token
        let ident_token = self.current_token();
        if ident_token.kind != TokenKind::Ident {
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

        let ident_node = AstNode::new_terminal(NodeType::Ident, self.current);
        let ident_idx = self.ast.add_node(ident_node);
        self.ast.add_child(var_decl_idx, ident_idx);

        self.current += 1; // consume identifier // TODO make a funcion advance()

        // 3. Expect colon
        self.consume(TokenKind::Colon, "':'")?;

        // 4. Parse expression
        let expr_idx = self.parse_expression(depth + 1, 0)?;
        self.ast.add_child(var_decl_idx, expr_idx);

        // 5. Expect newline or EOF
        let token = self.current_token();
        match &token.kind {
            TokenKind::Newline => {
                self.current += 1;
            }
            TokenKind::Eof => {
                // EOF is fine, don't consume
            }
            _ => {
                return Err(ParseError::unexpected_token(
                    "newline or end of file",
                    token,
                    self.current,
                    self.source,
                ));
            }
        }

        Ok(var_decl_idx)
    }

    /// Helper: Consume a specific token kind or error
    fn consume(&mut self, kind: TokenKind, expected: &str) -> Result<(), ParseError> {
        let token = self.current_token();
        if token.kind != kind {
            return Err(ParseError::unexpected_token(
                expected,
                token,
                self.current,
                self.source,
            ));
        }
        self.current += 1;
        Ok(())
    }

    // Recursive expression parsing methods

    /// Parse an expression recursively with precedence climbing
    /// Returns the AST node index of the parsed expression
    fn parse_expression(&mut self, depth: usize, min_precedence: u8) -> Result<usize, ParseError> {
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

        // POSTFIX PHASE: Handle function calls
        // Check if this is a function call (identifier followed by '(')
        if self.ast.nodes[left_idx].node_type == NodeType::Ident {
            if self.current_token().kind == TokenKind::LParen {
                left_idx = self.parse_function_call(depth, left_idx)?;
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
            self.current += 1;

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
                self.current += 1; // Consume 'not'

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
                self.current += 1;
                Ok(literal_node_idx)
            }

            TokenKind::Number(_) => {
                let literal_node = AstNode::new_terminal(NodeType::LiteralNumber, self.current);
                let literal_node_idx = self.ast.add_node(literal_node);
                self.current += 1;
                Ok(literal_node_idx)
            }

            TokenKind::String(_) => {
                let literal_node = AstNode::new_terminal(NodeType::LiteralString, self.current);
                let literal_node_idx = self.ast.add_node(literal_node);
                self.current += 1;
                Ok(literal_node_idx)
            }

            // Identifiers (for function calls and variable references)
            TokenKind::Ident => {
                let ident_node = AstNode::new_terminal(NodeType::Ident, self.current);
                let ident_node_idx = self.ast.add_node(ident_node);
                self.current += 1;
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

        // Consume '('
        debug_assert!(self.current_token().kind == TokenKind::LParen);
        self.current += 1;

        // Create FunctionCall node
        let call_node = AstNode::new(NodeType::FunctionCall);
        let call_node_idx = self.ast.add_node(call_node);

        // Add identifier as first child
        self.ast.add_child(call_node_idx, ident_idx);

        // Parse arguments (comma-separated list)
        loop {
            // Check for closing paren (empty args or end of list)
            if self.current_token().kind == TokenKind::RParen {
                self.current += 1; // Consume ')'
                break;
            }

            // Parse argument
            let arg_idx = self.parse_function_argument(depth + 1)?;
            self.ast.add_child(call_node_idx, arg_idx);

            // Check for comma or closing paren
            let token = self.current_token();
            match token.kind {
                TokenKind::Comma => {
                    self.current += 1; // Consume comma, continue to next argument
                }
                TokenKind::RParen => {
                    self.current += 1; // Consume ')'
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

        Ok(call_node_idx)
    }

    /// Parse a function call argument
    /// Uses parse_expression but prevents nested function calls
    fn parse_function_argument(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Parse the argument as an expression
        let arg_idx = self.parse_expression(depth + 1, 0)?;

        // Check if it's a function call (nested calls not allowed)
        if self.ast.nodes[arg_idx].node_type == NodeType::FunctionCall {
            let token = self.current_token();
            return Err(ParseError::from_token(
                "Nested function calls are not supported".to_string(),
                token,
                self.current,
            ));
        }

        Ok(arg_idx)
    }

    // Helper: Get current token (with bounds checking)
    fn current_token(&self) -> &Token {
        // If we've gone past the end, return the EOF token (always last)
        if self.current >= self.tokens.len() {
            &self.tokens[self.tokens.len() - 1]
        } else {
            &self.tokens[self.current]
        }
    }

    // Helper: Skip consecutive newlines
    fn skip_newlines(&mut self) {
        while self.current < self.tokens.len()
            && self.tokens[self.current].kind == TokenKind::Newline
        {
            self.current += 1;
        }
    }
}

// Public API
pub fn parse(source: &str, tokens: Vec<Token>, max_expr_depth: usize) -> Result<Ast, ParseError> {
    // Backward compatibility - convert to limits
    let mut limits = crate::limits::CompilerLimits::default();
    limits.max_expr_depth = max_expr_depth;
    parse_with_limits(source, tokens, limits)
}

pub fn parse_with_limits(
    source: &str,
    tokens: Vec<Token>,
    limits: crate::limits::CompilerLimits,
) -> Result<Ast, ParseError> {
    let parser = Parser::new(source, tokens, limits);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn test_single_boolean_decl() {
        let source = "flag: true\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Should have: Program, VarDecl, Ident, LiteralBoolean
        assert_eq!(ast.nodes.len(), 4);
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[2].node_type, NodeType::Ident);
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralBoolean);

        // Verify tree structure
        assert_eq!(ast.nodes[0].first_child, Some(1)); // Program -> VarDecl
        assert_eq!(ast.nodes[1].first_child, Some(2)); // VarDecl -> Ident
        assert_eq!(ast.nodes[2].next_sibling, Some(3)); // Ident -> Literal
    }

    #[test]
    fn test_multiple_decls() {
        let source = "x: 42\ny: \"hello\"\nz: false\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Should have: Program + 3 VarDecls (each with Ident + Literal) = 1 + 3*3 = 10 nodes
        assert_eq!(ast.nodes.len(), 10);
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);

        // Verify first VarDecl
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[2].node_type, NodeType::Ident);
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralNumber);

        // Verify second VarDecl
        assert_eq!(ast.nodes[4].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[5].node_type, NodeType::Ident);
        assert_eq!(ast.nodes[6].node_type, NodeType::LiteralString);

        // Verify third VarDecl
        assert_eq!(ast.nodes[7].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[8].node_type, NodeType::Ident);
        assert_eq!(ast.nodes[9].node_type, NodeType::LiteralBoolean);

        // Verify sibling links between VarDecls
        assert_eq!(ast.nodes[1].next_sibling, Some(4));
        assert_eq!(ast.nodes[4].next_sibling, Some(7));
        assert_eq!(ast.nodes[7].next_sibling, None);
    }

    #[test]
    fn test_all_number_kinds() {
        let source = "a: 42\nb: 0xFF\nc: 0b1010\nd: 3.14\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Should have 1 Program + 4 VarDecls (each with Ident + Literal) = 1 + 4*3 = 13 nodes
        assert_eq!(ast.nodes.len(), 13);

        // All literals should be LiteralNumber
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralNumber);
        assert_eq!(ast.nodes[6].node_type, NodeType::LiteralNumber);
        assert_eq!(ast.nodes[9].node_type, NodeType::LiteralNumber);
        assert_eq!(ast.nodes[12].node_type, NodeType::LiteralNumber);
    }

    #[test]
    fn test_string_literals() {
        let source = "a: \"hello\"\nb: 'world'\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Should have 1 Program + 2 VarDecls (each with Ident + Literal) = 1 + 2*3 = 7 nodes
        assert_eq!(ast.nodes.len(), 7);

        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralString);
        assert_eq!(ast.nodes[6].node_type, NodeType::LiteralString);
    }

    #[test]
    fn test_error_missing_colon() {
        let source = "x 42";
        let tokens = lex(source).unwrap();
        let result = parse(source, tokens, 256);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Expected ':'"));
    }

    #[test]
    fn test_error_missing_value() {
        let source = "x:\n";
        let tokens = lex(source).unwrap();
        let result = parse(source, tokens, 256);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("literal") || err.message.contains("identifier"));
    }

    #[test]
    fn test_error_missing_newline() {
        let source = "x: 42 y: 5";
        let tokens = lex(source).unwrap();
        let result = parse(source, tokens, 256);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("newline"));
    }

    #[test]
    fn test_empty_file() {
        let source = "";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Should have just the Program root node
        assert_eq!(ast.nodes.len(), 1);
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);
        assert_eq!(ast.nodes[0].first_child, None);
    }

    #[test]
    fn test_multiple_newlines() {
        let source = "\n\nx: 42\n\n\ny: true\n\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Should have 1 Program + 2 VarDecls (each with Ident + Literal) = 1 + 2*3 = 7 nodes
        assert_eq!(ast.nodes.len(), 7);
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[4].node_type, NodeType::VarDecl);
    }

    #[test]
    fn test_tree_traversal() {
        let source = "x: 42\ny: true\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Test iterating through children of Program
        let program_node = &ast.nodes[0];
        assert_eq!(program_node.first_child, Some(1));

        // Iterate through VarDecl siblings
        let mut var_decl_count = 0;
        if let Some(first_child) = program_node.first_child {
            let mut current = first_child;
            loop {
                assert_eq!(ast.nodes[current].node_type, NodeType::VarDecl);
                var_decl_count += 1;

                if let Some(next) = ast.nodes[current].next_sibling {
                    current = next;
                } else {
                    break;
                }
            }
        }

        assert_eq!(var_decl_count, 2);
    }

    // Boolean expression tests
    #[test]
    fn test_not_operator() {
        let source = "x: not true\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Should have: Program, VarDecl, Ident, LiteralBoolean, Not
        assert_eq!(ast.nodes.len(), 5);
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[2].node_type, NodeType::Ident);
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralBoolean);
        assert_eq!(ast.nodes[4].node_type, NodeType::Not);

        // Verify tree: VarDecl -> [Ident, Not], Not -> LiteralBoolean
        assert_eq!(ast.nodes[1].first_child, Some(2)); // VarDecl -> Ident
        assert_eq!(ast.nodes[2].next_sibling, Some(4)); // Ident -> Not
        assert_eq!(ast.nodes[4].first_child, Some(3)); // Not -> LiteralBoolean
    }

    #[test]
    fn test_and_operator() {
        let source = "x: true and false\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Nodes: Program, VarDecl, Ident, LiteralBoolean(true), LiteralBoolean(false), And
        assert_eq!(ast.nodes.len(), 6);
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[2].node_type, NodeType::Ident);
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralBoolean); // left: true
        assert_eq!(ast.nodes[4].node_type, NodeType::LiteralBoolean); // right: false
        assert_eq!(ast.nodes[5].node_type, NodeType::And);

        // Verify tree: VarDecl -> [Ident, And], And -> [true, false]
        assert_eq!(ast.nodes[1].first_child, Some(2)); // VarDecl -> Ident
        assert_eq!(ast.nodes[2].next_sibling, Some(5)); // Ident -> And
        assert_eq!(ast.nodes[5].first_child, Some(3)); // And -> true
        assert_eq!(ast.nodes[3].next_sibling, Some(4)); // true -> false
    }

    #[test]
    fn test_or_operator() {
        let source = "x: true or false\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Nodes: Program, VarDecl, Ident, LiteralBoolean(true), Or, LiteralBoolean(false)
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
        let source = "x: true or false and true\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

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
        let source = "x: not true and false\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Just verify it parses and has correct number of nodes
        assert_eq!(ast.nodes.len(), 7);

        // Find the expression - should be And at top level
        let var_decl_idx = ast.nodes[0].first_child.unwrap();
        let ident_idx = ast.nodes[var_decl_idx].first_child.unwrap();
        let expr_idx = ast.nodes[ident_idx].next_sibling.unwrap();

        // Top level should be And
        assert_eq!(ast.nodes[expr_idx].node_type, NodeType::And);

        // And's left child should be Not (higher precedence)
        let and_left = ast.nodes[expr_idx].first_child.unwrap();
        assert_eq!(ast.nodes[and_left].node_type, NodeType::Not);
    }

    #[test]
    fn test_double_not() {
        let source = "x: not not true\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Should have: Program, VarDecl, Ident, LiteralBoolean, Not(inner), Not(outer)
        assert_eq!(ast.nodes.len(), 6);
        assert_eq!(ast.nodes[3].node_type, NodeType::LiteralBoolean);
        assert_eq!(ast.nodes[4].node_type, NodeType::Not);
        assert_eq!(ast.nodes[5].node_type, NodeType::Not);

        // Verify tree: Not(outer) -> Not(inner) -> true
        assert_eq!(ast.nodes[5].first_child, Some(4)); // Not(outer) -> Not(inner)
        assert_eq!(ast.nodes[4].first_child, Some(3)); // Not(inner) -> true
    }

    #[test]
    fn test_complex_boolean_expression() {
        // "not false or true and not true" should parse as "(not false) or (true and (not true))"
        let source = "x: not false or true and not true\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Just verify it parses successfully
        assert_eq!(ast.nodes[0].node_type, NodeType::Program);
        assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
        assert_eq!(ast.nodes[2].node_type, NodeType::Ident);

        // Find the expression - should be Or at top level (lowest precedence)
        let var_decl_idx = ast.nodes[0].first_child.unwrap();
        let ident_idx = ast.nodes[var_decl_idx].first_child.unwrap();
        let expr_idx = ast.nodes[ident_idx].next_sibling.unwrap();

        assert_eq!(ast.nodes[expr_idx].node_type, NodeType::Or);
    }

    #[test]
    fn test_left_associativity() {
        // "true and false and true" should parse as "(true and false) and true"
        let source = "x: true and false and true\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens, 256).unwrap();

        // Top-level should be And, with left child being another And
        let var_decl_idx = 1;
        let top_and_idx = ast.nodes[var_decl_idx].first_child.unwrap();
        let top_and_idx = ast.nodes[top_and_idx].next_sibling.unwrap(); // Skip Ident

        assert_eq!(ast.nodes[top_and_idx].node_type, NodeType::And);

        let left_child_idx = ast.nodes[top_and_idx].first_child.unwrap();
        assert_eq!(ast.nodes[left_child_idx].node_type, NodeType::And);
    }

    #[test]
    fn test_recursion_depth_limit() {
        // Create deeply nested expression: "not not not not true" (4 nots)
        // With max depth of 3, this should fail
        let source = "x: not not not not true\n";
        let tokens = lex(source).unwrap();
        let result = parse(source, tokens, 3);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("nesting too deep"));
        assert!(err.message.contains("max 3"));
    }

    #[test]
    fn test_recursion_depth_limit_ok() {
        // Same expression with sufficient depth limit should succeed
        let source = "x: not not not not true\n";
        let tokens = lex(source).unwrap();
        let result = parse(source, tokens, 10);

        assert!(result.is_ok());
    }

    // === Function Call Tests ===

    #[test]
    fn test_simple_function_call_no_args() {
        let source = "x: print()\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"x\"
    FunctionCall
      Ident \"print\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    #[test]
    fn test_function_call_single_arg() {
        let source = "x: print(42)\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"x\"
    FunctionCall
      Ident \"print\"
      LiteralNumber \"42\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    #[test]
    fn test_function_call_multiple_args() {
        let source = "x: add(1, 2, 3)\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"x\"
    FunctionCall
      Ident \"add\"
      LiteralNumber \"1\"
      LiteralNumber \"2\"
      LiteralNumber \"3\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    #[test]
    fn test_function_call_string_arg() {
        let source = "x: print(\"hello\")\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"x\"
    FunctionCall
      Ident \"print\"
      LiteralString \"\"hello\"\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    #[test]
    fn test_function_call_boolean_args() {
        let source = "x: test(true, false)\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"x\"
    FunctionCall
      Ident \"test\"
      LiteralBoolean \"true\"
      LiteralBoolean \"false\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    #[test]
    fn test_function_call_identifier_arg() {
        let source = "y: print(x)\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"y\"
    FunctionCall
      Ident \"print\"
      Ident \"x\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    #[test]
    fn test_function_call_mixed_args() {
        let source = "z: add(42, x, \"test\", true)\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"z\"
    FunctionCall
      Ident \"add\"
      LiteralNumber \"42\"
      Ident \"x\"
      LiteralString \"\"test\"\"
      LiteralBoolean \"true\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    #[test]
    fn test_standalone_identifier() {
        let source = "x: y\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"x\"
    Ident \"y\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    #[test]
    fn test_function_call_with_boolean_expr() {
        let source = "x: not print()\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"x\"
    Not
      FunctionCall
        Ident \"print\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    #[test]
    fn test_function_calls_in_boolean_expr() {
        let source = "x: f() and g()\n";
        let tokens = lex(source).unwrap();
        let ast = parse(source, tokens.clone(), 256).unwrap();

        let expected = "\
Program
  VarDecl
    Ident \"x\"
    And
      FunctionCall
        Ident \"f\"
      FunctionCall
        Ident \"g\"
";
        assert_eq!(ast.tree_string(&tokens), expected);
    }

    // === Error Tests ===

    #[test]
    fn test_error_nested_function_call() {
        let source = "x: print(add(1, 2))\n";
        let tokens = lex(source).unwrap();
        let result = parse(source, tokens, 256);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Nested function calls are not supported"));
    }

    #[test]
    fn test_error_missing_closing_paren() {
        let source = "x: print(42\n";
        let tokens = lex(source).unwrap();
        let result = parse(source, tokens, 256);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("',' or ')'") || err.message.contains("Expected"));
    }

    #[test]
    fn test_error_missing_comma() {
        let source = "x: print(1 2)\n";
        let tokens = lex(source).unwrap();
        let result = parse(source, tokens, 256);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("',' or ')'") || err.message.contains("Expected"));
    }

    #[test]
    fn test_error_trailing_comma() {
        let source = "x: print(1, 2,)\n";
        let tokens = lex(source).unwrap();
        let result = parse(source, tokens, 256);

        // Trailing commas are currently allowed (parser accepts them without error)
        // This test verifies current behavior - can be changed if we want to reject trailing commas
        assert!(result.is_ok());
    }
}
