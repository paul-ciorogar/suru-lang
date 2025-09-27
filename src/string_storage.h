#ifndef STRING_STORAGE_H
#define STRING_STORAGE_H

#include <stddef.h>

// Forward declaration for Arena (assuming it's defined elsewhere)
typedef struct Arena Arena;

// String structure with length and null-terminated data
typedef struct {
    size_t length;
    char* data;
} String;

// Opaque string storage handle
typedef struct StringStorage StringStorage;

// Statistics about string storage usage
typedef struct {
    size_t total_strings;
    size_t memory_used_strings;  // Approximate bytes used for strings
} StringStorageStats;

// Core functions
StringStorage* string_storage_init(Arena* arena);
String* store_from_buffer(StringStorage* storage, const char* buffer, size_t start, size_t count);

// Utility functions
String* store_cstring(StringStorage* storage, const char* cstr);
String* store_literal(StringStorage* storage, const char* literal);

// Information and debugging
StringStorageStats get_storage_stats(StringStorage* storage);
void debug_print_strings(StringStorage* storage);

#endif // STRING_STORAGE_H