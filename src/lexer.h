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
} Lexer;

Lexer *create_lexer(Arena *arena, StringStorage *strings, char *source, size_t length);

// Reads the source and returns the next recognized token
Token next_token(Lexer *lexer);

void print_tokens(Lexer *lexter);

#endif
