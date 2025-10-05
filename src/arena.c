#include "arena.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

// Get the system page size
#define ARENA_PAGE_SIZE (sysconf(_SC_PAGESIZE))

// Structure for each memory chunk in the arena
typedef struct Chunk {
    void *memory;       // Pointer to the memory block
    size_t size;        // Total size of this chunk
    size_t used;        // Amount used in this chunk
    struct Chunk *next; // Next chunk in the list
} Chunk;

// Define the arena structure
typedef struct Arena {
    Chunk *first_chunk;   // First chunk in the linked list
    Chunk *current_chunk; // Current chunk we're allocating from
} Arena;

// Create a new chunk with the specified size (rounded up to page boundary if
// needed)
static Chunk *chunk_create(size_t min_size) {
    size_t page_size = ARENA_PAGE_SIZE;

    // Calculate chunk size: if min_size > page_size, use min_size rounded up to
    // page boundary Otherwise use exactly one page
    size_t chunk_size;
    if (min_size > page_size) {
        // Round up to next page boundary
        chunk_size = ((min_size + page_size - 1) / page_size) * page_size;
    } else {
        chunk_size = page_size;
    }

    Chunk *chunk = (Chunk *)malloc(sizeof(Chunk));
    if (!chunk)
        return NULL;

    chunk->memory = malloc(chunk_size);
    if (!chunk->memory) {
        free(chunk);
        return NULL;
    }

    chunk->size = chunk_size;
    chunk->used = 0;
    chunk->next = NULL;

    return chunk;
}

// Initialize an arena
Arena *arena_create(size_t size) {
    Arena *arena = (Arena *)malloc(sizeof(Arena));
    if (!arena)
        return NULL;

    arena->first_chunk = chunk_create(size);
    if (!arena->first_chunk) {
        free(arena);
        return NULL;
    }

    arena->current_chunk = arena->first_chunk;

    return arena;
}

// Allocate memory from the arena
void *arena_alloc(Arena *arena, size_t size) {
    // Align to 8 bytes for better memory access
    size_t aligned_size = (size + 7) & ~7;

    // Try to find an existing chunk
    // with space
    Chunk *chunk = arena->first_chunk;
    while (chunk) {
        if (chunk->used + aligned_size <= chunk->size) {
            arena->current_chunk = chunk;
            void *ptr = (char *)chunk->memory + chunk->used;
            chunk->used += aligned_size;
            return ptr;
        }
        chunk = chunk->next;
    }

    // No existing chunk has enough space, create a new one
    Chunk *new_chunk = chunk_create(aligned_size);
    if (!new_chunk) {
        return NULL; // Failed to allocate new chunk
    }

    // Add new chunk to the end of the list
    chunk = arena->first_chunk;
    while (chunk->next) {
        chunk = chunk->next;
    }
    chunk->next = new_chunk;

    // Allocate from the new chunk
    arena->current_chunk = new_chunk;
    void *ptr = (char *)new_chunk->memory + new_chunk->used;
    new_chunk->used += aligned_size;

    return ptr;
}

// Reset the arena (resets all chunks to unused)
void arena_reset(Arena *arena) {
    Chunk *chunk = arena->first_chunk;
    while (chunk) {
        chunk->used = 0;
        chunk = chunk->next;
    }
    arena->current_chunk = arena->first_chunk;
}

// Free the arena and all its chunks
void arena_destroy(Arena *arena) {
    if (!arena)
        return;

    Chunk *chunk = arena->first_chunk;
    while (chunk) {
        Chunk *next = chunk->next;
        free(chunk->memory);
        free(chunk);
        chunk = next;
    }

    free(arena);
}

// Allocate and zero-initialize memory
void *arena_calloc(Arena *arena, size_t count, size_t size) {
    size_t total_size = count * size;
    void *ptr = arena_alloc(arena, total_size);
    if (ptr) {
        memset(ptr, 0, total_size);
    }
    return ptr;
}

// Get the total amount of free space across all chunks
size_t arena_available(Arena *arena) {
    size_t available = 0;
    Chunk *chunk = arena->first_chunk;
    while (chunk) {
        available += (chunk->size - chunk->used);
        chunk = chunk->next;
    }
    return available;
}

// Create a marker to allow partial reset later
// size_t arena_mark(Arena* arena) {
//    // For simplicity, we'll just return the used amount in the current chunk
//    // A more sophisticated implementation could store chunk + offset
//    return arena->current_chunk->used;
//}

// Reset to a previously saved marker (simplified version)
// void arena_reset_to_mark(Arena* arena, size_t mark) {
// This is a simplified implementation that only works with the current chunk
//    if (mark <= arena->current_chunk->size) {
//        arena->current_chunk->used = mark;
//    }
//}
