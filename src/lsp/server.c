#include "server.h"
#include "../log.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// ============================================================================
// Server lifecycle
// ============================================================================

LspServer *lsp_server_create() {
    LspServer *server = malloc(sizeof(LspServer));
    if (!server) return NULL;

    server->temp_arena = arena_create(TMP_ARENA_SIZE);
    server->initialized = 0;
    server->shutdown_requested = 0;

    // Initialize document storage (linked list)
    server->documents_head = NULL;
    server->document_count = 0;

    return server;
}

int lsp_server_run() {
    LspServer *server = lsp_server_create();
    if (!server) {
        log_error("LSP: Failed to create server");
        return 1;
    }

    log_info("LSP: Suru language server starting...");

    int exit_requested = 0;

    while (!exit_requested) {
        // Read message from stdin
        LspMessage *lsp_message = lsp_read_message(server->temp_arena);
        if (!lsp_message) {
            log_error("LSP: Failed to read message, exiting");
            break;
        }

        // Parse JSON-RPC message
        JsonRpcMessage *message = jsonrpc_parse_message(
            server->temp_arena,
            lsp_message->content,
            lsp_message->length);

        if (!message) {
            log_error("LSP: Failed to parse JSON-RPC message");
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
        server->temp_arena = arena_create(TMP_ARENA_SIZE);
    }

    log_info("LSP: Server shutting down");

    // Free all documents
    LspDocument *doc = server->documents_head;
    while (doc) {
        LspDocument *next = doc->next;
        free(doc->uri);
        free(doc->content);
        free(doc);
        doc = next;
    }

    arena_destroy(server->temp_arena);
    free(server);
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

        log_debug("LSP: Received request: %s", method);

        if (strcmp(method, "initialize") == 0) {
            lsp_handle_initialize(server, request);
        } else if (strcmp(method, "shutdown") == 0) {
            lsp_handle_shutdown(server, request);
        } else {
            log_debug("LSP: Unknown request method: %s", method);
            char *error = jsonrpc_serialize_error(
                server->temp_arena,
                -32601, // Method not found
                "Method not found",
                request->id);
            lsp_write_message(error);
        }
        break;
    }

    case JSONRPC_NOTIFICATION: {
        JsonRpcNotification *notification = &message->as.notification;
        const char *method = notification->method;

        log_debug("LSP: Received notification: %s", method);

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
            log_debug("LSP: Unknown notification: %s", method);
        }
        break;
    }

    case JSONRPC_RESPONSE:
        log_debug("LSP: Received response (unexpected from client)");
        break;

    case JSONRPC_ERROR:
        log_debug("LSP: Received error (unexpected from client)");
        break;
    }
}

// ============================================================================
// Lifecycle handlers
// ============================================================================

void lsp_handle_initialize(LspServer *server, JsonRpcRequest *request) {
    log_debug("LSP: Handling initialize request");

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
        request->id);
    lsp_write_message(response);

    log_debug("LSP: Initialize response sent");
}

void lsp_handle_initialized(LspServer *server, JsonRpcNotification *notification) {
    (void)notification; // Unused
    server->initialized = 1;
    log_debug("LSP: Server initialized");
}

void lsp_handle_shutdown(LspServer *server, JsonRpcRequest *request) {
    log_debug("LSP: Handling shutdown request");

    // Send null response
    char *response = jsonrpc_serialize_response(
        server->temp_arena,
        json_null(server->temp_arena),
        request->id);
    lsp_write_message(response);

    log_debug("LSP: Shutdown response sent");
}

void lsp_handle_exit(LspServer *server, JsonRpcNotification *notification) {
    (void)server;       // Unused
    (void)notification; // Unused
    log_debug("LSP: Handling exit notification");
}

// ============================================================================
// Document management
// ============================================================================

LspDocument *lsp_server_get_document(LspServer *server, const char *uri) {
    LspDocument *doc = server->documents_head;
    while (doc) {
        if (strcmp(doc->uri, uri) == 0) {
            return doc;
        }
        doc = doc->next;
    }
    return NULL;
}

void lsp_server_add_document(LspServer *server, const char *uri, const char *content, int version) {
    // Check if document already exists
    LspDocument *existing = lsp_server_get_document(server, uri);
    if (existing) {
        log_debug("LSP: Document already exists: %s", uri);
        return;
    }

    // Create new document
    LspDocument *doc = malloc(sizeof(LspDocument));
    if (!doc) {
        log_error("LSP: Failed to allocate document");
        return;
    }

    size_t uri_len = strlen(uri);
    doc->uri = malloc(uri_len + 1);
    if (!doc->uri) {
        free(doc);
        log_error("LSP: Failed to allocate document URI");
        return;
    }
    strcpy(doc->uri, uri);

    size_t content_len = strlen(content);
    doc->content = malloc(content_len + 1);
    if (!doc->content) {
        free(doc->uri);
        free(doc);
        log_error("LSP: Failed to allocate document content");
        return;
    }
    strcpy(doc->content, content);

    doc->version = version;
    doc->next = server->documents_head;
    server->documents_head = doc;
    server->document_count++;

    log_debug("LSP: Document added: %s (version %d)", uri, version);
}

