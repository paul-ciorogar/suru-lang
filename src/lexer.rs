use std::iter::Peekable;
use std::str::CharIndices;

use crate::string_storage::{StringId, StringStorage};

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
    Identifier,
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

#[derive(Debug, Clone)]
pub struct Tokens {
    pub list: Vec<Token>,
    pub string_storage: StringStorage,
}

impl Tokens {
    pub fn new(tokens: Vec<Token>, storage: StringStorage) -> Self {
        Self {
            list: tokens,
            string_storage: storage,
        }
    }

    pub fn peek_kind(&self, index: usize) -> TokenKind {
        match self.list.get(index) {
            Some(token) => token.kind.clone(),
            _ => TokenKind::Eof,
        }
    }

    pub fn get(&self, index: usize) -> &Token {
        self.list.get(index).unwrap_or(self.list.last().unwrap())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,                 // 1-indexed
    pub column: usize,               // 1-indexed
    pub string_id: Option<StringId>, // For identifiers and string literals
}

impl Token {
    pub fn text<'a>(&self, storage: &'a StringStorage) -> Option<&'a str> {
        self.string_id.map(|id| storage.resolve(id))
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

pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    pos: usize,
    line: usize,
    column: usize,
    limits: &'a crate::limits::CompilerLimits,
    token_count: usize,
    string_storage: StringStorage,
}

