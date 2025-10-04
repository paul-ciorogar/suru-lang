#ifndef LEXER_H
#define LEXER_H

#include "arena.h"
#include "string_storage.h"

typedef enum {
    TOKEN_EOF,
    TOKEN_NEWLINE,
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

#endif
