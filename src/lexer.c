#include "lexer.h"
#include "arena.h"
#include "string_storage.h"
#include <cstring>
#include <ctype.h>
#include <stddef.h>
#include <stdio.h>
#include <unistd.h>

static char current_char(Lexer *lexer) {
    if (lexer->position >= lexer->length)
        return '\0';
    return lexer->source[lexer->position];
}

static void advance_lexer(Lexer *lexer) {
    if (lexer->position < lexer->length) {
        if (lexer->source[lexer->position] == '\n') {
            lexer->line++;
            lexer->column = 1;
        } else {
            lexer->column++;
        }
        lexer->position++;
    }
}

static void skip_whitespace(Lexer *lexer) {
    while (lexer->position < lexer->length) {
        char c = current_char(lexer);
        // Skip white space except new lines
        if (c == ' ' || c == '\t' || c == '\r') {
            advance_lexer(lexer);
        } else {
            break;
        }
    }
}

static Token create_simple_token(TokenType type, Lexer *lexer) {
    Token token;
    token.type = type;
    token.line = lexer->line;
    token.column = lexer->column;
    token.text = NULL;

    return token;
}

static int is_identifier_start(char c) { return isalpha(c) || c == '_'; }

static int is_identifier_char(char c) { return isalnum(c) || c == '_'; }

static Token read_identifier_or_keyword(Lexer *lexer) {
    Token token = {0};
    token.line = lexer->line;
    token.column = lexer->column;

    size_t start = lexer->position;

    while (is_identifier_char(current_char(lexer))) {
        advance_lexer(lexer);
    }

    char text[lexer->position - start + 1] = {'\0'};
    char *text =
        memcpy(void *__restrict dest, const void *__restrict src, size_t n)

            token.text = lexer->source + start;
    token.length = lexer->pos - start;

    // Check for keywords
    if (token.length == 6 && strncmp(token.text, "module", 6) == 0) {
        token.type = TOKEN_MODULE;
    } else if (token.length == 6 && strncmp(token.text, "import", 6) == 0) {
        token.type = TOKEN_IMPORT;
    } else if (token.length == 6 && strncmp(token.text, "export", 6) == 0) {
        token.type = TOKEN_EXPORT;
    } else if (token.length == 4 && strncmp(token.text, "type", 4) == 0) {
        token.type = TOKEN_TYPE;
    } else if (token.length == 6 && strncmp(token.text, "return", 6) == 0) {
        token.type = TOKEN_RETURN;
    } else if (token.length == 5 && strncmp(token.text, "match", 5) == 0) {
        token.type = TOKEN_MATCH;
    } else if (token.length == 3 && strncmp(token.text, "and", 3) == 0) {
        token.type = TOKEN_AND;
    } else if (token.length == 2 && strncmp(token.text, "or", 2) == 0) {
        token.type = TOKEN_OR;
    } else if (token.length == 4 && strncmp(token.text, "true", 4) == 0) {
        token.type = TOKEN_TRUE;
    } else if (token.length == 5 && strncmp(token.text, "false", 5) == 0) {
        token.type = TOKEN_FALSE;
    } else {
        token.type = TOKEN_IDENTIFIER;
    }

    return token;
}

static Token next_token(Lexer *lexer) {
    skip_whitespace(lexer);

    // End of file
    if (lexer->position >= lexer->length) {
        return create_simple_token(TOKEN_EOF, lexer);
    }

    char c = current_char(lexer);

    // New line
    if (c == '\n') {
        Token token = create_simple_token(TOKEN_NEWLINE, lexer);
        advance_lexer(lexer);
        return token;
    }

    if (is_identifier_start(c)) {
        return read_identifier_or_keyword(lexer);
    }

    if (is_digit(c)) {
        return read_number(lexer);
    }
}

Lexer *create_lexer(Arena *arena, StringStorage *strings, char *source,
                    size_t length) {
    Lexer *lexer = arena_alloc(arena, sizeof(Lexer));
    lexer->source = source;
    lexer->position = 0;
    lexer->length = length;
    lexer->line = 1;
    lexer->column = 1;
    lexer->arena = arena;
    lexer->current_token = next_token(lexer);

    return lexer;
}
