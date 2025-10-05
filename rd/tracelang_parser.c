#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <stdbool.h>

// Forward declarations
typedef struct Arena Arena;
void* arena_alloc(Arena* arena, size_t size);

// Token types
typedef enum {
    TOKEN_EOF,
    TOKEN_IDENTIFIER,
    TOKEN_NUMBER,
    TOKEN_STRING,
    TOKEN_BOOLEAN,
    TOKEN_MODULE,
    TOKEN_IMPORT,
    TOKEN_EXPORT,
    TOKEN_TYPE,
    TOKEN_RETURN,
    TOKEN_MATCH,
    TOKEN_AND,
    TOKEN_OR,
    TOKEN_TRUE,
    TOKEN_FALSE,
    
    // Symbols
    TOKEN_COLON,
    TOKEN_SEMICOLON,
    TOKEN_COMMA,
    TOKEN_DOT,
    TOKEN_PIPE,
    TOKEN_UNDERSCORE,
    TOKEN_STAR,
    TOKEN_EQUALS,
    
    // Brackets
    TOKEN_LPAREN,
    TOKEN_RPAREN,
    TOKEN_LBRACE,
    TOKEN_RBRACE,
    TOKEN_LBRACKET,
    TOKEN_RBRACKET,
    TOKEN_LANGLE,
    TOKEN_RANGLE,
    
    // String interpolation
    TOKEN_BACKTICK,
    TOKEN_DOUBLE_BACKTICK,
    TOKEN_TRIPLE_BACKTICK,
    TOKEN_QUAD_BACKTICK,
    
    // Documentation
    TOKEN_DOC_DELIMITER,
    TOKEN_DOC_KEYWORD,
    TOKEN_DOC_TEXT,
    
    // Newlines and comments
    TOKEN_NEWLINE,
    TOKEN_COMMENT,
    
    TOKEN_ERROR
} TokenType;

typedef struct {
    TokenType type;
    char* text;
    size_t length;
    int line;
    int column;
} Token;

typedef struct {
    char* source;
    size_t pos;
    size_t length;
    int line;
    int column;
    Token current_token;
    Arena* arena;
} Lexer;

// AST Node types
typedef enum {
    NODE_SOURCE_FILE,
    NODE_MODULE_STATEMENT,
    NODE_IMPORT_BLOCK,
    NODE_EXPORT_BLOCK,
    NODE_FUNCTION_DECLARATION,
    NODE_VARIABLE_DECLARATION,
    NODE_TYPE_DECLARATION,
    NODE_EXPRESSION_STATEMENT,
    NODE_IDENTIFIER,
    NODE_NUMBER,
    NODE_STRING,
    NODE_BOOLEAN,
    NODE_LIST,
    NODE_DICTIONARY,
    NODE_BINARY_EXPRESSION,
    NODE_CALL_EXPRESSION,
    NODE_MEMBER_ACCESS,
    NODE_PIPE_EXPRESSION,
    NODE_ASSIGNMENT_EXPRESSION,
    NODE_RETURN_STATEMENT,
    NODE_MATCH_EXPRESSION,
    NODE_TYPE_IMPLEMENTATION,
    NODE_DOCUMENTATION_BLOCK,
    NODE_PARAMETER,
    NODE_TYPE_PARAMETER,
    NODE_PATTERN,
    NODE_MATCH_ARM,
    NODE_FIELD_DECLARATION,
    NODE_METHOD_DECLARATION,
    NODE_BLOCK
} NodeType;

typedef struct ASTNode ASTNode;
typedef struct ASTList ASTList;

struct ASTList {
    ASTNode** nodes;
    size_t count;
    size_t capacity;
};

struct ASTNode {
    NodeType type;
    int line;
    int column;
    
    union {
        // Leaf nodes
        struct {
            char* value;
        } identifier;
        
        struct {
            char* value;
        } number;
        
        struct {
            char* value;
        } string;
        
        struct {
            bool value;
        } boolean;
        
        // Container nodes
        struct {
            ASTNode* module_statement;
            ASTNode* import_block;
            ASTNode* export_block;
            ASTList* items;
        } source_file;
        
        struct {
            ASTNode* documentation;
            ASTNode* name;
        } module_statement;
        
        struct {
            ASTList* imports;
        } import_block;
        
        struct {
            ASTList* exports;
        } export_block;
        
        struct {
            ASTNode* documentation;
            ASTNode* name;
            ASTList* parameters;
            ASTNode* return_type;
            ASTNode* body;
        } function_declaration;
        
        struct {
            ASTNode* documentation;
            ASTNode* name;
            ASTNode* type;
            ASTNode* value;
        } variable_declaration;
        
        struct {
            ASTNode* documentation;
            ASTNode* name;
            ASTList* type_parameters;
            ASTNode* definition;
        } type_declaration;
        
        struct {
            ASTNode* left;
            char* operator;
            ASTNode* right;
        } binary_expression;
        
        struct {
            ASTNode* function;
            ASTList* arguments;
        } call_expression;
        
        struct {
            ASTNode* object;
            ASTNode* property;
        } member_access;
        
        struct {
            ASTNode* left;
            ASTNode* right;
        } pipe_expression;
        
        struct {
            ASTNode* left;
            ASTNode* right;
        } assignment_expression;
        
        struct {
            ASTNode* value;
        } return_statement;
        
        struct {
            ASTNode* value;
            ASTList* arms;
        } match_expression;
        
        struct {
            ASTNode* pattern;
            ASTNode* value;
        } match_arm;
        
        struct {
            ASTList* elements;
        } list;
        
        struct {
            ASTList* entries;
        } dictionary;
        
        struct {
            ASTNode* name;
            ASTNode* type;
        } parameter;
        
        struct {
            ASTList* statements;
        } block;
        
        struct {
            char* content;
        } documentation_block;
        
        struct {
            ASTList* fields;
            ASTList* methods;
        } type_implementation;
    };
};

typedef struct {
    Lexer* lexer;
    Arena* arena;
    bool has_error;
    char* error_message;
} Parser;

// Utility functions
static ASTList* create_ast_list(Arena* arena) {
    ASTList* list = arena_alloc(arena, sizeof(ASTList));
    list->nodes = NULL;
    list->count = 0;
    list->capacity = 0;
    return list;
}

