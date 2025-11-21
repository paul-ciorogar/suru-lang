// Debug configuration
// Uncomment the line below to enable parser loop debugging output
// This will print each iteration of the parse loop with state, step, and token info
// Useful for diagnosing infinite loops or understanding parser flow
// #define DEBUG_PARSER_LOOP

#include "parser.h"
#include "arena.h"
#include "array.h"
#include "lexer.h"
#include "parse_tree.h"
#include <assert.h>
#include <stdbool.h>
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
    parser->temp_nodes = array_init(sizeof(ParseNode));
    parser->expr_stack = array_init(sizeof(int));

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

// ===== Operator Helpers =====

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
        assert(false);
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

static void new_error(Parser *parser, int parent_node_idx, char *message) {
    report_syntax_error(parser, message);
    skip_to_newline(parser);
    // Return to parse with the parent node
    push_new_frame(parser, PARSE, parent_node_idx, parent_node_idx, 0);
}

// Top-level parsing - handles program-level declarations
static void parse_top_level(Parser *parser, ParserStackFrame *frame) {

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

    // Parsing: return statement
    if (match_token(parser, TOKEN_RETURN)) {
        push_new_frame(parser, PARSE_RETURN_STATEMENT, frame->parent_node_idx, -1, 0);
        return;
    }

    // Parse match statement
    if (match_token(parser, TOKEN_MATCH)) {
        push_new_frame(parser, PARSE_MATCH_STMT, frame->parent_node_idx, -1, 0);
        return;
    }

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
        // It's a call statement: identifier(args)
        push_new_frame(parser, PARSE_CALL_STATEMENT, frame->parent_node_idx, -1, 0);
    } else {
        new_error(parser, frame->parent_node_idx, "Expected ':' or '(' after identifier");
    }
}

static void parse_block(Parser *parser, ParserStackFrame *frame) {
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
        // Consume whitespace before checking for end of block
        consume_whitespace(parser, block_idx);

        if (match_token(parser, TOKEN_RBRACE)) {
            advance(parser);
            return;
        }
        if (match_token(parser, TOKEN_EOF)) {
            new_error(parser, block_idx, "Expected '}' to close the block");
            return;
        }
        // Push continuation to keep parsing this block
        push_new_frame(parser, PARSE_BLOCK, frame->parent_node_idx, block_idx, 1);

        // Parse block statements
        push_new_frame(parser, PARSE_STATEMENT, block_idx, -1, 0);
    }
}

static void parse_variable_declaration(Parser *parser, ParserStackFrame *frame) {
    // Pop identifier token from token stack
    if (array_length(parser->token_stack) == 0) {
        new_error(parser, frame->parent_node_idx, "Expected identifier token on token stack");
        return;
    }
    Token id_token;
    array_pop(parser->token_stack, &id_token);

    // Create variable declaration node
    ParseNode var_node = create_nonterminal_node(NODE_VAR_DECL);
    int var_idx = add_node(parser->tree, &var_node);
    add_child(parser->tree, frame->parent_node_idx, var_idx);

    // Add identifier as child
    ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
    int id_idx = add_node(parser->tree, &id_node);
    add_child(parser->tree, var_idx, id_idx);

    // Parse value as an expression
    push_new_frame(parser, PARSE_EXPRESSION, var_idx, -1, 0);
}

static void parse_call_statement(Parser *parser, ParserStackFrame *frame) {
    if (frame->step == 0) {
        push_new_frame(parser, PARSE_CALL_STATEMENT, frame->parent_node_idx, -1, 1);
        push_new_frame(parser, PARSE_CALL_EXPRESSION, frame->parent_node_idx, -1, 0);
    } else if (frame->step == 1) {
        if (array_length(parser->expr_stack) < 1) {
            new_error(parser, frame->parent_node_idx, "Call statement missing expression");
            array_clear(parser->expr_stack);
            return;
        }
        int expr_idx;
        array_pop(parser->expr_stack, &expr_idx);
        add_child(parser->tree, frame->parent_node_idx, expr_idx);
    }
}

static void parse_call_expression(Parser *parser, ParserStackFrame *frame) {
    // Pop identifier token from token stack
    if (array_length(parser->token_stack) == 0) {
        new_error(parser, frame->parent_node_idx, "Expected identifier token on token stack");
        return;
    }
    Token id_token;
    array_pop(parser->token_stack, &id_token);

    // Create call expression node
    ParseNode call_node = create_nonterminal_node(NODE_CALL_EXPR);
    int call_idx = add_node(parser->tree, &call_node);
    array_append(parser->expr_stack, &call_idx);

    // Add identifier as first child of call
    ParseNode id_node = create_terminal_node(NODE_IDENTIFIER, id_token);
    int id_idx = add_node(parser->tree, &id_node);
    add_child(parser->tree, call_idx, id_idx);

    // Delegate to PARSE_CALL_ARGS to parse arguments
    push_new_frame(parser, PARSE_CALL_ARGS, call_idx, -1, 0);
}

