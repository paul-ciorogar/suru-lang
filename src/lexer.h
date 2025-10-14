#ifndef LEXER_H
#define LEXER_H

#include "arena.h"
#include "string_storage.h"

typedef enum {
    TOKEN_EOF,
    TOKEN_NEWLINE,

    // keywords
    TOKEN_MODULE,
    TOKEN_IMPORT,
    TOKEN_EXPORT,
    TOKEN_RETURN,
    TOKEN_MATCH,
    TOKEN_TYPE,
    TOKEN_TRY,
    TOKEN_AND,
    TOKEN_OR,
    TOKEN_TRUE,
    TOKEN_FALSE,
    TOKEN_THIS,
    TOKEN_PARTIAL,

    TOKEN_IDENTIFIER,

    // numbers
    TOKEN_NUMBER_BINARY,
    TOKEN_NUMBER_OCTAL,
    TOKEN_NUMBER_HEX,
    TOKEN_NUMBER_FLOAT,
    TOKEN_NUMBER,

    // Single character tokens
    TOKEN_COLON,
    TOKEN_SEMICOLON,
    TOKEN_COMMA,
    TOKEN_DOT,
    TOKEN_PIPE,
    TOKEN_UNDERSCORE,
    TOKEN_STAR,
    TOKEN_LPAREN,
    TOKEN_RPAREN,
    TOKEN_LBRACE,
    TOKEN_RBRACE,
    TOKEN_LBRACKET,
    TOKEN_RBRACKET,
    TOKEN_LANGLE,
    TOKEN_RANGLE,
    TOKEN_PLUS,
    TOKEN_MINUS,

    TOKEN_STRING,
    TOKEN_STRING_I_START,
    TOKEN_STRING_I_END,
    TOKEN_STRING_I,
    TOKEN_STRING_I_INDENT,
    TOKEN_STRING_I_EXPR_START,
    TOKEN_STRING_I_EXPR_END,

    TOKEN_COMMENT,
    TOKEN_DOCUMENTATION,

    TOKEN_UNKNOWN,

} TokenType;

typedef struct Token {
    String *text;
    size_t length;
    int line;
    int column;
    TokenType type;
} Token;

typedef struct Lexer {
    Arena *arena;
    StringStorage *strings;
    char *source;
    size_t position;
    size_t length;
    Token current_token;
    int line;
    int column;
    int in_string_interpolation; // >0 when inside interpolated string, value = backtick count
    int is_multiline_string;     // 1 if current string is multiline
    int in_expression;           // 1 when inside an expression (after opening braces)
} Lexer;

Lexer *create_lexer(Arena *arena, StringStorage *strings, char *source, size_t length);

// Reads the source and returns the next recognized token
Token next_token(Lexer *lexer);

void print_tokens(Lexer *lexter);

#endif