static void ast_list_add(Arena* arena, ASTList* list, ASTNode* node) {
    if (list->count >= list->capacity) {
        size_t new_capacity = list->capacity == 0 ? 4 : list->capacity * 2;
        ASTNode** new_nodes = arena_alloc(arena, sizeof(ASTNode*) * new_capacity);
        if (list->nodes) {
            memcpy(new_nodes, list->nodes, sizeof(ASTNode*) * list->count);
        }
        list->nodes = new_nodes;
        list->capacity = new_capacity;
    }
    list->nodes[list->count++] = node;
}

static ASTNode* create_node(Arena* arena, NodeType type, int line, int column) {
    ASTNode* node = arena_alloc(arena, sizeof(ASTNode));
    memset(node, 0, sizeof(ASTNode));
    node->type = type;
    node->line = line;
    node->column = column;
    return node;
}

static char* clone_string(Arena* arena, const char* str, size_t len) {
    char* result = arena_alloc(arena, len + 1);
    memcpy(result, str, len);
    result[len] = '\0';
    return result;
}

// Lexer functions
static bool is_identifier_start(char c) {
    return isalpha(c) || c == '_';
}

static bool is_identifier_char(char c) {
    return isalnum(c) || c == '_';
}

static bool is_digit(char c) {
    return c >= '0' && c <= '9';
}

static void advance_lexer(Lexer* lexer) {
    if (lexer->pos < lexer->length) {
        if (lexer->source[lexer->pos] == '\n') {
            lexer->line++;
            lexer->column = 1;
        } else {
            lexer->column++;
        }
        lexer->pos++;
    }
}

static char current_char(Lexer* lexer) {
    if (lexer->pos >= lexer->length) return '\0';
    return lexer->source[lexer->pos];
}

static char peek_char(Lexer* lexer, size_t offset) {
    size_t pos = lexer->pos + offset;
    if (pos >= lexer->length) return '\0';
    return lexer->source[pos];
}

static void skip_whitespace(Lexer* lexer) {
    while (lexer->pos < lexer->length) {
        char c = current_char(lexer);
        if (c == ' ' || c == '\t' || c == '\r') {
            advance_lexer(lexer);
        } else {
            break;
        }
    }
}

static Token read_identifier_or_keyword(Lexer* lexer) {
    Token token = {0};
    token.line = lexer->line;
    token.column = lexer->column;
    
    size_t start = lexer->pos;
    while (lexer->pos < lexer->length && is_identifier_char(current_char(lexer))) {
        advance_lexer(lexer);
    }
    
    token.text = lexer->source + start;
    token.length = lexer->pos - start;
    
    // Check for keywords
    if (token.length == 6 && strncmp(token.text, "module", 6) == 0) {
        token.type = TOKEN_MODULE;
    } else if (token.length == 6 && strncmp(token.text, "import", 6) == 0) {
        token.type = TOKEN_IMPORT;
    } else if (token.length == 6 && strncmp(token.text, "export", 6) == 0) {
        token.type = TOKEN_EXPORT;
    } else if (token.length == 4 && strncmp(token.text, "type", 4) == 0) {
        token.type = TOKEN_TYPE;
    } else if (token.length == 6 && strncmp(token.text, "return", 6) == 0) {
        token.type = TOKEN_RETURN;
    } else if (token.length == 5 && strncmp(token.text, "match", 5) == 0) {
        token.type = TOKEN_MATCH;
    } else if (token.length == 3 && strncmp(token.text, "and", 3) == 0) {
        token.type = TOKEN_AND;
    } else if (token.length == 2 && strncmp(token.text, "or", 2) == 0) {
        token.type = TOKEN_OR;
    } else if (token.length == 4 && strncmp(token.text, "true", 4) == 0) {
        token.type = TOKEN_TRUE;
    } else if (token.length == 5 && strncmp(token.text, "false", 5) == 0) {
        token.type = TOKEN_FALSE;
    } else {
        token.type = TOKEN_IDENTIFIER;
    }
    
    return token;
}

static Token read_number(Lexer* lexer) {
    Token token = {0};
    token.type = TOKEN_NUMBER;
    token.line = lexer->line;
    token.column = lexer->column;
    
    size_t start = lexer->pos;
    
    // Read integer part
    while (lexer->pos < lexer->length && is_digit(current_char(lexer))) {
        advance_lexer(lexer);
    }
    
    // Check for decimal point
    if (current_char(lexer) == '.' && is_digit(peek_char(lexer, 1))) {
        advance_lexer(lexer); // skip '.'
        while (lexer->pos < lexer->length && is_digit(current_char(lexer))) {
            advance_lexer(lexer);
        }
    }
    
    token.text = lexer->source + start;
    token.length = lexer->pos - start;
    return token;
}

static Token read_string(Lexer* lexer, char quote) {
    Token token = {0};
    token.type = TOKEN_STRING;
    token.line = lexer->line;
    token.column = lexer->column;
    
    size_t start = lexer->pos;
    advance_lexer(lexer); // skip opening quote
    
    while (lexer->pos < lexer->length && current_char(lexer) != quote) {
        if (current_char(lexer) == '\\') {
            advance_lexer(lexer); // skip backslash
            if (lexer->pos < lexer->length) {
                advance_lexer(lexer); // skip escaped char
            }
        } else {
            advance_lexer(lexer);
        }
    }
    
    if (current_char(lexer) == quote) {
        advance_lexer(lexer); // skip closing quote
    }
    
    token.text = lexer->source + start;
    token.length = lexer->pos - start;
    return token;
}

static Token read_comment(Lexer* lexer) {
    Token token = {0};
    token.type = TOKEN_COMMENT;
    token.line = lexer->line;
    token.column = lexer->column;
    
    size_t start = lexer->pos;
    advance_lexer(lexer); // skip first '/'
    advance_lexer(lexer); // skip second '/'
    
    while (lexer->pos < lexer->length && current_char(lexer) != '\n') {
        advance_lexer(lexer);
    }
    
    token.text = lexer->source + start;
    token.length = lexer->pos - start;
    return token;
}

