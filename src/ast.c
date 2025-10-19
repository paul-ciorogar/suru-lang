#include "ast.h"
#include "arena.h"
#include "array.h"
#include <stdlib.h>
#include <string.h>

AST *create_ast(Arena *arena) {
    AST *ast = arena_alloc(arena, sizeof(AST));
    if (!ast) {
        return NULL;
    }

    ast->nodes = array_init(sizeof(ASTNode));
    if (!ast->nodes) {
        return NULL;
    }

    ast->arena = arena;
    ast->root = -1;

    return ast;
}

ASTNode create_ast_nonterminal(ASTNodeType type) {
    ASTNode node;
    memset(&node, 0, sizeof(ASTNode));
    node.type = type;
    node.first_child = -1;
    node.next_sibling = -1;
    node.parent = -1;
    return node;
}

ASTNode create_ast_terminal(ASTNodeType type, Token token) {
    ASTNode node = create_ast_nonterminal(type);
    node.token = token;
    return node;
}

int add_ast_node(AST *ast, ASTNode *node) {
    if (!ast || !ast->nodes || !node) {
        return -1;
    }

    size_t index = array_length(ast->nodes);
    array_append(ast->nodes, node);
    return (int)index;
}

ASTNode *get_ast_node(AST *ast, int index) {
    if (!ast || !ast->nodes || index < 0) {
        return NULL;
    }

    if ((size_t)index >= array_length(ast->nodes)) {
        return NULL;
    }

    return (ASTNode *)array_get(ast->nodes, (size_t)index);
}

void add_ast_child(AST *ast, int parent_idx, int child_idx) {
    if (!ast) {
        return;
    }

    ASTNode *parent = get_ast_node(ast, parent_idx);
    ASTNode *child = get_ast_node(ast, child_idx);

    if (!parent || !child) {
        return;
    }

    // Set parent reference
    child->parent = parent_idx;

    // If parent has no children, this becomes the first child
    if (parent->first_child == -1) {
        parent->first_child = child_idx;
        return;
    }

    // Otherwise, append to the end of sibling list
    int current_idx = parent->first_child;
    ASTNode *current = get_ast_node(ast, current_idx);

    while (current && current->next_sibling != -1) {
        current_idx = current->next_sibling;
        current = get_ast_node(ast, current_idx);
    }

    if (current) {
        current->next_sibling = child_idx;
    }
}

void traverse_ast_children(AST *ast, int parent_idx, ASTNodeCallback callback, void *data) {
    if (!ast || !callback) {
        return;
    }

    ASTNode *parent = get_ast_node(ast, parent_idx);
    if (!parent || parent->first_child == -1) {
        return;
    }

    int child_idx = parent->first_child;
    while (child_idx != -1) {
        ASTNode *child = get_ast_node(ast, child_idx);
        if (!child) {
            break;
        }

        callback(ast, child_idx, data);
        child_idx = child->next_sibling;
    }
}

int get_ast_child_count(AST *ast, int parent_idx) {
    if (!ast) {
        return 0;
    }

    ASTNode *parent = get_ast_node(ast, parent_idx);
    if (!parent || parent->first_child == -1) {
        return 0;
    }

    int count = 0;
    int child_idx = parent->first_child;
    while (child_idx != -1) {
        count++;
        ASTNode *child = get_ast_node(ast, child_idx);
        if (!child) {
            break;
        }
        child_idx = child->next_sibling;
    }

    return count;
}

void free_ast(AST *ast) {
    if (!ast) {
        return;
    }

    if (ast->nodes) {
        array_free(ast->nodes);
    }

    // Note: We don't free the arena here - that's the caller's responsibility
}
