#ifndef PARSER_H
#define PARSER_H

#include "arena.c"
#include "arena.h"
#include "lexer.c"
#include <unistd.h>

typedef struct Parser {
    Arena *arena;
    Lexer *lexer;
} Parser;

typedef struct ASTNode {
} ASTNode;

Parser *create_parser(Arena *arena, Lexer *lexer) { return NULL; }

ASTNode *parse_statement(Parser *parser) { return NULL; }

#endif
