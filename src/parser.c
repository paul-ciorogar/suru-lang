#include "parser.h"
#include "arena.h"
#include "array.h"
#include "lexer.h"
#include "parse_tree.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

Parser *create_parser(Arena *arena, Lexer *lexer) {
    Parser *parser = arena_alloc(arena, sizeof(Parser));
    if (!parser) {
        return NULL;
    }

    parser->lexer = lexer;
    parser->arena = arena;

    // Initialize error list
    parser->errors = arena_alloc(arena, sizeof(ParserErrors));
    if (!parser->errors) {
        return NULL;
    }
    parser->errors->head = NULL;
    parser->errors->tail = NULL;
    parser->errors->count = 0;

    parser->tree = create_parse_tree(arena);
    parser->stack = array_init(sizeof(ParserStackFrame));

    if (!parser->stack) {
        return NULL;
    }

    return parser;
}

// ===== Helper Functions =====

// Get current token
static Token current_token(Parser *parser) {
    return parser->lexer->current_token;
}

// Advance to next token
static void advance(Parser *parser) {
    parser->lexer->current_token = next_token(parser->lexer);
}

// Check if current token matches expected type (without consuming)
static int match_token(Parser *parser, TokenType type) {
    return current_token(parser).type == type;
}

// Report a syntax error
static void report_syntax_error(Parser *parser, const char *message) {
    ParserError *error = arena_alloc(parser->arena, sizeof(ParserError));
    if (!error) {
        return;
    }

    Token token = current_token(parser);
    error->line = token.line;
    error->column = token.column;

    // Allocate and copy message
    size_t len = strlen(message);
    error->message = arena_alloc(parser->arena, len + 1);
    if (error->message) {
        strcpy(error->message, message);
    }

    error->next = NULL;

    // Add to error list
    if (parser->errors->tail) {
        parser->errors->tail->next = error;
    } else {
        parser->errors->head = error;
    }
    parser->errors->tail = error;
    parser->errors->count++;
}

static void skip_to_newline(Parser *parser) {
    while (!match_token(parser, TOKEN_EOF) &&
           !match_token(parser, TOKEN_NEWLINE)) {
        advance(parser);
    }
}

// ===== Stack Operations =====

static void push_new_frame(Parser *parser, ParserState state, int parent_idx, int current_idx) {
    ParserStackFrame frame;
    frame.state = state;
    frame.parent_node_idx = parent_idx;
    frame.current_node_idx = current_idx;
    frame.precedence = 0;
    array_append(parser->stack, &frame);
}

static int pop_frame(Parser *parser, ParserStackFrame *out_frame) {
    if (array_length(parser->stack) == 0) {
        return 0;
    }
    return array_pop(parser->stack, out_frame);
}

static ParserStackFrame *peek_frame(Parser *parser) {
    size_t len = array_length(parser->stack);
    if (len == 0) {
        return NULL;
    }
    return (ParserStackFrame *)array_get(parser->stack, len - 1);
}

// ===== Operator Precedence =====

// Precedence levels (higher = tighter binding)
#define PREC_NONE 0
#define PREC_PIPELINE 1 // |
#define PREC_OR 2       // or
#define PREC_AND 3      // and
#define PREC_COMPOSE 4  // + (composition)
#define PREC_UNARY 5    // -, not
#define PREC_MEMBER 6   // . (member access)
#define PREC_CALL 7     // ()

static int get_operator_precedence(TokenType type) {
    switch (type) {
    case TOKEN_PIPE:
        return PREC_PIPELINE;
    case TOKEN_OR:
        return PREC_OR;
    case TOKEN_AND:
        return PREC_AND;
    case TOKEN_PLUS:
        return PREC_COMPOSE;
    case TOKEN_DOT:
        return PREC_MEMBER;
    default:
        return PREC_NONE;
    }
}

static int is_binary_operator(TokenType type) {
    return type == TOKEN_PIPE || type == TOKEN_OR || type == TOKEN_AND ||
           type == TOKEN_PLUS;
}

static int is_unary_operator(TokenType type) {
    return type == TOKEN_MINUS || type == TOKEN_NOT;
}

void new_error(Parser *parser, int parent_node_idx, char *message) {
    report_syntax_error(parser, message);
    skip_to_newline(parser);
    // Return to parse with the parent node
    push_new_frame(parser, PARSE, parent_node_idx, parent_node_idx);
}