static Token read_doc_delimiter(Lexer* lexer) {
    Token token = {0};
    token.type = TOKEN_DOC_DELIMITER;
    token.line = lexer->line;
    token.column = lexer->column;
    
    size_t start = lexer->pos;
    while (lexer->pos < lexer->length && current_char(lexer) == '=') {
        advance_lexer(lexer);
    }
    
    token.text = lexer->source + start;
    token.length = lexer->pos - start;
    return token;
}

static Token next_token(Lexer* lexer) {
    skip_whitespace(lexer);
    
    if (lexer->pos >= lexer->length) {
        Token eof = {TOKEN_EOF, NULL, 0, lexer->line, lexer->column};
        return eof;
    }
    
    char c = current_char(lexer);
    
    if (c == '\n') {
        Token token = {TOKEN_NEWLINE, lexer->source + lexer->pos, 1, lexer->line, lexer->column};
        advance_lexer(lexer);
        return token;
    }
    
    if (is_identifier_start(c)) {
        return read_identifier_or_keyword(lexer);
    }
    
    if (is_digit(c)) {
        return read_number(lexer);
    }
    
    if (c == '"' || c == '\'') {
        return read_string(lexer, c);
    }
    
    if (c == '/' && peek_char(lexer, 1) == '/') {
        return read_comment(lexer);
    }
    
    if (c == '=' && peek_char(lexer, 1) == '=' && peek_char(lexer, 2) == '=' && peek_char(lexer, 3) == '=') {
        return read_doc_delimiter(lexer);
    }
    
    // Single character tokens
    Token token = {TOKEN_ERROR, lexer->source + lexer->pos, 1, lexer->line, lexer->column};
    
    switch (c) {
        case ':': token.type = TOKEN_COLON; break;
        case ';': token.type = TOKEN_SEMICOLON; break;
        case ',': token.type = TOKEN_COMMA; break;
        case '.': token.type = TOKEN_DOT; break;
        case '|': token.type = TOKEN_PIPE; break;
        case '_': token.type = TOKEN_UNDERSCORE; break;
        case '*': token.type = TOKEN_STAR; break;
        case '(': token.type = TOKEN_LPAREN; break;
        case ')': token.type = TOKEN_RPAREN; break;
        case '{': token.type = TOKEN_LBRACE; break;
        case '}': token.type = TOKEN_RBRACE; break;
        case '[': token.type = TOKEN_LBRACKET; break;
        case ']': token.type = TOKEN_RBRACKET; break;
        case '<': token.type = TOKEN_LANGLE; break;
        case '>': token.type = TOKEN_RANGLE; break;
        case '`': token.type = TOKEN_BACKTICK; break;
    }
    
    advance_lexer(lexer);
    return token;
}

static void lexer_init(Lexer* lexer, char* source, Arena* arena) {
    lexer->source = source;
    lexer->pos = 0;
    lexer->length = strlen(source);
    lexer->line = 1;
    lexer->column = 1;
    lexer->arena = arena;
    lexer->current_token = next_token(lexer);
}

// Parser functions
static void parser_init(Parser* parser, Lexer* lexer, Arena* arena) {
    parser->lexer = lexer;
    parser->arena = arena;
    parser->has_error = false;
    parser->error_message = NULL;
}

static void parser_error(Parser* parser, const char* message) {
    parser->has_error = true;
    size_t len = strlen(message);
    parser->error_message = arena_alloc(parser->arena, len + 1);
    strcpy(parser->error_message, message);
}

static Token current_token(Parser* parser) {
    return parser->lexer->current_token;
}

static void advance_parser(Parser* parser) {
    parser->lexer->current_token = next_token(parser->lexer);
}

static bool match_token(Parser* parser, TokenType type) {
    return current_token(parser).type == type;
}

static bool consume_token(Parser* parser, TokenType type) {
    if (match_token(parser, type)) {
        advance_parser(parser);
        return true;
    }
    return false;
}

static void skip_newlines(Parser* parser) {
    while (match_token(parser, TOKEN_NEWLINE)) {
        advance_parser(parser);
    }
}

// Forward declarations for recursive parsing functions
static ASTNode* parse_expression(Parser* parser);
static ASTNode* parse_type(Parser* parser);
static ASTNode* parse_block(Parser* parser);
static ASTNode* parse_documentation_block(Parser* parser);

static ASTNode* parse_identifier(Parser* parser) {
    if (!match_token(parser, TOKEN_IDENTIFIER)) {
        parser_error(parser, "Expected identifier");
        return NULL;
    }
    
    Token token = current_token(parser);
    advance_parser(parser);
    
    ASTNode* node = create_node(parser->arena, NODE_IDENTIFIER, token.line, token.column);
    node->identifier.value = clone_string(parser->arena, token.text, token.length);
    return node;
}

static ASTNode* parse_number(Parser* parser) {
    if (!match_token(parser, TOKEN_NUMBER)) {
        parser_error(parser, "Expected number");
        return NULL;
    }
    
    Token token = current_token(parser);
    advance_parser(parser);
    
    ASTNode* node = create_node(parser->arena, NODE_NUMBER, token.line, token.column);
    node->number.value = clone_string(parser->arena, token.text, token.length);
    return node;
}

static ASTNode* parse_string(Parser* parser) {
    if (!match_token(parser, TOKEN_STRING)) {
        parser_error(parser, "Expected string");
        return NULL;
    }
    
    Token token = current_token(parser);
    advance_parser(parser);
    
    ASTNode* node = create_node(parser->arena, NODE_STRING, token.line, token.column);
    node->string.value = clone_string(parser->arena, token.text, token.length);
    return node;
}

static ASTNode* parse_boolean(Parser* parser) {
    if (!match_token(parser, TOKEN_TRUE) && !match_token(parser, TOKEN_FALSE)) {
        parser_error(parser, "Expected boolean");
        return NULL;
    }
    
    Token token = current_token(parser);
    advance_parser(parser);
    
    ASTNode* node = create_node(parser->arena, NODE_BOOLEAN, token.line, token.column);
    node->boolean.value = (token.type == TOKEN_TRUE);
    return node;
}

