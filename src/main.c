#include "arena.c"
#include "generator.c"
#include "lexer.c"
#include "parser.c"
#include "string_storage.c"
#include <stdio.h>
#include <stdlib.h>

char *read_file(const char *filename) {
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
    return content;
}

int main(int argc, char *argv[]) {
    if (argc != 2) {
        printf("Usege: %s <source file>.suru", argv[0]);
        return 1;
    }
    Arena *strings_arena = arena_create(1);

    StringStorage *strings = string_storage_init(strings_arena)

        // Read source file
        char *source = read_file(argv[1]);

    if (!source) {
        return 1;
    }

    // Lexical analysis
    Arena *arena = arena_create(1);
    Lexer *lexer = create_lexer(arena, strings, source);
    Parser *parser = create_parser(arena, lexer);

    // Parse the source code
    ASTNode *ast = parse_statement(parser);

    if (!ast) {
        return 1;
    }

    // Generate machine code
    Code *code = generate_code(ast);

    // Write the new executable
    write_file(argv[1], code);

    return 0;
}
