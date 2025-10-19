#include "formatter.h"
#include "io.h"
#include "parse_tree.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

// Forward declarations
static void format_node(Formatter *formatter, int node_idx);
static void append_string(Formatter *formatter, const char *str);
static void append_char(Formatter *formatter, char c);
static void format_indentation(Formatter *formatter);
static void format_newline(Formatter *formatter);

// Create default formatter configuration
FormatterConfig *create_default_config() {
    FormatterConfig *config = malloc(sizeof(FormatterConfig));
    if (!config) {
        return NULL;
    }

    config->indent_size = 1;
    config->max_line_width = 100;
    config->use_tabs = true;

    return config;
}

// Create a formatter
Formatter *create_formatter(ParseTree *tree, FormatterConfig *config) {
    if (!tree || !config) {
        return NULL;
    }

    Formatter *formatter = malloc(sizeof(Formatter));
    if (!formatter) {
        return NULL;
    }

    formatter->config = config;
    formatter->tree = tree;
    formatter->output = create_buffer(4096);  // Start with 4KB
    formatter->current_indent = 0;
    formatter->current_column = 0;
    formatter->at_line_start = true;

    if (!formatter->output) {
        free(formatter);
        return NULL;
    }

    return formatter;
}

// Append a string to the output buffer
static void append_string(Formatter *formatter, const char *str) {
    if (!str) return;

    size_t len = strlen(str);
    for (size_t i = 0; i < len; i++) {
        append_char(formatter, str[i]);
    }
}

// Append a single character to the output buffer
static void append_char(Formatter *formatter, char c) {
    if (formatter->output->length >= formatter->output->capacity) {
        // Grow buffer
        size_t new_capacity = formatter->output->capacity * 2;
        char *new_data = realloc(formatter->output->data, new_capacity);
        if (new_data) {
            formatter->output->data = new_data;
            formatter->output->capacity = new_capacity;
        }
    }

    formatter->output->data[formatter->output->length++] = c;

    if (c == '\n') {
        formatter->current_column = 0;
        formatter->at_line_start = true;
    } else {
        formatter->current_column++;
        formatter->at_line_start = false;
    }
}

// Format indentation at the start of a line
static void format_indentation(Formatter *formatter) {
    if (!formatter->at_line_start) {
        return;
    }

    int total_spaces = formatter->current_indent * formatter->config->indent_size;

    if (formatter->config->use_tabs) {
        for (int i = 0; i < formatter->current_indent; i++) {
            append_char(formatter, '\t');
        }
    } else {
        for (int i = 0; i < total_spaces; i++) {
            append_char(formatter, ' ');
        }
    }

    formatter->at_line_start = false;
}

// Format a newline
static void format_newline(Formatter *formatter) {
    append_char(formatter, '\n');
}

// Get string representation of a token type (for tokens without text)
static const char *token_type_to_string(TokenType type) {
    switch (type) {
        case TOKEN_MODULE: return "module";
        case TOKEN_IMPORT: return "import";
        case TOKEN_EXPORT: return "export";
        case TOKEN_RETURN: return "return";
        case TOKEN_MATCH: return "match";
        case TOKEN_TYPE: return "type";
        case TOKEN_TRY: return "try";
        case TOKEN_AND: return "and";
        case TOKEN_OR: return "or";
        case TOKEN_TRUE: return "true";
        case TOKEN_FALSE: return "false";
        case TOKEN_THIS: return "this";
        case TOKEN_PARTIAL: return "partial";
        case TOKEN_COLON: return ":";
        case TOKEN_SEMICOLON: return ";";
        case TOKEN_COMMA: return ",";
        case TOKEN_DOT: return ".";
        case TOKEN_PIPE: return "|";
        case TOKEN_UNDERSCORE: return "_";
        case TOKEN_STAR: return "*";
        case TOKEN_LPAREN: return "(";
        case TOKEN_RPAREN: return ")";
        case TOKEN_LBRACE: return "{";
        case TOKEN_RBRACE: return "}";
        case TOKEN_LBRACKET: return "[";
        case TOKEN_RBRACKET: return "]";
        case TOKEN_LANGLE: return "<";
        case TOKEN_RANGLE: return ">";
        case TOKEN_PLUS: return "+";
        case TOKEN_MINUS: return "-";
        default: return "";
    }
}

// Check if last character in output is a specific character
static bool last_char_is(Formatter *formatter, char c) {
    if (formatter->output->length == 0) {
        return false;
    }
    return formatter->output->data[formatter->output->length - 1] == c;
}

