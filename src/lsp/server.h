#ifndef LSP_SERVER_H
#define LSP_SERVER_H

#include "json.h"
#include "jsonrpc.h"
#include "../arena.h"

// Arena sizes
#define TMP_ARENA_SIZE (1024 * 1024)  // 1MB for temporary allocations

// Document state (stored in linked list)
typedef struct LspDocument {
    char *uri;
    char *content;
    int version;
    struct LspDocument *next;
} LspDocument;

// LSP server state
typedef struct {
    Arena *temp_arena; // For temporary allocations during message processing
    int initialized;
    int shutdown_requested;

    // Document storage (linked list)
    LspDocument *documents_head;
    int document_count;
} LspServer;

// Server lifecycle
LspServer *lsp_server_create();
int lsp_server_run();
void lsp_server_shutdown(LspServer *server);

// Message handlers
void lsp_handle_message(LspServer *server, JsonRpcMessage *message);
void lsp_handle_initialize(LspServer *server, JsonRpcRequest *request);
void lsp_handle_initialized(LspServer *server, JsonRpcNotification *notification);
void lsp_handle_shutdown(LspServer *server, JsonRpcRequest *request);
void lsp_handle_exit(LspServer *server, JsonRpcNotification *notification);

// textDocument handlers
void lsp_handle_text_document_did_open(LspServer *server, JsonRpcNotification *notification);
void lsp_handle_text_document_did_change(LspServer *server, JsonRpcNotification *notification);
void lsp_handle_text_document_did_close(LspServer *server, JsonRpcNotification *notification);

// Document management
LspDocument *lsp_server_get_document(LspServer *server, const char *uri);
void lsp_server_add_document(LspServer *server, const char *uri, const char *content, int version);
void lsp_server_update_document(LspServer *server, const char *uri, const char *content, int version);
void lsp_server_remove_document(LspServer *server, const char *uri);

#endif // LSP_SERVER_H
