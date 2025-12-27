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
    pub(super) fn from_token(message: String, token: &Token, token_idx: usize) -> Self {
        Self {
            message,
            line: token.line,
            column: token.column,
            token_idx,
        }
    }

    pub(super) fn unexpected_token(expected: &str, token: &Token, token_idx: usize, source: &str) -> Self {
        let found = match &token.kind {
            TokenKind::Eof => "end of file".to_string(),
            TokenKind::Newline => "newline".to_string(),
            TokenKind::Identifier => format!("identifier '{}'", token.text(source)),
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