void lsp_server_update_document(LspServer *server, const char *uri, const char *content, int version) {
    LspDocument *doc = lsp_server_get_document(server, uri);
    if (!doc) {
        log_debug("LSP: Cannot update non-existent document: %s", uri);
        return;
    }

    // Free old content and allocate new
    free(doc->content);
    size_t content_len = strlen(content);
    doc->content = malloc(content_len + 1);
    if (!doc->content) {
        log_error("LSP: Failed to allocate updated content");
        return;
    }
    strcpy(doc->content, content);
    doc->version = version;

    log_debug("LSP: Document updated: %s (version %d)", uri, version);
}

void lsp_server_remove_document(LspServer *server, const char *uri) {
    LspDocument **curr = &server->documents_head;
    while (*curr) {
        LspDocument *doc = *curr;
        if (strcmp(doc->uri, uri) == 0) {
            *curr = doc->next;
            free(doc->uri);
            free(doc->content);
            free(doc);
            server->document_count--;
            log_debug("LSP: Document removed: %s", uri);
            return;
        }
        curr = &doc->next;
    }
    log_debug("LSP: Cannot remove non-existent document: %s", uri);
}

// ============================================================================
// textDocument handlers
// ============================================================================

void lsp_handle_text_document_did_open(LspServer *server, JsonRpcNotification *notification) {
    if (!notification->params || notification->params->type != JSON_OBJECT) {
        log_error("LSP: Invalid didOpen params");
        return;
    }

    JsonObject *params = notification->params->as.object_value;
    JsonValue *text_doc_val = json_object_get(params, "textDocument");

    if (!text_doc_val || text_doc_val->type != JSON_OBJECT) {
        log_error("LSP: Missing textDocument in didOpen");
        return;
    }

    JsonObject *text_doc = text_doc_val->as.object_value;
    JsonValue *uri_val = json_object_get(text_doc, "uri");
    JsonValue *text_val = json_object_get(text_doc, "text");
    JsonValue *version_val = json_object_get(text_doc, "version");

    if (!uri_val || uri_val->type != JSON_STRING ||
        !text_val || text_val->type != JSON_STRING ||
        !version_val || version_val->type != JSON_NUMBER) {
        log_error("LSP: Invalid textDocument fields in didOpen");
        return;
    }

    const char *uri = uri_val->as.string_value;
    const char *text = text_val->as.string_value;
    int version = (int)version_val->as.number_value;

    lsp_server_add_document(server, uri, text, version);
}

void lsp_handle_text_document_did_change(LspServer *server, JsonRpcNotification *notification) {
    if (!notification->params || notification->params->type != JSON_OBJECT) {
        log_error("LSP: Invalid didChange params");
        return;
    }

    JsonObject *params = notification->params->as.object_value;
    JsonValue *text_doc_val = json_object_get(params, "textDocument");
    JsonValue *content_changes_val = json_object_get(params, "contentChanges");

    if (!text_doc_val || text_doc_val->type != JSON_OBJECT ||
        !content_changes_val || content_changes_val->type != JSON_ARRAY) {
        log_error("LSP: Invalid didChange structure");
        return;
    }

    JsonObject *text_doc = text_doc_val->as.object_value;
    JsonValue *uri_val = json_object_get(text_doc, "uri");
    JsonValue *version_val = json_object_get(text_doc, "version");

    if (!uri_val || uri_val->type != JSON_STRING ||
        !version_val || version_val->type != JSON_NUMBER) {
        log_error("LSP: Invalid textDocument fields in didChange");
        return;
    }

    const char *uri = uri_val->as.string_value;
    int version = (int)version_val->as.number_value;

    // For full document sync (change = 1), we expect a single change with the full text
    JsonArray *content_changes = content_changes_val->as.array_value;
    if (content_changes->count == 0) {
        log_error("LSP: No content changes in didChange");
        return;
    }

    JsonValue *change_val = json_array_get(content_changes, 0);
    if (!change_val || change_val->type != JSON_OBJECT) {
        log_error("LSP: Invalid content change object");
        return;
    }

    JsonObject *change = change_val->as.object_value;
    JsonValue *text_val = json_object_get(change, "text");

    if (!text_val || text_val->type != JSON_STRING) {
        log_error("LSP: Missing text in content change");
        return;
    }

    const char *text = text_val->as.string_value;
    lsp_server_update_document(server, uri, text, version);
}

void lsp_handle_text_document_did_close(LspServer *server, JsonRpcNotification *notification) {
    if (!notification->params || notification->params->type != JSON_OBJECT) {
        log_error("LSP: Invalid didClose params");
        return;
    }

    JsonObject *params = notification->params->as.object_value;
    JsonValue *text_doc_val = json_object_get(params, "textDocument");

    if (!text_doc_val || text_doc_val->type != JSON_OBJECT) {
        log_error("LSP: Missing textDocument in didClose");
        return;
    }

    JsonObject *text_doc = text_doc_val->as.object_value;
    JsonValue *uri_val = json_object_get(text_doc, "uri");

    if (!uri_val || uri_val->type != JSON_STRING) {
        log_error("LSP: Invalid uri in didClose");
        return;
    }

    const char *uri = uri_val->as.string_value;
    lsp_server_remove_document(server, uri);
}