static ASTNode* parse_list(Parser* parser) {
    if (!consume_token(parser, TOKEN_LBRACKET)) {
        parser_error(parser, "Expected '['");
        return NULL;
    }
    
    ASTNode* node = create_node(parser->arena, NODE_LIST, parser->lexer->line, parser->lexer->column);
    node->list.elements = create_ast_list(parser->arena);
    
    skip_newlines(parser);
    
    if (!match_token(parser, TOKEN_RBRACKET)) {
        do {
            ASTNode* element = parse_expression(parser);
            if (element) {
                ast_list_add(parser->arena, node->list.elements, element);
            }
            
            skip_newlines(parser);
            if (match_token(parser, TOKEN_COMMA)) {
                advance_parser(parser);
                skip_newlines(parser);
            } else {
                break;
            }
        } while (!match_token(parser, TOKEN_RBRACKET) && !match_token(parser, TOKEN_EOF));
    }
    
    if (!consume_token(parser, TOKEN_RBRACKET)) {
        parser_error(parser, "Expected ']'");
    }
    
    return node;
}

static ASTNode* parse_parameter(Parser* parser) {
    ASTNode* name = parse_identifier(parser);
    if (!name) return NULL;
    
    ASTNode* node = create_node(parser->arena, NODE_PARAMETER, name->line, name->column);
    node->parameter.name = name;
    
    // Optional type annotation
    if (match_token(parser, TOKEN_IDENTIFIER)) {
        node->parameter.type = parse_type(parser);
    }
    
    return node;
}

static ASTList* parse_parameter_list(Parser* parser) {
    if (!consume_token(parser, TOKEN_LPAREN)) {
        parser_error(parser, "Expected '('");
        return NULL;
    }
    
    ASTList* parameters = create_ast_list(parser->arena);
    
    skip_newlines(parser);
    
    if (!match_token(parser, TOKEN_RPAREN)) {
        do {
            ASTNode* param = parse_parameter(parser);
            if (param) {
                ast_list_add(parser->arena, parameters, param);
            }
            
            skip_newlines(parser);
            if (match_token(parser, TOKEN_COMMA)) {
                advance_parser(parser);
                skip_newlines(parser);
            } else {
                break;
            }
        } while (!match_token(parser, TOKEN_RPAREN) && !match_token(parser, TOKEN_EOF));
    }
    
    if (!consume_token(parser, TOKEN_RPAREN)) {
        parser_error(parser, "Expected ')'");
    }
    
    return parameters;
}

static ASTNode* parse_function_declaration(Parser* parser) {
    Token start_token = current_token(parser);
    
    // Optional documentation
    ASTNode* documentation = NULL;
    if (match_token(parser, TOKEN_DOC_DELIMITER)) {
        documentation = parse_documentation_block(parser);
    }
    
    ASTNode* name = parse_identifier(parser);
    if (!name) return NULL;
    
    if (!consume_token(parser, TOKEN_COLON)) {
        parser_error(parser, "Expected ':' after function name");
        return NULL;
    }
    
    ASTList* parameters = parse_parameter_list(parser);
    if (!parameters) return NULL;
    
    // Optional return type
    ASTNode* return_type = NULL;
    if (match_token(parser, TOKEN_IDENTIFIER)) {
        return_type = parse_type(parser);
    }
    
    ASTNode* body = parse_block(parser);
    if (!body) return NULL;
    
    // Consume statement end
    if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
        advance_parser(parser);
    }
    
    ASTNode* node = create_node(parser->arena, NODE_FUNCTION_DECLARATION, start_token.line, start_token.column);
    node->function_declaration.documentation = documentation;
    node->function_declaration.name = name;
    node->function_declaration.parameters = parameters;
    node->function_declaration.return_type = return_type;
    node->function_declaration.body = body;
    
    return node;
}

static ASTNode* parse_variable_declaration(Parser* parser) {
    Token start_token = current_token(parser);
    
    // Optional documentation
    ASTNode* documentation = NULL;
    if (match_token(parser, TOKEN_DOC_DELIMITER)) {
        documentation = parse_documentation_block(parser);
    }
    
    ASTNode* name = parse_identifier(parser);
    if (!name) return NULL;
    
    // Optional type annotation
    ASTNode* type = NULL;
    if (match_token(parser, TOKEN_IDENTIFIER)) {
        type = parse_type(parser);
    }
    
    if (!consume_token(parser, TOKEN_COLON)) {
        parser_error(parser, "Expected ':' in variable declaration");
        return NULL;
    }
    
    ASTNode* value = parse_expression(parser);
    if (!value) return NULL;
    
    // Consume statement end
    if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
        advance_parser(parser);
    }
    
    ASTNode* node = create_node(parser->arena, NODE_VARIABLE_DECLARATION, start_token.line, start_token.column);
    node->variable_declaration.documentation = documentation;
    node->variable_declaration.name = name;
    node->variable_declaration.type = type;
    node->variable_declaration.value = value;
    
    return node;
}

static ASTNode* parse_primary_expression(Parser* parser) {
    if (match_token(parser, TOKEN_IDENTIFIER)) {
        return parse_identifier(parser);
    } else if (match_token(parser, TOKEN_NUMBER)) {
        return parse_number(parser);
    } else if (match_token(parser, TOKEN_STRING)) {
        return parse_string(parser);
    } else if (match_token(parser, TOKEN_TRUE) || match_token(parser, TOKEN_FALSE)) {
        return parse_boolean(parser);
    } else if (match_token(parser, TOKEN_LBRACKET)) {
        return parse_list(parser);
    } else if (match_token(parser, TOKEN_LBRACE)) {
        return parse_block(parser);
    }
    
    parser_error(parser, "Expected primary expression");
    return NULL;
}

static ASTNode* parse_call_expression(Parser* parser, ASTNode* function) {
    if (!match_token(parser, TOKEN_LPAREN)) {
        return function;
    }
    
    advance_parser(parser); // consume '('
    
    ASTNode* node = create_node(parser->arena, NODE_CALL_EXPRESSION, function->line, function->column);
    node->call_expression.function = function;
    node->call_expression.arguments = create_ast_list(parser->arena);
    
    skip_newlines(parser);
    
    if (!match_token(parser, TOKEN_RPAREN)) {
        do {
            ASTNode* arg = parse_expression(parser);
            if (arg) {
                ast_list_add(parser->arena, node->call_expression.arguments, arg);
            }
            
            skip_newlines(parser);
            if (match_token(parser, TOKEN_COMMA)) {
                advance_parser(parser);
                skip_newlines(parser);
            } else {
                break;
            }
        } while (!match_token(parser, TOKEN_RPAREN) && !match_token(parser, TOKEN_EOF));
    }
    
    if (!consume_token(parser, TOKEN_RPAREN)) {
        parser_error(parser, "Expected ')' after arguments");
    }
    
    return node;
}

