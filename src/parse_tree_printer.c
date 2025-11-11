#include "parse_tree_printer.h"
#include "parse_tree.h"
#include <stdio.h>
#include <string.h>

// Get a human-readable name for a parse node type
static const char *node_type_name(ParseNodeType type) {
    switch (type) {
    // Non-terminal nodes
    case NODE_PROGRAM:
        return "PROGRAM";
    case NODE_FUNCTION_DECL:
        return "FUNCTION_DECL";
    case NODE_PARAM_LIST:
        return "PARAM_LIST";
    case NODE_PARAM:
        return "PARAM";
    case NODE_BLOCK:
        return "BLOCK";
    case NODE_CALL_EXPR:
        return "CALL_EXPR";
    case NODE_ARG_LIST:
        return "ARG_LIST";
    // Terminal nodes
    case NODE_IDENTIFIER:
        return "IDENTIFIER";
    case NODE_STRING_LITERAL:
        return "STRING_LITERAL";
    case NODE_COMMENT:
        return "COMMENT";
    case NODE_NEWLINE:
        return "NEWLINE";
    default:
        return "UNKNOWN";
    }
}

// Print a single node with indentation
static void print_node(ParseTree *tree, int node_idx, int depth) {
    if (node_idx < 0) {
        return;
    }

    ParseNode *node = get_node(tree, node_idx);
    if (!node) {
        return;
    }

    // Print indentation
    for (int i = 0; i < depth; i++) {
        printf("  ");
    }

    // Print node type
    printf("%s", node_type_name(node->type));

    // For terminal nodes, print the token text
    if (node->token.text && node->token.text->data) {
        printf(": ");
        // Print token text, escaping newlines and other special chars
        for (size_t i = 0; i < node->token.text->length; i++) {
            char c = node->token.text->data[i];
            if (c == '\n') {
                printf("\\n");
            } else if (c == '\t') {
                printf("\\t");
            } else if (c == '\r') {
                printf("\\r");
            } else if (c == '"') {
                printf("\\\"");
            } else if (c == '\\') {
                printf("\\\\");
            } else {
                printf("%c", c);
            }
        }
    }

    printf("\n");

    // Recursively print children
    int child_idx = node->first_child;
    while (child_idx >= 0) {
        print_node(tree, child_idx, depth + 1);
        ParseNode *child = get_node(tree, child_idx);
        child_idx = child ? child->next_sibling : -1;
    }
}

// Print the entire parse tree
void print_parse_tree(ParseTree *tree) {
    if (!tree) {
        printf("(null tree)\n");
        return;
    }

    if (tree->root < 0) {
        printf("(empty tree)\n");
        return;
    }

    print_node(tree, tree->root, 0);
}
