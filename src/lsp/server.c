#include "server.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// ============================================================================
// Server lifecycle
// ============================================================================

LspServer *lsp_server_create(Arena *arena) {
    LspServer *server = arena_alloc(arena, sizeof(LspServer));
    server->arena = arena;
    server->temp_arena = arena_create(1024 * 1024); // 1MB for temporary allocations
    server->initialized = 0;
    server->shutdown_requested = 0;

    // Initialize document storage
    server->document_capacity = 16;
    server->document_count = 0;
    server->documents = arena_alloc(arena, sizeof(LspDocument *) * server->document_capacity);

    return server;
}

int lsp_server_run() {
    Arena *arena = arena_create(16 * 1024 * 1024); // 16MB for server state
    LspServer *server = lsp_server_create(arena);

    fprintf(stderr, "LSP: Suru language server starting...\n");

    int exit_requested = 0;

    while (!exit_requested) {
        // Read message from stdin
        LspMessage *lsp_message = lsp_read_message(server->temp_arena);
        if (!lsp_message) {
            fprintf(stderr, "LSP: Failed to read message, exiting\n");
            break;
        }

        // Parse JSON-RPC message
        JsonRpcMessage *message = jsonrpc_parse_message(
            server->temp_arena,
            lsp_message->content,
            lsp_message->length
        );

        if (!message) {
            fprintf(stderr, "LSP: Failed to parse JSON-RPC message\n");
            continue;
        }

        // Check for exit notification
        if (message->type == JSONRPC_NOTIFICATION &&
            strcmp(message->as.notification.method, "exit") == 0) {
            exit_requested = 1;
        }

        // Handle message
        lsp_handle_message(server, message);

        // Clear temporary arena for next message
        arena_destroy(server->temp_arena);
        server->temp_arena = arena_create(1024 * 1024);
    }

    fprintf(stderr, "LSP: Server shutting down\n");
    arena_destroy(server->temp_arena);
    arena_destroy(arena);
    return 0;
}

void lsp_server_shutdown(LspServer *server) {
    server->shutdown_requested = 1;
}

// ============================================================================
// Message routing
// ============================================================================

void lsp_handle_message(LspServer *server, JsonRpcMessage *message) {
    switch (message->type) {
        case JSONRPC_REQUEST: {
            JsonRpcRequest *request = &message->as.request;
            const char *method = request->method;

            fprintf(stderr, "LSP: Received request: %s\n", method);

            if (strcmp(method, "initialize") == 0) {
                lsp_handle_initialize(server, request);
            } else if (strcmp(method, "shutdown") == 0) {
                lsp_handle_shutdown(server, request);
            } else {
                fprintf(stderr, "LSP: Unknown request method: %s\n", method);
                char *error = jsonrpc_serialize_error(
                    server->temp_arena,
                    -32601, // Method not found
                    "Method not found",
                    request->id
                );
                lsp_write_message(error);
            }
            break;
        }

        case JSONRPC_NOTIFICATION: {
            JsonRpcNotification *notification = &message->as.notification;
            const char *method = notification->method;

            fprintf(stderr, "LSP: Received notification: %s\n", method);

            if (strcmp(method, "initialized") == 0) {
                lsp_handle_initialized(server, notification);
            } else if (strcmp(method, "exit") == 0) {
                lsp_handle_exit(server, notification);
            } else if (strcmp(method, "textDocument/didOpen") == 0) {
                lsp_handle_text_document_did_open(server, notification);
            } else if (strcmp(method, "textDocument/didChange") == 0) {
                lsp_handle_text_document_did_change(server, notification);
            } else if (strcmp(method, "textDocument/didClose") == 0) {
                lsp_handle_text_document_did_close(server, notification);
            } else {
                fprintf(stderr, "LSP: Unknown notification: %s\n", method);
            }
            break;
        }

        case JSONRPC_RESPONSE:
            fprintf(stderr, "LSP: Received response (unexpected from client)\n");
            break;

        case JSONRPC_ERROR:
            fprintf(stderr, "LSP: Received error (unexpected from client)\n");
            break;
    }
}

// ============================================================================
// Lifecycle handlers
// ============================================================================

