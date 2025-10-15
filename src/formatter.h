#ifndef FORMATTER_H
#define FORMATTER_H

#include "io.h"
#include "parse_tree.h"
#include <stdbool.h>

// Formatter configuration
typedef struct {
    int indent_size;      // Number of spaces per indent level (default: 4)
    int max_line_width;   // Maximum line width (default: 100)
    bool use_tabs;        // Use tabs instead of spaces (default: false)
} FormatterConfig;

// Formatter state
typedef struct {
    FormatterConfig *config;
    ParseTree *tree;
    Buffer *output;        // Output buffer for formatted code
    int current_indent;    // Current indentation level
    int current_column;    // Current column position
    bool at_line_start;    // True if at the start of a line
} Formatter;

// Create default formatter configuration
FormatterConfig *create_default_config();

// Create a formatter
Formatter *create_formatter(ParseTree *tree, FormatterConfig *config);

// Format the parse tree and return formatted output
Buffer *format_parse_tree(ParseTree *tree, FormatterConfig *config);

// Free formatter resources
void free_formatter(Formatter *formatter);

#endif // FORMATTER_H
