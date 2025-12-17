use std::iter::Peekable;
use std::str::CharIndices;

// Token types

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (14 total)
    Module,
    Import,
    Export,
    Return,
    Match,
    Type,
    Try,
    And,
    Or,
    Not,
    True,
    False,
    This,
    Partial,

    // Identifiers and Literals
    Ident,
    Number(NumberKind),
    String(StringKind),

    // Operators (single char)
    Colon,     // :
    Semicolon, // ;
    Comma,     // ,
    Dot,       // .
    Pipe,      // |
    Star,      // *
    Plus,      // +
    Minus,     // -
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Lt,        // <
    Gt,        // >

    // Special
    Underscore, // _ (when standalone)
    Newline,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumberKind {
    Binary,  // 0b1010
    Octal,   // 0o755
    Hex,     // 0xFF
    Decimal, // 42
    Float,   // 3.14
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringKind {
    Standard,     // "..." or '...'
    Interpolated, // `...`
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,  // byte offset
    pub end: usize,    // byte offset (exclusive)
    pub line: usize,   // 1-indexed
    pub column: usize, // 1-indexed
}

impl Token {
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub pos: usize,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Lexical error at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for LexError {}

// Lexer

pub struct Lexer<'src> {
    source: &'src str,
    chars: Peekable<CharIndices<'src>>,
    pos: usize,
    line: usize,
    column: usize,
    limits: crate::limits::CompilerLimits,
    token_count: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Result<Self, LexError> {
        Self::new_with_limits(source, crate::limits::CompilerLimits::default())
    }

    pub fn new_with_limits(
        source: &'src str,
        limits: crate::limits::CompilerLimits,
    ) -> Result<Self, LexError> {
        // Check input size limit
        if source.len() > limits.max_input_size {
            return Err(LexError {
                message: format!(
                    "Input too large: {} bytes (max: {} bytes). Consider splitting into modules.",
                    source.len(),
                    limits.max_input_size
                ),
                line: 1,
                column: 1,
                pos: 0,
            });
        }

        Ok(Self {
            source,
            chars: source.char_indices().peekable(),
            pos: 0,
            line: 1,
            column: 1,
            limits,
            token_count: 0,
        })
    }

    // Character navigation methods

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn peek_char2(&mut self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next();
        iter.peek().map(|(_, c)| *c)
    }

    fn consume_char(&mut self) -> Option<char> {
        if let Some((pos, ch)) = self.chars.next() {
            self.pos = pos + ch.len_utf8();

            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }

            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.consume_char();
            } else {
                break;
            }
        }
    }

    fn consume_while<F>(&mut self, predicate: F) -> bool
    where
        F: Fn(char) -> bool,
    {
        let mut consumed = false;
        while let Some(c) = self.peek_char() {
            if predicate(c) {
                self.consume_char();
                consumed = true;
            } else {
                break;
            }
        }
        consumed
    }

    fn error(&self, message: String) -> LexError {
        LexError {
            message,
            line: self.line,
            column: self.column,
            pos: self.pos,
        }
    }

    // Main tokenization method

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        // Check token count limit before creating new token
        if self.token_count >= self.limits.max_token_count {
            return Err(LexError {
                message: format!(
                    "Token limit exceeded: {} tokens (max: {}). File is too complex.",
                    self.token_count,
                    self.limits.max_token_count
                ),
                line: self.line,
                column: self.column,
                pos: self.pos,
            });
        }

        self.skip_whitespace();

        let start_pos = self.pos;
        let start_line = self.line;
        let start_column = self.column;

        let kind = match self.peek_char() {
            None => TokenKind::Eof,
            Some('\n') => {
                self.consume_char();
                TokenKind::Newline
            }
            Some('/') => self.lex_slash_or_comment()?,
            Some(c) if c.is_ascii_digit() => self.lex_number()?,
            Some('_') => self.lex_underscore_or_ident()?,
            Some(c) if is_ident_start(c) => self.lex_ident_or_keyword()?,
            Some('"') | Some('\'') => self.lex_standard_string()?,
            Some('`') => self.lex_interpolated_string()?,
            Some(':') => {
                self.consume_char();
                TokenKind::Colon
            }
            Some(';') => {
                self.consume_char();
                TokenKind::Semicolon
            }
            Some(',') => {
                self.consume_char();
                TokenKind::Comma
            }
            Some('.') => {
                self.consume_char();
                TokenKind::Dot
            }
            Some('|') => {
                self.consume_char();
                TokenKind::Pipe
            }
            Some('*') => {
                self.consume_char();
                TokenKind::Star
            }
            Some('+') => {
                self.consume_char();
                TokenKind::Plus
            }
            Some('-') => {
                self.consume_char();
                TokenKind::Minus
            }
            Some('(') => {
                self.consume_char();
                TokenKind::LParen
            }
            Some(')') => {
                self.consume_char();
                TokenKind::RParen
            }
            Some('{') => {
                self.consume_char();
                TokenKind::LBrace
            }
            Some('}') => {
                self.consume_char();
                TokenKind::RBrace
            }
            Some('[') => {
                self.consume_char();
                TokenKind::LBracket
            }
            Some(']') => {
                self.consume_char();
                TokenKind::RBracket
            }
            Some('<') => {
                self.consume_char();
                TokenKind::Lt
            }
            Some('>') => {
                self.consume_char();
                TokenKind::Gt
            }
            Some(c) => {
                self.consume_char();
                return Err(self.error(format!("Unexpected character: '{}'", c)));
            }
        };

        // Increment token count after successful tokenization
        self.token_count += 1;

        Ok(Token {
            kind,
            start: start_pos,
            end: self.pos,
            line: start_line,
            column: start_column,
        })
    }

    // Comment handling

    fn lex_slash_or_comment(&mut self) -> Result<TokenKind, LexError> {
        self.consume_char(); // first '/'

        if self.peek_char() == Some('/') {
            let comment_start = self.pos;

            // Comment - consume until end of line (including the newline)
            while let Some(c) = self.peek_char() {
                self.consume_char();

                // Check comment length during consumption
                let comment_len = self.pos - comment_start;
                if comment_len > self.limits.max_comment_length {
                    return Err(self.error(format!(
                        "Comment too long: {} bytes (max: {} bytes)",
                        comment_len,
                        self.limits.max_comment_length
                    )));
                }

                if c == '\n' {
                    break;
                }
            }
            // Recursively get next token (skip the comment)
            self.next_token().map(|t| t.kind)
        } else {
            // Just a division operator (not in spec, treat as unknown)
            Err(self.error("Unexpected character: '/'".into()))
        }
    }

    // Identifier and keyword lexing

    fn lex_ident_or_keyword(&mut self) -> Result<TokenKind, LexError> {
        let start = self.pos;

        // Consume first character (already validated as ident start)
        let first_char = self.consume_char().unwrap();

        // Consume remaining identifier characters
        while let Some(c) = self.peek_char() {
            if is_ident_continue(c) {
                self.consume_char();
            } else {
                break;
            }
        }

        let text = &self.source[start..self.pos];

        // Check identifier length
        if text.len() > self.limits.max_identifier_length {
            return Err(self.error(format!(
                "Identifier too long: {} bytes (max: {} bytes)",
                text.len(),
                self.limits.max_identifier_length
            )));
        }

        // Keywords cannot start with uppercase
        if first_char.is_ascii_uppercase() {
            return Ok(TokenKind::Ident);
        }

        // Length-based filtering: keywords are max 7 chars
        if text.len() > 7 {
            return Ok(TokenKind::Ident);
        }

        // Match keywords
        let kind = match text {
            "module" => TokenKind::Module,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "return" => TokenKind::Return,
            "match" => TokenKind::Match,
            "type" => TokenKind::Type,
            "try" => TokenKind::Try,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "this" => TokenKind::This,
            "partial" => TokenKind::Partial,
            _ => TokenKind::Ident,
        };

        Ok(kind)
    }

    fn lex_underscore_or_ident(&mut self) -> Result<TokenKind, LexError> {
        self.consume_char(); // consume '_'

        // Check if next char continues identifier (but not another underscore alone)
        if let Some(c) = self.peek_char() {
            if c.is_ascii_alphanumeric() || (c == '_') {
                // It's an identifier starting with _
                while let Some(c) = self.peek_char() {
                    if is_ident_continue(c) {
                        self.consume_char();
                    } else {
                        break;
                    }
                }
                return Ok(TokenKind::Ident);
            }
        }

        // Standalone underscore
        Ok(TokenKind::Underscore)
    }

    // Number lexing

    fn lex_number(&mut self) -> Result<TokenKind, LexError> {
        // Check for prefix (0b, 0o, 0x)
        if self.peek_char() == Some('0') {
            if let Some(second) = self.peek_char2() {
                match second {
                    'b' | 'B' => return self.lex_binary_number(),
                    'o' | 'O' => return self.lex_octal_number(),
                    'x' | 'X' => return self.lex_hex_number(),
                    _ => {}
                }
            }
        }

        // Decimal or float
        self.lex_decimal_or_float()
    }

    fn lex_binary_number(&mut self) -> Result<TokenKind, LexError> {
        self.consume_char(); // '0'
        self.consume_char(); // 'b' or 'B'

        let has_digits = self.consume_while(|c| c == '0' || c == '1' || c == '_');

        if !has_digits {
            return Err(self.error("Binary number must have at least one digit".into()));
        }

        self.lex_number_suffix()?;
        Ok(TokenKind::Number(NumberKind::Binary))
    }

    fn lex_octal_number(&mut self) -> Result<TokenKind, LexError> {
        self.consume_char(); // '0'
        self.consume_char(); // 'o' or 'O'

        let has_digits = self.consume_while(|c| (c.is_ascii_digit() && c < '8') || c == '_');

        if !has_digits {
            return Err(self.error("Octal number must have at least one digit".into()));
        }

        self.lex_number_suffix()?;
        Ok(TokenKind::Number(NumberKind::Octal))
    }

    fn lex_hex_number(&mut self) -> Result<TokenKind, LexError> {
        self.consume_char(); // '0'
        self.consume_char(); // 'x' or 'X'

        let has_digits = self.consume_while(|c| c.is_ascii_hexdigit() || c == '_');

        if !has_digits {
            return Err(self.error("Hex number must have at least one digit".into()));
        }

        self.lex_number_suffix()?;
        Ok(TokenKind::Number(NumberKind::Hex))
    }

    fn lex_decimal_or_float(&mut self) -> Result<TokenKind, LexError> {
        // Integer part
        self.consume_while(|c| c.is_ascii_digit() || c == '_');

        // Check for decimal point
        if self.peek_char() == Some('.')
            && self.peek_char2().map_or(false, |c| c.is_ascii_digit())
        {
            self.consume_char(); // '.'
            self.consume_while(|c| c.is_ascii_digit() || c == '_');

            // Optional exponent
            if let Some('e') | Some('E') = self.peek_char() {
                self.consume_char();
                if let Some('+') | Some('-') = self.peek_char() {
                    self.consume_char();
                }
                let has_exp = self.consume_while(|c| c.is_ascii_digit() || c == '_');
                if !has_exp {
                    return Err(self.error("Float exponent must have digits".into()));
                }
            }

            self.lex_number_suffix()?;
            return Ok(TokenKind::Number(NumberKind::Float));
        }

        self.lex_number_suffix()?;
        Ok(TokenKind::Number(NumberKind::Decimal))
    }

    fn lex_number_suffix(&mut self) -> Result<(), LexError> {
        // Type suffixes: i8, u32, f64, etc.
        if let Some(c) = self.peek_char() {
            if c == 'i' || c == 'u' || c == 'f' {
                self.consume_char();
                // Consume digits for suffix
                self.consume_while(|c| c.is_ascii_digit());
            }
        }
        Ok(())
    }

    // String lexing

    fn lex_standard_string(&mut self) -> Result<TokenKind, LexError> {
        let string_start = self.pos;
        let quote = self.consume_char().unwrap(); // " or '

        loop {
            match self.peek_char() {
                None => {
                    return Err(self.error("Unterminated string literal".into()));
                }
                Some('\n') => {
                    return Err(self.error("Newline in string literal".into()));
                }
                Some('\\') => {
                    self.consume_char(); // backslash
                    if self.consume_char().is_none() {
                        return Err(self.error("Unterminated escape sequence".into()));
                    }
                }
                Some(c) if c == quote => {
                    self.consume_char();
                    break;
                }
                Some(_) => {
                    self.consume_char();
                }
            }
        }

        // Check string length
        let string_len = self.pos - string_start;
        if string_len > self.limits.max_string_length {
            return Err(self.error(format!(
                "String literal too long: {} bytes (max: {} bytes)",
                string_len,
                self.limits.max_string_length
            )));
        }

        Ok(TokenKind::String(StringKind::Standard))
    }

    fn lex_interpolated_string(&mut self) -> Result<TokenKind, LexError> {
        let string_start = self.pos;
        self.consume_char(); // opening `

        loop {
            match self.peek_char() {
                None => {
                    return Err(self.error("Unterminated interpolated string".into()));
                }
                Some('`') => {
                    self.consume_char();
                    break;
                }
                Some('\\') => {
                    self.consume_char();
                    if self.consume_char().is_none() {
                        return Err(self.error("Unterminated escape sequence".into()));
                    }
                }
                Some(_) => {
                    self.consume_char();
                }
            }
        }

        // Check string length
        let string_len = self.pos - string_start;
        if string_len > self.limits.max_string_length {
            return Err(self.error(format!(
                "String literal too long: {} bytes (max: {} bytes)",
                string_len,
                self.limits.max_string_length
            )));
        }

        Ok(TokenKind::String(StringKind::Interpolated))
    }
}