impl<'a> Lexer<'a> {
    pub fn new(
        source: &'a str,
        limits: &'a crate::limits::CompilerLimits,
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
            string_storage: StringStorage::new(),
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
                    self.token_count, self.limits.max_token_count
                ),
                line: self.line,
                column: self.column,
                pos: self.pos,
            });
        }

        self.skip_whitespace();

        let start_line = self.line;
        let start_column = self.column;

        let (kind, string_id) = match self.peek_char() {
            None => (TokenKind::Eof, None),
            Some('\n') => {
                self.consume_char();
                (TokenKind::Newline, None)
            }
            Some('/') => (self.lex_slash_or_comment()?, None),
            Some(c) if c.is_ascii_digit() => {
                let start = self.pos;
                let kind = self.lex_number()?;
                let text = &self.source[start..self.pos];
                let string_id = self.string_storage.intern(text);
                (kind, Some(string_id))
            }
            Some('_') => self.lex_underscore_or_ident()?,
            Some(c) if is_ident_start(c) => self.lex_ident_or_keyword()?,
            Some('"') | Some('\'') => self.lex_standard_string()?,
            Some('`') => self.lex_interpolated_string()?,
            Some(':') => {
                self.consume_char();
                (TokenKind::Colon, None)
            }
            Some(';') => {
                self.consume_char();
                (TokenKind::Semicolon, None)
            }
            Some(',') => {
                self.consume_char();
                (TokenKind::Comma, None)
            }
            Some('.') => {
                self.consume_char();
                (TokenKind::Dot, None)
            }
            Some('|') => {
                self.consume_char();
                (TokenKind::Pipe, None)
            }
            Some('*') => {
                self.consume_char();
                (TokenKind::Star, None)
            }
            Some('+') => {
                self.consume_char();
                (TokenKind::Plus, None)
            }
            Some('-') => {
                self.consume_char();
                (TokenKind::Minus, None)
            }
            Some('(') => {
                self.consume_char();
                (TokenKind::LParen, None)
            }
            Some(')') => {
                self.consume_char();
                (TokenKind::RParen, None)
            }
            Some('{') => {
                self.consume_char();
                (TokenKind::LBrace, None)
            }
            Some('}') => {
                self.consume_char();
                (TokenKind::RBrace, None)
            }
            Some('[') => {
                self.consume_char();
                (TokenKind::LBracket, None)
            }
            Some(']') => {
                self.consume_char();
                (TokenKind::RBracket, None)
            }
            Some('<') => {
                self.consume_char();
                (TokenKind::Lt, None)
            }
            Some('>') => {
                self.consume_char();
                (TokenKind::Gt, None)
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
            line: start_line,
            column: start_column,
            string_id,
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
                        comment_len, self.limits.max_comment_length
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

    fn lex_ident_or_keyword(&mut self) -> Result<(TokenKind, Option<StringId>), LexError> {
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
            let string_id = self.string_storage.intern(text);
            return Ok((TokenKind::Identifier, Some(string_id)));
        }

        // Length-based filtering: keywords are max 7 chars
        if text.len() > 7 {
            let string_id = self.string_storage.intern(text);
            return Ok((TokenKind::Identifier, Some(string_id)));
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
            _ => {
                // It's an identifier - intern it
                let string_id = self.string_storage.intern(text);
                return Ok((TokenKind::Identifier, Some(string_id)));
            }
        };

        // Keywords don't get interned
        Ok((kind, None))
    }

    fn lex_underscore_or_ident(&mut self) -> Result<(TokenKind, Option<StringId>), LexError> {
        let start = self.pos;
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
                let text = &self.source[start..self.pos];
                let string_id = self.string_storage.intern(text);
                return Ok((TokenKind::Identifier, Some(string_id)));
            }
        }

        // Standalone underscore (keyword, not interned)
        Ok((TokenKind::Underscore, None))
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
        if self.peek_char() == Some('.') && self.peek_char2().map_or(false, |c| c.is_ascii_digit())
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

    fn lex_standard_string(&mut self) -> Result<(TokenKind, Option<StringId>), LexError> {
        let string_start = self.pos; // For length checking (includes quotes)
        let quote = self.consume_char().unwrap(); // " or '
        let content_start = self.pos; // Content starts after opening quote

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
                    let content_end = self.pos; // Content ends before closing quote
                    self.consume_char(); // Consume closing quote

                    // Check string length (total literal including quotes for safety)
                    let string_len = self.pos - string_start;
                    if string_len > self.limits.max_string_length {
                        return Err(self.error(format!(
                            "String literal too long: {} bytes (max: {} bytes)",
                            string_len, self.limits.max_string_length
                        )));
                    }

                    // Extract content (without quotes) and intern it
                    let content = &self.source[content_start..content_end];
                    let string_id = self.string_storage.intern(content);

                    return Ok((TokenKind::String(StringKind::Standard), Some(string_id)));
                }
                Some(_) => {
                    self.consume_char();
                }
            }
        }
    }

    fn lex_interpolated_string(&mut self) -> Result<(TokenKind, Option<StringId>), LexError> {
        let string_start = self.pos; // For length checking (includes backticks)
        self.consume_char(); // opening `
        let content_start = self.pos; // Content starts after opening backtick

        loop {
            match self.peek_char() {
                None => {
                    return Err(self.error("Unterminated interpolated string".into()));
                }
                Some('`') => {
                    let content_end = self.pos; // Content ends before closing backtick
                    self.consume_char(); // Consume closing backtick

                    // Check string length (total literal including backticks for safety)
                    let string_len = self.pos - string_start;
                    if string_len > self.limits.max_string_length {
                        return Err(self.error(format!(
                            "String literal too long: {} bytes (max: {} bytes)",
                            string_len, self.limits.max_string_length
                        )));
                    }

                    // Extract content (without backticks) and intern it
                    let content = &self.source[content_start..content_end];
                    let string_id = self.string_storage.intern(content);

                    return Ok((TokenKind::String(StringKind::Interpolated), Some(string_id)));
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

pub fn lex(source: &str, limits: &crate::limits::CompilerLimits) -> Result<Tokens, LexError> {
    let mut lexer = Lexer::new(source, limits)?;
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token()?;
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(token);
        if is_eof {
            break;
        }
    }

    Ok(Tokens::new(tokens, lexer.string_storage))
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    // Helper
    fn lex_single(source: &str) -> Result<(Token, StringStorage), LexError> {
        let limits = crate::limits::CompilerLimits::default();
        let mut lexer = Lexer::new(source, &limits)?;
        let token = lexer.next_token()?;
        Ok((token, lexer.string_storage))
    }

    #[test]
    fn test_keywords() {
        // Test all 14 keywords (lowercase)
        assert_eq!(lex_single("module").unwrap().0.kind, TokenKind::Module);
        assert_eq!(lex_single("import").unwrap().0.kind, TokenKind::Import);
        assert_eq!(lex_single("export").unwrap().0.kind, TokenKind::Export);
        assert_eq!(lex_single("return").unwrap().0.kind, TokenKind::Return);
        assert_eq!(lex_single("match").unwrap().0.kind, TokenKind::Match);
        assert_eq!(lex_single("type").unwrap().0.kind, TokenKind::Type);
        assert_eq!(lex_single("try").unwrap().0.kind, TokenKind::Try);
        assert_eq!(lex_single("and").unwrap().0.kind, TokenKind::And);
        assert_eq!(lex_single("or").unwrap().0.kind, TokenKind::Or);
        assert_eq!(lex_single("not").unwrap().0.kind, TokenKind::Not);
        assert_eq!(lex_single("true").unwrap().0.kind, TokenKind::True);
        assert_eq!(lex_single("false").unwrap().0.kind, TokenKind::False);
        assert_eq!(lex_single("this").unwrap().0.kind, TokenKind::This);
        assert_eq!(lex_single("partial").unwrap().0.kind, TokenKind::Partial);

        // Uppercase first letter should be identifiers
        assert_eq!(lex_single("Module").unwrap().0.kind, TokenKind::Identifier);
        assert_eq!(lex_single("MODULE").unwrap().0.kind, TokenKind::Identifier);
    }

    #[test]
    fn test_identifiers() {
        assert_eq!(lex_single("foo").unwrap().0.kind, TokenKind::Identifier);
        assert_eq!(lex_single("_bar").unwrap().0.kind, TokenKind::Identifier);
        assert_eq!(lex_single("baz123").unwrap().0.kind, TokenKind::Identifier);
        assert_eq!(lex_single("MyClass").unwrap().0.kind, TokenKind::Identifier);
    }

    #[test]
    fn test_underscore() {
        assert_eq!(lex_single("_").unwrap().0.kind, TokenKind::Underscore);
    }

    #[test]
    fn test_numbers() {
        // Decimal
        let (tok, _storage) = lex_single("42").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Decimal));

        let (tok, _storage) = lex_single("1_000_000").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Decimal));

        // Binary
        let (tok, _storage) = lex_single("0b1010").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Binary));

        let (tok, _storage) = lex_single("0b1111_0000").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Binary));

        // Octal
        let (tok, _storage) = lex_single("0o755").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Octal));

        // Hex
        let (tok, _storage) = lex_single("0xFF").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Hex));

        let (tok, _storage) = lex_single("0xDEAD_BEEF").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Hex));

        // Float
        let (tok, _storage) = lex_single("3.14").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Float));

        let (tok, _storage) = lex_single("1.5e10").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Float));

        let (tok, _storage) = lex_single("2.5e-3").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Float));
    }

    #[test]
    fn test_number_suffixes() {
        let (tok, _storage) = lex_single("42i32").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Decimal));

        let (tok, _storage) = lex_single("100u64").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Decimal));

        let (tok, _storage) = lex_single("3.14f32").unwrap();
        assert_eq!(tok.kind, TokenKind::Number(NumberKind::Float));
    }

    #[test]
    fn test_strings() {
        let (tok, _storage) = lex_single(r#""hello""#).unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Standard));

        let (tok, _storage) = lex_single("'world'").unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Standard));

        let (tok, _storage) = lex_single("`interpolated`").unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Interpolated));
    }

    #[test]
    fn test_string_escapes() {
        let (tok, _storage) = lex_single(r#""hello\nworld""#).unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Standard));

        let (tok, _storage) = lex_single(r#""quote: \"hi\"""#).unwrap();
        assert_eq!(tok.kind, TokenKind::String(StringKind::Standard));
    }

    #[test]
    fn test_operators() {
        assert_eq!(lex_single(":").unwrap().0.kind, TokenKind::Colon);
        assert_eq!(lex_single(";").unwrap().0.kind, TokenKind::Semicolon);
        assert_eq!(lex_single(",").unwrap().0.kind, TokenKind::Comma);
        assert_eq!(lex_single(".").unwrap().0.kind, TokenKind::Dot);
        assert_eq!(lex_single("|").unwrap().0.kind, TokenKind::Pipe);
        assert_eq!(lex_single("*").unwrap().0.kind, TokenKind::Star);
        assert_eq!(lex_single("+").unwrap().0.kind, TokenKind::Plus);
        assert_eq!(lex_single("-").unwrap().0.kind, TokenKind::Minus);
    }

    #[test]
    fn test_brackets() {
        assert_eq!(lex_single("(").unwrap().0.kind, TokenKind::LParen);
        assert_eq!(lex_single(")").unwrap().0.kind, TokenKind::RParen);
        assert_eq!(lex_single("{").unwrap().0.kind, TokenKind::LBrace);
        assert_eq!(lex_single("}").unwrap().0.kind, TokenKind::RBrace);
        assert_eq!(lex_single("[").unwrap().0.kind, TokenKind::LBracket);
        assert_eq!(lex_single("]").unwrap().0.kind, TokenKind::RBracket);
        assert_eq!(lex_single("<").unwrap().0.kind, TokenKind::Lt);
        assert_eq!(lex_single(">").unwrap().0.kind, TokenKind::Gt);
    }

    #[test]
    fn test_comments() {
        // Comment should be skipped, next token is the number
        let (tok, _storage) = lex_single("// this is a comment\n42").unwrap();
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

    #[test]
    fn test_string_content_excludes_quotes() {
        // Standard strings with double quotes
        let source = r#""hello""#;
        let (tok, storage) = lex_single(source).unwrap();
        assert_eq!(tok.text(&storage), Some("hello")); // Content only, no quotes

        // Standard strings with single quotes
        let source = "'world'";
        let (tok, storage) = lex_single(source).unwrap();
        assert_eq!(tok.text(&storage), Some("world"));

        // Interpolated strings with backticks
        let source = "`test`";
        let (tok, storage) = lex_single(source).unwrap();
        assert_eq!(tok.text(&storage), Some("test"));
    }

    #[test]
    fn test_empty_strings_exclude_quotes() {
        let source = r#""""#;
        let (tok, storage) = lex_single(source).unwrap();
        assert_eq!(tok.text(&storage), Some("")); // Empty content, zero length

        let source = "''";
        let (tok, storage) = lex_single(source).unwrap();
        assert_eq!(tok.text(&storage), Some(""));

        let source = "``";
        let (tok, storage) = lex_single(source).unwrap();
        assert_eq!(tok.text(&storage), Some(""));
    }

    #[test]
    fn test_string_with_escapes_excludes_quotes() {
        let source = r#""hello\nworld""#;
        let (tok, storage) = lex_single(source).unwrap();
        assert_eq!(tok.text(&storage), Some(r"hello\nworld")); // Escapes preserved, quotes excluded

        let source = r#""quote: \"hi\"""#;
        let (tok, storage) = lex_single(source).unwrap();
        assert_eq!(tok.text(&storage), Some(r#"quote: \"hi\""#));
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::lexer::{self, LexError, NumberKind, TokenKind, Tokens};

    // use super::*;
    fn lex(source: &str) -> Result<Tokens, LexError> {
        let limits = crate::limits::CompilerLimits::default();
        lexer::lex(source, &limits)
    }

    #[test]
    fn test_complete_statement() {
        let source = "module main import std return 42";
        let tokens = lex(source).unwrap();

        assert_eq!(tokens.peek_kind(0), TokenKind::Module);
        assert_eq!(tokens.peek_kind(1), TokenKind::Identifier);
        assert_eq!(tokens.peek_kind(2), TokenKind::Import);
        assert_eq!(tokens.peek_kind(3), TokenKind::Identifier);
        assert_eq!(tokens.peek_kind(4), TokenKind::Return);
        assert_eq!(tokens.peek_kind(5), TokenKind::Number(NumberKind::Decimal));
        assert_eq!(tokens.peek_kind(6), TokenKind::Eof);
    }

    #[test]
    fn test_multiline() {
        let source = "module main\nreturn 42\n";
        let tokens = lex(source).unwrap();

        assert_eq!(tokens.peek_kind(0), TokenKind::Module);
        assert_eq!(tokens.peek_kind(1), TokenKind::Identifier);
        assert_eq!(tokens.peek_kind(2), TokenKind::Newline);
        assert_eq!(tokens.peek_kind(3), TokenKind::Return);
        assert_eq!(tokens.peek_kind(4), TokenKind::Number(NumberKind::Decimal));
        assert_eq!(tokens.peek_kind(5), TokenKind::Newline);
        assert_eq!(tokens.peek_kind(6), TokenKind::Eof);
    }

    #[test]
    fn test_position_tracking() {
        let source = "foo bar\nbaz";
        let tokens = lex(source).unwrap();

        let mut token = tokens.get(0);
        assert_eq!(token.line, 1);
        assert_eq!(token.column, 1);

        token = tokens.get(1);
        assert_eq!(token.line, 1);
        assert_eq!(token.column, 5);

        token = tokens.get(2);
        assert_eq!(token.line, 1); // newline

        token = tokens.get(3);
        assert_eq!(token.line, 2);
        assert_eq!(token.column, 1);
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
        assert!(tokens.list.iter().any(|t| t.kind == TokenKind::Type));
        assert!(tokens.list.iter().any(|t| t.kind == TokenKind::LBrace));
        assert!(tokens.list.iter().any(|t| t.kind == TokenKind::RBrace));
        assert!(tokens.list.iter().any(|t| t.kind == TokenKind::Colon));
    }

    #[test]
    fn test_empty_source() {
        let tokens = lex("").unwrap();
        assert_eq!(tokens.list.len(), 1);
        assert_eq!(tokens.peek_kind(0), TokenKind::Eof);
    }

    #[test]
    fn test_only_whitespace() {
        let tokens = lex("   \t  \n  ").unwrap();
        assert_eq!(tokens.peek_kind(0), TokenKind::Newline);
        assert_eq!(tokens.peek_kind(1), TokenKind::Eof);
    }
}
