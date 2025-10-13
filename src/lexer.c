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

static Token new_token_from_val(TokenType type, Lexer *lexer, int value) {
    Token token;
    token.type = type;
    token.line = lexer->line;
    token.column = lexer->column;

    // Convert the integer value to a string and store it
    char buffer[32];
    snprintf(buffer, sizeof(buffer), "%d", value);
    token.text = store_cstring(lexer->strings, buffer);

    return token;
}

static int is_identifier_start(char c) { return isalpha(c) || c == '_'; }

static int is_identifier_char(char c) { return isalnum(c) || c == '_'; }

static int is_digit(char c) { return c >= '0' && c <= '9'; }

static void read_type_suffix(Lexer *lexer) {
    // Check for type suffixes
    // Integer suffixes: i8, i16, i32, i64, i128, u8, u16, u32, u64, u128
    // Float suffixes: f16, f32, f64, f128
    char c = current_char(lexer);
    if (c == 'i' || c == 'u' || c == 'f') {
        char next1 = peek_char(lexer, 1);
        char next2 = peek_char(lexer, 2);
        char next3 = peek_char(lexer, 3);

        if (c == 'i' || c == 'u') {
            // Integer suffixes: i8, i16, i32, i64, i128, u8, u16, u32, u64,
            // u128
            if (next1 == '8' && !is_identifier_char(next2)) {
                advance_lexer(lexer); // consume 'i' or 'u'
                advance_lexer(lexer); // consume '8'
            } else if (next1 == '1' && next2 == '6' &&
                       !is_identifier_char(next3)) {
                advance_lexer(lexer); // consume 'i' or 'u'
                advance_lexer(lexer); // consume '1'
                advance_lexer(lexer); // consume '6'
            } else if (next1 == '3' && next2 == '2' &&
                       !is_identifier_char(next3)) {
                advance_lexer(lexer); // consume 'i' or 'u'
                advance_lexer(lexer); // consume '3'
                advance_lexer(lexer); // consume '2'
            } else if (next1 == '6' && next2 == '4' &&
                       !is_identifier_char(next3)) {
                advance_lexer(lexer); // consume 'i' or 'u'
                advance_lexer(lexer); // consume '6'
                advance_lexer(lexer); // consume '4'
            } else if (next1 == '1' && next2 == '2' && next3 == '8') {
                char next4 = peek_char(lexer, 4);
                if (!is_identifier_char(next4)) {
                    advance_lexer(lexer); // consume 'i' or 'u'
                    advance_lexer(lexer); // consume '1'
                    advance_lexer(lexer); // consume '2'
                    advance_lexer(lexer); // consume '8'
                }
            }
        } else if (c == 'f') {
            // Float suffixes: f16, f32, f64, f128
            if (next1 == '1' && next2 == '6' && !is_identifier_char(next3)) {
                advance_lexer(lexer); // consume 'f'
                advance_lexer(lexer); // consume '1'
                advance_lexer(lexer); // consume '6'
            } else if (next1 == '3' && next2 == '2' &&
                       !is_identifier_char(next3)) {
                advance_lexer(lexer); // consume 'f'
                advance_lexer(lexer); // consume '3'
                advance_lexer(lexer); // consume '2'
            } else if (next1 == '6' && next2 == '4' &&
                       !is_identifier_char(next3)) {
                advance_lexer(lexer); // consume 'f'
                advance_lexer(lexer); // consume '6'
                advance_lexer(lexer); // consume '4'
            } else if (next1 == '1' && next2 == '2' && next3 == '8') {
                char next4 = peek_char(lexer, 4);
                if (!is_identifier_char(next4)) {
                    advance_lexer(lexer); // consume 'f'
                    advance_lexer(lexer); // consume '1'
                    advance_lexer(lexer); // consume '2'
                    advance_lexer(lexer); // consume '8'
                }
            }
        }
    }
}

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
        advance_lexer(lexer); // skip '0'
        advance_lexer(lexer); // skip 'b'
        while (current_char(lexer) == '0' || current_char(lexer) == '1' ||
               current_char(lexer) == '_') {
            advance_lexer(lexer);
        }
        read_type_suffix(lexer);
        return new_token_from_text(TOKEN_NUMBER_BINARY, lexer, start);
    }

    // Read octal
    if (current_char(lexer) == '0' && peek_char(lexer, 1) == 'o') {
        advance_lexer(lexer); // skip '0'
        advance_lexer(lexer); // skip 'o'
        while ((current_char(lexer) >= '0' && current_char(lexer) <= '7') ||
               current_char(lexer) == '_') {
            advance_lexer(lexer);
        }
        read_type_suffix(lexer);
        return new_token_from_text(TOKEN_NUMBER_OCTAL, lexer, start);
    }

    // Read hexadecimal
    if (current_char(lexer) == '0' && peek_char(lexer, 1) == 'x') {
        advance_lexer(lexer); // skip '0'
        advance_lexer(lexer); // skip 'x'
        while ((current_char(lexer) >= '0' && current_char(lexer) <= '9') ||
               (current_char(lexer) >= 'a' && current_char(lexer) <= 'f') ||
               (current_char(lexer) >= 'A' && current_char(lexer) <= 'F') ||
               current_char(lexer) == '_') {
            advance_lexer(lexer);
        }
        read_type_suffix(lexer);
        return new_token_from_text(TOKEN_NUMBER_HEX, lexer, start);
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
        read_type_suffix(lexer);
        return new_token_from_text(TOKEN_NUMBER_FLOAT, lexer, start);
    }

    read_type_suffix(lexer);
    return new_token_from_text(TOKEN_NUMBER, lexer, start);
}