static ASTNode* parse_member_access(Parser* parser, ASTNode* object) {
    while (match_token(parser, TOKEN_DOT)) {
        advance_parser(parser); // consume '.'
        
        ASTNode* property = parse_identifier(parser);
        if (!property) return NULL;
        
        ASTNode* node = create_node(parser->arena, NODE_MEMBER_ACCESS, object->line, object->column);
        node->member_access.object = object;
        node->member_access.property = property;
        
        object = node;
    }
    
    return object;
}

static ASTNode* parse_postfix_expression(Parser* parser) {
    ASTNode* expr = parse_primary_expression(parser);
    if (!expr) return NULL;
    
    while (true) {
        if (match_token(parser, TOKEN_LPAREN)) {
            expr = parse_call_expression(parser, expr);
        } else if (match_token(parser, TOKEN_DOT)) {
            expr = parse_member_access(parser, expr);
        } else {
            break;
        }
    }
    
    return expr;
}

static ASTNode* parse_pipe_expression(Parser* parser) {
    ASTNode* left = parse_postfix_expression(parser);
    if (!left) return NULL;
    
    while (match_token(parser, TOKEN_PIPE)) {
        advance_parser(parser); // consume '|'
        
        ASTNode* right = parse_postfix_expression(parser);
        if (!right) return NULL;
        
        ASTNode* node = create_node(parser->arena, NODE_PIPE_EXPRESSION, left->line, left->column);
        node->pipe_expression.left = left;
        node->pipe_expression.right = right;
        
        left = node;
    }
    
    return left;
}

static ASTNode* parse_binary_expression(Parser* parser) {
    ASTNode* left = parse_pipe_expression(parser);
    if (!left) return NULL;
    
    while (match_token(parser, TOKEN_AND) || match_token(parser, TOKEN_OR)) {
        Token op_token = current_token(parser);
        advance_parser(parser);
        
        ASTNode* right = parse_pipe_expression(parser);
        if (!right) return NULL;
        
        ASTNode* node = create_node(parser->arena, NODE_BINARY_EXPRESSION, left->line, left->column);
        node->binary_expression.left = left;
        node->binary_expression.operator = clone_string(parser->arena, op_token.text, op_token.length);
        node->binary_expression.right = right;
        
        left = node;
    }
    
    return left;
}

static ASTNode* parse_assignment_expression(Parser* parser) {
    ASTNode* left = parse_binary_expression(parser);
    if (!left) return NULL;
    
    // Check if this is an assignment (identifier : expression)
    if (left->type == NODE_IDENTIFIER && match_token(parser, TOKEN_COLON)) {
        advance_parser(parser); // consume ':'
        
        ASTNode* right = parse_expression(parser);
        if (!right) return NULL;
        
        ASTNode* node = create_node(parser->arena, NODE_ASSIGNMENT_EXPRESSION, left->line, left->column);
        node->assignment_expression.left = left;
        node->assignment_expression.right = right;
        
        return node;
    }
    
    return left;
}

static ASTNode* parse_return_statement(Parser* parser) {
    if (!match_token(parser, TOKEN_RETURN)) {
        return parse_assignment_expression(parser);
    }
    
    Token return_token = current_token(parser);
    advance_parser(parser); // consume 'return'
    
    ASTNode* node = create_node(parser->arena, NODE_RETURN_STATEMENT, return_token.line, return_token.column);
    
    // Optional return value
    if (!match_token(parser, TOKEN_NEWLINE) && !match_token(parser, TOKEN_SEMICOLON) && !match_token(parser, TOKEN_RBRACE)) {
        node->return_statement.value = parse_assignment_expression(parser);
    }
    
    return node;
}

static ASTNode* parse_match_arm(Parser* parser) {
    ASTNode* pattern = parse_expression(parser); // Patterns are parsed as expressions for now
    if (!pattern) return NULL;
    
    if (!consume_token(parser, TOKEN_COLON)) {
        parser_error(parser, "Expected ':' after match pattern");
        return NULL;
    }
    
    ASTNode* value = parse_expression(parser);
    if (!value) return NULL;
    
    // Consume statement end
    if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
        advance_parser(parser);
    }
    
    ASTNode* node = create_node(parser->arena, NODE_MATCH_ARM, pattern->line, pattern->column);
    node->match_arm.pattern = pattern;
    node->match_arm.value = value;
    
    return node;
}

static ASTNode* parse_match_expression(Parser* parser) {
    if (!match_token(parser, TOKEN_MATCH)) {
        return parse_return_statement(parser);
    }
    
    Token match_token = current_token(parser);
    advance_parser(parser); // consume 'match'
    
    ASTNode* value = parse_expression(parser);
    if (!value) return NULL;
    
    if (!consume_token(parser, TOKEN_LBRACE)) {
        parser_error(parser, "Expected '{' after match expression");
        return NULL;
    }
    
    ASTNode* node = create_node(parser->arena, NODE_MATCH_EXPRESSION, match_token.line, match_token.column);
    node->match_expression.value = value;
    node->match_expression.arms = create_ast_list(parser->arena);
    
    skip_newlines(parser);
    
    while (!match_token(parser, TOKEN_RBRACE) && !match_token(parser, TOKEN_EOF)) {
        ASTNode* arm = parse_match_arm(parser);
        if (arm) {
            ast_list_add(parser->arena, node->match_expression.arms, arm);
        }
        skip_newlines(parser);
    }
    
    if (!consume_token(parser, TOKEN_RBRACE)) {
        parser_error(parser, "Expected '}' after match arms");
    }
    
    return node;
}

static ASTNode* parse_expression(Parser* parser) {
    return parse_match_expression(parser);
}

static ASTNode* parse_type(Parser* parser) {
    // For now, just parse identifiers as types
    // This can be extended to handle generic types, etc.
    return parse_identifier(parser);
}

