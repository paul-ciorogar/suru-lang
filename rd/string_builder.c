#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    char* data;
    size_t length;
    size_t capacity;
} StringBuilder;

// Initialize a new string builder
StringBuilder* sb_create(size_t initial_capacity) {
    StringBuilder* sb = (StringBuilder*)malloc(sizeof(StringBuilder));
    if (!sb) return NULL;
    
    sb->capacity = initial_capacity > 0 ? initial_capacity : 16;
    sb->data = (char*)malloc(sb->capacity);
    if (!sb->data) {
        free(sb);
        return NULL;
    }
    
    sb->data[0] = '\0';
    sb->length = 0;
    return sb;
}

// Append a string to the string builder
int sb_append(StringBuilder* sb, const char* str) {
    if (!sb || !str) return -1;
    
    size_t str_len = strlen(str);
    size_t required = sb->length + str_len + 1;
    
    // Resize if needed
    if (required > sb->capacity) {
        size_t new_capacity = sb->capacity;
        while (new_capacity < required) {
            new_capacity *= 2;
        }
        
        char* new_data = (char*)realloc(sb->data, new_capacity);
        if (!new_data) return -1;
        
        sb->data = new_data;
        sb->capacity = new_capacity;
    }
    
    memcpy(sb->data + sb->length, str, str_len);
    sb->length += str_len;
    sb->data[sb->length] = '\0';
    
    return 0;
}

// Get the current length
size_t sb_length(const StringBuilder* sb) {
    return sb ? sb->length : 0;
}

// Copy contents to a buffer
int sb_copy_to_buffer(const StringBuilder* sb, char* buffer, size_t buffer_size) {
    if (!sb || !buffer || buffer_size == 0) return -1;
    
    size_t copy_len = sb->length < buffer_size - 1 ? sb->length : buffer_size - 1;
    memcpy(buffer, sb->data, copy_len);
    buffer[copy_len] = '\0';
    
    return copy_len;
}

// Free the string builder
void sb_free(StringBuilder* sb) {
    if (sb) {
        free(sb->data);
        free(sb);
    }
}

// Example usage
int main() {
    StringBuilder* sb = sb_create(16);
    
    sb_append(sb, "Hello");
    sb_append(sb, " ");
    sb_append(sb, "World");
    sb_append(sb, "!");
    
    printf("Length: %zu\n", sb_length(sb));
    printf("Content: %s\n", sb->data);
    
    // Copy to buffer
    char buffer[50];
    sb_copy_to_buffer(sb, buffer, sizeof(buffer));
    printf("Copied to buffer: %s\n", buffer);
    
    sb_free(sb);
    return 0;
}
