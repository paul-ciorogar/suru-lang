#ifndef FORMATTER_H
#define FORMATTER_H

#include "arena.h"
#include "array.h"
#include "parse_tree.h"
#include <stdbool.h>
#include <stdio.h>

#define PAGE_SIZE 4096

// Formatter state machine states
typedef enum {
    FORMAT_NODE,           // Format a single node
    FORMAT_CHILDREN,       // Format all children of a node
    FORMAT_TERMINAL,       // Format a terminal node
    FORMAT_COMMENT,
    FORMAT_LITERAL_TEXT,   // Output literal text
    FORMAT_INDENT_DEC,     // Decrease indentation
} FormatterState;

// Stack frame for formatting
typedef struct {
    FormatterState state;
    int node_idx;          // Index of node to format
    int child_idx;         // For FORMAT_CHILDREN: current child index (-1 = not started)
    const char *text;      // For FORMAT_LITERAL_TEXT: text to output
} FormatterStackFrame;

// Formatter state
typedef struct {
    Arena *arena;
    ParseTree *tree;
    Array *stack;          // Stack of FormatterStackFrame

    // Output page (single page buffer)
    char page[PAGE_SIZE];
    size_t page_used;      // Bytes used in current page

    // Output destination
    FILE *output_file;     // NULL for stdout

    // Formatting state
    int current_indent;    // Current indentation level
    int current_column;    // Current column position
    bool at_line_start;    // True if at the start of a line
} Formatter;

// Create a formatter with file output (NULL for stdout)
Formatter *create_formatter(Arena *arena, ParseTree *tree, FILE *output_file);

// Format the parse tree to stdout
void format_to_stdout(Arena *arena, ParseTree *tree);

// Format the parse tree to a file
void format_to_file(Arena *arena, ParseTree *tree, FILE *file);

#endif // FORMATTER_H
