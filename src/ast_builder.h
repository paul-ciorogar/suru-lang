#ifndef AST_BUILDER_H
#define AST_BUILDER_H

#include "ast.h"
#include "parse_tree.h"
#include "arena.h"

// Build an AST from a parse tree, filtering out formatting nodes
AST *build_ast_from_parse_tree(Arena *arena, ParseTree *tree);

#endif // AST_BUILDER_H