static void parse_function_declaration(Parser *parser, ParserStackFrame *frame) {
    if (array_length(parser->token_stack) == 0) {
        new_error(parser, frame->parent_node_idx, "Expected identifier token on token stack");
    }
    // Create function declaration node
    ParseNode func_node = create_nonterminal_node(NODE_FUNCTION_DECL);
    int func_idx = add_node(parser->tree, &func_node);
    add_child(parser->tree, frame->parent_node_idx, func_idx);

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

static void parse_param_list(Parser *parser, ParserStackFrame *frame) {
    // Parsing: ( [param [, param]*] )

    // Create param list node
    ParseNode param_list_node = create_nonterminal_node(NODE_PARAM_LIST);
    int param_list_idx = add_node(parser->tree, &param_list_node);
    add_child(parser->tree, frame->parent_node_idx, param_list_idx);

    // Expect LPAREN
    if (!match_token(parser, TOKEN_LPAREN)) {
        new_error(parser, param_list_idx, "Expected '(' for parameter list");
        return;
    }
    advance(parser);

    // Consume whitespace
    consume_whitespace(parser, param_list_idx);

    // Check for empty param list
    if (match_token(parser, TOKEN_RPAREN)) {
        advance(parser);
        return;
    }

    // Parse parameters: identifier [, identifier]*
    while (!match_token(parser, TOKEN_RPAREN) && !match_token(parser, TOKEN_EOF)) {
        // Expect identifier (parameter name)
        if (!match_token(parser, TOKEN_IDENTIFIER)) {
            new_error(parser, param_list_idx, "Expected parameter name");
            return;
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
        return;
    }
    advance(parser);
}

static void parse_unary_operator(Parser *parser, ParserStackFrame *frame) {
    // Parsing: operator expression
    // step 0: Create unary node and push PARSE_EXPRESSION for operand

    if (frame->step == 0) {
        // Pop operator token from token stack
        if (array_length(parser->token_stack) == 0) {
            new_error(parser, frame->parent_node_idx, "Expected operator token on token stack");
            return;
        }
        Token op_token;
        array_pop(parser->token_stack, &op_token);

        // Get the node type for this operator
        ParseNodeType node_type = get_unary_node_type(op_token.type);

        // Create unary operator node
        ParseNode unary_node = create_nonterminal_node(node_type);
        int unary_idx = add_node(parser->tree, &unary_node);

        // Parse operand as full expression
        push_new_frame(parser, PARSE_EXPRESSION, unary_idx, -1, 0);

        // Store unary node index to add to expr_stack after operand is parsed
        // Use step 1 to signal completion
        push_new_frame(parser, PARSE_UNARY_OPERATOR, frame->parent_node_idx, unary_idx, 1);
        return;
    }

    if (frame->step == 1) {
        // Operand has been parsed and added as child to unary node
        // Add unary node to expression stack
        int unary_idx = frame->current_node_idx;
        array_append(parser->expr_stack, &unary_idx);
        return;
    }
}

static void parse_binary_expression(Parser *parser, ParserStackFrame *frame) {
    // Parsing: left_operand BINARY_OP right_operand
    // Operator token is on token_stack, left operand is on expr_stack
    // step 0: Create binary node, add left child, push frame to parse right operand
    // step 1: Right operand parsed, push binary node to expr_stack

    if (frame->step == 0) {
        // Pop operator token from token stack
        if (array_length(parser->token_stack) == 0) {
            new_error(parser, frame->parent_node_idx, "Expected operator token on token stack");
            return;
        }
        Token op_token;
        array_pop(parser->token_stack, &op_token);

        // Pop left operand from expr_stack
        if (array_length(parser->expr_stack) == 0) {
            new_error(parser, frame->parent_node_idx, "Binary operator missing left operand");
            return;
        }
        int left_idx;
        array_pop(parser->expr_stack, &left_idx);

        // Get the node type for this operator
        ParseNodeType node_type = get_binary_node_type(op_token.type);

        // Create binary operator node
        ParseNode binary_node = create_nonterminal_node(node_type);
        int binary_idx = add_node(parser->tree, &binary_node);

        // Add left child
        add_child(parser->tree, binary_idx, left_idx);

        // Store binary node index to add to expr_stack after operand is parsed
        push_new_frame(parser, PARSE_BINARY_EXPRESSION, frame->parent_node_idx, binary_idx, 1);

        // Parse right operand as full expression
        push_new_frame(parser, PARSE_EXPRESSION, binary_idx, -1, 0);
        return;
    }

    if (frame->step == 1) {
        // Right operand has been parsed and added as child to binary node
        // Add binary node to expression stack
        int binary_idx = frame->current_node_idx;
        array_append(parser->expr_stack, &binary_idx);
        return;
    }
}

static void parse_expression(Parser *parser, ParserStackFrame *frame) {
    // Stack-based expression parser
    // step 0: Initialize and check for match expression
    // step 1: Parse primary expressions and operators
    // step 2: Finalize expression

    if (frame->step == 0) {
        // Check if this is a match expression
        if (match_token(parser, TOKEN_MATCH)) {
            push_new_frame(parser, PARSE_MATCH_EXPR, frame->parent_node_idx, -1, 0);
            return;
        }

        // Move to step 1 to parse primary expressions and operators
        push_new_frame(parser, PARSE_EXPRESSION, frame->parent_node_idx, -1, 1);
        return;
    }

    if (frame->step == 1) {
        // Parse primary expressions and operators

        // Check for end of expression
        if (match_token(parser, TOKEN_EOF) ||
            match_token(parser, TOKEN_NEWLINE) ||
            match_token(parser, TOKEN_COMMA) ||
            match_token(parser, TOKEN_RPAREN) ||
            match_token(parser, TOKEN_RBRACE)) {
            // Move to finalization
            push_new_frame(parser, PARSE_EXPRESSION, frame->parent_node_idx, -1, 2);
            return;
        }

        // Parse primary expression
        int primary_idx = -1;

        // Parse boolean terminals
        if (match_token(parser, TOKEN_TRUE) || match_token(parser, TOKEN_FALSE)) {
            ParseNode node = create_terminal_node(NODE_BOOLEAN_LITERAL, current_token(parser));
            primary_idx = add_node(parser->tree, &node);
            advance(parser);
        } else if (match_token(parser, TOKEN_STRING)) { // Parse string
            ParseNode node = create_terminal_node(NODE_STRING_LITERAL, current_token(parser));
            primary_idx = add_node(parser->tree, &node);
            advance(parser);
        } else if (match_token(parser, TOKEN_IDENTIFIER)) {
            Token id_token = current_token(parser);
            advance(parser);

            if (match_token(parser, TOKEN_LPAREN)) {
                // Call expression - delegate to parse_call_expression
                // Push continuation frame
                push_new_frame(parser, PARSE_EXPRESSION, frame->parent_node_idx, -1, 1);
                array_append(parser->token_stack, &id_token);
                push_new_frame(parser, PARSE_CALL_EXPRESSION, frame->parent_node_idx, -1, 1);
                return;
            } else {
                // Just identifier
                ParseNode node = create_terminal_node(NODE_IDENTIFIER, id_token);
                primary_idx = add_node(parser->tree, &node);
            }
        } else if (is_unary_operator(current_token(parser).type)) {
            // Unary operator - delegate to PARSE_UNARY_OPERATOR
            Token op_token = current_token(parser);
            advance(parser);

            // Push continuation to come back and continue parsing expression
            push_new_frame(parser, PARSE_EXPRESSION, frame->parent_node_idx, -1, 1);

            // Push operator token and delegate to PARSE_UNARY_OPERATOR
            array_append(parser->token_stack, &op_token);
            push_new_frame(parser, PARSE_UNARY_OPERATOR, frame->parent_node_idx, -1, 0);
            return;
        } else {
            // Unknown - end of expression
            push_new_frame(parser, PARSE_EXPRESSION, frame->parent_node_idx, -1, 2);
            return;
        }

        // Push primary to stack
        if (primary_idx != -1) {
            array_append(parser->expr_stack, &primary_idx);
        }

        // Check for binary operator
        if (is_binary_operator(current_token(parser).type)) {
            // Binary operator - delegate to PARSE_BINARY_EXPRESSION
            Token op_token = current_token(parser);
            advance(parser);

            // Push continuation to come back and continue parsing expression
            push_new_frame(parser, PARSE_EXPRESSION, frame->parent_node_idx, -1, 1);

            // Push operator token and delegate to PARSE_BINARY_EXPRESSION
            array_append(parser->token_stack, &op_token);
            push_new_frame(parser, PARSE_BINARY_EXPRESSION, frame->parent_node_idx, -1, 0);
            return;
        }

        // Continue parsing
        push_new_frame(parser, PARSE_EXPRESSION, frame->parent_node_idx, -1, 1);
        return;
    }

    if (frame->step == 2) {
        // Finalize: pop single item from stack and add to parent
        if (array_length(parser->expr_stack) == 0) {
            new_error(parser, frame->parent_node_idx, "Empty expression");
            return;
        }

        int expr_idx;
        array_pop(parser->expr_stack, &expr_idx);
        add_child(parser->tree, frame->parent_node_idx, expr_idx);
        return;
    }
}

static void parse_call_args(Parser *parser, ParserStackFrame *frame) {
    // Parsing: ( [expr [, expr]*] )
    // Step 0: Create arg_list node and consume LPAREN
    // Step 1: Parse arguments

    int arg_list_idx = frame->current_node_idx;

    if (frame->step == 0) {
        // Create arg list node
        ParseNode arg_list_node = create_nonterminal_node(NODE_ARG_LIST);
        arg_list_idx = add_node(parser->tree, &arg_list_node);
        add_child(parser->tree, frame->parent_node_idx, arg_list_idx);

        // Consume LPAREN
        if (!match_token(parser, TOKEN_LPAREN)) {
            new_error(parser, arg_list_idx, "Expected '(' for argument list");
            return;
        }
        advance(parser);

        // Continue to step 1
        push_new_frame(parser, PARSE_CALL_ARGS, frame->parent_node_idx, arg_list_idx, 1);
        return;
    }

    if (frame->step == 1) {
        // Parse arguments
        while (!match_token(parser, TOKEN_RPAREN) && !match_token(parser, TOKEN_EOF)) {
            // Consume whitespace
            consume_whitespace(parser, arg_list_idx);

            // Check for RPAREN after consuming whitespace
            if (match_token(parser, TOKEN_RPAREN)) {
                break;
            }

            // Check for comma (between args)
            if (match_token(parser, TOKEN_COMMA)) {
                advance(parser);
                continue;
            }

            // Parse argument using PARSE_EXPRESSION
            // Push continuation to come back and parse more arguments
            push_new_frame(parser, PARSE_CALL_ARGS, frame->parent_node_idx, arg_list_idx, 1);
            // Push PARSE_EXPRESSION to parse the argument
            push_new_frame(parser, PARSE_EXPRESSION, arg_list_idx, -1, 0);
            return;
        }

        // Expect RPAREN
        if (!match_token(parser, TOKEN_RPAREN)) {
            new_error(parser, arg_list_idx, "Expected ')' after arguments");
            return;
        }
        advance(parser);
    }
}

static void parse_return_statement(Parser *parser, ParserStackFrame *frame) {
    // Parsing: return expression

    // Consume 'return' keyword
    if (!match_token(parser, TOKEN_RETURN)) {
        new_error(parser, frame->parent_node_idx, "Expected 'return' keyword");
        return;
    }
    advance(parser);

    // Create return statement node
    ParseNode return_node = create_nonterminal_node(NODE_RETURN_STMT);
    int return_idx = add_node(parser->tree, &return_node);
    add_child(parser->tree, frame->parent_node_idx, return_idx);

    // Parse the return value expression
    push_new_frame(parser, PARSE_EXPRESSION, return_idx, -1, 0);
}

static void parse_match_expr(Parser *parser, ParserStackFrame *frame) {
    // Parsing: match <subject_expr> { <pattern>: <expr> ... }
    // Uses step to track progress through parsing stages

    int match_idx = frame->current_node_idx;

    if (frame->step == 0) {
        // Step 0: Create match node and parse subject
        // Consume 'match' keyword
        if (!match_token(parser, TOKEN_MATCH)) {
            new_error(parser, frame->parent_node_idx, "Expected 'match' keyword");
            return;
        }
        advance(parser);

        // Create match expression node
        ParseNode match_node = create_nonterminal_node(NODE_MATCH_EXPR);
        match_idx = add_node(parser->tree, &match_node);
        add_child(parser->tree, frame->parent_node_idx, match_idx);

        // Parse subject expression
        push_new_frame(parser, PARSE_MATCH_EXPR, match_idx, match_idx, 1);
        push_new_frame(parser, PARSE_EXPRESSION, match_idx, -1, 0);
        return;
    }

    if (frame->step == 1) {
        // Step 1: Subject parsed, expect '{'
        if (!match_token(parser, TOKEN_LBRACE)) {
            new_error(parser, match_idx, "Expected '{' after match subject");
            return;
        }
        advance(parser);

        // Continue to parse arms
        push_new_frame(parser, PARSE_MATCH_EXPR, match_idx, match_idx, 2);
        return;
    }

    // Step 2: Parse match arms
    if (frame->step == 2) {
        // Consume whitespace
        consume_whitespace(parser, match_idx);

        // Check if we're done parsing arms
        if (match_token(parser, TOKEN_RBRACE)) {
            advance(parser);
            return;
        }

        if (match_token(parser, TOKEN_EOF)) {
            new_error(parser, match_idx, "Expected '}' after match");
            return;
        }

        // Create match arm node
        ParseNode arm_node = create_nonterminal_node(NODE_MATCH_ARM);
        int arm_idx = add_node(parser->tree, &arm_node);
        add_child(parser->tree, match_idx, arm_idx);

        // Parse pattern (boolean literal, string literal, or wildcard)
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
            new_error(parser, arm_idx, "Expected pattern (boolean, string, or '_')");
            return;
        }

        ParseNode pattern_node = create_terminal_node(pattern_type, current_token(parser));
        int pattern_idx = add_node(parser->tree, &pattern_node);
        add_child(parser->tree, arm_idx, pattern_idx);
        advance(parser);

        // Expect colon
        if (!match_token(parser, TOKEN_COLON)) {
            new_error(parser, arm_idx, "Expected ':' after pattern");
            return;
        }
        advance(parser);

        // Parse arm expression - use existing expression parsing
        // Push continuation to come back and parse more arms
        push_new_frame(parser, PARSE_MATCH_EXPR, match_idx, match_idx, 2);
        // Push PARSE_EXPRESSION to parse the arm's expression
        push_new_frame(parser, PARSE_EXPRESSION, arm_idx, -1, 0);
        return;
    }
}

static void parse_match_stmt(Parser *parser, ParserStackFrame *frame) {
    // Parsing: match <subject_expr> { <pattern>: <statement> ... }
	
    // Create match statement node
    ParseNode match_node = create_nonterminal_node(NODE_MATCH_STMT);
    int match_idx = add_node(parser->tree, &match_node);
    add_child(parser->tree, frame->parent_node_idx, match_idx);

    push_new_frame(parser, PARSE_MATCH_EXPR, match_idx, -1, 0);
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
    push_new_frame(parser, PARSE, root_idx, root_idx, 0);

    // Main parsing loop - iterative stack-based parsing
#ifdef DEBUG_PARSER_LOOP
    int iteration_count = 0;
#endif
    while (array_length(parser->stack) > 0) {
        ParserStackFrame frame;
        if (!pop_frame(parser, &frame)) {
            break;
        }

#ifdef DEBUG_PARSER_LOOP
        iteration_count++;
        fprintf(stderr, "[PARSE LOOP %d] State: %d, Step: %d, Stack depth: %zu, Current token: %d\n",
                iteration_count, frame.state, frame.step, array_length(parser->stack),
                current_token(parser).type);

        if (iteration_count > 1000) {
            fprintf(stderr, "ERROR: Infinite loop detected! Breaking...\n");
            break;
        }
#endif

        switch (frame.state) {

        case PARSE: {
            parse_top_level(parser, &frame);
            break;
        }

        case PARSE_STATEMENT: {
            parse_statement(parser, &frame);
            break;
        }

        case PARSE_FUNCTION_DECL: {
            parse_function_declaration(parser, &frame);
            break;
        }

        case PARSE_PARAM_LIST: {
            parse_param_list(parser, &frame);
            break;
        }

        case PARSE_BLOCK: {
            parse_block(parser, &frame);
            break;
        }

        case PARSE_VAR_DECL: {
            parse_variable_declaration(parser, &frame);
            break;
        }

        case PARSE_CALL_STATEMENT: {
            parse_call_statement(parser, &frame);
            break;
        }

        case PARSE_CALL_EXPRESSION: {
            parse_call_expression(parser, &frame);
            break;
        }

        case PARSE_EXPRESSION: {
            parse_expression(parser, &frame);
            break;
        }

        case PARSE_BINARY_EXPRESSION: {
            parse_binary_expression(parser, &frame);
            break;
        }

        case PARSE_UNARY_OPERATOR: {
            parse_unary_operator(parser, &frame);
            break;
        }

        case PARSE_CALL_ARGS: {
            parse_call_args(parser, &frame);
            break;
        }

        case PARSE_MATCH_EXPR: {
            parse_match_expr(parser, &frame);
            break;
        }

        case PARSE_MATCH_STMT: {
            parse_match_stmt(parser, &frame);
            break;
        }

        case PARSE_RETURN_STATEMENT: {
            parse_return_statement(parser, &frame);
            break;
        }
        }
    }

    return parser->tree;
}
