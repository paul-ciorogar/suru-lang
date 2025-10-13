#include "../array.c" // Include the array implementation
#include <assert.h>
#include <stdio.h>
#include <string.h>

// Test basic array creation and destruction
void test_array_creation() {
    Array *array = array_init(sizeof(int));
    assert(array != NULL);                       // Array creation failed
    assert(array->element_size == sizeof(int));  // Element size not set correctly
    assert(array->length == 0);                  // Initial length should be 0
    assert(array->capacity == 0);                // Initial capacity should be 0
    assert(array->head == NULL);                 // Head should be NULL initially
    assert(array->tail == NULL);                 // Tail should be NULL initially

    // For small elements, elements_per_chunk should be PAGE_SIZE / element_size
    assert(array->elements_per_chunk == PAGE_SIZE / sizeof(int)); // Elements per chunk incorrect

    array_free(array);
}

// Test array creation with large elements
void test_array_creation_large() {
    size_t large_size = PAGE_SIZE + 100; // Larger than a page
    Array *array = array_init(large_size);
    assert(array != NULL);                         // Array creation failed
    assert(array->element_size == large_size);     // Element size not set correctly
    assert(array->elements_per_chunk == LARGE_CHUNK_ELEMENTS); // Should use fixed chunk size for large elements

    array_free(array);
}

// Test basic append operation
void test_basic_append() {
    Array *array = array_init(sizeof(int));
    assert(array != NULL); // Array creation failed

    int value1 = 42;
    bool result = array_append(array, &value1);
    assert(result);                         // First append failed
    assert(array->length == 1);             // Length not updated
    assert(array->capacity > 0);            // Capacity not allocated
    assert(array->head != NULL);            // First chunk not created
    assert(array->tail == array->head);     // Tail should equal head for single chunk

    int value2 = 100;
    result = array_append(array, &value2);
    assert(result);                         // Second append failed
    assert(array->length == 2);             // Length not updated correctly

    array_free(array);
}

// Test array_get operation
void test_array_get() {
    Array *array = array_init(sizeof(int));
    assert(array != NULL); // Array creation failed

    // Add some elements
    for (int i = 0; i < 10; i++) {
        array_append(array, &i);
    }

    // Get and verify elements
    for (int i = 0; i < 10; i++) {
        int *ptr = (int *)array_get(array, i);
        assert(ptr != NULL);        // Get failed for valid index
        assert(*ptr == i);          // Retrieved value incorrect
    }

    // Test out of bounds
    void *invalid = array_get(array, 100);
    assert(invalid == NULL);        // Out of bounds should return NULL

    array_free(array);
}

// Test array_set operation
void test_array_set() {
    Array *array = array_init(sizeof(int));
    assert(array != NULL); // Array creation failed

    // Add some elements
    for (int i = 0; i < 5; i++) {
        array_append(array, &i);
    }

    // Modify elements
    int new_value = 999;
    bool result = array_set(array, 2, &new_value);
    assert(result);                             // Set operation failed

    int *ptr = (int *)array_get(array, 2);
    assert(*ptr == 999);                        // Value not updated correctly

    // Test out of bounds set
    result = array_set(array, 100, &new_value);
    assert(!result);                            // Out of bounds set should fail

    array_free(array);
}

// Test growing across multiple chunks (small elements)
void test_multiple_chunks_small() {
    Array *array = array_init(sizeof(int));
    assert(array != NULL); // Array creation failed

    size_t elements_per_chunk = array->elements_per_chunk;
    size_t total_elements = elements_per_chunk * 3 + 10; // Span 4 chunks

    // Add enough elements to create multiple chunks
    for (size_t i = 0; i < total_elements; i++) {
        int value = (int)i;
        bool result = array_append(array, &value);
        assert(result);                         // Append failed during multi-chunk test
    }

    assert(array->length == total_elements);    // Length incorrect after multi-chunk append
    assert(array->capacity >= total_elements);  // Capacity insufficient

    // Verify all elements are accessible and correct
    for (size_t i = 0; i < total_elements; i++) {
        int *ptr = (int *)array_get(array, i);
        assert(ptr != NULL);                    // Get failed in multi-chunk array
        assert(*ptr == (int)i);                 // Value incorrect in multi-chunk array
    }

    // Count chunks
    int chunk_count = 0;
    ArrayChunk *current = array->head;
    while (current) {
        chunk_count++;
        current = current->next;
    }
    assert(chunk_count == 4);                   // Expected 4 chunks

    array_free(array);
}