// Helper functions

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

// Public API

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    lex_with_limits(source, crate::limits::CompilerLimits::default())
}

pub fn lex_with_limits(
    source: &str,
    limits: crate::limits::CompilerLimits,
) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::new_with_limits(source, limits)?;
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token()?;
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(token);
        if is_eof {
            break;
        }
    }

    Ok(tokens)
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    // Helper
    fn lex_single(source: &str) -> Result<Token, LexError> {
        Lexer::new(source)?.next_token()
    }

    #[test]
    fn test_keywords() {
        // Test all 14 keywords (lowercase)
        assert_eq!(lex_single("module").unwrap().kind, TokenKind::Module);
        assert_eq!(lex_single("import").unwrap().kind, TokenKind::Import);
        assert_eq!(lex_single("export").unwrap().kind, TokenKind::Export);
        assert_eq!(lex_single("return").unwrap().kind, TokenKind::Return);
        assert_eq!(lex_single("match").unwrap().kind, TokenKind::Match);
        assert_eq!(lex_single("type").unwrap().kind, TokenKind::Type);
        assert_eq!(lex_single("try").unwrap().kind, TokenKind::Try);
        assert_eq!(lex_single("and").unwrap().kind, TokenKind::And);
        assert_eq!(lex_single("or").unwrap().kind, TokenKind::Or);
        assert_eq!(lex_single("not").unwrap().kind, TokenKind::Not);
        assert_eq!(lex_single("true").unwrap().kind, TokenKind::True);
        assert_eq!(lex_single("false").unwrap().kind, TokenKind::False);
        assert_eq!(lex_single("this").unwrap().kind, TokenKind::This);
        assert_eq!(lex_single("partial").unwrap().kind, TokenKind::Partial);

        // Uppercase first letter should be identifiers
        assert_eq!(lex_single("Module").unwrap().kind, TokenKind::Ident);
        assert_eq!(lex_single("MODULE").unwrap().kind, TokenKind::Ident);
    }

    #[test]
    fn test_identifiers() {
        assert_eq!(lex_single("foo").unwrap().kind, TokenKind::Ident);
        assert_eq!(lex_single("_bar").unwrap().kind, TokenKind::Ident);
        assert_eq!(lex_single("baz123").unwrap().kind, TokenKind::Ident);
        assert_eq!(lex_single("MyClass").unwrap().kind, TokenKind::Ident);
    }

    #[test]
    fn test_underscore() {
        assert_eq!(lex_single("_").unwrap().kind, TokenKind::Underscore);
    }

    #[test]
    fn test_numbers() {
        // Decimal
        let tok = lex_single("42").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Decimal));

        let tok = lex_single("1_000_000").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Decimal));

        // Binary
        let tok = lex_single("0b1010").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Binary));

        let tok = lex_single("0b1111_0000").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Binary));

        // Octal
        let tok = lex_single("0o755").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Octal));

        // Hex
        let tok = lex_single("0xFF").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Hex));

        let tok = lex_single("0xDEAD_BEEF").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Hex));

        // Float
        let tok = lex_single("3.14").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Float));

        let tok = lex_single("1.5e10").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Float));

        let tok = lex_single("2.5e-3").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Float));
    }

    #[test]
    fn test_number_suffixes() {
        let tok = lex_single("42i32").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Decimal));

        let tok = lex_single("100u64").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Decimal));

        let tok = lex_single("3.14f32").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Float));
    }

    #[test]
    fn test_strings() {
        let tok = lex_single(r#""hello""#).unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Standard));

        let tok = lex_single("'world'").unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Standard));

        let tok = lex_single("`interpolated`").unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Interpolated));
    }

    #[test]
    fn test_string_escapes() {
        let tok = lex_single(r#""hello\nworld""#).unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Standard));

        let tok = lex_single(r#""quote: \"hi\"""#).unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Standard));
    }

    #[test]
    fn test_operators() {
        assert_eq!(lex_single(":").unwrap().kind, TokenKind::Colon);
        assert_eq!(lex_single(";").unwrap().kind, TokenKind::Semicolon);
        assert_eq!(lex_single(",").unwrap().kind, TokenKind::Comma);
        assert_eq!(lex_single(".").unwrap().kind, TokenKind::Dot);
        assert_eq!(lex_single("|").unwrap().kind, TokenKind::Pipe);
        assert_eq!(lex_single("*").unwrap().kind, TokenKind::Star);
        assert_eq!(lex_single("+").unwrap().kind, TokenKind::Plus);
        assert_eq!(lex_single("-").unwrap().kind, TokenKind::Minus);
    }

    #[test]
    fn test_brackets() {
        assert_eq!(lex_single("(").unwrap().kind, TokenKind::LParen);
        assert_eq!(lex_single(")").unwrap().kind, TokenKind::RParen);
        assert_eq!(lex_single("{").unwrap().kind, TokenKind::LBrace);
        assert_eq!(lex_single("}").unwrap().kind, TokenKind::RBrace);
        assert_eq!(lex_single("[").unwrap().kind, TokenKind::LBracket);
        assert_eq!(lex_single("]").unwrap().kind, TokenKind::RBracket);
        assert_eq!(lex_single("<").unwrap().kind, TokenKind::Lt);
        assert_eq!(lex_single(">").unwrap().kind, TokenKind::Gt);
    }

    #[test]
    fn test_comments() {
        // Comment should be skipped, next token is the number
        let tok = lex_single("// this is a comment\n42").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Decimal));
    }

    #[test]
    fn test_errors() {
        // Unterminated string
        assert!(lex_single(r#""hello"#).is_err());

        // Newline in string
        assert!(lex_single("\"hello\nworld\"").is_err());

        // Invalid binary number
        assert!(lex_single("0b").is_err());

        // Invalid hex number
        assert!(lex_single("0x").is_err());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_statement() {
        let source = "module main import std return 42";
        let tokens = lex(source).unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Module);
        assert_eq!(tokens[1].kind, TokenKind::Ident);
        assert_eq!(tokens[2].kind, TokenKind::Import);
        assert_eq!(tokens[3].kind, TokenKind::Ident);
        assert_eq!(tokens[4].kind, TokenKind::Return);
        assert_eq!(tokens[5].kind, TokenKind::Number(NumberKind::Decimal));
        assert_eq!(tokens[6].kind, TokenKind::Eof);
    }

    #[test]
    fn test_multiline() {
        let source = "module main\nreturn 42\n";
        let tokens = lex(source).unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Module);
        assert_eq!(tokens[1].kind, TokenKind::Ident);
        assert_eq!(tokens[2].kind, TokenKind::Newline);
        assert_eq!(tokens[3].kind, TokenKind::Return);
        assert_eq!(tokens[4].kind, TokenKind::Number(NumberKind::Decimal));
        assert_eq!(tokens[5].kind, TokenKind::Newline);
        assert_eq!(tokens[6].kind, TokenKind::Eof);
    }

    #[test]
    fn test_position_tracking() {
        let source = "foo bar\nbaz";
        let tokens = lex(source).unwrap();

        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);

        assert_eq!(tokens[1].line, 1);
        assert_eq!(tokens[1].column, 5);

        assert_eq!(tokens[2].line, 1); // newline

        assert_eq!(tokens[3].line, 2);
        assert_eq!(tokens[3].column, 1);
    }

    #[test]
    fn test_complex_expression() {
        let source = r#"
            type Point {
                x: i32,
                y: i32
            }
        "#;

        let tokens = lex(source).unwrap();

        // Just verify it doesn't error and has expected structure
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Type));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::LBrace));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::RBrace));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Colon));
    }

    #[test]
    fn test_empty_source() {
        let tokens = lex("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_only_whitespace() {
        let tokens = lex("   \t  \n  ").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Newline);
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }
}
