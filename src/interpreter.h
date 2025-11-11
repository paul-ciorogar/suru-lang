#ifndef INTERPRETER_H
#define INTERPRETER_H

#include "ast.h"
#include "arena.h"
#include "array.h"

// Value types
typedef enum {
    VALUE_STRING,
    VALUE_BOOLEAN,
    VALUE_FUNCTION,
} ValueType;

// Forward declaration for Scope
typedef struct Scope Scope;

// Function value (reference to AST node + closure scope)
typedef struct {
    int function_node_idx;  // Index of AST_FUNCTION_DECL node
    Scope *closure_scope;   // Scope where function was defined (for nested functions)
} FunctionValue;

// Variable binding (name -> value)
typedef struct {
    String *name;
    ValueType type;
    union {
        String *string_value;    // For strings (includes quotes)
        int bool_value;          // For booleans (0 = false, 1 = true)
        FunctionValue func_value;// For functions
    };
} Variable;

// Scope - represents a lexical scope with local variables
struct Scope {
    Scope *parent;           // Parent scope (NULL for global scope)
    int scope_id;            // Scope ID from symbol table
    Array *variables;        // Array of Variable (local variables)
};

// Interpreter context
typedef struct Interpreter {
    Arena *arena;
    AST *ast;
    Scope *global_scope;     // Global scope
    Scope *current_scope;    // Current execution scope
} Interpreter;

// Create a new interpreter
Interpreter *create_interpreter(Arena *arena, AST *ast);

// Execute the AST
// Returns 0 on success, non-zero on error
int interpret(Interpreter *interpreter);

#endif // INTERPRETER_H
