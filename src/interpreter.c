#include "interpreter.h"
#include "arena.h"
#include "ast.h"
#include "symbol_table.h"
#include <stdio.h>
#include <string.h>

// Forward declarations
static int execute_program(Interpreter *interp);
static int execute_function_call(Interpreter *interp, Symbol *func_sym, Array *arg_values,
                                 ValueType *out_type, String **out_string, int *out_bool, FunctionValue *out_func, int *has_return);
static int execute_block(Interpreter *interp, int block_node_idx,
                         ValueType *out_type, String **out_string, int *out_bool, FunctionValue *out_func, int *has_return);
static int execute_statement(Interpreter *interp, int stmt_node_idx,
                             ValueType *out_type, String **out_string, int *out_bool, FunctionValue *out_func, int *has_return);
static int evaluate_expression(Interpreter *interp, int node_idx,
                               ValueType *out_type, String **out_string, int *out_bool, FunctionValue *out_func);

// ==== Print Helpers ====

static void print_string(String *str) {
    // Print string (without quotes, handle escape sequences)
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
    printf("%s", bool_value ? "true" : "false");
}

// ==== Scope Management ====

static Scope *create_scope(Interpreter *interp, Scope *parent, int scope_id) {
    Scope *scope = arena_alloc(interp->arena, sizeof(Scope));
    if (!scope)
        return NULL;

    scope->parent = parent;
    scope->scope_id = scope_id;
    // TODO: array_init allocates one page per array. Need a smaller array implementation
    scope->variables = array_init(sizeof(Variable));
    if (!scope->variables)
        return NULL;

    return scope;
}

// ==== Variable Storage and Lookup ====

static void store_variable(Scope *scope, String *name, ValueType type,
                           String *str_val, int bool_val, FunctionValue func_val) {
    if (!scope)
        return;

    // Check if variable already exists in current scope (update it)
    size_t count = array_length(scope->variables);
    for (size_t i = 0; i < count; i++) {
        Variable *var = (Variable *)array_get(scope->variables, i);
        if (var->name == name) {
            var->type = type;
            if (type == VALUE_STRING)
                var->string_value = str_val;
            else if (type == VALUE_BOOLEAN)
                var->bool_value = bool_val;
            else if (type == VALUE_FUNCTION)
                var->func_value = func_val;
            return;
        }
    }

    // Add new variable to current scope
    Variable new_var;
    new_var.name = name;
    new_var.type = type;
    if (type == VALUE_STRING)
        new_var.string_value = str_val;
    else if (type == VALUE_BOOLEAN)
        new_var.bool_value = bool_val;
    else if (type == VALUE_FUNCTION)
        new_var.func_value = func_val;
    array_append(scope->variables, &new_var);
}

static Variable *lookup_variable_in_scope_chain(Scope *scope, String *name) {
    while (scope) {
        size_t count = array_length(scope->variables);
        for (size_t i = 0; i < count; i++) {
            Variable *var = (Variable *)array_get(scope->variables, i);
            if (var->name == name) {
                return var;
            }
        }
        scope = scope->parent;
    }
    return NULL;
}

// ==== Interpreter Creation ====

Interpreter *create_interpreter(Arena *arena, AST *ast) {
    if (!arena || !ast)
        return NULL;

    Interpreter *interp = arena_alloc(arena, sizeof(Interpreter));
    if (!interp)
        return NULL;

    interp->arena = arena;
    interp->ast = ast;
    interp->global_scope = create_scope(interp, NULL, 0);
    interp->current_scope = interp->global_scope;

    if (!interp->global_scope)
        return NULL;

    return interp;
}

int interpret(Interpreter *interp) {
    if (!interp || !interp->ast) {
        fprintf(stderr, "Error: Invalid interpreter state\n");
        return 1;
    }

    return execute_program(interp);
}

// ==== Program Execution ====