static ASTNode* parse_block(Parser* parser) {
    if (!consume_token(parser, TOKEN_LBRACE)) {
        parser_error(parser, "Expected '{'");
        return NULL;
    }
    
    ASTNode* node = create_node(parser->arena, NODE_BLOCK, parser->lexer->line, parser->lexer->column);
    node->block.statements = create_ast_list(parser->arena);
    
    skip_newlines(parser);
    
    while (!match_token(parser, TOKEN_RBRACE) && !match_token(parser, TOKEN_EOF)) {
        ASTNode* stmt = parse_expression(parser);
        if (stmt) {
            ast_list_add(parser->arena, node->block.statements, stmt);
        }
        
        // Consume optional statement separator
        if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
            advance_parser(parser);
        }
        
        skip_newlines(parser);
    }
    
    if (!consume_token(parser, TOKEN_RBRACE)) {
        parser_error(parser, "Expected '}'");
    }
    
    return node;
}

static ASTNode* parse_documentation_block(Parser* parser) {
    if (!match_token(parser, TOKEN_DOC_DELIMITER)) {
        return NULL;
    }
    
    Token start_token = current_token(parser);
    advance_parser(parser); // consume opening delimiter
    
    // For now, just consume everything until the closing delimiter
    // In a real implementation, you'd parse the documentation content properly
    size_t start_pos = parser->lexer->pos;
    size_t content_length = 0;
    
    while (!match_token(parser, TOKEN_DOC_DELIMITER) && !match_token(parser, TOKEN_EOF)) {
        advance_parser(parser);
        content_length = parser->lexer->pos - start_pos;
    }
    
    if (!consume_token(parser, TOKEN_DOC_DELIMITER)) {
        parser_error(parser, "Expected closing documentation delimiter");
        return NULL;
    }
    
    ASTNode* node = create_node(parser->arena, NODE_DOCUMENTATION_BLOCK, start_token.line, start_token.column);
    node->documentation_block.content = clone_string(parser->arena, parser->lexer->source + start_pos, content_length);
    
    return node;
}

static ASTNode* parse_module_statement(Parser* parser) {
    Token start_token = current_token(parser);
    
    // Optional documentation
    ASTNode* documentation = NULL;
    if (match_token(parser, TOKEN_DOC_DELIMITER)) {
        documentation = parse_documentation_block(parser);
    }
    
    if (!consume_token(parser, TOKEN_MODULE)) {
        parser_error(parser, "Expected 'module'");
        return NULL;
    }
    
    ASTNode* name = parse_identifier(parser);
    if (!name) return NULL;
    
    // Consume statement end
    if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
        advance_parser(parser);
    }
    
    ASTNode* node = create_node(parser->arena, NODE_MODULE_STATEMENT, start_token.line, start_token.column);
    node->module_statement.documentation = documentation;
    node->module_statement.name = name;
    
    return node;
}

static ASTNode* parse_import_block(Parser* parser) {
    if (!match_token(parser, TOKEN_IMPORT)) {
        return NULL;
    }
    
    Token start_token = current_token(parser);
    advance_parser(parser); // consume 'import'
    
    if (!consume_token(parser, TOKEN_LBRACE)) {
        parser_error(parser, "Expected '{' after 'import'");
        return NULL;
    }
    
    ASTNode* node = create_node(parser->arena, NODE_IMPORT_BLOCK, start_token.line, start_token.column);
    node->import_block.imports = create_ast_list(parser->arena);
    
    skip_newlines(parser);
    
    // Parse import items (simplified for now)
    while (!match_token(parser, TOKEN_RBRACE) && !match_token(parser, TOKEN_EOF)) {
        // For now, just parse identifiers as import items
        // In a full implementation, handle the different import syntaxes
        ASTNode* import_item = parse_identifier(parser);
        if (import_item) {
            ast_list_add(parser->arena, node->import_block.imports, import_item);
        }
        
        if (match_token(parser, TOKEN_COLON)) {
            advance_parser(parser);
            ASTNode* module = parse_identifier(parser);
            // In full implementation, associate module with import item
        }
        
        if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
            advance_parser(parser);
        }
        
        skip_newlines(parser);
    }
    
    if (!consume_token(parser, TOKEN_RBRACE)) {
        parser_error(parser, "Expected '}' after import block");
    }
    
    // Consume statement end
    if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
        advance_parser(parser);
    }
    
    return node;
}

static ASTNode* parse_export_block(Parser* parser) {
    if (!match_token(parser, TOKEN_EXPORT)) {
        return NULL;
    }
    
    Token start_token = current_token(parser);
    advance_parser(parser); // consume 'export'
    
    if (!consume_token(parser, TOKEN_LBRACE)) {
        parser_error(parser, "Expected '{' after 'export'");
        return NULL;
    }
    
    ASTNode* node = create_node(parser->arena, NODE_EXPORT_BLOCK, start_token.line, start_token.column);
    node->export_block.exports = create_ast_list(parser->arena);
    
    skip_newlines(parser);
    
    // Parse export items (simplified)
    while (!match_token(parser, TOKEN_RBRACE) && !match_token(parser, TOKEN_EOF)) {
        ASTNode* export_item = parse_identifier(parser);
        if (export_item) {
            ast_list_add(parser->arena, node->export_block.exports, export_item);
        }
        
        if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
            advance_parser(parser);
        }
        
        skip_newlines(parser);
    }
    
    if (!consume_token(parser, TOKEN_RBRACE)) {
        parser_error(parser, "Expected '}' after export block");
    }
    
    // Consume statement end
    if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
        advance_parser(parser);
    }
    
    return node;
}