// Test with large elements that need special chunking
void test_large_elements() {
    size_t large_size = PAGE_SIZE * 2; // 2 pages per element
    Array *array = array_init(large_size);
    assert(array != NULL);                              // Array creation failed
    assert(array->elements_per_chunk == LARGE_CHUNK_ELEMENTS); // Should use LARGE_CHUNK_ELEMENTS

    // Add elements
    char *buffer = (char *)malloc(large_size);
    for (int i = 0; i < 25; i++) { // Span 2 chunks (20 + 5)
        memset(buffer, i % 256, large_size);
        bool result = array_append(array, buffer);
        assert(result);                                 // Append failed for large element
    }

    assert(array->length == 25);                        // Length incorrect for large elements

    // Verify elements
    for (int i = 0; i < 25; i++) {
        char *ptr = (char *)array_get(array, i);
        assert(ptr != NULL);                            // Get failed for large element
        assert(ptr[0] == (char)(i % 256));              // Value incorrect for large element
        assert(ptr[large_size - 1] == (char)(i % 256)); // End of large element corrupted
    }

    free(buffer);
    array_free(array);
}

// Test array_pop operation
void test_array_pop() {
    Array *array = array_init(sizeof(int));
    assert(array != NULL); // Array creation failed

    // Add elements
    for (int i = 0; i < 10; i++) {
        array_append(array, &i);
    }

    // Pop elements
    int popped;
    for (int i = 9; i >= 0; i--) {
        bool result = array_pop(array, &popped);
        assert(result);                         // Pop failed
        assert(popped == i);                    // Popped value incorrect
        assert(array->length == (size_t)i);     // Length not updated after pop
    }

    // Try to pop from empty array
    bool result = array_pop(array, &popped);
    assert(!result);                            // Pop from empty array should fail

    array_free(array);
}

// Test array_clear operation
void test_array_clear() {
    Array *array = array_init(sizeof(int));
    assert(array != NULL); // Array creation failed

    // Add elements
    for (int i = 0; i < 100; i++) {
        array_append(array, &i);
    }

    size_t old_capacity = array->capacity;
    ArrayChunk *old_head = array->head;

    array_clear(array);

    assert(array->length == 0);                 // Length not cleared
    assert(array->capacity == old_capacity);    // Capacity should remain unchanged
    assert(array->head == old_head);            // Chunks should not be freed

    // Should be able to add elements again
    int value = 42;
    bool result = array_append(array, &value);
    assert(result);                             // Append after clear failed
    assert(array->length == 1);                 // Length not updated after clear+append

    array_free(array);
}

// Test with different data types
void test_different_types() {
    // Test with struct
    typedef struct {
        int id;
        double value;
        char name[32];
    } TestStruct;

    Array *array = array_init(sizeof(TestStruct));
    assert(array != NULL); // Array creation failed for struct

    TestStruct data = {.id = 1, .value = 3.14, .name = "test"};
    bool result = array_append(array, &data);
    assert(result);                                     // Append struct failed

    TestStruct *retrieved = (TestStruct *)array_get(array, 0);
    assert(retrieved != NULL);                          // Get struct failed
    assert(retrieved->id == 1);                         // Struct field 'id' incorrect
    assert(retrieved->value == 3.14);                   // Struct field 'value' incorrect
    assert(strcmp(retrieved->name, "test") == 0);       // Struct field 'name' incorrect

    array_free(array);
}