static int execute_program(Interpreter *interp) {
    // Look up main function in symbol table by searching all symbols
    Symbol *main_sym = NULL;
    size_t count = array_length(interp->ast->symbols->symbols);
    for (size_t i = 0; i < count; i++) {
        Symbol *sym = (Symbol *)array_get(interp->ast->symbols->symbols, i);
        if (sym->type == SYMBOL_FUNCTION &&
            sym->name->length == 4 &&
            strncmp(sym->name->data, "main", 4) == 0 &&
            sym->parent_scope_id == 0) {
            main_sym = sym;
            break;
        }
    }

    if (!main_sym) {
        fprintf(stderr, "Error: No main function found\n");
        return 1;
    }

    // Execute main with no arguments
    Array *empty_args = array_init(sizeof(Variable));
    ValueType ret_type;
    String *ret_str;
    int ret_bool;
    FunctionValue ret_func;
    int has_return = 0;

    int result = execute_function_call(interp, main_sym, empty_args,
                                       &ret_type, &ret_str, &ret_bool, &ret_func, &has_return);
    array_free(empty_args);
    return result;
}

// ==== Function Execution ====

static int execute_function_call(Interpreter *interp, Symbol *func_sym, Array *arg_values,
                                 ValueType *out_type, String **out_string, int *out_bool,
                                 FunctionValue *out_func, int *has_return) {
    if (!func_sym || func_sym->type != SYMBOL_FUNCTION) {
        fprintf(stderr, "Error: Invalid function symbol\n");
        return 1;
    }

    ASTNode *func_node = get_ast_node(interp->ast, func_sym->ast_node_idx);
    if (!func_node || func_node->type != AST_FUNCTION_DECL) {
        fprintf(stderr, "Error: Invalid function node\n");
        return 1;
    }

    // Get function name
    ASTNode *name_node = get_ast_node(interp->ast, func_node->first_child);
    if (!name_node || name_node->type != AST_IDENTIFIER) {
        fprintf(stderr, "Error: Function missing name\n");
        return 1;
    }

    // Get parameter list and block
    ASTNode *param_list_node = get_ast_node(interp->ast, name_node->next_sibling);
    if (!param_list_node || param_list_node->type != AST_PARAM_LIST) {
        fprintf(stderr, "Error: Function missing parameter list\n");
        return 1;
    }

    ASTNode *block_node = get_ast_node(interp->ast, param_list_node->next_sibling);
    if (!block_node || block_node->type != AST_BLOCK) {
        fprintf(stderr, "Error: Function has no body\n");
        return 1;
    }

    // Create new scope for function execution
    Scope *func_scope = create_scope(interp, interp->current_scope, func_sym->scope_id);
    if (!func_scope) {
        fprintf(stderr, "Error: Failed to create function scope\n");
        return 1;
    }

    Scope *saved_scope = interp->current_scope;
    interp->current_scope = func_scope;

    // Bind parameters to argument values
    int param_idx = param_list_node->first_child;
    size_t arg_i = 0;
    while (param_idx != -1 && arg_i < array_length(arg_values)) {
        ASTNode *param_node = get_ast_node(interp->ast, param_idx);
        if (!param_node || param_node->type != AST_PARAM) {
            param_idx = param_node ? param_node->next_sibling : -1;
            continue;
        }

        // Get parameter name
        ASTNode *param_name_node = get_ast_node(interp->ast, param_node->first_child);
        if (!param_name_node || param_name_node->type != AST_IDENTIFIER) {
            fprintf(stderr, "Error: Invalid parameter\n");
            interp->current_scope = saved_scope;
            return 1;
        }

        // Bind argument to parameter
        Variable *arg = (Variable *)array_get(arg_values, arg_i);
        FunctionValue dummy_func = {0, NULL};
        store_variable(func_scope, param_name_node->token.text, arg->type,
                       arg->type == VALUE_STRING ? arg->string_value : NULL,
                       arg->type == VALUE_BOOLEAN ? arg->bool_value : 0,
                       arg->type == VALUE_FUNCTION ? arg->func_value : dummy_func);

        param_idx = param_node->next_sibling;
        arg_i++;
    }

    // Execute function body
    int result = execute_block(interp, block_node->first_child,
                               out_type, out_string, out_bool, out_func, has_return);

    // Restore scope
    interp->current_scope = saved_scope;

    return result;
}

