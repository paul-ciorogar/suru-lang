#ifndef PARSER_H
#define PARSER_H

#include "arena.h"
#include "array.h"
#include "lexer.h"
#include "parse_tree.h"
#include <unistd.h>

// Parser state machine states
typedef enum {
    PARSE,                  // Top-level parsing
    PARSE_STATEMENT,        // Parse any statement (variable decl, call expr, etc.)
    PARSE_FUNCTION_DECL,    // Parsing function declaration
    PARSE_VAR_DECL,         // Parsing variable declaration
    PARSE_PARAM_LIST,       // Parsing parameter list
    PARSE_BLOCK,            // Parsing block statements
    PARSE_EXPRESSION,       // Parsing expressions
    PARSE_CALL_ARGS,        // Parsing function call arguments
} ParserState;

// Stack frame for parsing
typedef struct {
    ParserState state;
    int parent_node_idx;      // Index of parent node in parse tree
    int current_node_idx;     // Index of node being built (-1 if none)
    int precedence;           // Operator precedence level for expression parsing
} ParserStackFrame;

typedef struct ParserError {
    struct ParserError *next;
    int line;
    int column;
    char *message;
} ParserError;

typedef struct ParserErrors {
    ParserError *head;
    ParserError *tail;
    int count;
} ParserErrors;

typedef struct Parser {
    Arena *arena;
    Lexer *lexer;
    ParserErrors *errors;
    ParseTree *tree;
    Array *stack;  // Stack of ParserStackFrame
} Parser;

Parser *create_parser(Arena *arena, Lexer *lexer);

// Parse source code and build parse tree with syntax analysis
ParseTree *parse(Parser *parser);

#endif
