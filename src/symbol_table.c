#include "symbol_table.h"
#include "arena.h"
#include "array.h"
#include <string.h>

SymbolTable *create_symbol_table(Arena *arena) {
    SymbolTable *table = arena_alloc(arena, sizeof(SymbolTable));
    if (!table) {
        return NULL;
    }

    // TODO: array_init allocates one page per array. Need a smaller array implementation
    table->symbols = array_init(sizeof(Symbol));
    if (!table->symbols) {
        return NULL;
    }

    table->arena = arena;
    table->next_scope_id = 1;  // 0 is reserved for global scope

    return table;
}

int add_function_symbol(SymbolTable *table, String *name, int ast_node_idx, int parent_scope_id) {
    Symbol symbol;
    symbol.type = SYMBOL_FUNCTION;
    symbol.name = name;
    symbol.ast_node_idx = ast_node_idx;
    symbol.parent_scope_id = parent_scope_id;
    symbol.scope_id = table->next_scope_id++;

    array_append(table->symbols, &symbol);

    return symbol.scope_id;
}

void add_variable_symbol(SymbolTable *table, String *name, int ast_node_idx, int parent_scope_id) {
    Symbol symbol;
    symbol.type = SYMBOL_VARIABLE;
    symbol.name = name;
    symbol.ast_node_idx = ast_node_idx;
    symbol.parent_scope_id = parent_scope_id;
    symbol.scope_id = -1;  // Variables don't create new scopes

    array_append(table->symbols, &symbol);
}

// Helper: Check if scope_id is an ancestor of or equal to current_scope_id
static int is_scope_accessible(SymbolTable *table, int target_scope_id, int current_scope_id) {
    if (target_scope_id == current_scope_id) {
        return 1;  // Same scope
    }

    // Search through symbols to build scope chain
    // Walk up from current_scope_id to see if we reach target_scope_id
    int search_scope = current_scope_id;
    while (search_scope != -1 && search_scope != 0) {
        // Find the symbol that created this scope
        size_t count = array_length(table->symbols);
        for (size_t i = 0; i < count; i++) {
            Symbol *sym = (Symbol *)array_get(table->symbols, i);
            if (sym->scope_id == search_scope) {
                if (sym->parent_scope_id == target_scope_id) {
                    return 1;  // Found target in ancestor chain
                }
                search_scope = sym->parent_scope_id;
                break;
            }
        }

        // If we didn't find a symbol with this scope_id, break
        if (search_scope == current_scope_id) {
            break;
        }
    }

    return target_scope_id == 0;  // Global scope (0) is always accessible
}

Symbol *lookup_symbol(SymbolTable *table, String *name, int scope_id) {
    // Search in current scope and all parent scopes
    size_t count = array_length(table->symbols);

    // First, try to find in current scope
    for (size_t i = 0; i < count; i++) {
        Symbol *sym = (Symbol *)array_get(table->symbols, i);
        if (sym->name == name && sym->parent_scope_id == scope_id) {
            return sym;
        }
    }

    // Then search in parent scopes
    for (size_t i = 0; i < count; i++) {
        Symbol *sym = (Symbol *)array_get(table->symbols, i);
        if (sym->name == name && is_scope_accessible(table, sym->parent_scope_id, scope_id)) {
            return sym;
        }
    }

    return NULL;
}

Symbol *lookup_function(SymbolTable *table, String *name, int scope_id) {
    Symbol *sym = lookup_symbol(table, name, scope_id);
    if (sym && sym->type == SYMBOL_FUNCTION) {
        return sym;
    }
    return NULL;
}