// ==== Block Execution ====

static int execute_block(Interpreter *interp, int block_child_idx,
                         ValueType *out_type, String **out_string, int *out_bool,
                         FunctionValue *out_func, int *has_return) {
    *has_return = 0;

    // Execute each statement in the block
    int child_idx = block_child_idx;
    while (child_idx != -1) {
        ASTNode *child = get_ast_node(interp->ast, child_idx);
        if (!child)
            break;

        int stmt_has_return = 0;
        int result = execute_statement(interp, child_idx, out_type, out_string,
                                       out_bool, out_func, &stmt_has_return);
        if (result != 0)
            return result;

        // If we hit a return, stop executing
        if (stmt_has_return) {
            *has_return = 1;
            return 0;
        }

        child_idx = child->next_sibling;
    }

    return 0;
}

// ==== Statement Execution ====

static int execute_statement(Interpreter *interp, int stmt_node_idx,
                             ValueType *out_type, String **out_string, int *out_bool,
                             FunctionValue *out_func, int *has_return) {
    ASTNode *node = get_ast_node(interp->ast, stmt_node_idx);
    if (!node) {
        fprintf(stderr, "Error: Invalid statement node\n");
        return 1;
    }

    *has_return = 0;

    switch (node->type) {
    case AST_VAR_DECL: {
        // Variable declaration: IDENTIFIER, value
        ASTNode *var_name = get_ast_node(interp->ast, node->first_child);
        if (!var_name || var_name->type != AST_IDENTIFIER) {
            fprintf(stderr, "Error: Invalid variable name\n");
            return 1;
        }

        int value_idx = var_name->next_sibling;
        if (value_idx == -1) {
            fprintf(stderr, "Error: Missing variable value\n");
            return 1;
        }

        // Evaluate the expression
        ValueType value_type;
        String *string_value = NULL;
        int bool_value = 0;
        FunctionValue func_value = {0, NULL};

        if (evaluate_expression(interp, value_idx, &value_type, &string_value,
                                &bool_value, &func_value) != 0) {
            return 1;
        }

        // Store the variable
        store_variable(interp->current_scope, var_name->token.text, value_type,
                       string_value, bool_value, func_value);
        return 0;
    }

    case AST_FUNCTION_DECL: {
        // Nested function declaration - register it in current scope
        ASTNode *func_name = get_ast_node(interp->ast, node->first_child);
        if (!func_name || func_name->type != AST_IDENTIFIER) {
            fprintf(stderr, "Error: Function missing name\n");
            return 1;
        }

        // Look up in symbol table
        Symbol *func_sym = lookup_function(interp->ast->symbols, func_name->token.text,
                                           interp->current_scope->scope_id);
        if (!func_sym) {
            fprintf(stderr, "Error: Nested function not in symbol table\n");
            return 1;
        }

        // Store as function value in current scope
        FunctionValue func_val;
        func_val.function_node_idx = stmt_node_idx;
        func_val.closure_scope = interp->current_scope;

        store_variable(interp->current_scope, func_name->token.text, VALUE_FUNCTION,
                       NULL, 0, func_val);
        return 0;
    }

    case AST_CALL_EXPR: {
        // Call expression as statement (ignore return value)
        ValueType dummy_type;
        String *dummy_str;
        int dummy_bool;
        FunctionValue dummy_func;
        return evaluate_expression(interp, stmt_node_idx, &dummy_type, &dummy_str,
                                   &dummy_bool, &dummy_func);
    }

    case AST_RETURN_STMT: {
        // Return statement with value
        if (node->first_child == -1) {
            fprintf(stderr, "Error: Return statement missing value\n");
            return 1;
        }

        if (evaluate_expression(interp, node->first_child, out_type, out_string,
                                out_bool, out_func) != 0) {
            return 1;
        }

        *has_return = 1;
        return 0;
    }

    case AST_MATCH_STMT: {
        // Match statement: match_expression
        if (node->first_child == -1) {
            fprintf(stderr, "Error: MATCH statement missing expression\n");
            return 1;
        }

        ASTNode *expression_node = get_ast_node(interp->ast, node->first_child);
        if (!expression_node || expression_node->type != AST_MATCH_EXPR) {
            fprintf(stderr, "Error: Invalid match expression node\n");
            return 1;
        }

        return evaluate_expression(interp, node->first_child, out_type, out_string, out_bool, out_func);
    }

    default:
        fprintf(stderr, "Error: Unsupported statement type\n");
        return 1;
    }
}

