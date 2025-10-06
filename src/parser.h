#ifndef PARSER_H
#define PARSER_H


#include "arena.h"
#include "lexer.h"
#include <unistd.h>

typedef struct ParserError {
    struct ParserError *next;
    int line;
    int column;
    char *message;
} ParserError;

typedef struct ParserErrors {
    ParserError *head;
    ParserError *tail;
} ParserErrors;

typedef struct Parser {
    Arena *arena;
    Lexer *lexer;
    ParserErrors *errors;
} Parser;

typedef struct ASTNode {
} ASTNode;

Parser *create_parser(Arena *arena, Lexer *lexer) ;

ASTNode *parse_statement(Parser *parser);

#endif