static Token read_comment(Lexer *lexer) {
    size_t start = lexer->position;
    advance_lexer(lexer); // skip first '/'
    advance_lexer(lexer); // skip second '/'

    char c = current_char(lexer);

    while (c != '\n' && c != '\0') {
        advance_lexer(lexer);
        c = current_char(lexer);
    }

    return new_token_from_text(TOKEN_COMMENT, lexer, start);
}

static Token read_string(Lexer *lexer, char quote) {
    size_t start = lexer->position;
    advance_lexer(lexer); // skip opening quote

    char c = current_char(lexer);

    while (c != quote && c != '\0') {
        if (current_char(lexer) == '\\') {
            advance_lexer(lexer); // skip backslash
            advance_lexer(lexer); // skip escaped char
        } else {
            advance_lexer(lexer);
        }
        c = current_char(lexer);
    }

    if (current_char(lexer) == quote) {
        advance_lexer(lexer); // skip closing quote
    }

    return new_token_from_text(TOKEN_STRING, lexer, start);
}

// Count consecutive backticks
static int count_backticks(Lexer *lexer) {
    int count = 0;
    size_t pos = 0;
    while (peek_char(lexer, pos) == '`') {
        count++;
        pos++;
    }
    return count;
}

// Count consecutive opening braces
static int count_open_braces(Lexer *lexer) {
    int count = 0;
    size_t pos = 0;
    while (peek_char(lexer, pos) == '{') {
        count++;
        pos++;
    }
    return count;
}

// Count consecutive closing braces
static int count_close_braces(Lexer *lexer) {
    int count = 0;
    size_t pos = 0;
    while (peek_char(lexer, pos) == '}') {
        count++;
        pos++;
    }
    return count;
}

// Start of string interpolation - returns TOKEN_STRING_I_START
static Token read_string_interpolation_start(Lexer *lexer) {
    int backtick_count = count_backticks(lexer);

    // Skip the opening backticks
    for (int i = 0; i < backtick_count; i++) {
        advance_lexer(lexer);
    }

    // Check if this is a multiline string (backticks followed by newline)
    if (current_char(lexer) == '\n') {
        lexer->is_multiline_string = 1;
        advance_lexer(lexer); // skip the newline
    } else {
        lexer->is_multiline_string = 0;
    }

    // Set the state
    lexer->in_string_interpolation = backtick_count;

    // Return token with just the backtick count
    return new_token_from_val(TOKEN_STRING_I_START, lexer, backtick_count);
}

