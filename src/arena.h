#ifndef ARENA_H
#define ARENA_H

#include <stddef.h>

// Forward declarations
typedef struct Arena Arena;

// Public API
Arena* arena_create(size_t size);
void* arena_alloc(Arena* arena, size_t size);
void* arena_calloc(Arena* arena, size_t count, size_t size);
void arena_reset(Arena* arena);
void arena_destroy(Arena* arena);
size_t arena_available(Arena* arena);
//size_t arena_mark(Arena* arena);
//void arena_reset_to_mark(Arena* arena, size_t mark);

#endif // ARENA_H