static ASTNode* parse_type_declaration(Parser* parser) {
    Token start_token = current_token(parser);
    
    // Optional documentation
    ASTNode* documentation = NULL;
    if (match_token(parser, TOKEN_DOC_DELIMITER)) {
        documentation = parse_documentation_block(parser);
    }
    
    if (!consume_token(parser, TOKEN_TYPE)) {
        parser_error(parser, "Expected 'type'");
        return NULL;
    }
    
    ASTNode* name = parse_identifier(parser);
    if (!name) return NULL;
    
    // Optional type parameters (simplified)
    ASTList* type_parameters = NULL;
    if (match_token(parser, TOKEN_LANGLE)) {
        // Parse type parameters (not fully implemented)
        advance_parser(parser); // consume '<'
        while (!match_token(parser, TOKEN_RANGLE) && !match_token(parser, TOKEN_EOF)) {
            advance_parser(parser); // skip for now
        }
        consume_token(parser, TOKEN_RANGLE);
    }
    
    if (!consume_token(parser, TOKEN_COLON)) {
        parser_error(parser, "Expected ':' after type name");
        return NULL;
    }
    
    // Parse type definition (simplified - just parse as expression for now)
    ASTNode* definition = parse_expression(parser);
    if (!definition) return NULL;
    
    // Consume statement end
    if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
        advance_parser(parser);
    }
    
    ASTNode* node = create_node(parser->arena, NODE_TYPE_DECLARATION, start_token.line, start_token.column);
    node->type_declaration.documentation = documentation;
    node->type_declaration.name = name;
    node->type_declaration.type_parameters = type_parameters;
    node->type_declaration.definition = definition;
    
    return node;
}

static ASTNode* parse_top_level_item(Parser* parser) {
    skip_newlines(parser);
    
    if (match_token(parser, TOKEN_EOF)) {
        return NULL;
    }
    
    // Look ahead to determine what kind of declaration this is
    if (match_token(parser, TOKEN_DOC_DELIMITER)) {
        // Could be any declaration with documentation
        // We'll need to look further ahead, but for now, try function first
        size_t saved_pos = parser->lexer->pos;
        Token saved_token = parser->lexer->current_token;
        
        // Skip documentation
        parse_documentation_block(parser);
        
        if (match_token(parser, TOKEN_TYPE)) {
            // Restore position and parse type declaration
            parser->lexer->pos = saved_pos;
            parser->lexer->current_token = saved_token;
            return parse_type_declaration(parser);
        } else {
            // Restore position and try function or variable
            parser->lexer->pos = saved_pos;
            parser->lexer->current_token = saved_token;
            return parse_function_declaration(parser);
        }
    } else if (match_token(parser, TOKEN_TYPE)) {
        return parse_type_declaration(parser);
    } else if (match_token(parser, TOKEN_IDENTIFIER)) {
        // Could be function or variable declaration
        // Look ahead for the pattern: identifier : ( ... ) -> function
        // or identifier [type] : value -> variable
        size_t saved_pos = parser->lexer->pos;
        Token saved_token = parser->lexer->current_token;
        
        parse_identifier(parser); // consume identifier
        
        // Optional type
        if (match_token(parser, TOKEN_IDENTIFIER)) {
            parse_identifier(parser); // consume type
        }
        
        if (match_token(parser, TOKEN_COLON)) {
            advance_parser(parser); // consume ':'
            
            if (match_token(parser, TOKEN_LPAREN)) {
                // Function declaration
                parser->lexer->pos = saved_pos;
                parser->lexer->current_token = saved_token;
                return parse_function_declaration(parser);
            } else {
                // Variable declaration
                parser->lexer->pos = saved_pos;
                parser->lexer->current_token = saved_token;
                return parse_variable_declaration(parser);
            }
        } else {
            // Expression statement
            parser->lexer->pos = saved_pos;
            parser->lexer->current_token = saved_token;
            ASTNode* expr = parse_expression(parser);
            
            // Consume statement end
            if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
                advance_parser(parser);
            }
            
            ASTNode* node = create_node(parser->arena, NODE_EXPRESSION_STATEMENT, expr->line, expr->column);
            // In full implementation, would have expression_statement field
            return expr; // For now, just return the expression
        }
    } else {
        // Expression statement
        ASTNode* expr = parse_expression(parser);
        if (!expr) return NULL;
        
        // Consume statement end
        if (match_token(parser, TOKEN_NEWLINE) || match_token(parser, TOKEN_SEMICOLON)) {
            advance_parser(parser);
        }
        
        ASTNode* node = create_node(parser->arena, NODE_EXPRESSION_STATEMENT, expr->line, expr->column);
        return expr; // For now, just return the expression
    }
}

static ASTNode* parse_source_file(Parser* parser) {
    ASTNode* node = create_node(parser->arena, NODE_SOURCE_FILE, 1, 1);
    node->source_file.items = create_ast_list(parser->arena);
    
    skip_newlines(parser);
    
    // Optional module statement
    if (match_token(parser, TOKEN_MODULE) || match_token(parser, TOKEN_DOC_DELIMITER)) {
        node->source_file.module_statement = parse_module_statement(parser);
    }
    
    skip_newlines(parser);
    
    // Optional import block
    if (match_token(parser, TOKEN_IMPORT)) {
        node->source_file.import_block = parse_import_block(parser);
    }
    
    skip_newlines(parser);
    
    // Optional export block  
    if (match_token(parser, TOKEN_EXPORT)) {
        node->source_file.export_block = parse_export_block(parser);
    }
    
    skip_newlines(parser);
    
    // Parse top-level items
    while (!match_token(parser, TOKEN_EOF)) {
        ASTNode* item = parse_top_level_item(parser);
        if (item) {
            ast_list_add(parser->arena, node->source_file.items, item);
        } else {
            // Skip unknown tokens to avoid infinite loop
            if (!match_token(parser, TOKEN_EOF)) {
                advance_parser(parser);
            }
        }
        skip_newlines(parser);
    }
    
    return node;
}

// Main parser function
ASTNode* parse_tracelang(char* source, Arena* arena) {
    Lexer lexer;
    lexer_init(&lexer, source, arena);
    
    Parser parser;
    parser_init(&parser, &lexer, arena);
    
    ASTNode* ast = parse_source_file(&parser);
    
    if (parser.has_error) {
        printf("Parse error: %s\n", parser.error_message);
        return NULL;
    }
    
    return ast;
}

// Utility function to print AST (for debugging)
static void print_ast_indent(int depth) {
    for (int i = 0; i < depth * 2; i++) {
        printf(" ");
    }
}