// Format a terminal node (token)
static void format_terminal(Formatter *formatter, ParseNode *node) {
    if (!node) {
        return;
    }

    // Add leading newlines
    for (int i = 0; i < node->leading_newlines; i++) {
        format_newline(formatter);
    }

    // Add indentation if at line start (but not for the very first token)
    if (formatter->at_line_start && formatter->output->length > 0 && formatter->current_indent > 0) {
        format_indentation(formatter);
    }

    TokenType tt = node->token.type;

    // Get token text (either from stored text or from token type)
    const char *token_text = node->token.text ? node->token.text->data : token_type_to_string(tt);

    if (!token_text || token_text[0] == '\0') {
        return;  // Skip empty tokens
    }

    // Special handling for string interpolation start/end tokens
    // These tokens store the backtick count as a number, but we need to output backticks
    if (tt == TOKEN_STRING_I_START || tt == TOKEN_STRING_I_END) {
        // Convert text (like "1" or "2") to backtick count
        int backtick_count = atoi(token_text);

        // Add leading spaces
        if (node->leading_spaces > 0) {
            for (int i = 0; i < node->leading_spaces; i++) {
                append_char(formatter, ' ');
            }
        } else if (!formatter->at_line_start && tt == TOKEN_STRING_I_START) {
            append_char(formatter, ' ');
        }

        // Output the backticks
        for (int i = 0; i < backtick_count; i++) {
            append_char(formatter, '`');
        }

        // Handle trailing spaces
        if (node->trailing_spaces > 0) {
            for (int i = 0; i < node->trailing_spaces; i++) {
                append_char(formatter, ' ');
            }
        }

        return;  // Done with this token
    }

    // Special handling for string interpolation expression delimiters
    if (tt == TOKEN_STRING_I_EXPR_START) {
        append_char(formatter, '{');
        return;
    }

    if (tt == TOKEN_STRING_I_EXPR_END) {
        append_char(formatter, '}');
        return;
    }

    // Add leading spaces based on token type
    if (node->leading_spaces > 0) {
        for (int i = 0; i < node->leading_spaces; i++) {
            append_char(formatter, ' ');
        }
    } else if (!formatter->at_line_start) {
        // Determine if we need a space before this token
        bool needs_space = true;

        // No space after dot operator
        if (last_char_is(formatter, '.')) {
            needs_space = false;
        }
        // No space after opening parens/brackets or before closing parens/brackets
        else if (last_char_is(formatter, '(') || last_char_is(formatter, '[')) {
            needs_space = false;
        }
        else if (tt == TOKEN_RPAREN || tt == TOKEN_RBRACKET) {
            needs_space = false;
        }
        // Space after colon or comma
        else if (last_char_is(formatter, ':') || last_char_is(formatter, ',')) {
            needs_space = true;
        }
        // No space before dot operator
        else if (tt == TOKEN_DOT) {
            needs_space = false;
        }
        // No space before comma
        else if (tt == TOKEN_COMMA) {
            needs_space = false;
        }
        // Space before colon in type declarations (after type keyword)
        // Check if previous non-whitespace token suggests this is a type declaration
        else if (tt == TOKEN_COLON) {
            // For now, we'll need more context - skip adding space
            needs_space = false;
        }
        // No space after tab (indentation)
        else if (last_char_is(formatter, '\t')) {
            needs_space = false;
        }
        // Space after opening brace
        else if (last_char_is(formatter, '{')) {
            needs_space = true;
        }
        // Space before opening brace
        else if (tt == TOKEN_LBRACE) {
            needs_space = true;
        }
        // Space before closing brace (unless at start of line)
        else if (tt == TOKEN_RBRACE && !formatter->at_line_start) {
            needs_space = true;
        }

        if (needs_space) {
            append_char(formatter, ' ');
        }
    }

    // Append the token text
    append_string(formatter, token_text);

    // Handle trailing spaces if specified in parse tree
    if (node->trailing_spaces > 0) {
        for (int i = 0; i < node->trailing_spaces; i++) {
            append_char(formatter, ' ');
        }
    }
    // Don't add automatic spacing here - let the next token's leading logic handle it
}

// Format all children of a node
static void format_children(Formatter *formatter, int parent_idx) {
    ParseNode *parent = get_node(formatter->tree, parent_idx);
    if (!parent) {
        return;
    }

    int child_idx = parent->first_child;
    while (child_idx != -1) {
        format_node(formatter, child_idx);
        ParseNode *child = get_node(formatter->tree, child_idx);
        if (!child) break;
        child_idx = child->next_sibling;
    }
}

// Format a node based on its type
static void format_node(Formatter *formatter, int node_idx) {
    ParseNode *node = get_node(formatter->tree, node_idx);
    if (!node) {
        return;
    }

    switch (node->type) {
        // Terminal nodes - just output the token
        case NODE_IDENTIFIER:
        case NODE_STRING_LITERAL:
        // case NODE_NUMBER:
        // case NODE_STRING:
        // case NODE_KEYWORD:
        // case NODE_OPERATOR:
        // case NODE_PUNCTUATION:
            format_terminal(formatter, node);
            break;

        case NODE_COMMENT:
            // Preserve comments
            format_terminal(formatter, node);
            break;

        case NODE_NEWLINE:
            format_newline(formatter);
            break;

        // Non-terminal nodes - format children with appropriate rules
        case NODE_PROGRAM:
            format_children(formatter, node_idx);
            break;

        case NODE_FUNCTION_DECL:
            format_children(formatter, node_idx);
            format_newline(formatter);
            break;

        case NODE_BLOCK:
            formatter->current_indent++;
            format_children(formatter, node_idx);
            formatter->current_indent--;
            break;

        case NODE_CALL_EXPR:
        case NODE_ARG_LIST:
        case NODE_PARAM_LIST:
        case NODE_PARAM:
            // Module paths just format their children inline
            format_children(formatter, node_idx);
            break;

        default:
            // For unknown node types, just format children
            format_children(formatter, node_idx);
            break;
    }
}

// Format the parse tree and return formatted output
Buffer *format_parse_tree(ParseTree *tree, FormatterConfig *config) {
    if (!tree || !config) {
        return NULL;
    }

    Formatter *formatter = create_formatter(tree, config);
    if (!formatter) {
        return NULL;
    }

    // Start formatting from the root
    if (tree->root >= 0) {
        format_node(formatter, tree->root);
    }

    // Null-terminate the output
    if (formatter->output->length < formatter->output->capacity) {
        formatter->output->data[formatter->output->length] = '\0';
    }

    // Extract the output buffer before freeing formatter
    Buffer *result = formatter->output;
    formatter->output = NULL;  // Prevent double free
    free_formatter(formatter);

    return result;
}

// Free formatter resources
void free_formatter(Formatter *formatter) {
    if (!formatter) {
        return;
    }

    if (formatter->output) {
        free_buffer(formatter->output);
    }

    free(formatter);
}
