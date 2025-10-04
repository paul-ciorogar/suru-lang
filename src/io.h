#ifndef IO_H
#define IO_H

#include "code_generation.h"

char *read_file(const char *filename);
void write_file(char *filename, Code *code);

#endif