void lsp_handle_initialize(LspServer *server, JsonRpcRequest *request) {
    fprintf(stderr, "LSP: Handling initialize request\n");

    // Build capabilities object
    JsonObject *capabilities = json_object_create(server->temp_arena);

    // textDocumentSync capability
    JsonObject *text_doc_sync = json_object_create(server->temp_arena);
    json_object_set(server->temp_arena, text_doc_sync, "openClose", json_bool(server->temp_arena, 1));
    json_object_set(server->temp_arena, text_doc_sync, "change", json_number(server->temp_arena, 1)); // Full sync
    json_object_set(server->temp_arena, capabilities, "textDocumentSync", json_object_value(server->temp_arena, text_doc_sync));

    // Build result object
    JsonObject *result = json_object_create(server->temp_arena);
    json_object_set(server->temp_arena, result, "capabilities", json_object_value(server->temp_arena, capabilities));

    // Server info
    JsonObject *server_info = json_object_create(server->temp_arena);
    json_object_set(server->temp_arena, server_info, "name", json_string(server->temp_arena, "suru-lsp"));
    json_object_set(server->temp_arena, server_info, "version", json_string(server->temp_arena, "0.4.0"));
    json_object_set(server->temp_arena, result, "serverInfo", json_object_value(server->temp_arena, server_info));

    // Send response
    char *response = jsonrpc_serialize_response(
        server->temp_arena,
        json_object_value(server->temp_arena, result),
        request->id
    );
    lsp_write_message(response);

    fprintf(stderr, "LSP: Initialize response sent\n");
}

void lsp_handle_initialized(LspServer *server, JsonRpcNotification *notification) {
    (void)notification; // Unused
    server->initialized = 1;
    fprintf(stderr, "LSP: Server initialized\n");
}

void lsp_handle_shutdown(LspServer *server, JsonRpcRequest *request) {
    fprintf(stderr, "LSP: Handling shutdown request\n");

    // Send null response
    char *response = jsonrpc_serialize_response(
        server->temp_arena,
        json_null(server->temp_arena),
        request->id
    );
    lsp_write_message(response);

    fprintf(stderr, "LSP: Shutdown response sent\n");
}

void lsp_handle_exit(LspServer *server, JsonRpcNotification *notification) {
    (void)server; // Unused
    (void)notification; // Unused
    fprintf(stderr, "LSP: Handling exit notification\n");
}

// ============================================================================
// Document management
// ============================================================================

LspDocument *lsp_server_get_document(LspServer *server, const char *uri) {
    for (int i = 0; i < server->document_count; i++) {
        if (strcmp(server->documents[i]->uri, uri) == 0) {
            return server->documents[i];
        }
    }
    return NULL;
}

void lsp_server_add_document(LspServer *server, const char *uri, const char *content, int version) {
    // Check if document already exists
    LspDocument *existing = lsp_server_get_document(server, uri);
    if (existing) {
        fprintf(stderr, "LSP: Document already exists: %s\n", uri);
        return;
    }

    // Expand capacity if needed
    if (server->document_count >= server->document_capacity) {
        int new_capacity = server->document_capacity * 2;
        LspDocument **new_docs = arena_alloc(server->arena, sizeof(LspDocument *) * new_capacity);
        for (int i = 0; i < server->document_count; i++) {
            new_docs[i] = server->documents[i];
        }
        server->documents = new_docs;
        server->document_capacity = new_capacity;
    }

    // Create new document
    LspDocument *doc = arena_alloc(server->arena, sizeof(LspDocument));
    size_t uri_len = strlen(uri);
    doc->uri = arena_alloc(server->arena, uri_len + 1);
    strcpy(doc->uri, uri);

    size_t content_len = strlen(content);
    doc->content = arena_alloc(server->arena, content_len + 1);
    strcpy(doc->content, content);

    doc->version = version;

    server->documents[server->document_count++] = doc;
    fprintf(stderr, "LSP: Document added: %s (version %d)\n", uri, version);
}

void lsp_server_update_document(LspServer *server, const char *uri, const char *content, int version) {
    LspDocument *doc = lsp_server_get_document(server, uri);
    if (!doc) {
        fprintf(stderr, "LSP: Cannot update non-existent document: %s\n", uri);
        return;
    }

    // Allocate new content
    size_t content_len = strlen(content);
    doc->content = arena_alloc(server->arena, content_len + 1);
    strcpy(doc->content, content);
    doc->version = version;

    fprintf(stderr, "LSP: Document updated: %s (version %d)\n", uri, version);
}

void lsp_server_remove_document(LspServer *server, const char *uri) {
    for (int i = 0; i < server->document_count; i++) {
        if (strcmp(server->documents[i]->uri, uri) == 0) {
            // Shift remaining documents
            for (int j = i; j < server->document_count - 1; j++) {
                server->documents[j] = server->documents[j + 1];
            }
            server->document_count--;
            fprintf(stderr, "LSP: Document removed: %s\n", uri);
            return;
        }
    }
    fprintf(stderr, "LSP: Cannot remove non-existent document: %s\n", uri);
}