// ==== Expression Evaluation ====

static int evaluate_expression(Interpreter *interp, int node_idx,
                               ValueType *out_type, String **out_string, int *out_bool,
                               FunctionValue *out_func) {
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
        // Variable or function reference
        String *var_name = node->token.text;
        Variable *var = lookup_variable_in_scope_chain(interp->current_scope, var_name);
        if (!var) {
            fprintf(stderr, "Error: Undefined variable '");
            fwrite(var_name->data, 1, var_name->length, stderr);
            fprintf(stderr, "'\n");
            return 1;
        }
        *out_type = var->type;
        if (var->type == VALUE_STRING)
            *out_string = var->string_value;
        else if (var->type == VALUE_BOOLEAN)
            *out_bool = var->bool_value;
        else if (var->type == VALUE_FUNCTION)
            *out_func = var->func_value;
        return 0;
    }

    case AST_CALL_EXPR: {
        // Function call: IDENTIFIER, ARG_LIST
        ASTNode *func_name = get_ast_node(interp->ast, node->first_child);
        if (!func_name || func_name->type != AST_IDENTIFIER) {
            fprintf(stderr, "Error: Invalid function name in call\n");
            return 1;
        }

        int arg_list_idx = func_name->next_sibling;
        ASTNode *arg_list = get_ast_node(interp->ast, arg_list_idx);
        if (!arg_list || arg_list->type != AST_ARG_LIST) {
            fprintf(stderr, "Error: Invalid argument list\n");
            return 1;
        }

        // Check for built-in function "print"
        if (func_name->token.text && func_name->token.text->length == 5 &&
            strncmp(func_name->token.text->data, "print", 5) == 0) {

            // Built-in print function
            if (arg_list->first_child == -1) {
                fprintf(stderr, "Error: print() requires an argument\n");
                return 1;
            }

            ValueType arg_type;
            String *arg_str;
            int arg_bool;
            FunctionValue arg_func;

            if (evaluate_expression(interp, arg_list->first_child, &arg_type,
                                    &arg_str, &arg_bool, &arg_func) != 0) {
                return 1;
            }

            if (arg_type == VALUE_STRING)
                print_string(arg_str);
            else if (arg_type == VALUE_BOOLEAN)
                print_boolean(arg_bool);
            else {
                fprintf(stderr, "Error: print() requires string or boolean\n");
                return 1;
            }

            return 0;
        }

        // User-defined function - look up in symbol table
        Symbol *func_sym = lookup_function(interp->ast->symbols, func_name->token.text,
                                           interp->current_scope->scope_id);
        if (!func_sym) {
            fprintf(stderr, "Error: Unknown function '");
            fwrite(func_name->token.text->data, 1, func_name->token.text->length, stderr);
            fprintf(stderr, "'\n");
            return 1;
        }

        // Evaluate arguments
        Array *arg_values = array_init(sizeof(Variable));
        int arg_idx = arg_list->first_child;
        while (arg_idx != -1) {
            ValueType arg_type;
            String *arg_str = NULL;
            int arg_bool = 0;
            FunctionValue arg_func = {0, NULL};

            if (evaluate_expression(interp, arg_idx, &arg_type, &arg_str,
                                    &arg_bool, &arg_func) != 0) {
                array_free(arg_values);
                return 1;
            }

            Variable arg_var;
            arg_var.name = NULL;
            arg_var.type = arg_type;
            if (arg_type == VALUE_STRING)
                arg_var.string_value = arg_str;
            else if (arg_type == VALUE_BOOLEAN)
                arg_var.bool_value = arg_bool;
            else if (arg_type == VALUE_FUNCTION)
                arg_var.func_value = arg_func;

            array_append(arg_values, &arg_var);

            ASTNode *arg_node = get_ast_node(interp->ast, arg_idx);
            arg_idx = arg_node ? arg_node->next_sibling : -1;
        }

        // Call the function
        int has_return = 0;
        int result = execute_function_call(interp, func_sym, arg_values,
                                           out_type, out_string, out_bool, out_func, &has_return);
        array_free(arg_values);
        return result;
    }

    case AST_NOT_EXPR: {
        if (node->first_child == -1) {
            fprintf(stderr, "Error: NOT expression missing operand\n");
            return 1;
        }

        ValueType operand_type;
        String *operand_str;
        int operand_bool;
        FunctionValue operand_func;

        if (evaluate_expression(interp, node->first_child, &operand_type, &operand_str,
                                &operand_bool, &operand_func) != 0) {
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

    case AST_AND_EXPR:
    case AST_OR_EXPR: {
        if (node->first_child == -1) {
            fprintf(stderr, "Error: Binary expression missing operands\n");
            return 1;
        }

        ASTNode *left_node = get_ast_node(interp->ast, node->first_child);
        if (!left_node || left_node->next_sibling == -1) {
            fprintf(stderr, "Error: Binary expression missing right operand\n");
            return 1;
        }

        ValueType left_type, right_type;
        String *left_str, *right_str;
        int left_bool, right_bool;
        FunctionValue left_func, right_func;

        if (evaluate_expression(interp, node->first_child, &left_type, &left_str,
                                &left_bool, &left_func) != 0) {
            return 1;
        }
        if (left_type != VALUE_BOOLEAN) {
            fprintf(stderr, "Error: Logical operator requires boolean operands\n");
            return 1;
        }

        if (evaluate_expression(interp, left_node->next_sibling, &right_type, &right_str,
                                &right_bool, &right_func) != 0) {
            return 1;
        }
        if (right_type != VALUE_BOOLEAN) {
            fprintf(stderr, "Error: Logical operator requires boolean operands\n");
            return 1;
        }

        *out_type = VALUE_BOOLEAN;
        *out_bool = (node->type == AST_AND_EXPR) ? (left_bool && right_bool) : (left_bool || right_bool);
        return 0;
    }

    case AST_MATCH_EXPR: {
        // Match expression: match <subject> { <pattern>: <expr> ... }
        if (node->first_child == -1) {
            fprintf(stderr, "Error: MATCH expression missing subject\n");
            return 1;
        }

        // Evaluate the subject expression
        ValueType subject_type;
        String *subject_str = NULL;
        int subject_bool = 0;
        FunctionValue subject_func;

        if (evaluate_expression(interp, node->first_child, &subject_type, &subject_str,
                                &subject_bool, &subject_func) != 0) {
            return 1;
        }

        // Iterate through match arms
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
                matches = 1;
            } else if (pattern->type == AST_BOOLEAN_LITERAL && subject_type == VALUE_BOOLEAN) {
                int pattern_bool = (pattern->token.type == TOKEN_TRUE) ? 1 : 0;
                matches = (pattern_bool == subject_bool);
            } else if (pattern->type == AST_STRING_LITERAL && subject_type == VALUE_STRING) {
                matches = (pattern->token.text->length == subject_str->length &&
                           strncmp(pattern->token.text->data, subject_str->data, pattern->token.text->length) == 0);
            }

            if (matches) {
                // Evaluate the arm's expression
                ASTNode *pattern_node = get_ast_node(interp->ast, arm->first_child);
                if (!pattern_node || pattern_node->next_sibling == -1) {
                    fprintf(stderr, "Error: MATCH arm missing expression\n");
                    return 1;
                }

                return evaluate_expression(interp, pattern_node->next_sibling, out_type,
                                           out_string, out_bool, out_func);
            }

            arm_idx = arm->next_sibling;
        }

        fprintf(stderr, "Error: No matching pattern in match expression\n");
        return 1;
    }

    default:
        fprintf(stderr, "Error: Unsupported expression type\n");
        return 1;
    }
}
