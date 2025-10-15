#include "formatter.h"
#include "io.h"
#include "parse_tree.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

    config->indent_size = 4;
    config->max_line_width = 100;
    config->use_tabs = false;

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

// Format a terminal node (token)
static void format_terminal(Formatter *formatter, ParseNode *node) {
    if (!node) {
        return;
    }

    // Add leading newlines
    for (int i = 0; i < node->leading_newlines; i++) {
        format_newline(formatter);
    }

    // Add indentation if at line start
    if (formatter->at_line_start) {
        format_indentation(formatter);
    }

    // Get token text (either from stored text or from token type)
    const char *token_text = node->token.text ? node->token.text->data : token_type_to_string(node->token.type);

    if (!token_text || token_text[0] == '\0') {
        return;  // Skip empty tokens
    }

    // Add leading spaces (or default spacing based on token type)
    if (node->leading_spaces > 0) {
        for (int i = 0; i < node->leading_spaces; i++) {
            append_char(formatter, ' ');
        }
    } else if (!formatter->at_line_start && node->type != NODE_PUNCTUATION) {
        // Add default space before most tokens (except punctuation)
        TokenType tt = node->token.type;
        if (tt != TOKEN_LPAREN && tt != TOKEN_RPAREN &&
            tt != TOKEN_LBRACE && tt != TOKEN_RBRACE &&
            tt != TOKEN_LBRACKET && tt != TOKEN_RBRACKET &&
            tt != TOKEN_COMMA && tt != TOKEN_DOT) {
            append_char(formatter, ' ');
        }
    }

    // Append the token text
    append_string(formatter, token_text);

    // Add trailing spaces
    for (int i = 0; i < node->trailing_spaces; i++) {
        append_char(formatter, ' ');
    }

    // Add default spacing after certain operators
    TokenType tt = node->token.type;
    if (tt == TOKEN_COLON || tt == TOKEN_COMMA) {
        append_char(formatter, ' ');
    }
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
        case NODE_NUMBER:
        case NODE_STRING:
        case NODE_KEYWORD:
        case NODE_OPERATOR:
        case NODE_PUNCTUATION:
            format_terminal(formatter, node);
            break;

        case NODE_COMMENT:
            // Preserve comments
            format_terminal(formatter, node);
            break;

        case NODE_DOCUMENTATION:
            // Preserve documentation blocks
            format_terminal(formatter, node);
            format_newline(formatter);
            break;

        case NODE_WHITESPACE:
            // Skip whitespace - we control formatting
            break;

        case NODE_NEWLINE:
            format_newline(formatter);
            break;

        // Non-terminal nodes - format children with appropriate rules
        case NODE_PROGRAM:
        case NODE_STATEMENT_LIST:
            format_children(formatter, node_idx);
            break;

        case NODE_VARIABLE_DECL:
        case NODE_TYPE_DECL:
        case NODE_FUNCTION_DECL:
            format_children(formatter, node_idx);
            format_newline(formatter);
            break;

        case NODE_BLOCK:
            // Increase indent for block content
            formatter->current_indent++;
            format_children(formatter, node_idx);
            formatter->current_indent--;
            break;

        case NODE_STRUCT_LITERAL:
            format_children(formatter, node_idx);
            break;

        case NODE_PIPELINE_EXPR:
        case NODE_BINARY_OP:
        case NODE_UNARY_OP:
        case NODE_FUNCTION_CALL:
        case NODE_ARGUMENT_LIST:
        case NODE_PARAMETER_LIST:
        case NODE_TYPE_ANNOTATION:
        case NODE_ARRAY_LITERAL:
        case NODE_STRING_INTERP:
            format_children(formatter, node_idx);
            break;

        case NODE_MATCH_EXPR:
            format_children(formatter, node_idx);
            break;

        case NODE_MODULE_DECL:
        case NODE_IMPORT_DECL:
        case NODE_EXPORT_DECL:
            format_children(formatter, node_idx);
            format_newline(formatter);
            format_newline(formatter);  // Extra blank line after module declarations
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
