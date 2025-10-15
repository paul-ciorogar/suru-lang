#ifndef PARSE_TREE_H
#define PARSE_TREE_H

#include "array.h"
#include "arena.h"
#include "lexer.h"

// Parse node types - both terminal (tokens) and non-terminal (grammar rules)
typedef enum {
    // Terminal nodes (from lexer tokens)
    NODE_IDENTIFIER,
    NODE_NUMBER,
    NODE_STRING,
    NODE_KEYWORD,
    NODE_OPERATOR,
    NODE_PUNCTUATION,
    NODE_COMMENT,
    NODE_DOCUMENTATION,
    NODE_WHITESPACE,
    NODE_NEWLINE,

    // Non-terminal nodes (grammar constructs)
    NODE_PROGRAM,           // Root node
    NODE_MODULE_DECL,       // module Name
    NODE_IMPORT_DECL,       // import { ... }
    NODE_EXPORT_DECL,       // export { ... }
    NODE_TYPE_DECL,         // type Name: ...
    NODE_FUNCTION_DECL,     // name: (...) Type { ... }
    NODE_VARIABLE_DECL,     // name: value or name Type: value
    NODE_STRUCT_LITERAL,    // { field: value, ... }
    NODE_MATCH_EXPR,        // match value { ... }
    NODE_PIPELINE_EXPR,     // value | fn1 | fn2
    NODE_FUNCTION_CALL,     // name(args)
    NODE_BINARY_OP,         // a + b, a.b, etc
    NODE_UNARY_OP,          // -a, not a
    NODE_STATEMENT_LIST,    // List of statements
    NODE_PARAMETER_LIST,    // Function parameters
    NODE_ARGUMENT_LIST,     // Function call arguments
    NODE_TYPE_ANNOTATION,   // Type specification
    NODE_BLOCK,             // { ... }
    NODE_ARRAY_LITERAL,     // [1, 2, 3]
    NODE_STRING_INTERP,     // String interpolation `text {expr}`
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

// Add a node to the parse tree, returns the node's index
int add_node(ParseTree *tree, ParseNode *node);

// Get a node by index (returns NULL if invalid index)
ParseNode *get_node(ParseTree *tree, int index);

// Add a child node to a parent (handles sibling linking automatically)
void add_child(ParseTree *tree, int parent_idx, int child_idx);

// Create a terminal node from a token
ParseNode create_terminal_node(Token token, ParseNodeType type);

// Create a non-terminal node
ParseNode create_nonterminal_node(ParseNodeType type);

// Traverse all children of a node, calling callback for each
typedef void (*NodeCallback)(ParseTree *tree, int node_idx, void *data);
void traverse_children(ParseTree *tree, int parent_idx, NodeCallback callback, void *data);

// Get the number of children for a node
int get_child_count(ParseTree *tree, int parent_idx);

// Free parse tree (note: doesn't free arena, that's caller's responsibility)
void free_parse_tree(ParseTree *tree);

#endif // PARSE_TREE_H
