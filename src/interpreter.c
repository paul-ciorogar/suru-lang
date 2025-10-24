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
static int execute_var_decl(Interpreter *interp, int node_idx);
static int evaluate_expression(Interpreter *interp, int node_idx, ValueType *out_type, String **out_string, int *out_bool);

// Print helpers
static void print_string(String *str) {
    // Print string (without quotes, handle escape sequences)
    // The token includes quotes, so skip first and last character
    const char *s = str->data;
    int len = (int)str->length;

    // Skip opening quote
    s++;
    len -= 2; // Skip both quotes

    // Print character by character, handling escape sequences
    for (int i = 0; i < len; i++) {
        if (s[i] == '\\' && i + 1 < len) {
            switch (s[i + 1]) {
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
                printf("%c", s[i]);
                break;
            }
        } else {
            printf("%c", s[i]);
        }
    }
}

static void print_boolean(int bool_value) {
    if (bool_value) {
        printf("true");
    } else {
        printf("false");
    }
}

// Variable storage helpers
static void store_string_variable(Interpreter *interp, String *name, String *value) {
    // Check if variable already exists (update it)
    size_t count = array_length(interp->variables);
    for (size_t i = 0; i < count; i++) {
        Variable *var = (Variable *)array_get(interp->variables, i);
        if (var->name == name) {  // Pointer comparison works because strings are interned
            var->type = VALUE_STRING;
            var->string_value = value;
            return;
        }
    }

    // Add new variable
    Variable new_var;
    new_var.name = name;
    new_var.type = VALUE_STRING;
    new_var.string_value = value;
    array_append(interp->variables, &new_var);
}

static void store_boolean_variable(Interpreter *interp, String *name, int bool_value) {
    // Check if variable already exists (update it)
    size_t count = array_length(interp->variables);
    for (size_t i = 0; i < count; i++) {
        Variable *var = (Variable *)array_get(interp->variables, i);
        if (var->name == name) {  // Pointer comparison works because strings are interned
            var->type = VALUE_BOOLEAN;
            var->bool_value = bool_value;
            return;
        }
    }

    // Add new variable
    Variable new_var;
    new_var.name = name;
    new_var.type = VALUE_BOOLEAN;
    new_var.bool_value = bool_value;
    array_append(interp->variables, &new_var);
}

