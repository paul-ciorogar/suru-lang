#include "json.h"
#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// ============================================================================
// Helper functions
// ============================================================================

static void skip_whitespace(JsonParser *parser) {
    while (parser->pos < parser->length) {
        char c = parser->input[parser->pos];
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
            parser->pos++;
        } else {
            break;
        }
    }
}

static char peek_char(JsonParser *parser) {
    skip_whitespace(parser);
    if (parser->pos < parser->length) {
        return parser->input[parser->pos];
    }
    return '\0';
}

static char next_char(JsonParser *parser) {
    skip_whitespace(parser);
    if (parser->pos < parser->length) {
        return parser->input[parser->pos++];
    }
    return '\0';
}

static int match_string(JsonParser *parser, const char *str) {
    size_t len = strlen(str);
    if (parser->pos + len > parser->length) {
        return 0;
    }
    if (strncmp(parser->input + parser->pos, str, len) == 0) {
        parser->pos += len;
        return 1;
    }
    return 0;
}

// ============================================================================
// Value constructors
// ============================================================================

JsonValue *json_null(Arena *arena) {
    JsonValue *value = arena_alloc(arena, sizeof(JsonValue));
    value->type = JSON_NULL;
    return value;
}

JsonValue *json_bool(Arena *arena, int val) {
    JsonValue *value = arena_alloc(arena, sizeof(JsonValue));
    value->type = JSON_BOOL;
    value->as.bool_value = val;
    return value;
}

JsonValue *json_number(Arena *arena, double val) {
    JsonValue *value = arena_alloc(arena, sizeof(JsonValue));
    value->type = JSON_NUMBER;
    value->as.number_value = val;
    return value;
}

JsonValue *json_string(Arena *arena, const char *val) {
    JsonValue *value = arena_alloc(arena, sizeof(JsonValue));
    value->type = JSON_STRING;
    size_t len = strlen(val);
    value->as.string_value = arena_alloc(arena, len + 1);
    strcpy(value->as.string_value, val);
    return value;
}

JsonValue *json_array(Arena *arena, JsonArray *arr) {
    JsonValue *value = arena_alloc(arena, sizeof(JsonValue));
    value->type = JSON_ARRAY;
    value->as.array_value = arr;
    return value;
}

JsonValue *json_object_value(Arena *arena, JsonObject *obj) {
    JsonValue *value = arena_alloc(arena, sizeof(JsonValue));
    value->type = JSON_OBJECT;
    value->as.object_value = obj;
    return value;
}

// ============================================================================
// Object manipulation
// ============================================================================

JsonObject *json_object_create(Arena *arena) {
    JsonObject *obj = arena_alloc(arena, sizeof(JsonObject));
    obj->head = NULL;
    obj->tail = NULL;
    obj->count = 0;
    return obj;
}

void json_object_set(Arena *arena, JsonObject *obj, const char *key, JsonValue *value) {
    JsonObjectEntry *entry = arena_alloc(arena, sizeof(JsonObjectEntry));
    size_t key_len = strlen(key);
    entry->key = arena_alloc(arena, key_len + 1);
    strcpy(entry->key, key);
    entry->value = value;
    entry->next = NULL;

    if (!obj->head) {
        obj->head = entry;
        obj->tail = entry;
    } else {
        obj->tail->next = entry;
        obj->tail = entry;
    }
    obj->count++;
}

JsonValue *json_object_get(JsonObject *obj, const char *key) {
    JsonObjectEntry *entry = obj->head;
    while (entry) {
        if (strcmp(entry->key, key) == 0) {
            return entry->value;
        }
        entry = entry->next;
    }
    return NULL;
}

// ============================================================================
// Array manipulation
// ============================================================================

JsonArray *json_array_create(Arena *arena) {
    JsonArray *arr = arena_alloc(arena, sizeof(JsonArray));
    arr->capacity = 8;
    arr->count = 0;
    arr->values = arena_alloc(arena, sizeof(JsonValue *) * arr->capacity);
    return arr;
}

