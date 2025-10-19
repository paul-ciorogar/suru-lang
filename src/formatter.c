#include "formatter.h"
#include "arena.h"
#include "array.h"
#include "parse_tree.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

// ===== Stack Operations =====

static void push_frame(Formatter *formatter, FormatterState state, int node_idx, int child_idx) {
    FormatterStackFrame frame;
    frame.state = state;
    frame.node_idx = node_idx;
    frame.child_idx = child_idx;
    frame.text = NULL;
    array_append(formatter->stack, &frame);
}

static void push_literal(Formatter *formatter, const char *text) {
    FormatterStackFrame frame;
    frame.state = FORMAT_LITERAL_TEXT;
    frame.node_idx = -1;
    frame.child_idx = -1;
    frame.text = text;
    array_append(formatter->stack, &frame);
}

static void push_indent_dec(Formatter *formatter) {
    FormatterStackFrame frame;
    frame.state = FORMAT_INDENT_DEC;
    frame.node_idx = -1;
    frame.child_idx = -1;
    frame.text = NULL;
    array_append(formatter->stack, &frame);
}

static int pop_frame(Formatter *formatter, FormatterStackFrame *out_frame) {
    if (array_length(formatter->stack) == 0) {
        return 0;
    }
    return array_pop(formatter->stack, out_frame);
}

// ===== Page Management =====

static void flush_page(Formatter *formatter) {
    if (formatter->page_used == 0) {
        return;
    }

    FILE *out = formatter->output_file ? formatter->output_file : stdout;
    fwrite(formatter->page, 1, formatter->page_used, out);
    formatter->page_used = 0;
}

static void append_char(Formatter *formatter, char c) {
    // Flush if page is full
    if (formatter->page_used >= PAGE_SIZE) {
        flush_page(formatter);
    }

    formatter->page[formatter->page_used++] = c;

    // Update formatting state
    if (c == '\n') {
        formatter->current_column = 0;
        formatter->at_line_start = true;
    } else {
        formatter->current_column++;
        formatter->at_line_start = false;
    }
}

static void append_string(Formatter *formatter, const char *str) {
    if (!str)
        return;

    while (*str) {
        append_char(formatter, *str);
        str++;
    }
}

// ===== Helper Functions =====

static void format_newline(Formatter *formatter) {
    append_char(formatter, '\n');
}

static void format_indentation(Formatter *formatter) {
    if (!formatter->at_line_start) {
        return;
    }

    for (int i = 0; i < formatter->current_indent; i++) {
        append_char(formatter, '\t');
    }

    formatter->at_line_start = false;
}

static bool last_char_is(Formatter *formatter, char c) {
    if (formatter->page_used == 0) {
        return false;
    }
    return formatter->page[formatter->page_used - 1] == c;
}

// Get string representation of a token type (for tokens without text)
static const char *token_type_to_string(TokenType type) {
    switch (type) {
    case TOKEN_MODULE:
        return "module";
    case TOKEN_IMPORT:
        return "import";
    case TOKEN_EXPORT:
        return "export";
    case TOKEN_RETURN:
        return "return";
    case TOKEN_MATCH:
        return "match";
    case TOKEN_TYPE:
        return "type";
    case TOKEN_TRY:
        return "try";
    case TOKEN_AND:
        return "and";
    case TOKEN_OR:
        return "or";
    case TOKEN_TRUE:
        return "true";
    case TOKEN_FALSE:
        return "false";
    case TOKEN_THIS:
        return "this";
    case TOKEN_PARTIAL:
        return "partial";
    case TOKEN_COLON:
        return ":";
    case TOKEN_SEMICOLON:
        return ";";
    case TOKEN_COMMA:
        return ",";
    case TOKEN_DOT:
        return ".";
    case TOKEN_PIPE:
        return "|";
    case TOKEN_UNDERSCORE:
        return "_";
    case TOKEN_STAR:
        return "*";
    case TOKEN_LPAREN:
        return "(";
    case TOKEN_RPAREN:
        return ")";
    case TOKEN_LBRACE:
        return "{";
    case TOKEN_RBRACE:
        return "}";
    case TOKEN_LBRACKET:
        return "[";
    case TOKEN_RBRACKET:
        return "]";
    case TOKEN_LANGLE:
        return "<";
    case TOKEN_RANGLE:
        return ">";
    case TOKEN_PLUS:
        return "+";
    case TOKEN_MINUS:
        return "-";
    default:
        return "";
    }
}

