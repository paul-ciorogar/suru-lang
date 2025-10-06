#include "parser.h"
#include "arena.h"
#include "lexer.h"
#include <stdio.h>
#include <unistd.h>

static const char* token_type_to_string(TokenType type) {
    switch (type) {
        case TOKEN_EOF: return "TOKEN_EOF";
        case TOKEN_NEWLINE: return "TOKEN_NEWLINE";
        case TOKEN_MODULE: return "TOKEN_MODULE";
        case TOKEN_IMPORT: return "TOKEN_IMPORT";
        case TOKEN_EXPORT: return "TOKEN_EXPORT";
        case TOKEN_RETURN: return "TOKEN_RETURN";
        case TOKEN_MATCH: return "TOKEN_MATCH";
        case TOKEN_TYPE: return "TOKEN_TYPE";
        case TOKEN_TRY: return "TOKEN_TRY";
        case TOKEN_AND: return "TOKEN_AND";
        case TOKEN_OR: return "TOKEN_OR";
        case TOKEN_TRUE: return "TOKEN_TRUE";
        case TOKEN_FALSE: return "TOKEN_FALSE";
        case TOKEN_THIS: return "TOKEN_THIS";
        case TOKEN_PARTIAL: return "TOKEN_PARTIAL";
        case TOKEN_IDENTIFIER: return "TOKEN_IDENTIFIER";
        case TOKEN_NUMBER_BINARY: return "TOKEN_NUMBER_BINARY";
        case TOKEN_NUMBER_OCTAL: return "TOKEN_NUMBER_OCTAL";
        case TOKEN_NUMBER_FLOAT: return "TOKEN_NUMBER_FLOAT";
        case TOKEN_NUMBER: return "TOKEN_NUMBER";
        case TOKEN_UNKNOWN: return "TOKEN_UNKNOWN";
        default: return "UNKNOWN_TOKEN_TYPE";
    }
}

Parser *create_parser(Arena *arena, Lexer *lexer) {
    Parser *parser = arena_alloc(arena, sizeof(Parser));

    parser->lexer = lexer;
    parser->arena = arena;
    parser->errors = NULL;

    return parser;
}

static void advance_parser(Parser *parser) {
    parser->lexer->current_token = next_token(parser->lexer);
}

ASTNode *parse_statement(Parser *parser) {

    while (parser->lexer->current_token.type != TOKEN_EOF) {
        printf("Token: %s", token_type_to_string(parser->lexer->current_token.type));
        if (parser->lexer->current_token.text != NULL) {
            printf(" Text: %s", parser->lexer->current_token.text->data);
        }
        printf("\n");
        advance_parser(parser);
    }

    return NULL;
}
