#include "arena.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

// String structure with length and null-terminated data
typedef struct {
    size_t length;
    char *data;
} String;

// Node in the linked list of strings
typedef struct StringNode {
    String string;
    struct StringNode *next;
} StringNode;

// String storage system - linked list with head and tail
typedef struct {
    Arena *arena;
    StringNode *head;
    StringNode *tail;
    size_t string_count;
} StringStorage;

// Initialize string storage
StringStorage *string_storage_init(Arena *arena) {
    StringStorage *storage = arena_alloc(arena, sizeof(StringStorage));
    storage->arena = arena;
    storage->head = NULL;
    storage->tail = NULL;
    storage->string_count = 0;
    return storage;
}

// Compare strings for equality (length first for fast rejection)
static bool strings_equal(const char *a, size_t len_a, const char *b,
                          size_t len_b) {
    if (len_a != len_b)
        return false; // Fast rejection by length
    return memcmp(a, b, len_a) == 0;
}

// Find existing string in linked list
static String *find_string(StringStorage *storage, const char *data,
                           size_t length) {
    StringNode *current = storage->head;
    while (current) {
        if (strings_equal(current->string.data, current->string.length, data,
                          length)) {
            return &current->string;
        }
        current = current->next;
    }
    return NULL;
}

// Create new string and add to end of list
static String *create_string(StringStorage *storage, const char *data,
                             size_t length) {
    // Allocate new node
    StringNode *node = arena_alloc(storage->arena, sizeof(StringNode));

    // Set up the string
    node->string.length = length;
    node->string.data = arena_alloc(storage->arena, length + 1);
    memcpy(node->string.data, data, length);
    node->string.data[length] = '\0';
    node->next = NULL; // This will be the last node

    // Add to end of list
    if (storage->tail) {
        storage->tail->next = node;
        storage->tail = node;
    } else {
        // First string in the list
        storage->head = storage->tail = node;
    }
    storage->string_count++;

    return &node->string;
}

// Main function: store string from buffer
String *store_from_buffer(StringStorage *storage, const char *buffer,
                          size_t start, size_t count) {
    const char *data = buffer + start;

    // First, check if string already exists
    String *existing = find_string(storage, data, count);
    if (existing) {
        return existing;
    }

    // String doesn't exist, create new one
    return create_string(storage, data, count);
}

// Utility function: store from null-terminated string
String *store_cstring(StringStorage *storage, const char *cstr) {
    return store_from_buffer(storage, cstr, 0, strlen(cstr));
}

// Utility function: create string from literal
String *store_literal(StringStorage *storage, const char *literal) {
    return store_cstring(storage, literal);
}

// Get statistics about the string storage
typedef struct {
    size_t total_strings;
    size_t memory_used_strings; // Approximate
} StringStorageStats;

StringStorageStats get_storage_stats(StringStorage *storage) {
    StringStorageStats stats = {0};
    stats.total_strings = storage->string_count;

    // Calculate approximate memory usage
    StringNode *current = storage->head;
    while (current) {
        stats.memory_used_strings +=
            sizeof(StringNode) + current->string.length + 1;
        current = current->next;
    }

    return stats;
}

// Optional: print all stored strings (for debugging)
void debug_print_strings(StringStorage *storage) {
    printf("Stored strings (%zu total):\n", storage->string_count);
    StringNode *current = storage->head;
    int index = 0;
    while (current) {
        printf("  [%d] len=%zu: \"%s\"\n", index++, current->string.length,
               current->string.data);
        current = current->next;
    }
}
