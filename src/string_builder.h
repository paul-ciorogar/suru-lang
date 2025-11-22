#ifndef STRING_BUILDER_H
#define STRING_BUILDER_H

#include <stddef.h>

// String builder using malloc/realloc
typedef struct {
    char *data;
    size_t length;
    size_t capacity;
} StringBuilder;

// Initialize a new string builder
StringBuilder *sb_create(size_t initial_capacity);

// Reset the string builder (clear contents but keep buffer)
void sb_reset(StringBuilder *sb);

// Append a string to the string builder
int sb_append(StringBuilder *sb, const char *str);

// Append a single character to the string builder
int sb_append_char(StringBuilder *sb, char c);

// Free the string builder
void sb_free(StringBuilder *sb);

// Copy contents to a buffer
int sb_copy_to_buffer(const StringBuilder *sb, char *buffer, size_t buffer_size);

#endif // STRING_BUILDER_H
