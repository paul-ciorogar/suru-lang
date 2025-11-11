#include "parser.h" parc
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
    parser->token_stack = array_init(sizeof(Token));

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

// Consume and preserve whitespace (comments and newlines) as child nodes
static void consume_whitespace(Parser *parser, int parent_idx) {
    while (match_token(parser, TOKEN_COMMENT) || match_token(parser, TOKEN_NEWLINE)) {
        ParseNodeType node_type = match_token(parser, TOKEN_COMMENT) ? NODE_COMMENT : NODE_NEWLINE;
        ParseNode node = create_terminal_node(node_type, current_token(parser));
        int node_idx = add_node(parser->tree, &node);
        add_child(parser->tree, parent_idx, node_idx);
        advance(parser);
    }
}

// ===== Stack Operations =====

static void push_new_frame(Parser *parser, ParserState state, int parent_idx, int current_idx, int step) {
    ParserStackFrame frame;
    frame.state = state;
    frame.parent_node_idx = parent_idx;
    frame.current_node_idx = current_idx;
    frame.precedence = 0;
    frame.step = step;
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
    case TOKEN_NOT:
    case TOKEN_MINUS:
        return PREC_UNARY;
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

// ===== Expression Parsing with Token Stack =====

// Get the specific node type for a unary operator
static ParseNodeType get_unary_node_type(TokenType op) {
    switch (op) {
    case TOKEN_NOT:
        return NODE_NOT_EXPR;
    case TOKEN_MINUS:
        return NODE_NEGATE_EXPR;
    default:
        return NODE_IDENTIFIER; // Should not happen
    }
}

// Get the specific node type for a binary operator
static ParseNodeType get_binary_node_type(TokenType op) {
    switch (op) {
    case TOKEN_AND:
        return NODE_AND_EXPR;
    case TOKEN_OR:
        return NODE_OR_EXPR;
    case TOKEN_PLUS:
        return NODE_PLUS_EXPR;
    case TOKEN_PIPE:
        return NODE_PIPE_EXPR;
    default:
        return NODE_IDENTIFIER; // Should not happen
    }
}

// Build expression tree from postfix token array
// Uses a stack-based algorithm to construct the parse tree from postfix notation.
//
// Example: Build tree from: true false and true or
//
// Token | Stack (showing tree structure)
// ------|--------------------------------
// true  | [true]
// false | [true, false]
// and   | [(true and false)]      // Pop false, pop true, make tree
// true  | [(true and false), true]
// or    | [((true and false) or true)]  // Pop true, pop (true and false), make tree
//
// Final tree:
//            or
//           / \
//         and  true
//        / \
//     true false
//
// Algorithm:
//   - For operands (literals, identifiers): create node and push to stack
//   - For unary operators: pop 1 operand, create unary node, push result
//   - For binary operators: pop 2 operands (right then left), create binary node, push result
//   - Final stack should contain exactly 1 node (the expression root)
//
// Parameters:
//   parser: Parser context containing the parse tree
//   postfix_tokens: Array of tokens in postfix order
//   parent_idx: Index of parent node to attach the expression to
//
// Returns:
//   Index of the expression root node, or -1 on error
static int build_expr_tree_from_postfix(Parser *parser, Array *postfix_tokens, int parent_idx) {
    // Stack to hold node indices
    Array *node_stack = array_init(sizeof(int));
    if (!node_stack) {
        return -1;
    }

    // Process each token in postfix order
    size_t count = array_length(postfix_tokens);
    for (size_t i = 0; i < count; i++) {
        Token *token = (Token *)array_get(postfix_tokens, i);

        if (is_unary_operator(token->type)) {
            // Pop one operand
            if (array_length(node_stack) < 1) {
                array_free(node_stack);
                return -1;
            }
            int operand_idx;
            array_pop(node_stack, &operand_idx);

            // Create specific unary expression node
            ParseNodeType node_type = get_unary_node_type(token->type);
            ParseNode unary_node = create_nonterminal_node(node_type);
            int unary_idx = add_node(parser->tree, &unary_node);

            // Add operand (no operator token needed - node type tells us the operator)
            add_child(parser->tree, unary_idx, operand_idx);

            // Push result
            array_append(node_stack, &unary_idx);
        } else if (is_binary_operator(token->type)) {
            // Pop two operands (right first, then left)
            if (array_length(node_stack) < 2) {
                array_free(node_stack);
                return -1;
            }
            int right_idx, left_idx;
            array_pop(node_stack, &right_idx);
            array_pop(node_stack, &left_idx);

            // Create specific binary expression node
            ParseNodeType node_type = get_binary_node_type(token->type);
            ParseNode binary_node = create_nonterminal_node(node_type);
            int binary_idx = add_node(parser->tree, &binary_node);

            // Add left and right operands (no operator token needed)
            add_child(parser->tree, binary_idx, left_idx);
            add_child(parser->tree, binary_idx, right_idx);

            // Push result
            array_append(node_stack, &binary_idx);
        } else {
            // Operand (literal, identifier, or call expression)
            int operand_idx = -1;

            if (token->type == TOKEN_TRUE || token->type == TOKEN_FALSE) {
                ParseNode bool_node = create_terminal_node(NODE_BOOLEAN_LITERAL, *token);
                operand_idx = add_node(parser->tree, &bool_node);
            } else if (token->type == TOKEN_STRING) {
                ParseNode str_node = create_terminal_node(NODE_STRING_LITERAL, *token);
                operand_idx = add_node(parser->tree, &str_node);
            } else if (token->type == TOKEN_IDENTIFIER) {
                ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, *token);
                operand_idx = add_node(parser->tree, &id_node);
            }

            if (operand_idx != -1) {
                array_append(node_stack, &operand_idx);
            }
        }
    }

    // Should have exactly one node left - the root of the expression tree
    if (array_length(node_stack) != 1) {
        array_free(node_stack);
        return -1;
    }

    int result_idx;
    array_pop(node_stack, &result_idx);
    array_free(node_stack);

    // Add to parent
    add_child(parser->tree, parent_idx, result_idx);

    return result_idx;
}

