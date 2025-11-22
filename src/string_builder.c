#include "string_builder.h"
#include <stdlib.h>
#include <string.h>

// Initialize a new string builder
StringBuilder *sb_create(size_t initial_capacity) {
    StringBuilder *sb = (StringBuilder *)malloc(sizeof(StringBuilder));
    if (!sb)
        return NULL;

    sb->capacity = initial_capacity > 0 ? initial_capacity : 16;
    sb->data = (char *)malloc(sb->capacity);
    if (!sb->data) {
        free(sb);
        return NULL;
    }

    sb->data[0] = '\0';
    sb->length = 0;
    return sb;
}

void sb_reset(StringBuilder *sb) {
    if (sb) {
        sb->data[0] = '\0';
        sb->length = 0;
    }
}

// Append a string to the string builder
int sb_append(StringBuilder *sb, const char *str) {
    if (!sb || !str)
        return -1;

    size_t str_len = strlen(str);
    size_t required = sb->length + str_len + 1;

    // Resize if needed
    if (required > sb->capacity) {
        size_t new_capacity = sb->capacity;
        while (new_capacity < required) {
            new_capacity *= 2;
        }

        char *new_data = (char *)realloc(sb->data, new_capacity);
        if (!new_data)
            return -1;

        sb->data = new_data;
        sb->capacity = new_capacity;
    }

    memcpy(sb->data + sb->length, str, str_len);
    sb->length += str_len;
    sb->data[sb->length] = '\0';

    return 0;
}

// Append a single character to the string builder
int sb_append_char(StringBuilder *sb, char c) {
    if (!sb)
        return -1;

    size_t required = sb->length + 2; // +1 for char, +1 for null terminator

    // Resize if needed
    if (required > sb->capacity) {
        size_t new_capacity = sb->capacity * 2;
        char *new_data = (char *)realloc(sb->data, new_capacity);
        if (!new_data)
            return -1;

        sb->data = new_data;
        sb->capacity = new_capacity;
    }

    sb->data[sb->length++] = c;
    sb->data[sb->length] = '\0';

    return 0;
}

// Free the string builder
void sb_free(StringBuilder *sb) {
    if (sb) {
        free(sb->data);
        free(sb);
    }
}

// Copy contents to a buffer
int sb_copy_to_buffer(const StringBuilder *sb, char *buffer, size_t buffer_size) {
    if (!sb || !buffer || buffer_size == 0)
        return -1;

    if (sb->length >= buffer_size)
        return -1;

    memcpy(buffer, sb->data, sb->length + 1);
    return 0;
}
