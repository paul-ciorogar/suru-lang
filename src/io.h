#ifndef IO_H
#define IO_H

#include <stdio.h>

typedef struct Buffer {
    size_t length;
    size_t capacity;
    char* data;
} Buffer;

Buffer *read_file(const char *filename);
Buffer *create_buffer(size_t capacity);
void free_buffer(Buffer *buffer);
int write_file(const char *filename, Buffer *buffer);

#endif
