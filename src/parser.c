#ifndef PARSER_H
#define PARSER_H

#include "arena.h"
#include "lexer.h"
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
