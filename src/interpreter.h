#ifndef INTERPRETER_H
#define INTERPRETER_H

#include "ast.h"
#include "arena.h"

// Interpreter context
typedef struct Interpreter {
    Arena *arena;
    AST *ast;
} Interpreter;

// Create a new interpreter
Interpreter *create_interpreter(Arena *arena, AST *ast);

// Execute the AST
// Returns 0 on success, non-zero on error
int interpret(Interpreter *interpreter);

#endif // INTERPRETER_H
