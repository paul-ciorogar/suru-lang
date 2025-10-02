#ifndef LEXER_H
#define LEXER_H

#include "arena.c"
#include "string_storage.h"
#include <unistd.h>

typedef struct Lexer {
    Arena *arena;
    StringStorage *strings;
    char *source;
    long position;

} Lexer;

Lexer *create_lexer(Arena *arena, StringStorage *strings, char *source) {
    return NULL;
}

#endif
