#include "arena.h"
#include "ast_builder.h"
#include "code_generation.h"
#include "formatter.h"
#include "interpreter.h"
#include "io.h"
#include "lexer.h"
#include "parse_tree_printer.h"
#include "parser.h"
#include "string_storage.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void print_usage(char *program_name) {
    printf("Usage: %s run <source file>.suru\n", program_name);
    printf("       %s lex <source file>.suru\n", program_name);
    printf("       %s parse <source file>.suru\n", program_name);
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

int command_parse(char *source_file) {
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

    // Check for syntax errors
    if (parser->errors && parser->errors->count > 0) {
        fprintf(stderr, "Syntax errors found in %s:\n", source_file);
        ParserError *error = parser->errors->head;
        while (error) {
            fprintf(stderr, "  Line %d:%d: %s\n", error->line, error->column, error->message);
            error = error->next;
        }
        return 1;
    }

    // Print the parse tree
    print_parse_tree(tree);

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

    // Check for syntax errors
    if (parser->errors && parser->errors->count > 0) {
        fprintf(stderr, "Syntax errors found in %s:\n", source_file);
        ParserError *error = parser->errors->head;
        while (error) {
            fprintf(stderr, "  Line %d:%d: %s\n", error->line, error->column, error->message);
            error = error->next;
        }
        return 1;
    }

    // Format the parse tree
    if (write_to_file) {
        // Write back to the original file
        FILE *file = fopen(source_file, "w");
        if (!file) {
            fprintf(stderr, "Error: Failed to open %s for writing\n", source_file);
            return 1;
        }
        format_to_file(arena, tree, file);
        fclose(file);
        printf("Formatted %s\n", source_file);
    } else {
        // Print to stdout
        format_to_stdout(arena, tree);
    }

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

    // Check for syntax errors
    if (parser->errors && parser->errors->count > 0) {
        fprintf(stderr, "Syntax errors found in %s:\n", source_file);
        ParserError *error = parser->errors->head;
        while (error) {
            fprintf(stderr, "  Line %d:%d: %s\n", error->line, error->column, error->message);
            error = error->next;
        }
        return 1;
    }

    // Build AST from parse tree
    AST *ast = build_ast_from_parse_tree(arena, tree);
    if (!ast) {
        fprintf(stderr, "Error: Failed to build AST\n");
        return 1;
    }

    // Create and run interpreter
    Interpreter *interpreter = create_interpreter(arena, ast);
    if (!interpreter) {
        fprintf(stderr, "Error: Failed to create interpreter\n");
        return 1;
    }

    return interpret(interpreter);
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

    if (strcmp(argv[1], "parse") == 0) {
        return command_parse(argv[2]);
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