// ===== Terminal Node Formatting =====

static void format_terminal_node(Formatter *formatter, ParseNode *node) {
    if (!node) {
        return;
    }

    // Add leading newlines
    for (int i = 0; i < node->leading_newlines; i++) {
        format_newline(formatter);
    }

    // Add indentation if at line start (but not for the very first token)
    if (formatter->at_line_start && formatter->page_used > 0 && formatter->current_indent > 0) {
        format_indentation(formatter);
    }

    TokenType tt = node->token.type;

    // Get token text (either from stored text or from token type)
    const char *token_text = node->token.text ? node->token.text->data : token_type_to_string(tt);

    if (!token_text || token_text[0] == '\0') {
        return; // Skip empty tokens
    }

    // Append the token text
    append_string(formatter, token_text);
}

// ===== Main Formatting Loop =====

static void format_tree(Formatter *formatter) {
    if (!formatter || !formatter->tree) {
        return;
    }

    // Start formatting from the root
    if (formatter->tree->root >= 0) {
        push_frame(formatter, FORMAT_NODE, formatter->tree->root, -1);
    }

    // Main formatting loop - iterative stack-based formatting
    while (array_length(formatter->stack) > 0) {
        FormatterStackFrame frame;
        if (!pop_frame(formatter, &frame)) {
            break;
        }

        switch (frame.state) {

        case FORMAT_NODE: {
            ParseNode *node = get_node(formatter->tree, frame.node_idx);
            if (!node) {
                continue;
            }
            // Decide how to format this node based on its type
            switch (node->type) {
            // Terminal nodes
            case NODE_IDENTIFIER:
            case NODE_STRING_LITERAL:
                push_frame(formatter, FORMAT_TERMINAL, frame.node_idx, -1);
                break;
            case NODE_COMMENT:
                push_frame(formatter, FORMAT_COMMENT, frame.node_idx, -1);
                break;

            case NODE_NEWLINE:
                format_newline(formatter);
                break;

            // Non-terminal nodes that need special handling
            case NODE_FUNCTION_DECL: {
                // Format: identifier : param_list block
                // Get children
                ParseNode *first_child = get_node(formatter->tree, node->first_child);
                if (!first_child)
                    break;

                ParseNode *second_child = get_node(formatter->tree, first_child->next_sibling);
                ParseNode *third_child = second_child ? get_node(formatter->tree, second_child->next_sibling) : NULL;

                // Push in reverse order (LIFO stack):
                // 1. Newline at end
                push_literal(formatter, "\n");

                // 2. Block (third child)
                if (third_child) {
                    push_frame(formatter, FORMAT_NODE, second_child->next_sibling, -1);
                }

                // 3. Param list (second child)
                if (second_child) {
                    push_frame(formatter, FORMAT_NODE, first_child->next_sibling, -1);
                }

                // 4. Colon and space
                push_literal(formatter, ": ");

                // 5. Identifier (first child)
                push_frame(formatter, FORMAT_NODE, node->first_child, -1);
                break;
            }

            case NODE_BLOCK: {
                // Format: { statements }
                // Push in reverse order:
                // 1. Closing brace
                push_literal(formatter, "}");

                // 2. Decrease indent (after children are done)
                push_indent_dec(formatter);

                // 3. Children (with increased indent)
                push_frame(formatter, FORMAT_CHILDREN, frame.node_idx, -1);

                // 4. Increase indent (before children)
                formatter->current_indent++;

                // 5. Opening brace
                push_literal(formatter, " {");
                break;
            }

            case NODE_PARAM_LIST: {
                // Format: ( params )
                // Push in reverse order:
                push_literal(formatter, ")");
                push_frame(formatter, FORMAT_CHILDREN, frame.node_idx, -1);
                push_literal(formatter, "(");
                break;
            }

            case NODE_CALL_EXPR: {
                // Format: expression ( args )
                ParseNode *first_child = get_node(formatter->tree, node->first_child);
                if (!first_child)
                    break;

                // Push in reverse order:
                // 1. Closing paren
                push_literal(formatter, ")");

                // 2. Args (second child)
                if (first_child->next_sibling != -1) {
                    push_frame(formatter, FORMAT_NODE, first_child->next_sibling, -1);
                }

                // 3. Opening paren
                push_literal(formatter, "(");

                // 4. Expression (first child)
                push_frame(formatter, FORMAT_NODE, node->first_child, -1);
                break;
            }

            // Non-terminal nodes that just format their children
            case NODE_PROGRAM:
            case NODE_ARG_LIST:
            case NODE_PARAM:
                push_frame(formatter, FORMAT_CHILDREN, frame.node_idx, -1);
                break;

            default:
                // Unknown node type - just format children
                push_frame(formatter, FORMAT_CHILDREN, frame.node_idx, -1);
                break;
            }
            break;
        }

        case FORMAT_CHILDREN: {
            ParseNode *node = get_node(formatter->tree, frame.node_idx);
            if (!node) {
                break;
            }

            // Format all children of this node
            if (frame.child_idx == -1) {
                // First time - start with first child
                frame.child_idx = node->first_child;
            } else {
                // Get the next sibling
                ParseNode *child = get_node(formatter->tree, frame.child_idx);
                if (child) {
                    frame.child_idx = child->next_sibling;
                } else {
                    frame.child_idx = -1;
                }
            }

            // Check if we have more children to format
            if (frame.child_idx != -1) {
                // Push continuation to come back for next child
                push_frame(formatter, FORMAT_CHILDREN, frame.node_idx, frame.child_idx);
                // Push formatting for current child
                push_frame(formatter, FORMAT_NODE, frame.child_idx, -1);
            }
            break;
        }

        case FORMAT_TERMINAL: {
            ParseNode *node = get_node(formatter->tree, frame.node_idx);
            if (!node) {
                break;
            }
            format_terminal_node(formatter, node);
            break;
        }

        case FORMAT_COMMENT: {
            if (!formatter->at_line_start) {
                append_char(formatter, ' ');
            }
            ParseNode *node = get_node(formatter->tree, frame.node_idx);
            if (!node) {
                break;
            }
            format_terminal_node(formatter, node);
            break;
        }

        case FORMAT_LITERAL_TEXT: {
            if (frame.text) {
                append_string(formatter, frame.text);
            }
            break;
        }

        case FORMAT_INDENT_DEC: {
            formatter->current_indent--;
            break;
        }
        }
    }

    // Flush any remaining data
    flush_page(formatter);
}