// Convert infix expression to postfix notation using Shunting Yard algorithm
// Reads tokens from the parser's lexer and converts them from infix to postfix order.
//
// Example: Convert: true and false or not true
//
// Token  | Stack         | Output
// -------|---------------|------------------
// true   | []            | true
// and    | [and]         | true
// false  | [and]         | true false
// or     | [or]          | true false and
// not    | [or, not]     | true false and
// true   | [or, not]     | true false and true
// [end]  | []            | true false and true not or
//
// Result: true false and true not or
//
// Algorithm (Shunting Yard):
//   1. Read token from input
//   2. If operand: append to output
//   3. If operator:
//      - Pop operators from stack with higher/equal precedence to output
//      - Push current operator to stack
//   4. At end: pop all remaining operators to output
//
// Precedence handling:
//   - Unary operators (not, -): right-associative, pop if top_prec > current_prec
//   - Binary operators (and, or, +): left-associative, pop if top_prec >= current_prec
//
// Terminators (stop parsing):
//   - EOF, newline, comma, closing parens/braces
//
// Parameters:
//   parser: Parser context with lexer positioned at expression start
//   parent_idx: Parent node index (unused, for future use)
//
// Returns:
//   Array of tokens in postfix order, or NULL on error
static Array *infix_to_postfix(Parser *parser, int parent_idx) {
    Array *output = array_init(sizeof(Token));
    Array *operator_stack = array_init(sizeof(Token));

    if (!output || !operator_stack) {
        if (output)
            array_free(output);
        if (operator_stack)
            array_free(operator_stack);
        return NULL;
    }

    // Read tokens until we hit a terminator (newline, comma, closing brace, etc.)
    while (!match_token(parser, TOKEN_EOF) &&
           !match_token(parser, TOKEN_NEWLINE) &&
           !match_token(parser, TOKEN_COMMA) &&
           !match_token(parser, TOKEN_RPAREN) &&
           !match_token(parser, TOKEN_RBRACE)) {

        Token current = current_token(parser);

        if (current.type == TOKEN_TRUE || current.type == TOKEN_FALSE ||
            current.type == TOKEN_STRING || current.type == TOKEN_IDENTIFIER) {
            // Operand - add to output
            array_append(output, &current);
            advance(parser);
        } else if (is_unary_operator(current.type) || is_binary_operator(current.type)) {
            // Operator - pop operators with higher/equal precedence
            int current_prec = get_operator_precedence(current.type);

            while (array_length(operator_stack) > 0) {
                Token *top = (Token *)array_get(operator_stack, array_length(operator_stack) - 1);
                int top_prec = get_operator_precedence(top->type);

                // For right-associative operators (unary), use >
                // For left-associative operators (binary), use >=
                int should_pop = is_unary_operator(current.type) ? (top_prec > current_prec) : (top_prec >= current_prec);

                if (should_pop) {
                    Token op;
                    array_pop(operator_stack, &op);
                    array_append(output, &op);
                } else {
                    break;
                }
            }

            array_append(operator_stack, &current);
            advance(parser);
        } else {
            // Unknown token - stop parsing expression
            break;
        }
    }

    // Pop remaining operators
    while (array_length(operator_stack) > 0) {
        Token op;
        array_pop(operator_stack, &op);
        array_append(output, &op);
    }

    array_free(operator_stack);
    return output;
}

