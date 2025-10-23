#include "ast_builder.h"
#include "ast.h"
#include "parse_tree.h"
#include "arena.h"
#include <stdio.h>

// Context for AST building
typedef struct {
    Arena *arena;
    ParseTree *parse_tree;
    AST *ast;
} ASTBuildContext;

// Forward declaration
static int convert_node(ASTBuildContext *ctx, int parse_node_idx);

// Map ParseNodeType to ASTNodeType
// Returns -1 (cast to int) if no mapping exists
static int map_node_type(ParseNodeType parse_type) {
    switch (parse_type) {
    case NODE_PROGRAM:
        return AST_PROGRAM;
    case NODE_FUNCTION_DECL:
        return AST_FUNCTION_DECL;
    case NODE_VAR_DECL:
        return AST_VAR_DECL;
    case NODE_PARAM_LIST:
        return AST_PARAM_LIST;
    case NODE_PARAM:
        return AST_PARAM;
    case NODE_BLOCK:
        return AST_BLOCK;
    case NODE_CALL_EXPR:
        return AST_CALL_EXPR;
    case NODE_ARG_LIST:
        return AST_ARG_LIST;
    case NODE_AND_EXPR:
        return AST_AND_EXPR;
    case NODE_OR_EXPR:
        return AST_OR_EXPR;
    case NODE_PLUS_EXPR:
        return AST_PLUS_EXPR;
    case NODE_PIPE_EXPR:
        return AST_PIPE_EXPR;
    case NODE_NOT_EXPR:
        return AST_NOT_EXPR;
    case NODE_NEGATE_EXPR:
        return AST_NEGATE_EXPR;
    case NODE_IDENTIFIER:
        return AST_IDENTIFIER;
    case NODE_STRING_LITERAL:
        return AST_STRING_LITERAL;
    case NODE_BOOLEAN_LITERAL:
        return AST_BOOLEAN_LITERAL;
    default:
        // Formatting nodes (comments, newlines) are not mapped
        return -1;
    }
}

// Check if a parse node should be included in AST (exclude formatting nodes)
static int should_include_node(ParseNodeType type) {
    return type != NODE_COMMENT && type != NODE_NEWLINE;
}

// Recursively convert a parse tree node and its children to AST
static int convert_node(ASTBuildContext *ctx, int parse_node_idx) {
    if (parse_node_idx < 0) {
        return -1;
    }

    ParseNode *parse_node = get_node(ctx->parse_tree, parse_node_idx);
    if (!parse_node) {
        return -1;
    }

    // Skip formatting nodes
    if (!should_include_node(parse_node->type)) {
        return -1;
    }

    // Create corresponding AST node
    int ast_type_int = map_node_type(parse_node->type);
    if (ast_type_int == -1) {
        return -1;
    }

    ASTNodeType ast_type = (ASTNodeType)ast_type_int;
    ASTNode ast_node;
    if (parse_node->type == NODE_IDENTIFIER ||
        parse_node->type == NODE_STRING_LITERAL ||
        parse_node->type == NODE_BOOLEAN_LITERAL) {
        // Terminal nodes - preserve token
        ast_node = create_ast_terminal(ast_type, parse_node->token);
    } else {
        // Non-terminal nodes
        ast_node = create_ast_nonterminal(ast_type);
    }

    int ast_node_idx = add_ast_node(ctx->ast, &ast_node);

    // Recursively convert children
    if (parse_node->first_child != -1) {
        int child_idx = parse_node->first_child;
        while (child_idx != -1) {
            ParseNode *child = get_node(ctx->parse_tree, child_idx);
            if (!child) {
                break;
            }

            // Convert child (will skip formatting nodes)
            int ast_child_idx = convert_node(ctx, child_idx);
            if (ast_child_idx != -1) {
                add_ast_child(ctx->ast, ast_node_idx, ast_child_idx);
            }

            child_idx = child->next_sibling;
        }
    }

    return ast_node_idx;
}

AST *build_ast_from_parse_tree(Arena *arena, ParseTree *tree) {
    if (!arena || !tree) {
        return NULL;
    }

    // Create AST
    AST *ast = create_ast(arena);
    if (!ast) {
        return NULL;
    }

    // Set up context
    ASTBuildContext ctx;
    ctx.arena = arena;
    ctx.parse_tree = tree;
    ctx.ast = ast;

    // Convert from root
    int root_idx = convert_node(&ctx, tree->root);
    ast->root = root_idx;

    return ast;
}
