use crate::lexer::{Token, TokenKind};
use super::error::ParseError;

// Operator precedence levels
pub(super) fn get_precedence(token_kind: &TokenKind) -> Option<u8> {
    match token_kind {
        TokenKind::Or => Some(1),
        TokenKind::And => Some(2),
        TokenKind::Not => Some(3), // Unary operator
        _ => None,
    }
}

// Parser helper methods
impl<'a> super::Parser<'a> {
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
                self.source,
            ));
        }
        self.current += 1;
        Ok(())
    }

    /// Helper: Advance to the next token without validation
    pub(super) fn advance(&mut self) {
        self.current += 1;
    }

    // Helper: Get current token (with bounds checking)
    pub(super) fn current_token(&self) -> &Token {
        // If we've gone past the end, return the EOF token (always last)
        if self.current >= self.tokens.len() {
            &self.tokens[self.tokens.len() - 1]
        } else {
            &self.tokens[self.current]
        }
    }

    // Helper: Skip consecutive newlines
    pub(super) fn skip_newlines(&mut self) {
        while self.current < self.tokens.len()
            && self.tokens[self.current].kind == TokenKind::Newline
        {
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
        assert_eq!(get_precedence(&TokenKind::And), Some(2));
        assert_eq!(get_precedence(&TokenKind::Not), Some(3));
        assert_eq!(get_precedence(&TokenKind::Identifier), None);
        assert_eq!(get_precedence(&TokenKind::Plus), None);
    }
}