void new_error(Parser *parser, int parent_node_idx, char *message) {
    report_syntax_error(parser, message);
    skip_to_newline(parser);
    // Return to parse with the parent node
    push_new_frame(parser, PARSE, parent_node_idx, parent_node_idx, 0);
}

// Top-level parsing - handles program-level declarations
void parse_top_level(Parser *parser, ParserStackFrame *frame) {

    // Consume whitespace
    consume_whitespace(parser, frame->parent_node_idx);

    // Check for EOF
    if (match_token(parser, TOKEN_EOF)) {
        return; // Done parsing
    }

    // Look for declaration: identifier followed by colon
    if (match_token(parser, TOKEN_IDENTIFIER)) {
        // Push continuation to come back to PARSE after declaration
        push_new_frame(parser, PARSE, frame->parent_node_idx, frame->parent_node_idx, 0);
        // Push PARSE_STATEMENT to determine type of declaration
        push_new_frame(parser, PARSE_STATEMENT, frame->parent_node_idx, -1, 0);
        return;
    }

    // Unknown token at top level
    new_error(parser, frame->parent_node_idx, "Expected function declaration");
    return;
}

// Handles statement parsing
static void parse_statement(Parser *parser, ParserStackFrame *frame) {
    // Determine statement type: variable decl, function decl, or call expression
    // Parsing: identifier (: | () )

    if (!match_token(parser, TOKEN_IDENTIFIER)) {
        return;
    }

    Token id_token = current_token(parser);
    array_append(parser->token_stack, &id_token);
    advance(parser);

    // Check what follows the identifier
    if (match_token(parser, TOKEN_COLON)) {
        // It's a declaration (function or variable)
        advance(parser);

        // Now determine what follows the colon
        if (match_token(parser, TOKEN_LPAREN)) {
            // It's a function declaration: identifier : (params) block
            // Push PARSE_FUNCTION_DECL to continue parsing
            push_new_frame(parser, PARSE_FUNCTION_DECL, frame->parent_node_idx, -1, 0);
        } else {
            // It's a variable declaration: identifier : expression
            // Push PARSE_VAR_DECL to continue parsing
            push_new_frame(parser, PARSE_VAR_DECL, frame->parent_node_idx, -1, 0);
        }
    } else if (match_token(parser, TOKEN_LPAREN)) {
        // It's a call expression: identifier(args)
        // Push PARSE_CALL_ARGS to parse the arguments
        push_new_frame(parser, PARSE_CALL_EXPRESSION, frame->parent_node_idx, -1, 0);
    } else {
        new_error(parser, frame->parent_node_idx, "Expected ':' or '(' after identifier");
    }
}

void parse_block(Parser *parser, ParserStackFrame *frame) {
    // Parsing: { statements }
    // step 0 parse {
    // step 1 parse statements }

    int block_idx = frame->current_node_idx;

    // First time: create block node and consume LBRACE
    if (frame->step == 0) {
        // Create block node
        ParseNode block_node = create_nonterminal_node(NODE_BLOCK);
        block_idx = add_node(parser->tree, &block_node);
        add_child(parser->tree, frame->parent_node_idx, block_idx);

        // Expect LBRACE
        if (!match_token(parser, TOKEN_LBRACE)) {
            new_error(parser, block_idx, "Expected '{' for block");
            return;
        }
        advance(parser);
        // Push continuation to keep parsing this block
        push_new_frame(parser, PARSE_BLOCK, frame->parent_node_idx, block_idx, 1);

    } else if (frame->step == 1) {
        if (match_token(parser, TOKEN_RBRACE)) {
            advance(parser);
            return;
        }
        // Push continuation to keep parsing this block
        push_new_frame(parser, PARSE_BLOCK, frame->parent_node_idx, block_idx, 1);

        // Parse block statements
        push_new_frame(parser, PARSE_STATEMENT, block_idx, -1, 0);
        push_new_frame(parser, TRY_PARSE_RETURN, block_idx, -1, 0);
    }
}

