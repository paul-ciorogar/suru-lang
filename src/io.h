#ifndef IO_H
#define IO_H

#include <stdio.h>

typedef struct Buffer {
    size_t length;
    char* data;
} Buffer;

Buffer *read_file(const char *filename);
void write_file(char *filename, Buffer *buffer);

#endif
