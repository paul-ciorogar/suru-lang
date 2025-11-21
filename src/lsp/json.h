#ifndef JSON_H
#define JSON_H

#include "../arena.h"
#include <stddef.h>

// JSON value types
typedef enum {
    JSON_NULL,
    JSON_BOOL,
    JSON_NUMBER,
    JSON_STRING,
    JSON_ARRAY,
    JSON_OBJECT
} JsonType;

// Forward declarations
typedef struct JsonValue JsonValue;
typedef struct JsonObject JsonObject;
typedef struct JsonArray JsonArray;

// JSON object entry (key-value pair)
typedef struct JsonObjectEntry {
    char *key;
    JsonValue *value;
    struct JsonObjectEntry *next;
} JsonObjectEntry;

// JSON object
struct JsonObject {
    JsonObjectEntry *head;
    JsonObjectEntry *tail;
    int count;
};

// JSON array
struct JsonArray {
    JsonValue **values;
    int count;
    int capacity;
};

// JSON value (tagged union)
struct JsonValue {
    JsonType type;
    union {
        int bool_value;
        double number_value;
        char *string_value;
        JsonArray *array_value;
        JsonObject *object_value;
    } as;
};

// JSON parser
typedef struct {
    Arena *arena;
    const char *input;
    size_t pos;
    size_t length;
    char *error;
} JsonParser;

// JSON serializer (builds string in buffer)
typedef struct {
    Arena *arena;
    char *buffer;
    size_t capacity;
    size_t length;
} JsonSerializer;

// Parser functions
JsonParser *json_parser_create(Arena *arena, const char *input, size_t length);
JsonValue *json_parse(JsonParser *parser);
JsonValue *json_parse_value(JsonParser *parser);
JsonObject *json_parse_object(JsonParser *parser);
JsonArray *json_parse_array(JsonParser *parser);
char *json_parse_string(JsonParser *parser);
double json_parse_number(JsonParser *parser);

// Serializer functions
JsonSerializer *json_serializer_create(Arena *arena);
void json_serialize(JsonSerializer *serializer, JsonValue *value);
void json_serialize_object(JsonSerializer *serializer, JsonObject *obj);
void json_serialize_array(JsonSerializer *serializer, JsonArray *arr);
void json_serialize_string(JsonSerializer *serializer, const char *str);
char *json_serializer_get_string(JsonSerializer *serializer);

// Object manipulation
JsonObject *json_object_create(Arena *arena);
void json_object_set(Arena *arena, JsonObject *obj, const char *key, JsonValue *value);
JsonValue *json_object_get(JsonObject *obj, const char *key);

// Array manipulation
JsonArray *json_array_create(Arena *arena);
void json_array_add(Arena *arena, JsonArray *arr, JsonValue *value);
JsonValue *json_array_get(JsonArray *arr, int index);

// Value constructors
JsonValue *json_null(Arena *arena);
JsonValue *json_bool(Arena *arena, int value);
JsonValue *json_number(Arena *arena, double value);
JsonValue *json_string(Arena *arena, const char *value);
JsonValue *json_array(Arena *arena, JsonArray *arr);
JsonValue *json_object_value(Arena *arena, JsonObject *obj);

#endif // JSON_H