// Read string content until we hit opening braces or closing backticks
static Token read_string_interpolation_content(Lexer *lexer) {
    size_t start = lexer->position;
    int backtick_count = lexer->in_string_interpolation;

    // If we're at the start of a line in multiline mode, check for closing backticks with indentation
    if (lexer->is_multiline_string && lexer->column == 1) {
        // Look ahead to find the closing backticks
        size_t temp_pos = 0;
        while (lexer->source[lexer->position + temp_pos] == ' ' ||
               lexer->source[lexer->position + temp_pos] == '\t') {
            temp_pos++;
        }

        // Check if after the whitespace we have the closing backticks
        int has_closing = 1;
        for (int i = 0; i < backtick_count; i++) {
            if (lexer->source[lexer->position + temp_pos + i] != '`') {
                has_closing = 0;
                break;
            }
        }

        if (has_closing && temp_pos > 0) {
            // Emit the indentation token (whitespace before backticks)
            while (current_char(lexer) == ' ' || current_char(lexer) == '\t') {
                advance_lexer(lexer);
            }
            return new_token_from_text(TOKEN_STRING_I_INDENT, lexer, start);
        } else if (has_closing && temp_pos == 0) {
            // No indentation, just emit the end token
            for (int i = 0; i < backtick_count; i++) {
                advance_lexer(lexer);
            }
            lexer->in_string_interpolation = 0;
            lexer->is_multiline_string = 0;
            return new_token_from_val(TOKEN_STRING_I_END, lexer, backtick_count);
        }
    }

    // Check for closing backticks
    int closing_backticks = count_backticks(lexer);
    if (closing_backticks >= backtick_count) {
        // End of string - emit TOKEN_STRING_I_END with the count
        for (int i = 0; i < backtick_count; i++) {
            advance_lexer(lexer);
        }
        lexer->in_string_interpolation = 0;
        lexer->is_multiline_string = 0;
        return new_token_from_val(TOKEN_STRING_I_END, lexer, backtick_count);
    }

    // Read string content until we hit opening braces or end
    while (current_char(lexer) != '\0') {
        char c = current_char(lexer);

        // Check for closing backticks
        int backtick_check = count_backticks(lexer);
        if (backtick_check >= backtick_count) {
            // Found closing backticks
            if (lexer->position > start) {
                // Return the string content before the backticks
                return new_token_from_text(TOKEN_STRING_I, lexer, start);
            }
            // The closing backticks will be handled in the next call
            break;
        }

        // Check for opening braces (start of expression)
        int brace_count = count_open_braces(lexer);
        if (brace_count >= backtick_count) {
            // Found the required number of opening braces
            if (lexer->position > start) {
                // Return the string content before the braces
                return new_token_from_text(TOKEN_STRING_I, lexer, start);
            }
            // The braces will be handled by normal tokenization
            return read_string_interpolation_content(lexer);
        }

        // Handle escape sequences
        if (c == '\\') {
            advance_lexer(lexer); // skip backslash
            if (current_char(lexer) != '\0') {
                advance_lexer(lexer); // skip escaped char
            }
        } else if (c == '\n' && lexer->is_multiline_string) {
            // Look ahead to see if next line has closing backticks
            size_t look_pos = 1; // Start after the newline
            while (lexer->source[lexer->position + look_pos] == ' ' ||
                   lexer->source[lexer->position + look_pos] == '\t') {
                look_pos++;
            }
            // Check if we have closing backticks
            int has_closing = 1;
            for (int i = 0; i < backtick_count; i++) {
                if (lexer->source[lexer->position + look_pos + i] != '`') {
                    has_closing = 0;
                    break;
                }
            }
            if (has_closing && lexer->position > start) {
                // Return content up to (but not including) the newline
                Token token = new_token_from_text(TOKEN_STRING_I, lexer, start);
                // Now advance past the newline so next call starts at column 1
                advance_lexer(lexer);
                return token;
            }
            // Not the closing line, include the newline and continue
            advance_lexer(lexer);
        } else if (c == '\n' && !lexer->is_multiline_string) {
            // Single-line string shouldn't have newlines (unless escaped)
            break;
        } else {
            advance_lexer(lexer);
        }
    }

    // Reached end of file or line without proper closing
    if (lexer->position > start) {
        return new_token_from_text(TOKEN_STRING_I, lexer, start);
    }

    return new_token(TOKEN_UNKNOWN, lexer);
}