void json_array_add(Arena *arena, JsonArray *arr, JsonValue *value) {
    if (arr->count >= arr->capacity) {
        int new_capacity = arr->capacity * 2;
        JsonValue **new_values = arena_alloc(arena, sizeof(JsonValue *) * new_capacity);
        for (int i = 0; i < arr->count; i++) {
            new_values[i] = arr->values[i];
        }
        arr->values = new_values;
        arr->capacity = new_capacity;
    }
    arr->values[arr->count++] = value;
}

JsonValue *json_array_get(JsonArray *arr, int index) {
    if (index >= 0 && index < arr->count) {
        return arr->values[index];
    }
    return NULL;
}

// ============================================================================
// Parser
// ============================================================================

JsonParser *json_parser_create(Arena *arena, const char *input, size_t length) {
    JsonParser *parser = arena_alloc(arena, sizeof(JsonParser));
    parser->arena = arena;
    parser->input = input;
    parser->pos = 0;
    parser->length = length;
    parser->error = NULL;
    return parser;
}

char *json_parse_string(JsonParser *parser) {
    if (next_char(parser) != '"') {
        parser->error = "Expected '\"' at start of string";
        return NULL;
    }

    size_t start = parser->pos;
    size_t len = 0;

    // First pass: calculate length and validate
    while (parser->pos < parser->length) {
        char c = parser->input[parser->pos];
        if (c == '"') {
            break;
        }
        if (c == '\\') {
            parser->pos++;
            if (parser->pos >= parser->length) {
                parser->error = "Unterminated string escape";
                return NULL;
            }
            char escape = parser->input[parser->pos];
            // Simple escape validation
            if (escape != '"' && escape != '\\' && escape != '/' &&
                escape != 'b' && escape != 'f' && escape != 'n' &&
                escape != 'r' && escape != 't' && escape != 'u') {
                parser->error = "Invalid escape sequence";
                return NULL;
            }
            parser->pos++;
            len++;
        } else {
            parser->pos++;
            len++;
        }
    }

    if (parser->pos >= parser->length) {
        parser->error = "Unterminated string";
        return NULL;
    }

    // Allocate and copy with escape processing
    char *str = arena_alloc(parser->arena, len + 1);
    size_t j = 0;
    for (size_t i = start; i < parser->pos; i++) {
        char c = parser->input[i];
        if (c == '\\') {
            i++;
            char escape = parser->input[i];
            switch (escape) {
                case '"': str[j++] = '"'; break;
                case '\\': str[j++] = '\\'; break;
                case '/': str[j++] = '/'; break;
                case 'b': str[j++] = '\b'; break;
                case 'f': str[j++] = '\f'; break;
                case 'n': str[j++] = '\n'; break;
                case 'r': str[j++] = '\r'; break;
                case 't': str[j++] = '\t'; break;
                case 'u':
                    // Simplified: just copy the unicode escape as-is
                    str[j++] = '\\';
                    str[j++] = 'u';
                    break;
                default: str[j++] = escape; break;
            }
        } else {
            str[j++] = c;
        }
    }
    str[j] = '\0';

    parser->pos++; // Skip closing quote
    return str;
}

double json_parse_number(JsonParser *parser) {
    size_t start = parser->pos;

    // Skip minus
    if (parser->pos < parser->length && parser->input[parser->pos] == '-') {
        parser->pos++;
    }

    // Parse digits
    int has_digits = 0;
    while (parser->pos < parser->length && isdigit(parser->input[parser->pos])) {
        parser->pos++;
        has_digits = 1;
    }

    if (!has_digits) {
        parser->error = "Invalid number";
        return 0;
    }

    // Parse decimal part
    if (parser->pos < parser->length && parser->input[parser->pos] == '.') {
        parser->pos++;
        while (parser->pos < parser->length && isdigit(parser->input[parser->pos])) {
            parser->pos++;
        }
    }

    // Parse exponent
    if (parser->pos < parser->length &&
        (parser->input[parser->pos] == 'e' || parser->input[parser->pos] == 'E')) {
        parser->pos++;
        if (parser->pos < parser->length &&
            (parser->input[parser->pos] == '+' || parser->input[parser->pos] == '-')) {
            parser->pos++;
        }
        while (parser->pos < parser->length && isdigit(parser->input[parser->pos])) {
            parser->pos++;
        }
    }

    // Extract and parse the number
    size_t num_len = parser->pos - start;
    char *num_str = arena_alloc(parser->arena, num_len + 1);
    strncpy(num_str, parser->input + start, num_len);
    num_str[num_len] = '\0';

    return atof(num_str);
}

