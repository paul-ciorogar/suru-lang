#include "parse_tree.h"
#include "arena.h"
#include "array.h"
#include "lexer.h"
#include <stdio.h>
#include <string.h>

// Create a new parse tree
ParseTree *create_parse_tree(Arena *arena) {
    if (!arena) {
        return NULL;
    }

    ParseTree *tree = arena_alloc(arena, sizeof(ParseTree));
    if (!tree) {
        return NULL;
    }

    tree->nodes = array_init(sizeof(ParseNode));
    if (!tree->nodes) {
        return NULL;
    }

    tree->arena = arena;
    tree->root = -1; // No root initially

    return tree;
}

// Add a node to the parse tree, returns the node's index
int add_node(ParseTree *tree, ParseNode *node) {
    if (!tree || !node) {
        return -1;
    }

    // Get the index before adding (this will be the new node's index)
    int index = (int)array_length(tree->nodes);

    // Add node to array
    if (!array_append(tree->nodes, node)) {
        return -1;
    }

    // If this is the first node, make it root
    if (tree->root == -1) {
        tree->root = index;
    }

    return index;
}

// Get a node by index
ParseNode *get_node(ParseTree *tree, int index) {
    if (!tree || index < 0) {
        return NULL;
    }

    return (ParseNode *)array_get(tree->nodes, (size_t)index);
}

// Add a child node to a parent (handles sibling linking automatically)
void add_child(ParseTree *tree, int parent_idx, int child_idx) {
    if (!tree || parent_idx < 0 || child_idx < 0) {
        return;
    }

    ParseNode *parent = get_node(tree, parent_idx);
    ParseNode *child = get_node(tree, child_idx);

    if (!parent || !child) {
        return;
    }

    // Set child's parent
    child->parent = parent_idx;

    // If parent has no children, this becomes the first child
    if (parent->first_child == -1) {
        parent->first_child = child_idx;
    } else {
        // Find the last sibling and append
        int sibling_idx = parent->first_child;
        ParseNode *sibling = get_node(tree, sibling_idx);

        while (sibling && sibling->next_sibling != -1) {
            sibling_idx = sibling->next_sibling;
            sibling = get_node(tree, sibling_idx);
        }

        if (sibling) {
            sibling->next_sibling = child_idx;
        }
    }
}

// Traverse all children of a node, calling callback for each
void traverse_children(ParseTree *tree, int parent_idx, NodeCallback callback, void *data) {
    if (!tree || parent_idx < 0 || !callback) {
        return;
    }

    ParseNode *parent = get_node(tree, parent_idx);
    if (!parent) {
        return;
    }

    int child_idx = parent->first_child;
    while (child_idx != -1) {
        ParseNode *child = get_node(tree, child_idx);
        if (!child) {
            break;
        }

        // Call the callback
        callback(tree, child_idx, data);

        // Move to next sibling
        child_idx = child->next_sibling;
    }
}

// Get the number of children for a node
int get_child_count(ParseTree *tree, int parent_idx) {
    if (!tree || parent_idx < 0) {
        return 0;
    }

    ParseNode *parent = get_node(tree, parent_idx);
    if (!parent) {
        return 0;
    }

    int count = 0;
    int child_idx = parent->first_child;

    while (child_idx != -1) {
        count++;
        ParseNode *child = get_node(tree, child_idx);
        if (!child) {
            break;
        }
        child_idx = child->next_sibling;
    }

    return count;
}

// Create a non-terminal node
ParseNode create_nonterminal_node(ParseNodeType type) {
    ParseNode node;
    node.type = type;

    // Initialize empty token
    node.token.type = TOKEN_UNKNOWN;
    node.token.text = NULL;
    node.token.length = 0;
    node.token.line = 0;
    node.token.column = 0;

    node.first_child = -1;
    node.next_sibling = -1;
    node.parent = -1;
    node.leading_spaces = 0;
    node.trailing_spaces = 0;
    node.leading_newlines = 0;
    return node;
}

// Create a terminal node from a token
ParseNode create_terminal_node(ParseNodeType type, Token token) {
    ParseNode node;
    node.type = type;
    node.token = token;

    node.first_child = -1;
    node.next_sibling = -1;
    node.parent = -1;
    node.leading_spaces = 0;
    node.trailing_spaces = 0;
    node.leading_newlines = 0;
    return node;
}

// Free parse tree
void free_parse_tree(ParseTree *tree) {
    if (!tree) {
        return;
    }

    // Free the array (but not the arena - caller manages that)
    if (tree->nodes) {
        array_free(tree->nodes);
    }

    // Note: We don't free the tree itself since it was allocated from arena
    // The arena will be freed by the caller
}
