#ifndef CODE_GENERATION_H
#define CODE_GENERATION_H

#include "io.h"
#include "parser.h"
#include <stdio.h>

Buffer* generate_code(ASTNode *ast);

#endif