static Variable *lookup_variable(Interpreter *interp, String *name) {
    size_t count = array_length(interp->variables);
    for (size_t i = 0; i < count; i++) {
        Variable *var = (Variable *)array_get(interp->variables, i);
        if (var->name == name) {  // Pointer comparison works because strings are interned
            return var;
        }
    }
    return NULL;  // Not found
}

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
    interp->variables = array_init(sizeof(Variable));
    if (!interp->variables) {
        return NULL;
    }

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

        // Execute statement
        if (child->type == AST_VAR_DECL) {
            int result = execute_var_decl(interp, child_idx);
            if (result != 0) {
                return result;
            }
        } else if (child->type == AST_CALL_EXPR) {
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
        if (!arg) {
            fprintf(stderr, "Error: Invalid argument\n");
            return 1;
        }

        // Handle different argument types
        if (arg->type == AST_STRING_LITERAL) {
            // Direct string literal
            print_string(arg->token.text);
        } else if (arg->type == AST_BOOLEAN_LITERAL) {
            // Direct boolean literal
            int bool_value = (arg->token.type == TOKEN_TRUE) ? 1 : 0;
            print_boolean(bool_value);
        } else if (arg->type == AST_IDENTIFIER) {
            // Variable reference - look it up
            String *var_name = arg->token.text;
            Variable *var = lookup_variable(interp, var_name);
            if (!var) {
                fprintf(stderr, "Error: Undefined variable '");
                fwrite(var_name->data, 1, var_name->length, stderr);
                fprintf(stderr, "'\n");
                return 1;
            }

            // Print based on variable type
            if (var->type == VALUE_STRING) {
                print_string(var->string_value);
            } else if (var->type == VALUE_BOOLEAN) {
                print_boolean(var->bool_value);
            }
        } else {
            fprintf(stderr, "Error: print() requires a string or boolean argument\n");
            return 1;
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

// Evaluate an expression node and return its value
// Returns 0 on success, non-zero on error
static int evaluate_expression(Interpreter *interp, int node_idx, ValueType *out_type, String **out_string, int *out_bool) {
    ASTNode *node = get_ast_node(interp->ast, node_idx);
    if (!node) {
        fprintf(stderr, "Error: Invalid expression node\n");
        return 1;
    }

    switch (node->type) {
    case AST_BOOLEAN_LITERAL:
        *out_type = VALUE_BOOLEAN;
        *out_bool = (node->token.type == TOKEN_TRUE) ? 1 : 0;
        return 0;

    case AST_STRING_LITERAL:
        *out_type = VALUE_STRING;
        *out_string = node->token.text;
        return 0;

    case AST_IDENTIFIER: {
        // Variable reference
        String *var_name = node->token.text;
        Variable *var = lookup_variable(interp, var_name);
        if (!var) {
            fprintf(stderr, "Error: Undefined variable '");
            fwrite(var_name->data, 1, var_name->length, stderr);
            fprintf(stderr, "'\n");
            return 1;
        }
        *out_type = var->type;
        if (var->type == VALUE_STRING) {
            *out_string = var->string_value;
        } else if (var->type == VALUE_BOOLEAN) {
            *out_bool = var->bool_value;
        }
        return 0;
    }

    case AST_NOT_EXPR: {
        // Unary not: evaluate operand and negate
        if (node->first_child == -1) {
            fprintf(stderr, "Error: NOT expression missing operand\n");
            return 1;
        }

        ValueType operand_type;
        String *operand_str;
        int operand_bool;
        if (evaluate_expression(interp, node->first_child, &operand_type, &operand_str, &operand_bool) != 0) {
            return 1;
        }

        if (operand_type != VALUE_BOOLEAN) {
            fprintf(stderr, "Error: NOT operator requires boolean operand\n");
            return 1;
        }

        *out_type = VALUE_BOOLEAN;
        *out_bool = !operand_bool;
        return 0;
    }

    case AST_AND_EXPR: {
        // Binary and: evaluate both operands
        if (node->first_child == -1) {
            fprintf(stderr, "Error: AND expression missing operands\n");
            return 1;
        }

        ASTNode *left_node = get_ast_node(interp->ast, node->first_child);
        if (!left_node || left_node->next_sibling == -1) {
            fprintf(stderr, "Error: AND expression missing right operand\n");
            return 1;
        }

        ValueType left_type, right_type;
        String *left_str, *right_str;
        int left_bool, right_bool;

        if (evaluate_expression(interp, node->first_child, &left_type, &left_str, &left_bool) != 0) {
            return 1;
        }
        if (left_type != VALUE_BOOLEAN) {
            fprintf(stderr, "Error: AND operator requires boolean operands\n");
            return 1;
        }

        if (evaluate_expression(interp, left_node->next_sibling, &right_type, &right_str, &right_bool) != 0) {
            return 1;
        }
        if (right_type != VALUE_BOOLEAN) {
            fprintf(stderr, "Error: AND operator requires boolean operands\n");
            return 1;
        }

        *out_type = VALUE_BOOLEAN;
        *out_bool = left_bool && right_bool;
        return 0;
    }

    case AST_OR_EXPR: {
        // Binary or: evaluate both operands
        if (node->first_child == -1) {
            fprintf(stderr, "Error: OR expression missing operands\n");
            return 1;
        }

        ASTNode *left_node = get_ast_node(interp->ast, node->first_child);
        if (!left_node || left_node->next_sibling == -1) {
            fprintf(stderr, "Error: OR expression missing right operand\n");
            return 1;
        }

        ValueType left_type, right_type;
        String *left_str, *right_str;
        int left_bool, right_bool;

        if (evaluate_expression(interp, node->first_child, &left_type, &left_str, &left_bool) != 0) {
            return 1;
        }
        if (left_type != VALUE_BOOLEAN) {
            fprintf(stderr, "Error: OR operator requires boolean operands\n");
            return 1;
        }

        if (evaluate_expression(interp, left_node->next_sibling, &right_type, &right_str, &right_bool) != 0) {
            return 1;
        }
        if (right_type != VALUE_BOOLEAN) {
            fprintf(stderr, "Error: OR operator requires boolean operands\n");
            return 1;
        }

        *out_type = VALUE_BOOLEAN;
        *out_bool = left_bool || right_bool;
        return 0;
    }

    case AST_MATCH_EXPR: {
        // Match expression: match <subject> { <pattern>: <expr> ... }
        // First child is the subject expression
        if (node->first_child == -1) {
            fprintf(stderr, "Error: MATCH expression missing subject\n");
            return 1;
        }

        // Evaluate the subject expression
        ValueType subject_type;
        String *subject_str = NULL;
        int subject_bool = 0;

        if (evaluate_expression(interp, node->first_child, &subject_type, &subject_str, &subject_bool) != 0) {
            return 1;
        }

        // Iterate through match arms (siblings of subject)
        ASTNode *subject_node = get_ast_node(interp->ast, node->first_child);
        if (!subject_node) {
            fprintf(stderr, "Error: Invalid subject node\n");
            return 1;
        }

        int arm_idx = subject_node->next_sibling;
        while (arm_idx != -1) {
            ASTNode *arm = get_ast_node(interp->ast, arm_idx);
            if (!arm || arm->type != AST_MATCH_ARM) {
                fprintf(stderr, "Error: Invalid MATCH arm\n");
                return 1;
            }

            // Match arm structure: PATTERN, EXPRESSION
            // First child is the pattern
            if (arm->first_child == -1) {
                fprintf(stderr, "Error: MATCH arm missing pattern\n");
                return 1;
            }

            ASTNode *pattern = get_ast_node(interp->ast, arm->first_child);
            if (!pattern) {
                fprintf(stderr, "Error: Invalid pattern node\n");
                return 1;
            }

            // Check if pattern matches
            int matches = 0;

            if (pattern->type == AST_MATCH_WILDCARD) {
                // Wildcard always matches
                matches = 1;
            } else if (pattern->type == AST_BOOLEAN_LITERAL && subject_type == VALUE_BOOLEAN) {
                // Match boolean pattern
                int pattern_bool = (pattern->token.type == TOKEN_TRUE) ? 1 : 0;
                matches = (pattern_bool == subject_bool);
            } else if (pattern->type == AST_STRING_LITERAL && subject_type == VALUE_STRING) {
                // Match string pattern
                String *pattern_str = pattern->token.text;
                if (pattern_str && subject_str) {
                    // Compare strings (both include quotes)
                    matches = (pattern_str->length == subject_str->length &&
                              strncmp(pattern_str->data, subject_str->data, pattern_str->length) == 0);
                }
            }

            if (matches) {
                // Evaluate the arm's expression (second child)
                ASTNode *pattern_node = get_ast_node(interp->ast, arm->first_child);
                if (!pattern_node || pattern_node->next_sibling == -1) {
                    fprintf(stderr, "Error: MATCH arm missing expression\n");
                    return 1;
                }

                return evaluate_expression(interp, pattern_node->next_sibling, out_type, out_string, out_bool);
            }

            arm_idx = arm->next_sibling;
        }

        // No pattern matched
        fprintf(stderr, "Error: No matching pattern in match expression\n");
        return 1;
    }

    default:
        fprintf(stderr, "Error: Unsupported expression type in evaluation\n");
        return 1;
    }
}

// Execute AST_VAR_DECL node
static int execute_var_decl(Interpreter *interp, int node_idx) {
    ASTNode *node = get_ast_node(interp->ast, node_idx);
    if (!node || node->type != AST_VAR_DECL) {
        fprintf(stderr, "Error: Invalid variable declaration\n");
        return 1;
    }

    // Variable structure: IDENTIFIER, value (STRING_LITERAL or IDENTIFIER)
    // Get variable name (first child)
    ASTNode *var_name = get_ast_node(interp->ast, node->first_child);
    if (!var_name || var_name->type != AST_IDENTIFIER) {
        fprintf(stderr, "Error: Invalid variable name\n");
        return 1;
    }

    // Get value (second child)
    int value_idx = var_name->next_sibling;
    if (value_idx == -1) {
        fprintf(stderr, "Error: Missing variable value\n");
        return 1;
    }

    // Evaluate the expression
    ValueType value_type;
    String *string_value;
    int bool_value;

    if (evaluate_expression(interp, value_idx, &value_type, &string_value, &bool_value) != 0) {
        return 1;
    }

    // Store the variable based on its type
    if (value_type == VALUE_STRING) {
        store_string_variable(interp, var_name->token.text, string_value);
    } else if (value_type == VALUE_BOOLEAN) {
        store_boolean_variable(interp, var_name->token.text, bool_value);
    }

    return 0;
}
