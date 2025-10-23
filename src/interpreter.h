#ifndef INTERPRETER_H
#define INTERPRETER_H

#include "ast.h"
#include "arena.h"
#include "array.h"

// Value types
typedef enum {
    VALUE_STRING,
    VALUE_BOOLEAN,
} ValueType;

// Variable binding (name -> value)
typedef struct {
    String *name;
    ValueType type;
    union {
        String *string_value;  // For strings (includes quotes)
        int bool_value;        // For booleans (0 = false, 1 = true)
    };
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