JsonArray *json_parse_array(JsonParser *parser) {
    if (next_char(parser) != '[') {
        parser->error = "Expected '[' at start of array";
        return NULL;
    }

    JsonArray *arr = json_array_create(parser->arena);

    if (peek_char(parser) == ']') {
        next_char(parser); // Consume ']'
        return arr;
    }

    while (1) {
        JsonValue *value = json_parse_value(parser);
        if (!value) {
            return NULL;
        }
        json_array_add(parser->arena, arr, value);

        char c = peek_char(parser);
        if (c == ']') {
            next_char(parser); // Consume ']'
            break;
        } else if (c == ',') {
            next_char(parser); // Consume ','
        } else {
            parser->error = "Expected ',' or ']' in array";
            return NULL;
        }
    }

    return arr;
}

JsonObject *json_parse_object(JsonParser *parser) {
    if (next_char(parser) != '{') {
        parser->error = "Expected '{' at start of object";
        return NULL;
    }

    JsonObject *obj = json_object_create(parser->arena);

    if (peek_char(parser) == '}') {
        next_char(parser); // Consume '}'
        return obj;
    }

    while (1) {
        // Parse key
        if (peek_char(parser) != '"') {
            parser->error = "Expected string key in object";
            return NULL;
        }
        char *key = json_parse_string(parser);
        if (!key) {
            return NULL;
        }

        // Parse colon
        if (next_char(parser) != ':') {
            parser->error = "Expected ':' after object key";
            return NULL;
        }

        // Parse value
        JsonValue *value = json_parse_value(parser);
        if (!value) {
            return NULL;
        }

        json_object_set(parser->arena, obj, key, value);

        // Check for comma or closing brace
        char c = peek_char(parser);
        if (c == '}') {
            next_char(parser); // Consume '}'
            break;
        } else if (c == ',') {
            next_char(parser); // Consume ','
        } else {
            parser->error = "Expected ',' or '}' in object";
            return NULL;
        }
    }

    return obj;
}

JsonValue *json_parse_value(JsonParser *parser) {
    char c = peek_char(parser);

    if (c == '"') {
        char *str = json_parse_string(parser);
        if (!str) return NULL;
        return json_string(parser->arena, str);
    } else if (c == '{') {
        JsonObject *obj = json_parse_object(parser);
        if (!obj) return NULL;
        return json_object_value(parser->arena, obj);
    } else if (c == '[') {
        JsonArray *arr = json_parse_array(parser);
        if (!arr) return NULL;
        return json_array(parser->arena, arr);
    } else if (c == 't') {
        if (match_string(parser, "true")) {
            return json_bool(parser->arena, 1);
        }
    } else if (c == 'f') {
        if (match_string(parser, "false")) {
            return json_bool(parser->arena, 0);
        }
    } else if (c == 'n') {
        if (match_string(parser, "null")) {
            return json_null(parser->arena);
        }
    } else if (c == '-' || isdigit(c)) {
        double num = json_parse_number(parser);
        if (parser->error) return NULL;
        return json_number(parser->arena, num);
    }

    parser->error = "Unexpected character";
    return NULL;
}

JsonValue *json_parse(JsonParser *parser) {
    return json_parse_value(parser);
}

// ============================================================================
// Serializer
// ============================================================================

