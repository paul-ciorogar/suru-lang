#include "interpreter.h"
#include "ast.h"
#include "arena.h"
#include <stdio.h>
#include <string.h>

// Forward declarations
static int execute_program(Interpreter *interp, int node_idx);
static int execute_function_decl(Interpreter *interp, int node_idx);
static int execute_block(Interpreter *interp, int node_idx);
static int execute_call_expr(Interpreter *interp, int node_idx);

Interpreter *create_interpreter(Arena *arena, AST *ast) {
    if (!arena || !ast) {
        return NULL;
    }

    Interpreter *interp = arena_alloc(arena, sizeof(Interpreter));
    if (!interp) {
        return NULL;
    }

    interp->arena = arena;
    interp->ast = ast;

    return interp;
}

int interpret(Interpreter *interp) {
    if (!interp || !interp->ast) {
        fprintf(stderr, "Error: Invalid interpreter state\n");
        return 1;
    }

    // Execute from root (should be AST_PROGRAM)
    return execute_program(interp, interp->ast->root);
}

// Execute AST_PROGRAM node
static int execute_program(Interpreter *interp, int node_idx) {
    ASTNode *node = get_ast_node(interp->ast, node_idx);
    if (!node || node->type != AST_PROGRAM) {
        fprintf(stderr, "Error: Invalid program node\n");
        return 1;
    }

    // Look for the main function and execute it
    int main_func_idx = -1;
    int child_idx = node->first_child;

    while (child_idx != -1) {
        ASTNode *child = get_ast_node(interp->ast, child_idx);
        if (!child) {
            break;
        }

        if (child->type == AST_FUNCTION_DECL) {
            // Check if this is the main function
            // First child of function decl should be the identifier
            ASTNode *func_name = get_ast_node(interp->ast, child->first_child);
            if (func_name && func_name->type == AST_IDENTIFIER) {
                // Compare the identifier name with "main"
                if (func_name->token.text && func_name->token.text->length == 4 &&
                    strncmp(func_name->token.text->data, "main", 4) == 0) {
                    main_func_idx = child_idx;
                    break;
                }
            }
        }

        child_idx = child->next_sibling;
    }

    if (main_func_idx == -1) {
        fprintf(stderr, "Error: No main function found\n");
        return 1;
    }

    // Execute main function
    return execute_function_decl(interp, main_func_idx);
}

// Execute AST_FUNCTION_DECL node (for now, just execute the body)
static int execute_function_decl(Interpreter *interp, int node_idx) {
    ASTNode *node = get_ast_node(interp->ast, node_idx);
    if (!node || node->type != AST_FUNCTION_DECL) {
        fprintf(stderr, "Error: Invalid function declaration\n");
        return 1;
    }

    // Function structure: IDENTIFIER, PARAM_LIST, BLOCK
    // Find the block (should be the last child)
    int child_idx = node->first_child;
    int block_idx = -1;

    while (child_idx != -1) {
        ASTNode *child = get_ast_node(interp->ast, child_idx);
        if (!child) {
            break;
        }

        if (child->type == AST_BLOCK) {
            block_idx = child_idx;
        }

        child_idx = child->next_sibling;
    }

    if (block_idx == -1) {
        fprintf(stderr, "Error: Function has no body\n");
        return 1;
    }

    // Execute the function body
    return execute_block(interp, block_idx);
}

// Execute AST_BLOCK node
static int execute_block(Interpreter *interp, int node_idx) {
    ASTNode *node = get_ast_node(interp->ast, node_idx);
    if (!node || node->type != AST_BLOCK) {
        fprintf(stderr, "Error: Invalid block\n");
        return 1;
    }

    // Execute each statement in the block
    int child_idx = node->first_child;
    while (child_idx != -1) {
        ASTNode *child = get_ast_node(interp->ast, child_idx);
        if (!child) {
            break;
        }

        // Execute statement (for now, only call expressions)
        if (child->type == AST_CALL_EXPR) {
            int result = execute_call_expr(interp, child_idx);
            if (result != 0) {
                return result;
            }
        }

        child_idx = child->next_sibling;
    }

    return 0;
}

// Execute AST_CALL_EXPR node
static int execute_call_expr(Interpreter *interp, int node_idx) {
    ASTNode *node = get_ast_node(interp->ast, node_idx);
    if (!node || node->type != AST_CALL_EXPR) {
        fprintf(stderr, "Error: Invalid call expression\n");
        return 1;
    }

    // Call structure: IDENTIFIER, ARG_LIST
    // Get function name
    ASTNode *func_name = get_ast_node(interp->ast, node->first_child);
    if (!func_name || func_name->type != AST_IDENTIFIER) {
        fprintf(stderr, "Error: Invalid function name in call\n");
        return 1;
    }

    // Get arg list (second child)
    int arg_list_idx = func_name->next_sibling;
    ASTNode *arg_list = get_ast_node(interp->ast, arg_list_idx);
    if (!arg_list || arg_list->type != AST_ARG_LIST) {
        fprintf(stderr, "Error: Invalid argument list\n");
        return 1;
    }

    // Check if this is a built-in function
    // For now, only support "print"
    if (func_name->token.text && func_name->token.text->length == 5 &&
        strncmp(func_name->token.text->data, "print", 5) == 0) {
        // Built-in print function
        // Expects one string argument

        // Get first argument
        int arg_idx = arg_list->first_child;
        if (arg_idx == -1) {
            fprintf(stderr, "Error: print() requires an argument\n");
            return 1;
        }

        ASTNode *arg = get_ast_node(interp->ast, arg_idx);
        if (!arg || arg->type != AST_STRING_LITERAL) {
            fprintf(stderr, "Error: print() requires a string argument\n");
            return 1;
        }

        // Print the string (without quotes)
        // The token includes quotes, so skip first and last character
        // Also handle escape sequences
        if (!arg->token.text) {
            fprintf(stderr, "Error: Invalid string literal\n");
            return 1;
        }
        const char *str = arg->token.text->data;
        int len = (int)arg->token.text->length;

        // Skip opening quote
        str++;
        len -= 2; // Skip both quotes

        // Print character by character, handling escape sequences
        for (int i = 0; i < len; i++) {
            if (str[i] == '\\' && i + 1 < len) {
                switch (str[i + 1]) {
                case 'n':
                    printf("\n");
                    i++;
                    break;
                case 't':
                    printf("\t");
                    i++;
                    break;
                case 'r':
                    printf("\r");
                    i++;
                    break;
                case '\\':
                    printf("\\");
                    i++;
                    break;
                case '"':
                    printf("\"");
                    i++;
                    break;
                default:
                    printf("%c", str[i]);
                    break;
                }
            } else {
                printf("%c", str[i]);
            }
        }

        return 0;
    }

    // Unknown function
    fprintf(stderr, "Error: Unknown function '");
    if (func_name->token.text) {
        fwrite(func_name->token.text->data, 1, func_name->token.text->length, stderr);
    }
    fprintf(stderr, "'\n");
    return 1;
}