static const char* node_type_name(NodeType type) {
    switch (type) {
        case NODE_SOURCE_FILE: return "SourceFile";
        case NODE_MODULE_STATEMENT: return "ModuleStatement";
        case NODE_IMPORT_BLOCK: return "ImportBlock";
        case NODE_EXPORT_BLOCK: return "ExportBlock";
        case NODE_FUNCTION_DECLARATION: return "FunctionDeclaration";
        case NODE_VARIABLE_DECLARATION: return "VariableDeclaration";
        case NODE_TYPE_DECLARATION: return "TypeDeclaration";
        case NODE_EXPRESSION_STATEMENT: return "ExpressionStatement";
        case NODE_IDENTIFIER: return "Identifier";
        case NODE_NUMBER: return "Number";
        case NODE_STRING: return "String";
        case NODE_BOOLEAN: return "Boolean";
        case NODE_LIST: return "List";
        case NODE_DICTIONARY: return "Dictionary";
        case NODE_BINARY_EXPRESSION: return "BinaryExpression";
        case NODE_CALL_EXPRESSION: return "CallExpression";
        case NODE_MEMBER_ACCESS: return "MemberAccess";
        case NODE_PIPE_EXPRESSION: return "PipeExpression";
        case NODE_ASSIGNMENT_EXPRESSION: return "AssignmentExpression";
        case NODE_RETURN_STATEMENT: return "ReturnStatement";
        case NODE_MATCH_EXPRESSION: return "MatchExpression";
        case NODE_TYPE_IMPLEMENTATION: return "TypeImplementation";
        case NODE_DOCUMENTATION_BLOCK: return "DocumentationBlock";
        case NODE_PARAMETER: return "Parameter";
        case NODE_TYPE_PARAMETER: return "TypeParameter";
        case NODE_PATTERN: return "Pattern";
        case NODE_MATCH_ARM: return "MatchArm";
        case NODE_FIELD_DECLARATION: return "FieldDeclaration";
        case NODE_METHOD_DECLARATION: return "MethodDeclaration";
        case NODE_BLOCK: return "Block";
        default: return "Unknown";
    }
}

void print_ast(ASTNode* node, int depth) {
    if (!node) return;
    
    print_ast_indent(depth);
    printf("%s", node_type_name(node->type));
    
    switch (node->type) {
        case NODE_IDENTIFIER:
            printf(": %s", node->identifier.value);
            break;
        case NODE_NUMBER:
            printf(": %s", node->number.value);
            break;
        case NODE_STRING:
            printf(": %s", node->string.value);
            break;
        case NODE_BOOLEAN:
            printf(": %s", node->boolean.value ? "true" : "false");
            break;
        case NODE_BINARY_EXPRESSION:
            printf(": %s", node->binary_expression.operator);
            break;
        default:
            break;
    }
    
    printf("\n");
    
    // Print children based on node type
    switch (node->type) {
        case NODE_SOURCE_FILE:
            if (node->source_file.module_statement) {
                print_ast(node->source_file.module_statement, depth + 1);
            }
            if (node->source_file.import_block) {
                print_ast(node->source_file.import_block, depth + 1);
            }
            if (node->source_file.export_block) {
                print_ast(node->source_file.export_block, depth + 1);
            }
            if (node->source_file.items) {
                for (size_t i = 0; i < node->source_file.items->count; i++) {
                    print_ast(node->source_file.items->nodes[i], depth + 1);
                }
            }
            break;
            
        case NODE_FUNCTION_DECLARATION:
            if (node->function_declaration.documentation) {
                print_ast(node->function_declaration.documentation, depth + 1);
            }
            print_ast(node->function_declaration.name, depth + 1);
            if (node->function_declaration.parameters) {
                for (size_t i = 0; i < node->function_declaration.parameters->count; i++) {
                    print_ast(node->function_declaration.parameters->nodes[i], depth + 1);
                }
            }
            if (node->function_declaration.return_type) {
                print_ast(node->function_declaration.return_type, depth + 1);
            }
            print_ast(node->function_declaration.body, depth + 1);
            break;
            
        case NODE_VARIABLE_DECLARATION:
            if (node->variable_declaration.documentation) {
                print_ast(node->variable_declaration.documentation, depth + 1);
            }
            print_ast(node->variable_declaration.name, depth + 1);
            if (node->variable_declaration.type) {
                print_ast(node->variable_declaration.type, depth + 1);
            }
            print_ast(node->variable_declaration.value, depth + 1);
            break;
            
        case NODE_BINARY_EXPRESSION:
            print_ast(node->binary_expression.left, depth + 1);
            print_ast(node->binary_expression.right, depth + 1);
            break;
            
        case NODE_CALL_EXPRESSION:
            print_ast(node->call_expression.function, depth + 1);
            if (node->call_expression.arguments) {
                for (size_t i = 0; i < node->call_expression.arguments->count; i++) {
                    print_ast(node->call_expression.arguments->nodes[i], depth + 1);
                }
            }
            break;
            
        case NODE_MEMBER_ACCESS:
            print_ast(node->member_access.object, depth + 1);
            print_ast(node->member_access.property, depth + 1);
            break;
            
        case NODE_PIPE_EXPRESSION:
            print_ast(node->pipe_expression.left, depth + 1);
            print_ast(node->pipe_expression.right, depth + 1);
            break;
            
        case NODE_ASSIGNMENT_EXPRESSION:
            print_ast(node->assignment_expression.left, depth + 1);
            print_ast(node->assignment_expression.right, depth + 1);
            break;
            
        case NODE_RETURN_STATEMENT:
            if (node->return_statement.value) {
                print_ast(node->return_statement.value, depth + 1);
            }
            break;
            
        case NODE_LIST:
            if (node->list.elements) {
                for (size_t i = 0; i < node->list.elements->count; i++) {
                    print_ast(node->list.elements->nodes[i], depth + 1);
                }
            }
            break;
            
        case NODE_PARAMETER:
            print_ast(node->parameter.name, depth + 1);
            if (node->parameter.type) {
                print_ast(node->parameter.type, depth + 1);
            }
            break;
            
        case NODE_BLOCK:
            if (node->block.statements) {
                for (size_t i = 0; i < node->block.statements->count; i++) {
                    print_ast(node->block.statements->nodes[i], depth + 1);
                }
            }
            break;
            
        default:
            // No children to print for leaf nodes
            break;
    }
}