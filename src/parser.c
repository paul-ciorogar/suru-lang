#include "parser.h"
#include "arena.h"
#include "lexer.h"
#include <stdio.h>
#include <unistd.h>

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

ASTNode *parse_statement(Parser *parser) { return NULL; }
