#ifndef JSONRPC_H
#define JSONRPC_H

#include "json.h"
#include "../arena.h"
#include <stddef.h>

// JSON-RPC message types
typedef enum {
    JSONRPC_REQUEST,
    JSONRPC_RESPONSE,
    JSONRPC_NOTIFICATION,
    JSONRPC_ERROR
} JsonRpcMessageType;

// JSON-RPC request
typedef struct {
    char *method;
    JsonValue *params;
    JsonValue *id; // Can be number, string, or null
} JsonRpcRequest;

// JSON-RPC response
typedef struct {
    JsonValue *result;
    JsonValue *id;
} JsonRpcResponse;

// JSON-RPC error
typedef struct {
    int code;
    char *message;
    JsonValue *data; // Optional
    JsonValue *id;
} JsonRpcError;

// JSON-RPC notification (request without id)
typedef struct {
    char *method;
    JsonValue *params;
} JsonRpcNotification;

// JSON-RPC message (tagged union)
typedef struct {
    JsonRpcMessageType type;
    union {
        JsonRpcRequest request;
        JsonRpcResponse response;
        JsonRpcNotification notification;
        JsonRpcError error;
    } as;
} JsonRpcMessage;

// Message parser/serializer
JsonRpcMessage *jsonrpc_parse_message(Arena *arena, const char *content, size_t length);
char *jsonrpc_serialize_response(Arena *arena, JsonValue *result, JsonValue *id);
char *jsonrpc_serialize_error(Arena *arena, int code, const char *message, JsonValue *id);
char *jsonrpc_serialize_notification(Arena *arena, const char *method, JsonValue *params);

// LSP protocol message reading/writing (handles Content-Length header)
typedef struct {
    char *content;
    size_t length;
} LspMessage;

LspMessage *lsp_read_message(Arena *arena);
void lsp_write_message(const char *content);

#endif // JSONRPC_H
