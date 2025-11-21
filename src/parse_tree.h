#ifndef PARSE_TREE_H
#define PARSE_TREE_H

#include "array.h"
#include "arena.h"
#include "lexer.h"

// Parse node types - both terminal (tokens) and non-terminal (grammar rules)
typedef enum {
    // Non-terminal nodes (grammar rules)
    NODE_PROGRAM,
    NODE_FUNCTION_DECL,
    NODE_PARAM_LIST,
    NODE_PARAM,
    NODE_BLOCK,
    NODE_VAR_DECL,
    NODE_CALL_EXPR,
    NODE_ARG_LIST,
    NODE_MATCH_EXPR,     // match expression
    NODE_MATCH_STMT,     // match statement
    NODE_MATCH_ARM,      // match arm (pattern: expression)
    NODE_RETURN_STMT,    // return statement

    // Expression nodes - binary operations
    NODE_AND_EXPR,       // and
    NODE_OR_EXPR,        // or
    NODE_PLUS_EXPR,      // + (composition)
    NODE_PIPE_EXPR,      // | (pipeline)

    // Expression nodes - unary operations
    NODE_NOT_EXPR,       // not
    NODE_NEGATE_EXPR,    // - (unary minus)

    // Terminal nodes (tokens)
    NODE_IDENTIFIER,
    NODE_STRING_LITERAL,
    NODE_BOOLEAN_LITERAL,
    NODE_MATCH_WILDCARD, // _ wildcard pattern
    NODE_COMMENT,
    NODE_NEWLINE,
} ParseNodeType;

// Parse tree node - uniform size for array storage
// Uses first-child/next-sibling representation
typedef struct {
    ParseNodeType type;

    // Token information (for terminal nodes)
    Token token;

    // Tree structure using indices (not pointers)
    int first_child;   // Index of first child (-1 if none)
    int next_sibling;  // Index of next sibling (-1 if none)
    int parent;        // Index of parent node (-1 if root)

    // Formatting metadata - preserve original spacing
    int leading_spaces;   // Spaces before this node
    int trailing_spaces;  // Spaces after this node
    int leading_newlines; // Newlines before this node

} ParseNode;

// Parse tree structure
typedef struct {
    Array *nodes;     // Array of ParseNode (using array.h)
    Arena *arena;     // Arena for memory allocation
    int root;         // Index of root node (-1 if empty)
} ParseTree;

// Create a new parse tree
ParseTree *create_parse_tree(Arena *arena);

// Create a non-terminal node
ParseNode create_nonterminal_node(ParseNodeType type);

// Create a terminal node from a token
ParseNode create_terminal_node(ParseNodeType type, Token token);

// Add a node to the parse tree, returns the node's index
int add_node(ParseTree *tree, ParseNode *node);

// Get a node by index (returns NULL if invalid index)
ParseNode *get_node(ParseTree *tree, int index);

// Add a child node to a parent (handles sibling linking automatically)
void add_child(ParseTree *tree, int parent_idx, int child_idx);

// Traverse all children of a node, calling callback for each
typedef void (*NodeCallback)(ParseTree *tree, int node_idx, void *data);
void traverse_children(ParseTree *tree, int parent_idx, NodeCallback callback, void *data);

// Get the number of children for a node
int get_child_count(ParseTree *tree, int parent_idx);

// Free parse tree (note: doesn't free arena, that's caller's responsibility)
void free_parse_tree(ParseTree *tree);

#endif // PARSE_TREE_H
