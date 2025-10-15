#include "arena.h"
#include "code_generation.h"
#include "formatter.h"
#include "io.h"
#include "lexer.h"
#include "parser.h"
#include "string_storage.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void print_usage(char *program_name) {
    printf("Usage: %s run <source file>.suru\n", program_name);
    printf("       %s lex <source file>.suru\n", program_name);
    printf("       %s format [--write] <source file>.suru\n", program_name);
}

int command_lex(char *source_file) {
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

    print_tokens(lexer);

    return 0;
}

int command_format(char *source_file, int write_to_file) {
    Arena *strings_arena = arena_create(1);
    StringStorage *strings = string_storage_init(strings_arena);

    // Read source file
    Buffer *source = read_file(source_file);
    if (!source) {
        fprintf(stderr, "Error: Could not read file %s\n", source_file);
        return 1;
    }

    // Lexical analysis
    Arena *arena = arena_create(1);
    Lexer *lexer = create_lexer(arena, strings, source->data, source->length);

    // Parse to build parse tree
    Parser *parser = create_parser(arena, lexer);
    ParseTree *tree = parse(parser);

    if (!tree) {
        fprintf(stderr, "Error: Failed to parse %s\n", source_file);
        return 1;
    }

    // Format the parse tree
    FormatterConfig *config = create_default_config();
    Buffer *formatted = format_parse_tree(tree, config);

    if (!formatted) {
        fprintf(stderr, "Error: Failed to format %s\n", source_file);
        free(config);
        return 1;
    }

    // Output formatted code
    if (write_to_file) {
        // Write back to the original file
        if (!write_file(source_file, formatted)) {
            fprintf(stderr, "Error: Failed to write to %s\n", source_file);
            free(config);
            free_buffer(formatted);
            return 1;
        }
        printf("Formatted %s\n", source_file);
    } else {
        // Print to stdout
        printf("%s", formatted->data);
    }

    free(config);
    free_buffer(formatted);
    return 0;
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
    ParseTree *tree = parse(parser);

    if (!tree) {
        return 1;
    }

    // Generate machine code
    Buffer *code = generate_code(tree);

    // Write the new executable
    write_file(source_file, code);

    return 0;
}

int main(int argc, char *argv[]) {
    if (argc < 3) {
        print_usage(argv[0]);
        return 1;
    }

    if (strcmp(argv[1], "run") == 0) {
        return command_run(argv[2]);
    }

    if (strcmp(argv[1], "lex") == 0) {
        return command_lex(argv[2]);
    }

    if (strcmp(argv[1], "format") == 0) {
        // Check for --write flag
        if (argc == 4 && strcmp(argv[2], "--write") == 0) {
            return command_format(argv[3], 1);
        } else if (argc == 3) {
            return command_format(argv[2], 0);
        } else {
            print_usage(argv[0]);
            return 1;
        }
    }

    print_usage(argv[0]);
    return 1;
}