void parse_function_declaration(Parser *parser, ParserStackFrame *frame) {
    if (array_length(parser->token_stack) == 0) {
        new_error(parser, frame.parent_node_idx, "Expected identifier token on token stack");
    }
    // Create function declaration node
    ParseNode func_node = create_nonterminal_node(NODE_FUNCTION_DECL);
    int func_idx = add_node(parser->tree, &func_node);
    add_child(parser->tree, frame.parent_node_idx, func_idx);

    // Add identifier as child
    Token id_token = {0};
    array_pop(parser->token_stack, &id_token);
    ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
    int id_idx = add_node(parser->tree, &id_node);
    add_child(parser->tree, func_idx, id_idx);

    // Parsing: param_list block

    // Push PARSE_BLOCK to be executed after PARSE_PARAM_LIST
    push_new_frame(parser, PARSE_BLOCK, func_idx, -1, 0);

    // Push PARSE_PARAM_LIST
    push_new_frame(parser, PARSE_PARAM_LIST, func_idx, -1, 0);
}

// Parse the source code and build parse tree with syntax analysis
ParseTree *parse(Parser *parser) {
    if (!parser || !parser->tree) {
        return NULL;
    }

    // Create expression helpers
    Array *temp_nodes = array_init(sizeof(ParseNode));
    Array *temp_stack = array_init(sizeof(ParseNode));

    // Create root program node
    ParseNode root = create_nonterminal_node(NODE_PROGRAM);
    int root_idx = add_node(parser->tree, &root);
    parser->tree->root = root_idx;

    // Push initial state
    push_new_frame(parser, PARSE, root_idx, root_idx, 0);

    // Main parsing loop - iterative stack-based parsing
    while (array_length(parser->stack) > 0) {
        ParserStackFrame frame;
        if (!pop_frame(parser, &frame)) {
            break;
        }

        switch (frame.state) {

        case PARSE: {
            parse_top_level(parser, &frame);
            break;
        }

        case PARSE_STATEMENT: {
            parse_statement(parser, &frame);
        }

        case PARSE_FUNCTION_DECL: {
            parse_function_declaration(parser, &frame);
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

            // Consume whitespace
            consume_whitespace(parser, param_list_idx);

            // Check for empty param list
            if (match_token(parser, TOKEN_RPAREN)) {
                advance(parser);
                break;
            }

            // Parse parameters: identifier [, identifier]*
            while (!match_token(parser, TOKEN_RPAREN) && !match_token(parser, TOKEN_EOF)) {
                // Expect identifier (parameter name)
                if (!match_token(parser, TOKEN_IDENTIFIER)) {
                    new_error(parser, param_list_idx, "Expected parameter name");
                    break;
                }

                // Create parameter node
                ParseNode param_node = create_nonterminal_node(NODE_PARAM);
                int param_idx = add_node(parser->tree, &param_node);
                add_child(parser->tree, param_list_idx, param_idx);

                // Add identifier as child of param
                ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, current_token(parser));
                int id_idx = add_node(parser->tree, &id_node);
                add_child(parser->tree, param_idx, id_idx);
                advance(parser);

                // Consume whitespace
                consume_whitespace(parser, param_list_idx);

                // Check for comma (more parameters)
                if (match_token(parser, TOKEN_COMMA)) {
                    advance(parser);
                    consume_whitespace(parser, param_list_idx);
                    continue;
                }

                // Otherwise, expect RPAREN
                break;
            }

            // Expect RPAREN
            if (!match_token(parser, TOKEN_RPAREN)) {
                new_error(parser, param_list_idx, "Expected ')' after parameter list");
                break;
            }
            advance(parser);
            break;
        }

        case PARSE_BLOCK: {
            parse_block(parser, &frame);
        }

        case PARSE_VAR_DECL: {
            // Create variable declaration node
            ParseNode var_node = create_nonterminal_node(NODE_VAR_DECL);
            int var_idx = add_node(parser->tree, &var_node);
            add_child(parser->tree, frame.parent_node_idx, var_idx);

            // Add identifier as child
            ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
            int id_idx = add_node(parser->tree, &id_node);
            add_child(parser->tree, var_idx, id_idx);

            // Parsing: value expression
            // Note: NODE_VAR_DECL and identifier already created by PARSE_DECL

            int var_idx = frame.current_node_idx;
            if (var_idx == -1) {
                new_error(parser, frame.parent_node_idx, "Invalid variable declaration state");
                break;
            }

            // Parse value as an expression
            push_new_frame(parser, PARSE_EXPRESSION, var_idx, -1, 0);
            break;
        }

        case PARSE_CALL_EXPRESSION: {
            // Create call expression node
            ParseNode call_node = create_nonterminal_node(NODE_CALL_EXPR);
            int call_idx = add_node(parser->tree, &call_node);
            add_child(parser->tree, frame.parent_node_idx, call_idx);

            // Add identifier as first child of call
            ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
            int id_idx = add_node(parser->tree, &id_node);
            add_child(parser->tree, call_idx, id_idx);

            break;
        }

        case PARSE_EXPRESSION: {
            // Stack-based expression parser
            // step 0: Initialize and check for match expression
            // step 1: Parse primary expressions and operators
            // step 2: Finalize expression

            if (frame.step == 0) {
                // Check if this is a match expression
                if (match_token(parser, TOKEN_MATCH)) {
                    push_new_frame(parser, PARSE_MATCH_EXPR, frame.parent_node_idx, -1, 0);
                    break;
                }

                // Move to step 1
                push_new_frame(parser, PARSE_EXPRESSION, frame.parent_node_idx, -1, 1);
                break;
            }

            if (frame.step == 1) {
                // Parse primary expressions and operators

                // Check for end of expression
                if (match_token(parser, TOKEN_EOF) ||
                    match_token(parser, TOKEN_NEWLINE) ||
                    match_token(parser, TOKEN_COMMA) ||
                    match_token(parser, TOKEN_RPAREN) ||
                    match_token(parser, TOKEN_RBRACE)) {
                    // Move to finalization
                    push_new_frame(parser, PARSE_EXPRESSION, frame.parent_node_idx, -1, 2);
                    break;
                }

                // Parse primary expression
                int primary_idx = -1;

                if (match_token(parser, TOKEN_TRUE) || match_token(parser, TOKEN_FALSE)) {
                    ParseNode node = create_terminal_node(NODE_BOOLEAN_LITERAL, current_token(parser));
                    primary_idx = add_node(parser->tree, &node);
                    advance(parser);
                } else if (match_token(parser, TOKEN_STRING)) {
                    ParseNode node = create_terminal_node(NODE_STRING_LITERAL, current_token(parser));
                    primary_idx = add_node(parser->tree, &node);
                    advance(parser);
                } else if (match_token(parser, TOKEN_IDENTIFIER)) {
                    Token id_token = current_token(parser);
                    advance(parser);

                    if (match_token(parser, TOKEN_LPAREN)) {
                        // Call expression
                        // Push continuation frame
                        push_new_frame(parser, PARSE_EXPRESSION, frame.parent_node_idx, -1, 1);

                        ParseNode call_node = create_nonterminal_node(NODE_CALL_EXPR);
                        int call_idx = add_node(parser->tree, &call_node);

                        ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
                        int id_idx = add_node(parser->tree, &id_node);
                        add_child(parser->tree, call_idx, id_idx);
                        // Push parse call expression
                        push_new_frame(parser, PARSE_CALL_EXPRESSION, frame.parent_node_idx, -1, 1);

                        ParseNode arg_list_node = create_nonterminal_node(NODE_ARG_LIST);
                        int arg_list_idx = add_node(parser->tree, &arg_list_node);
                        add_child(parser->tree, call_idx, arg_list_idx);

                        // Parse arguments
                        while (!match_token(parser, TOKEN_RPAREN) && !match_token(parser, TOKEN_EOF)) {
                            consume_whitespace(parser, arg_list_idx);
                            if (match_token(parser, TOKEN_RPAREN))
                                break;
                            if (match_token(parser, TOKEN_COMMA)) {
                                advance(parser);
                                continue;
                            }

                            // Parse argument (simple - no nested calls for now)
                            if (match_token(parser, TOKEN_STRING)) {
                                ParseNode str_node = create_terminal_node(NODE_STRING_LITERAL, current_token(parser));
                                int str_idx = add_node(parser->tree, &str_node);
                                add_child(parser->tree, arg_list_idx, str_idx);
                                advance(parser);
                            } else if (match_token(parser, TOKEN_TRUE) || match_token(parser, TOKEN_FALSE)) {
                                ParseNode bool_node = create_terminal_node(NODE_BOOLEAN_LITERAL, current_token(parser));
                                int bool_idx = add_node(parser->tree, &bool_node);
                                add_child(parser->tree, arg_list_idx, bool_idx);
                                advance(parser);
                            } else if (match_token(parser, TOKEN_IDENTIFIER)) {
                                ParseNode arg_id = create_terminal_node(NODE_IDENTIFIER, current_token(parser));
                                int arg_id_idx = add_node(parser->tree, &arg_id);
                                add_child(parser->tree, arg_list_idx, arg_id_idx);
                                advance(parser);
                            } else {
                                new_error(parser, arg_list_idx, "Expected argument expression");
                                if (parser->expr_stack) {
                                    array_free(parser->expr_stack);
                                    parser->expr_stack = NULL;
                                }
                                goto expr_done;
                            }
                        }

                        if (!match_token(parser, TOKEN_RPAREN)) {
                            new_error(parser, arg_list_idx, "Expected ')' after arguments");
                            if (parser->expr_stack) {
                                array_free(parser->expr_stack);
                                parser->expr_stack = NULL;
                            }
                            goto expr_done;
                        }
                        advance(parser);

                        primary_idx = call_idx;
                    } else {
                        // Just identifier
                        ParseNode node = create_terminal_node(NODE_IDENTIFIER, id_token);
                        primary_idx = add_node(parser->tree, &node);
                    }
                } else if (is_unary_operator(current_token(parser).type)) {
                    // Unary operator
                    TokenType op_type = current_token(parser).type;
                    advance(parser);

                    ParseNodeType node_type = get_unary_node_type(op_type);
                    ParseNode unary_node = create_nonterminal_node(node_type);
                    int unary_idx = add_node(parser->tree, &unary_node);

                    // Parse operand (simplified - no nested expressions)
                    if (match_token(parser, TOKEN_TRUE) || match_token(parser, TOKEN_FALSE)) {
                        ParseNode operand = create_terminal_node(NODE_BOOLEAN_LITERAL, current_token(parser));
                        int operand_idx = add_node(parser->tree, &operand);
                        add_child(parser->tree, unary_idx, operand_idx);
                        advance(parser);
                    } else if (match_token(parser, TOKEN_IDENTIFIER)) {
                        ParseNode operand = create_terminal_node(NODE_IDENTIFIER, current_token(parser));
                        int operand_idx = add_node(parser->tree, &operand);
                        add_child(parser->tree, unary_idx, operand_idx);
                        advance(parser);
                    } else {
                        new_error(parser, unary_idx, "Expected operand after unary operator");
                        if (parser->expr_stack) {
                            array_free(parser->expr_stack);
                            parser->expr_stack = NULL;
                        }
                        goto expr_done;
                    }

                    primary_idx = unary_idx;
                } else {
                    // Unknown - end of expression
                    push_new_frame(parser, PARSE_EXPRESSION, frame.parent_node_idx, -1, 2);
                    break;
                }

                // Push primary to stack
                if (primary_idx != -1) {
                    array_append(parser->expr_stack, &primary_idx);
                }

                // Check for binary operator
                if (is_binary_operator(current_token(parser).type)) {
                    TokenType op_type = current_token(parser).type;
                    advance(parser);

                    // Pop left operand
                    if (array_length(parser->expr_stack) < 1) {
                        new_error(parser, frame.parent_node_idx, "Binary operator missing left operand");
                        if (parser->expr_stack) {
                            array_free(parser->expr_stack);
                            parser->expr_stack = NULL;
                        }
                        goto expr_done;
                    }

                    int left_idx;
                    array_pop(parser->expr_stack, &left_idx);

                    // Create binary node
                    ParseNodeType node_type = get_binary_node_type(op_type);
                    ParseNode binary_node = create_nonterminal_node(node_type);
                    int binary_idx = add_node(parser->tree, &binary_node);

                    // Add left child
                    add_child(parser->tree, binary_idx, left_idx);

                    // Push binary node (right operand parsed in next iteration)
                    array_append(parser->expr_stack, &binary_idx);
                }

                // Continue parsing
                push_new_frame(parser, PARSE_EXPRESSION, frame.parent_node_idx, -1, 1);
                break;
            }

            if (frame.step == 2) {
                // Finalize: pop from stack and add to parent
                if (!parser->expr_stack || array_length(parser->expr_stack) == 0) {
                    new_error(parser, frame.parent_node_idx, "Empty expression");
                    if (parser->expr_stack) {
                        array_free(parser->expr_stack);
                        parser->expr_stack = NULL;
                    }
                    break;
                }

                if (array_length(parser->expr_stack) == 1) {
                    // Single node
                    int expr_idx;
                    array_pop(parser->expr_stack, &expr_idx);
                    add_child(parser->tree, frame.parent_node_idx, expr_idx);
                } else if (array_length(parser->expr_stack) == 2) {
                    // Binary operator with right operand
                    int right_idx, binary_idx;
                    array_pop(parser->expr_stack, &right_idx);
                    array_pop(parser->expr_stack, &binary_idx);

                    add_child(parser->tree, binary_idx, right_idx);
                    add_child(parser->tree, frame.parent_node_idx, binary_idx);
                } else {
                    new_error(parser, frame.parent_node_idx, "Invalid expression - too many items on stack");
                }

                array_free(parser->expr_stack);
                parser->expr_stack = NULL;
                break;
            }

        expr_done:
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
                // Consume whitespace
                consume_whitespace(parser, arg_list_idx);

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

                // Parse boolean literal
                if (match_token(parser, TOKEN_TRUE) || match_token(parser, TOKEN_FALSE)) {
                    ParseNode bool_node = create_terminal_node(NODE_BOOLEAN_LITERAL, current_token(parser));
                    int bool_idx = add_node(parser->tree, &bool_node);
                    add_child(parser->tree, arg_list_idx, bool_idx);
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

        case PARSE_MATCH_EXPR: {
            // Parsing: match <subject_expr> { <pattern>: <expr> ... }
            // Uses step to track progress through parsing stages

            int match_idx = frame.current_node_idx;

            if (frame.step == 0) {
                // Step 0: Create match node and parse subject
                // Consume 'match' keyword
                if (!match_token(parser, TOKEN_MATCH)) {
                    new_error(parser, frame.parent_node_idx, "Expected 'match' keyword");
                    break;
                }
                advance(parser);

                // Create match expression node
                ParseNode match_node = create_nonterminal_node(NODE_MATCH_EXPR);
                match_idx = add_node(parser->tree, &match_node);
                add_child(parser->tree, frame.parent_node_idx, match_idx);

                // Parse subject expression (identifier or literal)
                if (match_token(parser, TOKEN_IDENTIFIER)) {
                    ParseNode subj_node = create_terminal_node(NODE_IDENTIFIER, current_token(parser));
                    int subj_idx = add_node(parser->tree, &subj_node);
                    add_child(parser->tree, match_idx, subj_idx);
                    advance(parser);
                } else if (match_token(parser, TOKEN_TRUE) || match_token(parser, TOKEN_FALSE)) {
                    ParseNode subj_node = create_terminal_node(NODE_BOOLEAN_LITERAL, current_token(parser));
                    int subj_idx = add_node(parser->tree, &subj_node);
                    add_child(parser->tree, match_idx, subj_idx);
                    advance(parser);
                } else if (match_token(parser, TOKEN_STRING)) {
                    ParseNode subj_node = create_terminal_node(NODE_STRING_LITERAL, current_token(parser));
                    int subj_idx = add_node(parser->tree, &subj_node);
                    add_child(parser->tree, match_idx, subj_idx);
                    advance(parser);
                } else {
                    new_error(parser, match_idx, "Expected expression after 'match'");
                    break;
                }

                // Expect opening brace
                if (!match_token(parser, TOKEN_LBRACE)) {
                    new_error(parser, match_idx, "Expected '{' after match subject");
                    break;
                }
                advance(parser);

                // Continue with current match_idx set
                push_new_frame(parser, PARSE_MATCH_EXPR, match_idx, match_idx, 1);
                break;
            }

            if (frame.step == 1) {
                // Step 1: Parse match arms
                while (!match_token(parser, TOKEN_RBRACE) && !match_token(parser, TOKEN_EOF)) {
                    // Consume whitespace
                    consume_whitespace(parser, match_idx);

                    // Check for RBRACE after consuming whitespace
                    if (match_token(parser, TOKEN_RBRACE)) {
                        break;
                    }

                    // Create match arm node
                    ParseNode arm_node = create_nonterminal_node(NODE_MATCH_ARM);
                    int arm_idx = add_node(parser->tree, &arm_node);
                    add_child(parser->tree, match_idx, arm_idx);

                    // Parse pattern (boolean literal, string literal, or wildcard)
                    if (match_token(parser, TOKEN_TRUE) || match_token(parser, TOKEN_FALSE)) {
                        ParseNode pattern_node = create_terminal_node(NODE_BOOLEAN_LITERAL, current_token(parser));
                        int pattern_idx = add_node(parser->tree, &pattern_node);
                        add_child(parser->tree, arm_idx, pattern_idx);
                        advance(parser);
                    } else if (match_token(parser, TOKEN_STRING)) {
                        ParseNode pattern_node = create_terminal_node(NODE_STRING_LITERAL, current_token(parser));
                        int pattern_idx = add_node(parser->tree, &pattern_node);
                        add_child(parser->tree, arm_idx, pattern_idx);
                        advance(parser);
                    } else if (match_token(parser, TOKEN_UNDERSCORE)) {
                        ParseNode pattern_node = create_terminal_node(NODE_MATCH_WILDCARD, current_token(parser));
                        int pattern_idx = add_node(parser->tree, &pattern_node);
                        add_child(parser->tree, arm_idx, pattern_idx);
                        advance(parser);
                    } else {
                        new_error(parser, arm_idx, "Expected pattern (boolean, string, or '_')");
                        break;
                    }

                    // Expect colon
                    if (!match_token(parser, TOKEN_COLON)) {
                        new_error(parser, arm_idx, "Expected ':' after pattern");
                        break;
                    }
                    advance(parser);

                    // Parse arm expression - use existing expression parsing
                    // Push continuation to come back and parse more arms
                    push_new_frame(parser, PARSE_MATCH_EXPR, match_idx, match_idx, 1);
                    // Push PARSE_EXPRESSION to parse the arm's expression
                    push_new_frame(parser, PARSE_EXPRESSION, arm_idx, -1, 0);
                    break;
                }

                // Check if we're done parsing arms
                if (match_token(parser, TOKEN_RBRACE)) {
                    advance(parser);
                    break;
                }
            }

            break;
        }

        case PARSE_MATCH_STMT: {
            // Parsing: match <subject_expr> { <pattern>: <statement> ... }
            // Uses step to track progress through parsing stages

            int match_idx = frame.current_node_idx;

            if (frame.step == 0) {
                // Step 0: Create match statement node and consume 'match' keyword
                if (!match_token(parser, TOKEN_MATCH)) {
                    new_error(parser, frame.parent_node_idx, "Expected 'match' keyword");
                    break;
                }
                advance(parser);

                // Create match statement node
                ParseNode match_node = create_nonterminal_node(NODE_MATCH_STMT);
                match_idx = add_node(parser->tree, &match_node);
                add_child(parser->tree, frame.parent_node_idx, match_idx);

                // Parse subject expression
                push_new_frame(parser, PARSE_MATCH_STMT, match_idx, match_idx, 1);
                push_new_frame(parser, PARSE_EXPRESSION, match_idx, -1, 0);
                break;
            }

            if (frame.step == 1) {
                // Step 1: Subject parsed, expect '{'
                if (!match_token(parser, TOKEN_LBRACE)) {
                    new_error(parser, match_idx, "Expected '{' after match subject");
                    break;
                }
                advance(parser);

                // Continue to parse arms
                push_new_frame(parser, PARSE_MATCH_STMT, match_idx, match_idx, 2);
                break;
            }

            if (frame.step == 2) {
                // Step 2: Parse match arms
                while (!match_token(parser, TOKEN_RBRACE) && !match_token(parser, TOKEN_EOF)) {
                    // Consume whitespace
                    consume_whitespace(parser, match_idx);

                    // Check for RBRACE after consuming whitespace
                    if (match_token(parser, TOKEN_RBRACE)) {
                        break;
                    }

                    // Create match arm node
                    ParseNode arm_node = create_nonterminal_node(NODE_MATCH_ARM);
                    int arm_idx = add_node(parser->tree, &arm_node);
                    add_child(parser->tree, match_idx, arm_idx);

                    // Parse pattern (identifier, literal, or wildcard)
                    ParseNodeType pattern_type;
                    if (match_token(parser, TOKEN_IDENTIFIER)) {
                        pattern_type = NODE_IDENTIFIER;
                    } else if (match_token(parser, TOKEN_STRING)) {
                        pattern_type = NODE_STRING_LITERAL;
                    } else if (match_token(parser, TOKEN_TRUE) || match_token(parser, TOKEN_FALSE)) {
                        pattern_type = NODE_BOOLEAN_LITERAL;
                    } else if (match_token(parser, TOKEN_UNDERSCORE)) {
                        pattern_type = NODE_MATCH_WILDCARD;
                    } else {
                        new_error(parser, arm_idx, "Expected pattern in match arm");
                        break;
                    }

                    ParseNode pattern_node = create_terminal_node(pattern_type, current_token(parser));
                    int pattern_idx = add_node(parser->tree, &pattern_node);
                    add_child(parser->tree, arm_idx, pattern_idx);
                    advance(parser);

                    // Expect colon
                    if (!match_token(parser, TOKEN_COLON)) {
                        new_error(parser, arm_idx, "Expected ':' after pattern");
                        break;
                    }
                    advance(parser);

                    // Parse arm statement
                    // Push continuation to come back and parse more arms
                    push_new_frame(parser, PARSE_MATCH_STMT, match_idx, match_idx, 2);
                    // Push PARSE_STATEMENT to parse the arm's statement
                    push_new_frame(parser, PARSE_STATEMENT, arm_idx, -1, 0);
                    break;
                }

                // Check if we're done parsing arms
                if (match_token(parser, TOKEN_RBRACE)) {
                    advance(parser);
                    break;
                }
            }

            break;
        }
        }
    }

    return parser->tree;
}
