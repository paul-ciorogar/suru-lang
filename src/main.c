#include "arena.h"
#include "code_generation.h"
#include "io.h"
#include "lexer.h"
#include "parser.h"
#include "string_storage.h"
#include <stdio.h>

int main(int argc, char *argv[]) {
    if (argc != 2) {
        printf("Usege: %s <source file>.suru\n", argv[0]);
        return 1;
    }

    Arena *strings_arena = arena_create(1);

    StringStorage *strings = string_storage_init(strings_arena);

    // Read source file
    Buffer *source = read_file(argv[1]);

    if (!source) {
        return 1;
    }

    // Lexical analysis
    Arena *arena = arena_create(1);
    Lexer *lexer = create_lexer(arena, strings, source->data, source->length);
    Parser *parser = create_parser(arena, lexer);

    // Parse the source code
    ASTNode *ast = parse_statement(parser);

    if (!ast) {
        return 1;
    }

    // Generate machine code
    Buffer *code = generate_code(ast);

    // Write the new executable
    write_file(argv[1], code);

    return 0;
}
