#include "lexer.h"
#include "arena.h"
#include "string_storage.h"
#include <ctype.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static char current_char(Lexer *lexer) {
    if (lexer->position >= lexer->length)
        return '\0';
    return lexer->source[lexer->position];
}

static char peek_char(Lexer *lexer, size_t offset) {
    size_t pos = lexer->position + offset;

    if (pos >= lexer->length)
        return '\0';

    return lexer->source[pos];
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

static Token new_token(TokenType type, Lexer *lexer) {
    Token token;
    token.type = type;
    token.line = lexer->line;
    token.column = lexer->column;
    token.text = NULL;

    return token;
}

static Token new_token_from_text(TokenType type, Lexer *lexer, size_t start) {
    size_t length = lexer->position - start;

    Token token;
    token.type = type;
    token.line = lexer->line;
    token.column = lexer->column;
    token.text =
        store_from_buffer(lexer->strings, lexer->source, start, length);

    return token;
}

static int is_identifier_start(char c) { return isalpha(c) || c == '_'; }

static int is_identifier_char(char c) { return isalnum(c) || c == '_'; }

static int is_digit(char c) { return c >= '0' && c <= '9'; }

static Token read_identifier_or_keyword(Lexer *lexer) {
    size_t start = lexer->position;

    while (is_identifier_char(current_char(lexer))) {
        advance_lexer(lexer);
    }

    char *text = lexer->source + start;
    size_t length = lexer->position - start;

    // if it starts with a capital letter then we can rule out keywords
    if (isupper(text[0]) || length > 7) {
        return new_token_from_text(TOKEN_IDENTIFIER, lexer, start);
    }

    switch (length) {
    case 7: {
        if (strncmp(text, "partial", 7) == 0) {
            return new_token(TOKEN_PARTIAL, lexer);
        }
        break;
    }
    }

    // Check for keywords
    switch (length) {
    case 7:
        if (strncmp(text, "partial", 7) == 0) {
            return new_token(TOKEN_PARTIAL, lexer);
        }
        break;

    case 6:
        if (strncmp(text, "module", 6) == 0) {
            return new_token(TOKEN_MODULE, lexer);
        } else if (strncmp(text, "import", 6) == 0) {
            return new_token(TOKEN_IMPORT, lexer);
        } else if (strncmp(text, "export", 6) == 0) {
            return new_token(TOKEN_EXPORT, lexer);
        } else if (strncmp(text, "return", 6) == 0) {
            return new_token(TOKEN_RETURN, lexer);
        }
        break;

    case 5:
        if (strncmp(text, "match", 5) == 0) {
            return new_token(TOKEN_MATCH, lexer);
        } else if (strncmp(text, "false", 5) == 0) {
            return new_token(TOKEN_FALSE, lexer);
        }
        break;

    case 4:
        if (strncmp(text, "type", 4) == 0) {
            return new_token(TOKEN_TYPE, lexer);
        } else if (strncmp(text, "true", 4) == 0) {
            return new_token(TOKEN_TRUE, lexer);
        } else if (strncmp(text, "this", 4) == 0) {
            return new_token(TOKEN_THIS, lexer);
        }
        break;

    case 3:
        if (strncmp(text, "and", 3) == 0) {
            return new_token(TOKEN_AND, lexer);
        } else if (strncmp(text, "try", 3) == 0) {
            return new_token(TOKEN_TRY, lexer);
        }
        break;

    case 2:
        if (strncmp(text, "or", 2) == 0) {
            return new_token(TOKEN_OR, lexer);
        }
        break;
    }

    return new_token_from_text(TOKEN_IDENTIFIER, lexer, start);
}

static Token read_number(Lexer *lexer) {
    size_t start = lexer->position;

    // Read binary
    if (current_char(lexer) == '0' && peek_char(lexer, 1) == 'b') {
        while (current_char(lexer) == '0' || current_char(lexer) == '1') {
            advance_lexer(lexer);
        }
        return new_token_from_text(TOKEN_NUMBER_BINARY, lexer, start);
    }

    // Read octal
    if (current_char(lexer) == '0' && peek_char(lexer, 1) == 'o') {
        advance_lexer(lexer);
        while (current_char(lexer) >= '0' && current_char(lexer) <= '7') {
            advance_lexer(lexer);
        }
        return new_token_from_text(TOKEN_NUMBER_OCTAL, lexer, start);
    }

    // Read integer part with separators
    while (is_digit(current_char(lexer)) || current_char(lexer) == '_') {
        advance_lexer(lexer);
    }

    // Check for decimal point
    if (current_char(lexer) == '.' && is_digit(peek_char(lexer, 1))) {
        advance_lexer(lexer); // skip '.'
        while (is_digit(current_char(lexer))) {
            advance_lexer(lexer);
        }
        return new_token_from_text(TOKEN_NUMBER_FLOAT, lexer, start);
    }

    return new_token_from_text(TOKEN_NUMBER, lexer, start);
}

Token next_token(Lexer *lexer) {
    skip_whitespace(lexer);

    // End of file
    if (lexer->position >= lexer->length) {
        return new_token(TOKEN_EOF, lexer);
    }

    char c = current_char(lexer);

    // New line
    if (c == '\n') {
        Token token = new_token(TOKEN_NEWLINE, lexer);
        advance_lexer(lexer);
        return token;
    }

    if (is_identifier_start(c)) {
        return read_identifier_or_keyword(lexer);
    }

    if (is_digit(c)) {
        return read_number(lexer);
    }

    return new_token(TOKEN_UNKNOWN, lexer);
}

Lexer *create_lexer(Arena *arena, StringStorage *strings, char *source,
                    size_t length) {
    Lexer *lexer = arena_alloc(arena, sizeof(Lexer));
    lexer->source = source;
    lexer->strings = strings;
    lexer->position = 0;
    lexer->length = length;
    lexer->line = 1;
    lexer->column = 1;
    lexer->arena = arena;
    lexer->current_token = next_token(lexer);

    return lexer;
}

void reset(Lexer *lexer) {
    lexer->position = 0;
    lexer->line = 1;
    lexer->column = 1;
    lexer->current_token = next_token(lexer);
}

static const char *token_type_to_string(TokenType type) {
    switch (type) {
    case TOKEN_EOF:
        return "TOKEN_EOF";
    case TOKEN_NEWLINE:
        return "TOKEN_NEWLINE";
    case TOKEN_MODULE:
        return "TOKEN_MODULE";
    case TOKEN_IMPORT:
        return "TOKEN_IMPORT";
    case TOKEN_EXPORT:
        return "TOKEN_EXPORT";
    case TOKEN_RETURN:
        return "TOKEN_RETURN";
    case TOKEN_MATCH:
        return "TOKEN_MATCH";
    case TOKEN_TYPE:
        return "TOKEN_TYPE";
    case TOKEN_TRY:
        return "TOKEN_TRY";
    case TOKEN_AND:
        return "TOKEN_AND";
    case TOKEN_OR:
        return "TOKEN_OR";
    case TOKEN_TRUE:
        return "TOKEN_TRUE";
    case TOKEN_FALSE:
        return "TOKEN_FALSE";
    case TOKEN_THIS:
        return "TOKEN_THIS";
    case TOKEN_PARTIAL:
        return "TOKEN_PARTIAL";
    case TOKEN_IDENTIFIER:
        return "TOKEN_IDENTIFIER";
    case TOKEN_NUMBER_BINARY:
        return "TOKEN_NUMBER_BINARY";
    case TOKEN_NUMBER_OCTAL:
        return "TOKEN_NUMBER_OCTAL";
    case TOKEN_NUMBER_FLOAT:
        return "TOKEN_NUMBER_FLOAT";
    case TOKEN_NUMBER:
        return "TOKEN_NUMBER";
    case TOKEN_UNKNOWN:
        return "TOKEN_UNKNOWN";
    default:
        return "UNKNOWN_TOKEN_TYPE";
    }
}

void print_tokens(Lexer *lexer) {
    while (lexer->current_token.type != TOKEN_EOF) {
        printf("Token: %s", token_type_to_string(lexer->current_token.type));
        if (lexer->current_token.text != NULL) {
            printf(" Text: %s", lexer->current_token.text->data);
        }
        printf("\n");
        lexer->current_token = next_token(lexer);
    }
}
