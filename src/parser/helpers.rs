use super::error::ParseError;
use crate::lexer::{Token, TokenKind};

// Operator precedence levels
pub(super) fn get_precedence(token_kind: &TokenKind) -> Option<u8> {
    match token_kind {
        TokenKind::Or => Some(1),
        TokenKind::Pipe => Some(1), // Same precedence as Or
        TokenKind::And => Some(2),
        TokenKind::Not => Some(3), // Unary operator
        TokenKind::Dot => Some(4), // Postfix operator (highest precedence)
        _ => None,
    }
}

// Parser helper methods
impl<'a> super::Parser<'a> {
    pub(crate) fn new_unexpected_token(&self, message: &str) -> ParseError {
        ParseError::unexpected_token(
            message,
            self.current_token(),
            self.current,
            &self.ast.string_storage,
        )
    }

    // Helper: Check recursion depth limit
    pub(super) fn check_depth(&self, depth: usize) -> Result<(), ParseError> {
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

    /// Helper: Consume a specific token kind or error
    pub(super) fn consume(&mut self, kind: TokenKind, expected: &str) -> Result<(), ParseError> {
        let token = self.current_token();
        if token.kind != kind {
            return Err(ParseError::unexpected_token(
                expected,
                token,
                self.current,
                &self.ast.string_storage,
            ));
        }
        self.advance();
        Ok(())
    }

    /// Helper: Advance to the next token
    pub(super) fn advance(&mut self) {
        self.current = (self.current + 1).min(self.tokens.list.len());
    }

    /// Helper: peek current token
    pub(crate) fn peek_kind(&self) -> TokenKind {
        self.tokens.peek_kind(self.current)
    }

    /// Helper: peek n token
    pub(crate) fn peek_next_kind(&self, n: usize) -> TokenKind {
        self.tokens.peek_kind(self.current + n)
    }

    pub(crate) fn peek_kind_is(&self, kind: TokenKind) -> bool {
        self.tokens.peek_kind(self.current) == kind
    }

    // Helper: Get next token
    pub(super) fn get_next_token(&self, index: usize) -> &Token {
        self.tokens.get(index)
    }

    // Helper: Get current token
    pub(super) fn current_token(&self) -> &Token {
        self.tokens.get(self.current)
    }

    /// Helper: Get a clone of the current token (for storing in AST nodes)
    pub(super) fn clone_current_token(&self) -> Token {
        self.current_token().clone()
    }

    // Helper: Skip consecutive newlines
    pub(super) fn skip_newlines(&mut self) {
        while self.peek_kind_is(TokenKind::Newline) {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precedence_levels() {
        assert_eq!(get_precedence(&TokenKind::Or), Some(1));
        assert_eq!(get_precedence(&TokenKind::Pipe), Some(1));
        assert_eq!(get_precedence(&TokenKind::And), Some(2));
        assert_eq!(get_precedence(&TokenKind::Not), Some(3));
        assert_eq!(get_precedence(&TokenKind::Dot), Some(4));
        assert_eq!(get_precedence(&TokenKind::Identifier), None);
        assert_eq!(get_precedence(&TokenKind::Plus), None);
    }
}
