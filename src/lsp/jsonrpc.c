#include "jsonrpc.h"
#include "../log.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// ============================================================================
// Message parsing helpers
// ============================================================================

static JsonRpcMessage *parse_error_message(Arena *arena, JsonValue *error_value, JsonValue *id_value) {
    JsonRpcMessage *message = arena_alloc(arena, sizeof(JsonRpcMessage));
    message->type = JSONRPC_ERROR;
    message->as.error.id = id_value;

    if (error_value->type != JSON_OBJECT) {
        fprintf(stderr, "LSP: Invalid error object\n");
        return NULL;
    }

    JsonObject *error_obj = error_value->as.object_value;
    JsonValue *code_val = json_object_get(error_obj, "code");
    JsonValue *msg_val = json_object_get(error_obj, "message");

    if (!code_val || code_val->type != JSON_NUMBER) {
        fprintf(stderr, "LSP: Missing or invalid error code\n");
        return NULL;
    }
    if (!msg_val || msg_val->type != JSON_STRING) {
        fprintf(stderr, "LSP: Missing or invalid error message\n");
        return NULL;
    }

    message->as.error.code = (int)code_val->as.number_value;
    message->as.error.message = msg_val->as.string_value;
    message->as.error.data = json_object_get(error_obj, "data");

    return message;
}

static JsonRpcMessage *parse_response_message(Arena *arena, JsonValue *result_value, JsonValue *id_value) {
    JsonRpcMessage *message = arena_alloc(arena, sizeof(JsonRpcMessage));
    message->type = JSONRPC_RESPONSE;
    message->as.response.result = result_value;
    message->as.response.id = id_value;
    return message;
}

static JsonRpcMessage *parse_request_or_notification(Arena *arena, JsonValue *method_value,
                                                      JsonValue *params_value, JsonValue *id_value) {
    if (method_value->type != JSON_STRING) {
        fprintf(stderr, "LSP: Method must be a string\n");
        return NULL;
    }

    JsonRpcMessage *message = arena_alloc(arena, sizeof(JsonRpcMessage));

    if (id_value) {
        // Request (has id)
        message->type = JSONRPC_REQUEST;
        message->as.request.method = method_value->as.string_value;
        message->as.request.params = params_value;
        message->as.request.id = id_value;
    } else {
        // Notification (no id)
        message->type = JSONRPC_NOTIFICATION;
        message->as.notification.method = method_value->as.string_value;
        message->as.notification.params = params_value;
    }

    return message;
}

// ============================================================================
// Message parsing
// ============================================================================

JsonRpcMessage *jsonrpc_parse_message(Arena *arena, const char *content, size_t length) {
    JsonParser *parser = json_parser_create(arena, content, length);
    JsonValue *root = json_parse(parser);

    if (!root || root->type != JSON_OBJECT) {
        log_error("LSP: Failed to parse JSON-RPC message");
        if (parser->error) {
            log_error("LSP: JSON parse error: %s", parser->error);
        }
        return NULL;
    }

    JsonObject *obj = root->as.object_value;

    // Check for jsonrpc version
    JsonValue *version = json_object_get(obj, "jsonrpc");
    if (!version || version->type != JSON_STRING ||
        strcmp(version->as.string_value, "2.0") != 0) {
        log_error("LSP: Invalid or missing jsonrpc version");
        return NULL;
    }

    JsonValue *method_value = json_object_get(obj, "method");
    JsonValue *id_value = json_object_get(obj, "id");
    JsonValue *result_value = json_object_get(obj, "result");
    JsonValue *error_value = json_object_get(obj, "error");

    // Determine message type and parse
    if (error_value) {
        return parse_error_message(arena, error_value, id_value);
    } else if (result_value) {
        return parse_response_message(arena, result_value, id_value);
    } else if (method_value) {
        JsonValue *params_value = json_object_get(obj, "params");
        return parse_request_or_notification(arena, method_value, params_value, id_value);
    } else {
        log_error("LSP: Invalid JSON-RPC message structure");
        return NULL;
    }
}

// ============================================================================
// Message serialization
// ============================================================================

char *jsonrpc_serialize_response(Arena *arena, JsonValue *result, JsonValue *id) {
    JsonObject *obj = json_object_create(arena);
    json_object_set(arena, obj, "jsonrpc", json_string(arena, "2.0"));
    json_object_set(arena, obj, "result", result ? result : json_null(arena));
    json_object_set(arena, obj, "id", id ? id : json_null(arena));

    JsonSerializer *serializer = json_serializer_create(arena);
    json_serialize(serializer, json_object_value(arena, obj));
    return json_serializer_get_string(serializer);
}

char *jsonrpc_serialize_error(Arena *arena, int code, const char *message, JsonValue *id) {
    JsonObject *error_obj = json_object_create(arena);
    json_object_set(arena, error_obj, "code", json_number(arena, code));
    json_object_set(arena, error_obj, "message", json_string(arena, message));

    JsonObject *obj = json_object_create(arena);
    json_object_set(arena, obj, "jsonrpc", json_string(arena, "2.0"));
    json_object_set(arena, obj, "error", json_object_value(arena, error_obj));
    json_object_set(arena, obj, "id", id ? id : json_null(arena));

    JsonSerializer *serializer = json_serializer_create(arena);
    json_serialize(serializer, json_object_value(arena, obj));
    return json_serializer_get_string(serializer);
}

char *jsonrpc_serialize_notification(Arena *arena, const char *method, JsonValue *params) {
    JsonObject *obj = json_object_create(arena);
    json_object_set(arena, obj, "jsonrpc", json_string(arena, "2.0"));
    json_object_set(arena, obj, "method", json_string(arena, method));
    if (params) {
        json_object_set(arena, obj, "params", params);
    }

    JsonSerializer *serializer = json_serializer_create(arena);
    json_serialize(serializer, json_object_value(arena, obj));
    return json_serializer_get_string(serializer);
}

// ============================================================================
// LSP protocol message I/O (handles Content-Length header)
// ============================================================================

LspMessage *lsp_read_message(Arena *arena) {
    char header_buf[256];
    size_t content_length = 0;

    // Read headers
    while (1) {
        if (!fgets(header_buf, sizeof(header_buf), stdin)) {
            return NULL; // EOF or error
        }

        // Check for end of headers (blank line)
        if (strcmp(header_buf, "\r\n") == 0 || strcmp(header_buf, "\n") == 0) {
            break;
        }

        // Parse Content-Length header
        if (sscanf(header_buf, "Content-Length: %zu", &content_length) == 1) {
            // Found Content-Length
        }
        // Ignore other headers like Content-Type
    }

    if (content_length == 0) {
        fprintf(stderr, "LSP: No Content-Length header found\n");
        return NULL;
    }

    // Read content
    char *content = arena_alloc(arena, content_length + 1);
    size_t bytes_read = fread(content, 1, content_length, stdin);
    if (bytes_read != content_length) {
        fprintf(stderr, "LSP: Failed to read message content (expected %zu, got %zu)\n",
                content_length, bytes_read);
        return NULL;
    }
    content[content_length] = '\0';

    LspMessage *message = arena_alloc(arena, sizeof(LspMessage));
    message->content = content;
    message->length = content_length;
    return message;
}

void lsp_write_message(const char *content) {
    size_t content_length = strlen(content);
    printf("Content-Length: %zu\r\n\r\n%s", content_length, content);
    fflush(stdout);
}
