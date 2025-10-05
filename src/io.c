#include "io.h"
#include <stdio.h>
#include <stdlib.h>

void write_file(char *filename, Buffer *buffer) {}

Buffer *read_file(const char *filename) {
    FILE *file = fopen(filename, "r");
    if (!file) {
        printf("Error: Could not oppen file %s\n", filename);
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
    result->data = content;

    return result;
}
