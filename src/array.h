/*
 * Dynamic Array Implementation with Chunk-Based Memory Management
 *
 * Key design features:
 *
 * 1. Always uses chunks - All arrays use a linked list of chunks,
 *    no special case for contiguous memory
 *
 * 2. Smart chunk sizing:
 *    - Small elements (element_size < PAGE_SIZE): Each chunk is exactly
 *      one page (4096 bytes) and holds PAGE_SIZE / element_size elements
 *    - Large elements (element_size >= PAGE_SIZE): Each chunk holds
 *      exactly 20 elements
 *
 * 3. Efficient operations:
 *    - array_append(): Allocates new chunks only when needed
 *    - array_get(): Direct indexing by calculating chunk and offset
 *    - Fast tail pointer for O(1) append operations
 *
 * 4. Memory efficient: No realloc copying, just add new chunks as needed
 */

#ifndef ARRAY_H
#define ARRAY_H

#include <stddef.h>
#include <stdbool.h>

// System page size (typically 4096 bytes)
#define PAGE_SIZE 4096

// Number of elements per chunk for large elements
#define LARGE_CHUNK_ELEMENTS 20

// Node in the linked list of chunks
typedef struct ArrayChunk {
    void *data;                    // Pointer to the chunk data
    struct ArrayChunk *next;       // Next chunk in the list
} ArrayChunk;

// Dynamic array structure
typedef struct {
    ArrayChunk *head;              // First chunk in the list
    ArrayChunk *tail;              // Last chunk in the list (for fast append)
    size_t element_size;           // Size of each element in bytes
    size_t elements_per_chunk;     // Number of elements each chunk can hold
    size_t length;                 // Current number of elements
    size_t capacity;               // Total capacity across all chunks
} Array;

// Initialize a new array
Array *array_init(size_t element_size);

// Free the array and all its memory
void array_free(Array *array);

// Append an element to the array
bool array_append(Array *array, const void *element);

// Get a pointer to an element at a specific index
void *array_get(const Array *array, size_t index);

// Set an element at a specific index
bool array_set(Array *array, size_t index, const void *element);

// Get the current length of the array
size_t array_length(const Array *array);

// Get the current capacity of the array
size_t array_capacity(const Array *array);

// Remove and return the last element (caller must provide buffer)
bool array_pop(Array *array, void *out_element);

// Clear all elements but keep the allocated memory
void array_clear(Array *array);

#endif // ARRAY_H
