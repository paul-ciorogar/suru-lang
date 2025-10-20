#ifndef INTERPRETER_H
#define INTERPRETER_H

#include "ast.h"
#include "arena.h"
#include "array.h"

// Variable binding (name -> value)
// Both name and value are String* from string storage
typedef struct {
    String *name;
    String *value;
} Variable;

// Interpreter context
typedef struct Interpreter {
    Arena *arena;
    AST *ast;
    Array *variables;  // Array of Variable
} Interpreter;

// Create a new interpreter
Interpreter *create_interpreter(Arena *arena, AST *ast);

// Execute the AST
// Returns 0 on success, non-zero on error
int interpret(Interpreter *interpreter);

#endif // INTERPRETER_H
