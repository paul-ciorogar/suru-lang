#ifndef SYMBOL_TABLE_H
#define SYMBOL_TABLE_H

#include "arena.h"
#include "array.h"
#include "string_storage.h"

// Symbol types
typedef enum {
    SYMBOL_FUNCTION,
    SYMBOL_VARIABLE,
} SymbolType;

// Symbol entry
typedef struct {
    SymbolType type;
    String *name;
    int ast_node_idx;      // Index of the AST node (e.g., AST_FUNCTION_DECL)
    int parent_scope_id;   // ID of parent scope (-1 for global)
    int scope_id;          // Unique ID for this symbol's scope
} Symbol;

// Symbol table - flat array with scope IDs for hierarchy
typedef struct {
    Array *symbols;        // Array of Symbol
    Arena *arena;
    int next_scope_id;     // Counter for generating unique scope IDs
} SymbolTable;

// Create a new symbol table
SymbolTable *create_symbol_table(Arena *arena);

// Add a function symbol to the table
// Returns the scope_id assigned to this function
int add_function_symbol(SymbolTable *table, String *name, int ast_node_idx, int parent_scope_id);

// Add a variable symbol to the table
void add_variable_symbol(SymbolTable *table, String *name, int ast_node_idx, int parent_scope_id);

// Lookup a symbol by name in a given scope (searches parent scopes too)
Symbol *lookup_symbol(SymbolTable *table, String *name, int scope_id);

// Lookup a function symbol specifically
Symbol *lookup_function(SymbolTable *table, String *name, int scope_id);

#endif // SYMBOL_TABLE_H