// ===== Public API =====

Formatter *create_formatter(Arena *arena, ParseTree *tree, FILE *output_file) {
    if (!tree) {
        return NULL;
    }

    Formatter *formatter = arena_alloc(arena, sizeof(Formatter));
    if (!formatter) {
        return NULL;
    }

    formatter->arena = arena;
    formatter->tree = tree;
    formatter->stack = array_init(sizeof(FormatterStackFrame));

    // Initialize the page buffer
    memset(formatter->page, 0, PAGE_SIZE);
    formatter->page_used = 0;

    formatter->output_file = output_file;
    formatter->current_indent = 0;
    formatter->current_column = 0;
    formatter->at_line_start = true;

    if (!formatter->stack) {
        return NULL;
    }

    return formatter;
}

void format_to_stdout(Arena *arena, ParseTree *tree) {
    Formatter *formatter = create_formatter(arena, tree, NULL);
    if (!formatter) {
        return;
    }

    format_tree(formatter);

    // Stack cleanup handled by arena
    array_free(formatter->stack);
}

void format_to_file(Arena *arena, ParseTree *tree, FILE *file) {
    Formatter *formatter = create_formatter(arena, tree, file);
    if (!formatter) {
        return;
    }

    format_tree(formatter);

    // Stack cleanup handled by arena
    array_free(formatter->stack);
}