// ============================================================================
// textDocument handlers
// ============================================================================

void lsp_handle_text_document_did_open(LspServer *server, JsonRpcNotification *notification) {
    if (!notification->params || notification->params->type != JSON_OBJECT) {
        fprintf(stderr, "LSP: Invalid didOpen params\n");
        return;
    }

    JsonObject *params = notification->params->as.object_value;
    JsonValue *text_doc_val = json_object_get(params, "textDocument");

    if (!text_doc_val || text_doc_val->type != JSON_OBJECT) {
        fprintf(stderr, "LSP: Missing textDocument in didOpen\n");
        return;
    }

    JsonObject *text_doc = text_doc_val->as.object_value;
    JsonValue *uri_val = json_object_get(text_doc, "uri");
    JsonValue *text_val = json_object_get(text_doc, "text");
    JsonValue *version_val = json_object_get(text_doc, "version");

    if (!uri_val || uri_val->type != JSON_STRING ||
        !text_val || text_val->type != JSON_STRING ||
        !version_val || version_val->type != JSON_NUMBER) {
        fprintf(stderr, "LSP: Invalid textDocument fields in didOpen\n");
        return;
    }

    const char *uri = uri_val->as.string_value;
    const char *text = text_val->as.string_value;
    int version = (int)version_val->as.number_value;

    lsp_server_add_document(server, uri, text, version);
}

void lsp_handle_text_document_did_change(LspServer *server, JsonRpcNotification *notification) {
    if (!notification->params || notification->params->type != JSON_OBJECT) {
        fprintf(stderr, "LSP: Invalid didChange params\n");
        return;
    }

    JsonObject *params = notification->params->as.object_value;
    JsonValue *text_doc_val = json_object_get(params, "textDocument");
    JsonValue *content_changes_val = json_object_get(params, "contentChanges");

    if (!text_doc_val || text_doc_val->type != JSON_OBJECT ||
        !content_changes_val || content_changes_val->type != JSON_ARRAY) {
        fprintf(stderr, "LSP: Invalid didChange structure\n");
        return;
    }

    JsonObject *text_doc = text_doc_val->as.object_value;
    JsonValue *uri_val = json_object_get(text_doc, "uri");
    JsonValue *version_val = json_object_get(text_doc, "version");

    if (!uri_val || uri_val->type != JSON_STRING ||
        !version_val || version_val->type != JSON_NUMBER) {
        fprintf(stderr, "LSP: Invalid textDocument fields in didChange\n");
        return;
    }

    const char *uri = uri_val->as.string_value;
    int version = (int)version_val->as.number_value;

    // For full document sync (change = 1), we expect a single change with the full text
    JsonArray *content_changes = content_changes_val->as.array_value;
    if (content_changes->count == 0) {
        fprintf(stderr, "LSP: No content changes in didChange\n");
        return;
    }

    JsonValue *change_val = json_array_get(content_changes, 0);
    if (!change_val || change_val->type != JSON_OBJECT) {
        fprintf(stderr, "LSP: Invalid content change object\n");
        return;
    }

    JsonObject *change = change_val->as.object_value;
    JsonValue *text_val = json_object_get(change, "text");

    if (!text_val || text_val->type != JSON_STRING) {
        fprintf(stderr, "LSP: Missing text in content change\n");
        return;
    }

    const char *text = text_val->as.string_value;
    lsp_server_update_document(server, uri, text, version);
}

void lsp_handle_text_document_did_close(LspServer *server, JsonRpcNotification *notification) {
    if (!notification->params || notification->params->type != JSON_OBJECT) {
        fprintf(stderr, "LSP: Invalid didClose params\n");
        return;
    }

    JsonObject *params = notification->params->as.object_value;
    JsonValue *text_doc_val = json_object_get(params, "textDocument");

    if (!text_doc_val || text_doc_val->type != JSON_OBJECT) {
        fprintf(stderr, "LSP: Missing textDocument in didClose\n");
        return;
    }

    JsonObject *text_doc = text_doc_val->as.object_value;
    JsonValue *uri_val = json_object_get(text_doc, "uri");

    if (!uri_val || uri_val->type != JSON_STRING) {
        fprintf(stderr, "LSP: Invalid uri in didClose\n");
        return;
    }

    const char *uri = uri_val->as.string_value;
    lsp_server_remove_document(server, uri);
}