// Parse the source code and build parse tree with syntax analysis
ParseTree *parse(Parser *parser) {
    if (!parser || !parser->tree) {
        return NULL;
    }

    // Create root program node
    ParseNode root = create_nonterminal_node(NODE_PROGRAM);
    int root_idx = add_node(parser->tree, &root);
    parser->tree->root = root_idx;

    // Push initial state
    push_new_frame(parser, PARSE, root_idx, root_idx);

    // Main parsing loop - iterative stack-based parsing
    while (array_length(parser->stack) > 0) {
        ParserStackFrame frame;
        if (!pop_frame(parser, &frame)) {
            break;
        }

        switch (frame.state) {

        case PARSE: {
            // Top-level parsing - handles program-level declarations

            // Consume and preserve comments and newlines
            while (match_token(parser, TOKEN_COMMENT) || match_token(parser, TOKEN_NEWLINE)) {
                ParseNodeType node_type = match_token(parser, TOKEN_COMMENT) ? NODE_COMMENT : NODE_NEWLINE;
                ParseNode node = create_terminal_node(node_type, current_token(parser));
                int node_idx = add_node(parser->tree, &node);
                add_child(parser->tree, frame.parent_node_idx, node_idx);
                advance(parser);
            }

            // Check for EOF
            if (match_token(parser, TOKEN_EOF)) {
                break;  // Done parsing
            }

            // Look for declaration: identifier followed by colon
            if (match_token(parser, TOKEN_IDENTIFIER)) {
                // Push continuation to come back to PARSE after declaration
                push_new_frame(parser, PARSE, frame.parent_node_idx, frame.parent_node_idx);
                // Push PARSE_STATEMENT to determine type of declaration
                push_new_frame(parser, PARSE_STATEMENT, frame.parent_node_idx, -1);
                break;
            }

            // Unknown token at top level
            new_error(parser, frame.parent_node_idx, "Expected function declaration");
            break;
        }

        case PARSE_STATEMENT: {
            // Determine statement type: variable decl, function decl, or call expression
            // Parsing: identifier (: | () )

            // Expect identifier
            if (!match_token(parser, TOKEN_IDENTIFIER)) {
                new_error(parser, frame.parent_node_idx, "Expected identifier");
                break;
            }

            Token id_token = current_token(parser);
            advance(parser);

            // Check what follows the identifier
            if (match_token(parser, TOKEN_COLON)) {
                // It's a declaration (function or variable)
                advance(parser);

                // Now determine what follows the colon
                if (match_token(parser, TOKEN_LPAREN)) {
                    // It's a function declaration: identifier : (params) block
                    // Create function declaration node
                    ParseNode func_node = create_nonterminal_node(NODE_FUNCTION_DECL);
                    int func_idx = add_node(parser->tree, &func_node);
                    add_child(parser->tree, frame.parent_node_idx, func_idx);

                    // Add identifier as child
                    ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
                    int id_idx = add_node(parser->tree, &id_node);
                    add_child(parser->tree, func_idx, id_idx);

                    // Push PARSE_FUNCTION_DECL to continue parsing
                    push_new_frame(parser, PARSE_FUNCTION_DECL, func_idx, func_idx);
                    break;
                } else {
                    // It's a variable declaration: identifier : value
                    // Create variable declaration node
                    ParseNode var_node = create_nonterminal_node(NODE_VAR_DECL);
                    int var_idx = add_node(parser->tree, &var_node);
                    add_child(parser->tree, frame.parent_node_idx, var_idx);

                    // Add identifier as child
                    ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
                    int id_idx = add_node(parser->tree, &id_node);
                    add_child(parser->tree, var_idx, id_idx);

                    // Push PARSE_VAR_DECL to continue parsing
                    push_new_frame(parser, PARSE_VAR_DECL, var_idx, var_idx);
                    break;
                }
            } else if (match_token(parser, TOKEN_LPAREN)) {
                // It's a call expression: identifier(args)
                // Create call expression node
                ParseNode call_node = create_nonterminal_node(NODE_CALL_EXPR);
                int call_idx = add_node(parser->tree, &call_node);
                add_child(parser->tree, frame.parent_node_idx, call_idx);

                // Add identifier as first child of call
                ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
                int id_idx = add_node(parser->tree, &id_node);
                add_child(parser->tree, call_idx, id_idx);

                // Push PARSE_CALL_ARGS to parse the arguments
                push_new_frame(parser, PARSE_CALL_ARGS, call_idx, -1);
                break;
            } else {
                new_error(parser, frame.parent_node_idx, "Expected ':' or '(' after identifier");
                break;
            }
        }

        case PARSE_FUNCTION_DECL: {
            // Parsing: param_list block
            // Note: NODE_FUNCTION_DECL and identifier already created by PARSE_DECL

            int func_idx = frame.current_node_idx;
            if (func_idx == -1) {
                new_error(parser, frame.parent_node_idx, "Invalid function declaration state");
                break;
            }

            // Push PARSE_BLOCK to be executed after PARSE_PARAM_LIST
            push_new_frame(parser, PARSE_BLOCK, func_idx, -1);

            // Push PARSE_PARAM_LIST
            push_new_frame(parser, PARSE_PARAM_LIST, func_idx, -1);
            break;
        }

        case PARSE_VAR_DECL: {
            // Parsing: value (string literal for now)
            // Note: NODE_VAR_DECL and identifier already created by PARSE_DECL

            int var_idx = frame.current_node_idx;
            if (var_idx == -1) {
                new_error(parser, frame.parent_node_idx, "Invalid variable declaration state");
                break;
            }

            // For now, only support string literals as values
            if (match_token(parser, TOKEN_STRING)) {
                ParseNode str_node = create_terminal_node(NODE_STRING_LITERAL, current_token(parser));
                int str_idx = add_node(parser->tree, &str_node);
                add_child(parser->tree, var_idx, str_idx);
                advance(parser);
                break;
            }

            // Could also support identifiers (for variable assignment)
            if (match_token(parser, TOKEN_IDENTIFIER)) {
                ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, current_token(parser));
                int id_idx = add_node(parser->tree, &id_node);
                add_child(parser->tree, var_idx, id_idx);
                advance(parser);
                break;
            }

            new_error(parser, var_idx, "Expected value expression in variable declaration");
            break;
        }

        case PARSE_PARAM_LIST: {
            // Parsing: ( [param [, param]*] )

            // Create param list node
            ParseNode param_list_node = create_nonterminal_node(NODE_PARAM_LIST);
            int param_list_idx = add_node(parser->tree, &param_list_node);
            add_child(parser->tree, frame.parent_node_idx, param_list_idx);

            // Expect LPAREN
            if (!match_token(parser, TOKEN_LPAREN)) {
                new_error(parser, param_list_idx, "Expected '(' for parameter list");
                break;
            }
            advance(parser);

            // Consume and preserve newlines/comments inside params
            while (match_token(parser, TOKEN_COMMENT) || match_token(parser, TOKEN_NEWLINE)) {
                ParseNodeType node_type = match_token(parser, TOKEN_COMMENT) ? NODE_COMMENT : NODE_NEWLINE;
                ParseNode node = create_terminal_node(node_type, current_token(parser));
                int node_idx = add_node(parser->tree, &node);
                add_child(parser->tree, param_list_idx, node_idx);
                advance(parser);
            }

            // Check for empty param list
            if (match_token(parser, TOKEN_RPAREN)) {
                advance(parser);
                break;
            }

            // For now, we expect empty params for hello_world
            // Full param parsing would go here (identifier : type, ...)

            // Expect RPAREN
            if (!match_token(parser, TOKEN_RPAREN)) {
                new_error(parser, param_list_idx, "Expected ')' after parameter list");
                break;
            }
            advance(parser);
            break;
        }

        case PARSE_BLOCK: {
            // Parsing: { statements }

            int block_idx = frame.current_node_idx;

            // First time: create block node and consume LBRACE
            if (block_idx == -1) {
                // Create block node
                ParseNode block_node = create_nonterminal_node(NODE_BLOCK);
                block_idx = add_node(parser->tree, &block_node);
                add_child(parser->tree, frame.parent_node_idx, block_idx);

                // Expect LBRACE
                if (!match_token(parser, TOKEN_LBRACE)) {
                    new_error(parser, block_idx, "Expected '{' for block");
                    break;
                }
                advance(parser);
            }

            // Parse statements until RBRACE
            while (!match_token(parser, TOKEN_RBRACE) && !match_token(parser, TOKEN_EOF)) {
                // Consume and preserve newlines/comments
                if (match_token(parser, TOKEN_COMMENT) || match_token(parser, TOKEN_NEWLINE)) {
                    ParseNodeType node_type = match_token(parser, TOKEN_COMMENT) ? NODE_COMMENT : NODE_NEWLINE;
                    ParseNode node = create_terminal_node(node_type, current_token(parser));
                    int node_idx = add_node(parser->tree, &node);
                    add_child(parser->tree, block_idx, node_idx);
                    advance(parser);
                    continue;
                }

                // Look for statement (variable declaration or expression)
                if (match_token(parser, TOKEN_IDENTIFIER)) {
                    // Push continuation to keep parsing this block
                    push_new_frame(parser, PARSE_BLOCK, frame.parent_node_idx, block_idx);
                    // Push PARSE_STATEMENT to determine statement type
                    push_new_frame(parser, PARSE_STATEMENT, block_idx, -1);
                    // Exit - will resume after statement is parsed
                    break;
                }

                // Unknown token
                new_error(parser, block_idx, "Unexpected token in block");
                break;
            }

            // Check if we exited the loop early (pushed continuation)
            if (!match_token(parser, TOKEN_RBRACE) && !match_token(parser, TOKEN_EOF)) {
                // We pushed a continuation, will resume later
                break;
            }

            // Expect RBRACE
            if (!match_token(parser, TOKEN_RBRACE)) {
                new_error(parser, block_idx, "Expected '}' at end of block");
                break;
            }
            advance(parser);
            break;
        }

        case PARSE_EXPRESSION: {
            // Parse expressions - for hello_world, just call expressions

            // Check for call expression: identifier followed by LPAREN
            if (match_token(parser, TOKEN_IDENTIFIER)) {
                Token id_token = current_token(parser);
                advance(parser);

                // Check if this is a call
                if (match_token(parser, TOKEN_LPAREN)) {
                    // Create call expression node
                    ParseNode call_node = create_nonterminal_node(NODE_CALL_EXPR);
                    int call_idx = add_node(parser->tree, &call_node);
                    add_child(parser->tree, frame.parent_node_idx, call_idx);

                    // Add identifier as first child of call
                    ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
                    int id_idx = add_node(parser->tree, &id_node);
                    add_child(parser->tree, call_idx, id_idx);

                    // Push PARSE_CALL_ARGS to parse the arguments
                    push_new_frame(parser, PARSE_CALL_ARGS, call_idx, -1);
                    break;
                }

                // Not a call - just an identifier (shouldn't happen in hello_world)
                new_error(parser, frame.parent_node_idx, "Expected function call");
                break;
            }

            // Other expression types (literals, etc.) would go here
            new_error(parser, frame.parent_node_idx, "Expected expression");
            break;
        }

        case PARSE_CALL_ARGS: {
            // Parsing: ( [expr [, expr]*] )
            // Note: CALL_EXPR node and identifier already created by PARSE_EXPRESSION

            int arg_list_idx = frame.current_node_idx;

            // First time: create arg list node and consume LPAREN
            if (arg_list_idx == -1) {
                // Create arg list node
                ParseNode arg_list_node = create_nonterminal_node(NODE_ARG_LIST);
                arg_list_idx = add_node(parser->tree, &arg_list_node);
                add_child(parser->tree, frame.parent_node_idx, arg_list_idx);

                // Consume LPAREN
                if (!match_token(parser, TOKEN_LPAREN)) {
                    new_error(parser, arg_list_idx, "Expected '(' for argument list");
                    break;
                }
                advance(parser);
            }

            // Parse arguments until RPAREN
            while (!match_token(parser, TOKEN_RPAREN) && !match_token(parser, TOKEN_EOF)) {
                // Preserve comments/newlines inside args
                if (match_token(parser, TOKEN_COMMENT) || match_token(parser, TOKEN_NEWLINE)) {
                    ParseNodeType node_type = match_token(parser, TOKEN_COMMENT) ? NODE_COMMENT : NODE_NEWLINE;
                    ParseNode node = create_terminal_node(node_type, current_token(parser));
                    int node_idx = add_node(parser->tree, &node);
                    add_child(parser->tree, arg_list_idx, node_idx);
                    advance(parser);
                    continue;
                }

                // Check for comma (between args)
                if (match_token(parser, TOKEN_COMMA)) {
                    advance(parser);
                    continue;
                }

                // Parse string literal
                if (match_token(parser, TOKEN_STRING)) {
                    ParseNode str_node = create_terminal_node(NODE_STRING_LITERAL, current_token(parser));
                    int str_idx = add_node(parser->tree, &str_node);
                    add_child(parser->tree, arg_list_idx, str_idx);
                    advance(parser);
                    continue;
                }

                // Parse identifier (variable reference)
                if (match_token(parser, TOKEN_IDENTIFIER)) {
                    ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, current_token(parser));
                    int id_idx = add_node(parser->tree, &id_node);
                    add_child(parser->tree, arg_list_idx, id_idx);
                    advance(parser);
                    continue;
                }

                // For more complex expressions, we would push PARSE_EXPRESSION here

                // Unknown argument type
                new_error(parser, arg_list_idx, "Expected argument expression");
                break;
            }

            // Expect RPAREN
            if (!match_token(parser, TOKEN_RPAREN)) {
                new_error(parser, arg_list_idx, "Expected ')' after arguments");
                break;
            }
            advance(parser);
            break;
        }
        }
    }

    return parser->tree;
}