// Test edge cases
void test_edge_cases() {
    // Test creation with zero size (should fail)
    Array *invalid = array_init(0);
    assert(invalid == NULL);                    // Zero-size array should return NULL

    // Test operations on NULL array
    bool result = array_append(NULL, NULL);
    assert(!result);                            // Append to NULL array should fail

    void *ptr = array_get(NULL, 0);
    assert(ptr == NULL);                        // Get from NULL array should return NULL

    assert(array_length(NULL) == 0);            // Length of NULL array should be 0
    assert(array_capacity(NULL) == 0);          // Capacity of NULL array should be 0

    // Test free on NULL (should not crash)
    array_free(NULL);
}

// Test stress with many elements
void test_stress() {
    Array *array = array_init(sizeof(int));
    assert(array != NULL); // Array creation failed

    const int num_elements = 10000;

    // Add many elements
    for (int i = 0; i < num_elements; i++) {
        bool result = array_append(array, &i);
        assert(result);                         // Stress test append failed
    }

    assert(array->length == num_elements);      // Stress test length incorrect

    // Verify all elements
    for (int i = 0; i < num_elements; i++) {
        int *ptr = (int *)array_get(array, i);
        assert(ptr != NULL);                    // Stress test get failed
        assert(*ptr == i);                      // Stress test value incorrect
    }

    // Test random access pattern
    for (int i = 0; i < 1000; i++) {
        size_t index = (i * 97) % num_elements; // Pseudo-random access
        int *ptr = (int *)array_get(array, index);
        assert(*ptr == (int)index);             // Random access value incorrect
    }

    array_free(array);
}

// Test exact boundary between small and large elements
void test_boundary_size() {
    // Test element exactly at PAGE_SIZE
    Array *array = array_init(PAGE_SIZE);
    assert(array != NULL);                              // Array creation failed at boundary
    assert(array->elements_per_chunk == LARGE_CHUNK_ELEMENTS); // PAGE_SIZE element should use large chunk strategy

    char *buffer = (char *)malloc(PAGE_SIZE);
    memset(buffer, 0xAB, PAGE_SIZE);

    bool result = array_append(array, buffer);
    assert(result);                                     // Append at boundary size failed

    char *retrieved = (char *)array_get(array, 0);
    assert(retrieved != NULL);                          // Get at boundary size failed
    assert(retrieved[0] == (char)0xAB);                 // Boundary size value incorrect

    free(buffer);
    array_free(array);

    // Test element just below PAGE_SIZE
    Array *array2 = array_init(PAGE_SIZE - 1);
    assert(array2 != NULL);                             // Array creation failed below boundary
    assert(array2->elements_per_chunk == PAGE_SIZE / (PAGE_SIZE - 1)); // Should use small element strategy

    array_free(array2);
}

// Main test runner
int main() {
    printf("Starting Dynamic Array Tests\n");
    printf("============================\n\n");

    // Run all tests
    test_array_creation();
    printf("✓ test_array_creation passed\n");

    test_array_creation_large();
    printf("✓ test_array_creation_large passed\n");

    test_basic_append();
    printf("✓ test_basic_append passed\n");

    test_array_get();
    printf("✓ test_array_get passed\n");

    test_array_set();
    printf("✓ test_array_set passed\n");

    test_multiple_chunks_small();
    printf("✓ test_multiple_chunks_small passed\n");

    test_large_elements();
    printf("✓ test_large_elements passed\n");

    test_array_pop();
    printf("✓ test_array_pop passed\n");

    test_array_clear();
    printf("✓ test_array_clear passed\n");

    test_different_types();
    printf("✓ test_different_types passed\n");

    test_edge_cases();
    printf("✓ test_edge_cases passed\n");

    test_stress();
    printf("✓ test_stress passed\n");

    test_boundary_size();
    printf("✓ test_boundary_size passed\n");

    // Print summary
    printf("\n============================\n");
    printf("All tests passed!\n");

    return 0;
}
