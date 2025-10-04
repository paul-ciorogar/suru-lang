#ifndef LEXER_H
#define LEXER_H

#include "arena.h"
#include "string_storage.h"

typedef struct Lexer {
    Arena *arena;
    StringStorage *strings;
    char *source;
    long position;

} Lexer;

Lexer *create_lexer(Arena *arena, StringStorage *strings, char *source);

#endif
