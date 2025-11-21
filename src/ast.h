#ifndef AST_H
#define AST_H

#include "array.h"
#include "arena.h"
#include "lexer.h"
#include "symbol_table.h"

// AST node types - semantic nodes only (no formatting)
typedef enum {
    // Program structure
    AST_PROGRAM,
    AST_FUNCTION_DECL,
    AST_PARAM_LIST,
    AST_PARAM,
    AST_BLOCK,

    // Statements
    AST_VAR_DECL,
    AST_MATCH_STMT,      // match statement
    AST_RETURN_STMT,     // return statement

    // Expressions
    AST_CALL_EXPR,
    AST_ARG_LIST,
    AST_MATCH_EXPR,      // match expression
    AST_MATCH_ARM,       // match arm (pattern: expression)

    // Binary expressions
    AST_AND_EXPR,        // and
    AST_OR_EXPR,         // or
    AST_PLUS_EXPR,       // + (composition)
    AST_PIPE_EXPR,       // | (pipeline)

    // Unary expressions
    AST_NOT_EXPR,        // not
    AST_NEGATE_EXPR,     // - (unary minus)

    AST_IDENTIFIER,

    // Literals
    AST_STRING_LITERAL,
    AST_NUMBER_LITERAL,
    AST_BOOLEAN_LITERAL,
    AST_MATCH_WILDCARD,  // _ wildcard pattern
} ASTNodeType;

// AST node - uniform size for array storage
// Uses first-child/next-sibling representation (like ParseTree)
typedef struct {
    ASTNodeType type;

    // Token information (for terminals like identifiers, literals)
    Token token;

    // Tree structure using indices (not pointers)
    int first_child;   // Index of first child (-1 if none)
    int next_sibling;  // Index of next sibling (-1 if none)
    int parent;        // Index of parent node (-1 if root)
} ASTNode;

// AST tree structure
typedef struct {
    Array *nodes;          // Array of ASTNode
    Arena *arena;          // Arena for memory allocation
    int root;              // Index of root node (-1 if empty)
    SymbolTable *symbols;  // Symbol table for functions and variables
} AST;

// Create a new AST
AST *create_ast(Arena *arena);

// Create a non-terminal AST node
ASTNode create_ast_nonterminal(ASTNodeType type);

// Create a terminal AST node from a token
ASTNode create_ast_terminal(ASTNodeType type, Token token);

// Add a node to the AST, returns the node's index
int add_ast_node(AST *ast, ASTNode *node);

// Get a node by index (returns NULL if invalid index)
ASTNode *get_ast_node(AST *ast, int index);

// Add a child node to a parent (handles sibling linking automatically)
void add_ast_child(AST *ast, int parent_idx, int child_idx);

// Traverse all children of a node, calling callback for each
typedef void (*ASTNodeCallback)(AST *ast, int node_idx, void *data);
void traverse_ast_children(AST *ast, int parent_idx, ASTNodeCallback callback, void *data);

// Get the number of children for a node
int get_ast_child_count(AST *ast, int parent_idx);

// Free AST (note: doesn't free arena, that's caller's responsibility)
void free_ast(AST *ast);

#endif // AST_H
