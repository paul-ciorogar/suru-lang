#include "io.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Create a new buffer with given capacity
Buffer *create_buffer(size_t capacity) {
    Buffer *buffer = malloc(sizeof(Buffer));
    if (!buffer) {
        return NULL;
    }

    buffer->data = malloc(capacity);
    if (!buffer->data) {
        free(buffer);
        return NULL;
    }

    buffer->length = 0;
    buffer->capacity = capacity;
    buffer->data[0] = '\0';

    return buffer;
}

// Free a buffer
void free_buffer(Buffer *buffer) {
    if (buffer) {
        free(buffer->data);
        free(buffer);
    }
}

// Write buffer to file
int write_file(const char *filename, Buffer *buffer) {
    if (!filename || !buffer) {
        return 0;
    }

    FILE *file = fopen(filename, "w");
    if (!file) {
        fprintf(stderr, "Error: Could not open file %s for writing\n", filename);
        return 0;
    }

    size_t written = fwrite(buffer->data, 1, buffer->length, file);
    fclose(file);

    return written == buffer->length ? 1 : 0;
}

Buffer *read_file(const char *filename) {
    FILE *file = fopen(filename, "r");
    if (!file) {
        printf("Error: Could not open file %s\n", filename);
        return NULL;
    }

    fseek(file, 0, SEEK_END);
    long lenght = ftell(file);
    fseek(file, 0, SEEK_SET);

    char *content = malloc(lenght + 1);
    fread(content, 1, lenght, file);
    content[lenght] = '\0';

    fclose(file);
    Buffer *result = malloc(sizeof(Buffer));
    if (!result) {
        printf("Error: Could not allocate buffer.");
        return NULL;
    }

    result->length = lenght;
    result->capacity = lenght + 1;
    result->data = content;

    return result;
}