JsonSerializer *json_serializer_create(Arena *arena) {
    JsonSerializer *serializer = arena_alloc(arena, sizeof(JsonSerializer));
    serializer->arena = arena;
    serializer->capacity = 1024;
    serializer->length = 0;
    serializer->buffer = arena_alloc(arena, serializer->capacity);
    return serializer;
}

static void serializer_append(JsonSerializer *serializer, const char *str) {
    size_t str_len = strlen(str);
    while (serializer->length + str_len >= serializer->capacity) {
        size_t new_capacity = serializer->capacity * 2;
        char *new_buffer = arena_alloc(serializer->arena, new_capacity);
        memcpy(new_buffer, serializer->buffer, serializer->length);
        serializer->buffer = new_buffer;
        serializer->capacity = new_capacity;
    }
    memcpy(serializer->buffer + serializer->length, str, str_len);
    serializer->length += str_len;
}

static void serializer_append_char(JsonSerializer *serializer, char c) {
    if (serializer->length + 1 >= serializer->capacity) {
        size_t new_capacity = serializer->capacity * 2;
        char *new_buffer = arena_alloc(serializer->arena, new_capacity);
        memcpy(new_buffer, serializer->buffer, serializer->length);
        serializer->buffer = new_buffer;
        serializer->capacity = new_capacity;
    }
    serializer->buffer[serializer->length++] = c;
}

void json_serialize_string(JsonSerializer *serializer, const char *str) {
    serializer_append_char(serializer, '"');
    for (const char *p = str; *p; p++) {
        switch (*p) {
            case '"': serializer_append(serializer, "\\\""); break;
            case '\\': serializer_append(serializer, "\\\\"); break;
            case '\b': serializer_append(serializer, "\\b"); break;
            case '\f': serializer_append(serializer, "\\f"); break;
            case '\n': serializer_append(serializer, "\\n"); break;
            case '\r': serializer_append(serializer, "\\r"); break;
            case '\t': serializer_append(serializer, "\\t"); break;
            default:
                if (*p < 32) {
                    char buf[7];
                    snprintf(buf, sizeof(buf), "\\u%04x", (unsigned char)*p);
                    serializer_append(serializer, buf);
                } else {
                    serializer_append_char(serializer, *p);
                }
                break;
        }
    }
    serializer_append_char(serializer, '"');
}

void json_serialize_array(JsonSerializer *serializer, JsonArray *arr) {
    serializer_append_char(serializer, '[');
    for (int i = 0; i < arr->count; i++) {
        if (i > 0) {
            serializer_append_char(serializer, ',');
        }
        json_serialize(serializer, arr->values[i]);
    }
    serializer_append_char(serializer, ']');
}

void json_serialize_object(JsonSerializer *serializer, JsonObject *obj) {
    serializer_append_char(serializer, '{');
    JsonObjectEntry *entry = obj->head;
    int first = 1;
    while (entry) {
        if (!first) {
            serializer_append_char(serializer, ',');
        }
        first = 0;
        json_serialize_string(serializer, entry->key);
        serializer_append_char(serializer, ':');
        json_serialize(serializer, entry->value);
        entry = entry->next;
    }
    serializer_append_char(serializer, '}');
}

void json_serialize(JsonSerializer *serializer, JsonValue *value) {
    switch (value->type) {
        case JSON_NULL:
            serializer_append(serializer, "null");
            break;
        case JSON_BOOL:
            serializer_append(serializer, value->as.bool_value ? "true" : "false");
            break;
        case JSON_NUMBER: {
            char buf[64];
            snprintf(buf, sizeof(buf), "%g", value->as.number_value);
            serializer_append(serializer, buf);
            break;
        }
        case JSON_STRING:
            json_serialize_string(serializer, value->as.string_value);
            break;
        case JSON_ARRAY:
            json_serialize_array(serializer, value->as.array_value);
            break;
        case JSON_OBJECT:
            json_serialize_object(serializer, value->as.object_value);
            break;
    }
}

char *json_serializer_get_string(JsonSerializer *serializer) {
    serializer_append_char(serializer, '\0');
    return serializer->buffer;
}
