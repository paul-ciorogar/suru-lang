#ifndef CODE_GENERATION_H
#define CODE_GENERATION_H

#include "parser.h"
#include <stdio.h>

typedef struct Code {
    size_t length;
    char* buffer;
} Code;

Code* generate_code(ASTNode *ast);

#endif
