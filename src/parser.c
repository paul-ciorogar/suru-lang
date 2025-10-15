#include "parser.h"
#include "arena.h"
#include "lexer.h"
#include "parse_tree.h"
#include <stdio.h>
#include <unistd.h>

Parser *create_parser(Arena *arena, Lexer *lexer) {
    Parser *parser = arena_alloc(arena, sizeof(Parser));
    if (!parser) {
        return NULL;
    }

    parser->lexer = lexer;
    parser->arena = arena;
    parser->errors = NULL;
    parser->tree = create_parse_tree(arena);

    return parser;
}

// Get current token
static Token current_token(Parser *parser) {
    return parser->lexer->current_token;
}

// Advance to next token
static void advance(Parser *parser) {
    parser->lexer->current_token = next_token(parser->lexer);
}

// Map token type to parse node type
static ParseNodeType token_to_node_type(TokenType token_type) {
    switch (token_type) {
        case TOKEN_IDENTIFIER:
            return NODE_IDENTIFIER;
        case TOKEN_NUMBER:
        case TOKEN_NUMBER_BINARY:
        case TOKEN_NUMBER_OCTAL:
        case TOKEN_NUMBER_HEX:
        case TOKEN_NUMBER_FLOAT:
            return NODE_NUMBER;
        case TOKEN_STRING:
        case TOKEN_STRING_I_START:
        case TOKEN_STRING_I:
        case TOKEN_STRING_I_END:
            return NODE_STRING;
        case TOKEN_COMMENT:
            return NODE_COMMENT;
        case TOKEN_DOCUMENTATION:
            return NODE_DOCUMENTATION;
        case TOKEN_NEWLINE:
            return NODE_NEWLINE;
        case TOKEN_COLON:
        case TOKEN_SEMICOLON:
        case TOKEN_COMMA:
        case TOKEN_DOT:
        case TOKEN_PIPE:
        case TOKEN_STAR:
        case TOKEN_PLUS:
        case TOKEN_MINUS:
            return NODE_OPERATOR;
        case TOKEN_LPAREN:
        case TOKEN_RPAREN:
        case TOKEN_LBRACE:
        case TOKEN_RBRACE:
        case TOKEN_LBRACKET:
        case TOKEN_RBRACKET:
        case TOKEN_LANGLE:
        case TOKEN_RANGLE:
            return NODE_PUNCTUATION;
        default:
            return NODE_KEYWORD;
    }
}

// Add current token as a terminal node and advance
static int consume_token(Parser *parser) {
    Token token = current_token(parser);
    ParseNodeType node_type = token_to_node_type(token.type);
    ParseNode node = create_terminal_node(token, node_type);
    int node_idx = add_node(parser->tree, &node);
    advance(parser);
    return node_idx;
}

// Parse the source code and build parse tree
ParseTree *parse(Parser *parser) {
    if (!parser || !parser->tree) {
        return NULL;
    }

    // Create root program node
    ParseNode root = create_nonterminal_node(NODE_PROGRAM);
    int root_idx = add_node(parser->tree, &root);
    parser->tree->root = root_idx;

    // Parse all tokens and add them to the tree
    // For now, we'll just create a flat list of tokens as children of the root
    // A more sophisticated parser would recognize grammar rules and create
    // a proper hierarchical structure
    while (current_token(parser).type != TOKEN_EOF) {
        int token_idx = consume_token(parser);
        if (token_idx >= 0) {
            add_child(parser->tree, root_idx, token_idx);
        }
    }

    return parser->tree;
}