static int is_end_of_doc(Lexer *lexer) {
    int end_of_doc = current_char(lexer) == '\n' &&
                     peek_char(lexer, 1) == '=' && peek_char(lexer, 2) == '=' &&
                     peek_char(lexer, 3) == '=';

    return end_of_doc ||
           current_char(lexer) == '\n' && peek_char(lexer, 1) == '\r' &&
               peek_char(lexer, 2) == '=' && peek_char(lexer, 3) == '=' &&
               peek_char(lexer, 4) == '=';
}

static Token read_doc(Lexer *lexer) {
    size_t start = lexer->position;

    // advance to the end of the line
    while (current_char(lexer) != '\n') {
        advance_lexer(lexer);
    }

    while (!is_end_of_doc(lexer)) {
        advance_lexer(lexer);
    }

    advance_lexer(lexer); // skip the enter

    // advance to the end of the line
    while (current_char(lexer) != '\n') {
        advance_lexer(lexer);
    }

    return new_token_from_text(TOKEN_DOCUMENTATION, lexer, start);
}

Token next_token(Lexer *lexer) {
    // If we're inside a string interpolation, handle it specially
    if (lexer->in_string_interpolation > 0) {
        // If we're inside an expression (brace_depth > 0), use normal tokenization
        if (lexer->brace_depth > 0) {
            // Normal tokenization, but track braces
            skip_whitespace(lexer);

            if (lexer->position >= lexer->length) {
                return new_token(TOKEN_EOF, lexer);
            }

            char c = current_char(lexer);

            // Check for closing braces - need to match the opening count
            int close_brace_count = count_close_braces(lexer);
            if (close_brace_count >= lexer->in_string_interpolation && lexer->brace_depth == 1) {
                // This closes the expression - consume all required braces
                for (int i = 0; i < lexer->in_string_interpolation; i++) {
                    advance_lexer(lexer);
                }
                lexer->brace_depth = 0;
                return new_token(TOKEN_STRING_I_EXPR_END, lexer);
            }

            // Check for single closing brace (nested)
            if (c == '}') {
                lexer->brace_depth--;
                Token token = new_token(TOKEN_RBRACE, lexer);
                advance_lexer(lexer);
                return token;
            }

            // Check for opening brace (nested)
            if (c == '{') {
                lexer->brace_depth++;
                Token token = new_token(TOKEN_LBRACE, lexer);
                advance_lexer(lexer);
                return token;
            }

            // Fall through to normal tokenization below
        } else {
            // We're in string content, not inside an expression
            // Check for opening braces (start of expression)
            int open_brace_count = count_open_braces(lexer);
            if (open_brace_count >= lexer->in_string_interpolation) {
                // Consume the required number of braces
                for (int i = 0; i < lexer->in_string_interpolation; i++) {
                    advance_lexer(lexer);
                }
                // Set brace depth to 1 (we're now inside one level of expression)
                // The parser will need to match this with exactly one closing brace
                lexer->brace_depth = 1;
                return new_token(TOKEN_STRING_I_EXPR_START, lexer);
            }

            // Check for end of string (closing backticks)
            int backtick_count = count_backticks(lexer);
            if (backtick_count >= lexer->in_string_interpolation) {
                // This is the end
                return read_string_interpolation_content(lexer);
            }

            // Otherwise, it's string content
            return read_string_interpolation_content(lexer);
        }
    }

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

    // Line comment
    if (c == '/' && peek_char(lexer, 1) == '/') {
        return read_comment(lexer);
    }

    // Underscore
    if (c == '_' && !isalpha(peek_char(lexer, 1))) {
        Token token = new_token(TOKEN_UNDERSCORE, lexer);
        advance_lexer(lexer);
        return token;
    }

    // Identifier or keyword
    if (is_identifier_start(c)) {
        return read_identifier_or_keyword(lexer);
    }

    // Numbers
    if (is_digit(c)) {
        return read_number(lexer);
    }

    // Strings
    if (c == '"' || c == '\'') {
        return read_string(lexer, c);
    }

    // Interpolated strings
    if (c == '`') {
        return read_string_interpolation_start(lexer);
    }

    if (c == '=' && peek_char(lexer, 1) == '=' && peek_char(lexer, 2) == '=' &&
        peek_char(lexer, 3) == '=') {
        return read_doc(lexer);
    }

    // Single character tokens
    Token token;
    switch (c) {
    case ':':
        token = new_token(TOKEN_COLON, lexer);
        break;
    case ';':
        token = new_token(TOKEN_SEMICOLON, lexer);
        break;
    case ',':
        token = new_token(TOKEN_COMMA, lexer);
        break;
    case '.':
        token = new_token(TOKEN_DOT, lexer);
        break;
    case '|':
        token = new_token(TOKEN_PIPE, lexer);
        break;
    case '*':
        token = new_token(TOKEN_STAR, lexer);
        break;
    case '(':
        token = new_token(TOKEN_LPAREN, lexer);
        break;
    case ')':
        token = new_token(TOKEN_RPAREN, lexer);
        break;
    case '{':
        token = new_token(TOKEN_LBRACE, lexer);
        break;
    case '}':
        token = new_token(TOKEN_RBRACE, lexer);
        break;
    case '[':
        token = new_token(TOKEN_LBRACKET, lexer);
        break;
    case ']':
        token = new_token(TOKEN_RBRACKET, lexer);
        break;
    case '<':
        token = new_token(TOKEN_LANGLE, lexer);
        break;
    case '>':
        token = new_token(TOKEN_RANGLE, lexer);
        break;
    case '+':
        token = new_token(TOKEN_PLUS, lexer);
        break;
    case '-':
        token = new_token(TOKEN_MINUS, lexer);
        break;
    }
    advance_lexer(lexer);
    return token;

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
    lexer->in_string_interpolation = 0;
    lexer->is_multiline_string = 0;
    lexer->brace_depth = 0;
    lexer->current_token = next_token(lexer);

    return lexer;
}

