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

#include "array.h"
#include <stdlib.h>
#include <string.h>

// Initialize a new array
Array *array_init(size_t element_size) {
    if (element_size == 0) {
        return NULL;
    }

    Array *array = (Array *)malloc(sizeof(Array));
    if (!array) {
        return NULL;
    }

    array->head = NULL;
    array->tail = NULL;
    array->element_size = element_size;
    array->length = 0;
    array->capacity = 0;

    // Determine elements per chunk based on element size
    if (element_size < PAGE_SIZE) {
        // Small elements: fit as many as possible in one page
        array->elements_per_chunk = PAGE_SIZE / element_size;
    } else {
        // Large elements: fixed number per chunk
        array->elements_per_chunk = LARGE_CHUNK_ELEMENTS;
    }

    return array;
}

// Allocate a new chunk
static ArrayChunk *array_chunk_alloc(size_t element_size, size_t elements_per_chunk) {
    ArrayChunk *chunk = (ArrayChunk *)malloc(sizeof(ArrayChunk));
    if (!chunk) {
        return NULL;
    }

    size_t chunk_size;
    if (element_size < PAGE_SIZE) {
        // Allocate one page
        chunk_size = PAGE_SIZE;
    } else {
        // Allocate space for LARGE_CHUNK_ELEMENTS
        chunk_size = element_size * elements_per_chunk;
    }

    chunk->data = malloc(chunk_size);
    if (!chunk->data) {
        free(chunk);
        return NULL;
    }

    chunk->next = NULL;
    return chunk;
}

// Free the array and all its memory
void array_free(Array *array) {
    if (!array) {
        return;
    }

    ArrayChunk *current = array->head;
    while (current) {
        ArrayChunk *next = current->next;
        free(current->data);
        free(current);
        current = next;
    }

    free(array);
}

// Append an element to the array
bool array_append(Array *array, const void *element) {
    if (!array || !element) {
        return false;
    }

    // Check if we need to allocate a new chunk
    if (array->length >= array->capacity) {
        ArrayChunk *new_chunk = array_chunk_alloc(array->element_size, array->elements_per_chunk);
        if (!new_chunk) {
            return false;
        }

        // Add chunk to the list
        if (!array->head) {
            array->head = new_chunk;
            array->tail = new_chunk;
        } else {
            array->tail->next = new_chunk;
            array->tail = new_chunk;
        }

        array->capacity += array->elements_per_chunk;
    }

    // Find the correct chunk and offset
    size_t chunk_index = array->length / array->elements_per_chunk;
    size_t element_offset = array->length % array->elements_per_chunk;

    ArrayChunk *current = array->head;
    for (size_t i = 0; i < chunk_index; i++) {
        current = current->next;
    }

    // Copy the element
    char *dest = (char *)current->data + (element_offset * array->element_size);
    memcpy(dest, element, array->element_size);

    array->length++;
    return true;
}

// Get a pointer to an element at a specific index
void *array_get(const Array *array, size_t index) {
    if (!array || index >= array->length) {
        return NULL;
    }

    // Find the correct chunk and offset
    size_t chunk_index = index / array->elements_per_chunk;
    size_t element_offset = index % array->elements_per_chunk;

    ArrayChunk *current = array->head;
    for (size_t i = 0; i < chunk_index; i++) {
        current = current->next;
    }

    return (char *)current->data + (element_offset * array->element_size);
}

// Set an element at a specific index
bool array_set(Array *array, size_t index, const void *element) {
    if (!array || !element || index >= array->length) {
        return false;
    }

    void *dest = array_get(array, index);
    if (!dest) {
        return false;
    }

    memcpy(dest, element, array->element_size);
    return true;
}

// Get the current length of the array
size_t array_length(const Array *array) {
    return array ? array->length : 0;
}

// Get the current capacity of the array
size_t array_capacity(const Array *array) {
    return array ? array->capacity : 0;
}

// Remove and return the last element
bool array_pop(Array *array, void *out_element) {
    if (!array || array->length == 0) {
        return false;
    }

    void *element = array_get(array, array->length - 1);
    if (!element) {
        return false;
    }

    if (out_element) {
        memcpy(out_element, element, array->element_size);
    }

    array->length--;
    return true;
}

// Clear all elements but keep the allocated memory
void array_clear(Array *array) {
    if (array) {
        array->length = 0;
    }
}
