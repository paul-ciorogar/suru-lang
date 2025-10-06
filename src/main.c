#include "arena.h"
#include "code_generation.h"
#include "io.h"
#include "lexer.h"
#include "parser.h"
#include "string_storage.h"
#include <stdio.h>
#include <string.h>

void print_usage(char *program_name) {
    printf("Usege: %s run <source file>.suru\n", program_name);
}

int command_run(char *source_file) {
    Arena *strings_arena = arena_create(1);

    StringStorage *strings = string_storage_init(strings_arena);

    // Read source file
    Buffer *source = read_file(source_file);

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
    write_file(source_file, code);

    return 0;
}

int main(int argc, char *argv[]) {
    if (argc != 3) {
        print_usage(argv[0]);
        return 1;
    }

    if (strcmp(argv[1], "run") == 0) {
        return command_run(argv[2]);
    }

    print_usage(argv[0]);
    return 1;
}