void reset(Lexer *lexer) {
    lexer->position = 0;
    lexer->line = 1;
    lexer->column = 1;
    lexer->in_string_interpolation = 0;
    lexer->is_multiline_string = 0;
    lexer->brace_depth = 0;
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
    case TOKEN_NUMBER_HEX:
        return "TOKEN_NUMBER_HEX";
    case TOKEN_NUMBER_FLOAT:
        return "TOKEN_NUMBER_FLOAT";
    case TOKEN_NUMBER:
        return "TOKEN_NUMBER";
    case TOKEN_UNKNOWN:
        return "TOKEN_UNKNOWN";
    case TOKEN_COLON:
        return "TOKEN_COLON";
    case TOKEN_SEMICOLON:
        return "TOKEN_SEMICOLON";
    case TOKEN_COMMA:
        return "TOKEN_COMMA";
    case TOKEN_DOT:
        return "TOKEN_DOT";
    case TOKEN_PIPE:
        return "TOKEN_PIPE";
    case TOKEN_UNDERSCORE:
        return "TOKEN_UNDERSCORE";
    case TOKEN_STAR:
        return "TOKEN_STAR";
    case TOKEN_LPAREN:
        return "TOKEN_LPAREN";
    case TOKEN_RPAREN:
        return "TOKEN_RPAREN";
    case TOKEN_LBRACE:
        return "TOKEN_LBRACE";
    case TOKEN_RBRACE:
        return "TOKEN_RBRACE";
    case TOKEN_LBRACKET:
        return "TOKEN_LBRACKET";
    case TOKEN_RBRACKET:
        return "TOKEN_RBRACKET";
    case TOKEN_LANGLE:
        return "TOKEN_LANGLE";
    case TOKEN_RANGLE:
        return "TOKEN_RANGLE";
    case TOKEN_PLUS:
        return "TOKEN_PLUS";
    case TOKEN_MINUS:
        return "TOKEN_MINUS";
    case TOKEN_COMMENT:
        return "TOKEN_COMMENT";
    case TOKEN_STRING:
        return "TOKEN_STRING";
    case TOKEN_STRING_I_START:
        return "TOKEN_STRING_I_START";
    case TOKEN_STRING_I_END:
        return "TOKEN_STRING_I_END";
    case TOKEN_STRING_I:
        return "TOKEN_STRING_I";
    case TOKEN_STRING_I_INDENT:
        return "TOKEN_STRING_I_INDENT";
    case TOKEN_STRING_I_EXPR_START:
        return "TOKEN_STRING_I_EXPR_START";
    case TOKEN_STRING_I_EXPR_END:
        return "TOKEN_STRING_I_EXPR_END";
    case TOKEN_DOCUMENTATION:
        return "TOKEN_DOCUMENTATION";
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
