#include "../src/arena.c" // Include the arena implementation
#include <assert.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

// Test result tracking
static int tests_run = 0;
static int tests_passed = 0;

// Test macros
#define TEST_START(name)                                                       \
    do {                                                                       \
        printf("Running test: %s... ", name);                                  \
        tests_run++;                                                           \
    } while (0)

#define TEST_PASS()                                                            \
    do {                                                                       \
        printf("PASSED\n");                                                    \
        tests_passed++;                                                        \
    } while (0)

#define TEST_FAIL(msg)                                                         \
    do {                                                                       \
        printf("FAILED: %s\n", msg);                                           \
        return 0;                                                              \
    } while (0)

#define ASSERT(condition, msg)                                                 \
    do {                                                                       \
        if (!(condition)) {                                                    \
            TEST_FAIL(msg);                                                    \
        }                                                                      \
    } while (0)

// Test basic arena creation and destruction
int test_arena_creation() {
    TEST_START("Arena Creation and Destruction");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");
    ASSERT(arena->first_chunk != NULL, "First chunk not created");
    ASSERT(arena->current_chunk == arena->first_chunk,
           "Current chunk not set correctly");
    ASSERT(arena->first_chunk->size == (size_t)ARENA_PAGE_SIZE,
           "First chunk size incorrect");
    ASSERT(arena->first_chunk->used == 0, "First chunk should start empty");
    ASSERT(arena->first_chunk->next == NULL, "First chunk should have no next");

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Test basic allocation
int test_basic_allocation() {
    TEST_START("Basic Allocation");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    // Allocate some memory
    void *ptr1 = arena_alloc(arena, 64);
    ASSERT(ptr1 != NULL, "First allocation failed");
    ASSERT(arena->current_chunk->used >= 64,
           "Used memory not updated correctly");

    void *ptr2 = arena_alloc(arena, 128);
    ASSERT(ptr2 != NULL, "Second allocation failed");
    ASSERT(ptr2 > ptr1, "Second allocation should be after first");

    // Test that we can write to allocated memory
    memset(ptr1, 0xAB, 64);
    memset(ptr2, 0xCD, 128);

    ASSERT(((char *)ptr1)[0] == (char)0xAB, "Memory write failed for ptr1");
    ASSERT(((char *)ptr2)[0] == (char)0xCD, "Memory write failed for ptr2");

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Test memory alignment
int test_alignment() {
    TEST_START("Memory Alignment");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    // Test various sizes to ensure 8-byte alignment
    void *ptr1 = arena_alloc(arena, 1);
    void *ptr2 = arena_alloc(arena, 1);
    void *ptr3 = arena_alloc(arena, 7);
    void *ptr4 = arena_alloc(arena, 1);

    ASSERT(ptr1 != NULL && ptr2 != NULL && ptr3 != NULL && ptr4 != NULL,
           "Allocations failed");

    // Check 8-byte alignment
    ASSERT(((uintptr_t)ptr1) % 8 == 0, "ptr1 not 8-byte aligned");
    ASSERT(((uintptr_t)ptr2) % 8 == 0, "ptr2 not 8-byte aligned");
    ASSERT(((uintptr_t)ptr3) % 8 == 0, "ptr3 not 8-byte aligned");
    ASSERT(((uintptr_t)ptr4) % 8 == 0, "ptr4 not 8-byte aligned");

    // Check that allocations don't overlap
    ASSERT((char *)ptr2 >= (char *)ptr1 + 8, "ptr1 and ptr2 overlap");
    ASSERT((char *)ptr3 >= (char *)ptr2 + 8, "ptr2 and ptr3 overlap");
    ASSERT((char *)ptr4 >= (char *)ptr3 + 8, "ptr3 and ptr4 overlap");

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Test large allocation that requires new chunk
int test_large_allocation() {
    TEST_START("Large Allocation");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    size_t page_size = ARENA_PAGE_SIZE;

    // First, fill up most of the first chunk
    void *small_ptr = arena_alloc(arena, page_size - 64);
    ASSERT(small_ptr != NULL, "Small allocation failed");

    // Now allocate something that won't fit in remaining space
    void *large_ptr = arena_alloc(arena, page_size);
    ASSERT(large_ptr != NULL, "Large allocation failed");

    // This should have created a new chunk
    ASSERT(arena->first_chunk->next != NULL, "New chunk not created");
    ASSERT(arena->current_chunk == arena->first_chunk->next,
           "Current chunk not updated to new chunk");

    // Test that we can write to both allocations
    memset(small_ptr, 0x11, page_size - 64);
    memset(large_ptr, 0x22, page_size);

    ASSERT(((char *)small_ptr)[0] == 0x11, "Small allocation memory corrupted");
    ASSERT(((char *)large_ptr)[0] == 0x22, "Large allocation memory corrupted");

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Test very large allocation that needs multiple pages
int test_huge_allocation() {
    TEST_START("Huge Allocation");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    size_t page_size = ARENA_PAGE_SIZE;
    size_t huge_size = page_size * 3; // 3 pages

    void *huge_ptr = arena_alloc(arena, huge_size);
    ASSERT(huge_ptr != NULL, "Huge allocation failed");

    // Check that the chunk size is appropriate (rounded up to page boundary)
    Chunk *chunk = arena->first_chunk;
    while (chunk->next)
        chunk = chunk->next; // Find the chunk with our allocation

    ASSERT(chunk->size >= huge_size,
           "Chunk size too small for huge allocation");
    ASSERT(chunk->size % page_size == 0, "Chunk size not page-aligned");

    // Test writing to the huge allocation
    memset(huge_ptr, 0x33, huge_size);
    ASSERT(((char *)huge_ptr)[0] == 0x33, "Huge allocation start corrupted");
    ASSERT(((char *)huge_ptr)[huge_size - 1] == 0x33,
           "Huge allocation end corrupted");

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Test arena reset functionality
int test_arena_reset() {
    TEST_START("Arena Reset");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    // Make some allocations
    void *ptr1 = arena_alloc(arena, 100);
    void *ptr2 = arena_alloc(arena, 200);
    void *ptr3 = arena_alloc(arena, 300);

    ASSERT(ptr1 && ptr2 && ptr3, "Initial allocations failed");

    size_t used_before = arena->current_chunk->used;
    ASSERT(used_before > 0, "No memory marked as used");

    // Reset the arena
    arena_reset(arena);

    // Check that all chunks are reset
    Chunk *chunk = arena->first_chunk;
    while (chunk) {
        ASSERT(chunk->used == 0, "Chunk not properly reset");
        chunk = chunk->next;
    }

    ASSERT(arena->current_chunk == arena->first_chunk,
           "Current chunk not reset to first");

    // Allocate after reset - should get same address as first allocation
    void *new_ptr = arena_alloc(arena, 100);
    ASSERT(new_ptr == ptr1, "Reset didn't properly reclaim memory");

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Test arena_calloc functionality
int test_arena_calloc() {
    TEST_START("Arena Calloc");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    // Allocate zeroed memory
    size_t count = 50;
    size_t size = sizeof(int);
    int *ptr = (int *)arena_calloc(arena, count, size);

    ASSERT(ptr != NULL, "Calloc failed");

    // Check that all memory is zeroed
    for (size_t i = 0; i < count; i++) {
        ASSERT(ptr[i] == 0, "Memory not zeroed by calloc");
    }

    // Write some data and verify
    for (size_t i = 0; i < count; i++) {
        ptr[i] = (int)i;
    }

    for (size_t i = 0; i < count; i++) {
        ASSERT(ptr[i] == (int)i, "Memory corrupted after write");
    }

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Test arena_available functionality
int test_arena_available() {
    TEST_START("Arena Available Space");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    size_t initial_available = arena_available(arena);
    ASSERT(initial_available == (size_t)ARENA_PAGE_SIZE,
           "Initial available space incorrect");

    // Make an allocation
    void *ptr = arena_alloc(arena, 100);
    ASSERT(ptr != NULL, "Allocation failed");

    size_t after_alloc = arena_available(arena);
    ASSERT(after_alloc < initial_available,
           "Available space not reduced after allocation");
    ASSERT(after_alloc == initial_available - ((100 + 7) & ~7),
           "Available space calculation incorrect");

    // Reset and check available space is restored
    arena_reset(arena);
    size_t after_reset = arena_available(arena);
    ASSERT(after_reset == initial_available,
           "Available space not restored after reset");

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Test multiple chunks and chunk reuse
int test_chunk_reuse() {
    TEST_START("Chunk Reuse");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    size_t page_size = ARENA_PAGE_SIZE;

    // Fill first chunk almost completely
    void *ptr1 = arena_alloc(arena, page_size - 100);
    ASSERT(ptr1 != NULL, "First large allocation failed");

    // Force creation of second chunk
    void *ptr2 = arena_alloc(arena, page_size / 2);
    ASSERT(ptr2 != NULL, "Second allocation failed");
    ASSERT(arena->first_chunk->next != NULL, "Second chunk not created");

    // Now try to allocate something small that should fit in first chunk
    void *ptr3 = arena_alloc(arena, 50);
    ASSERT(ptr3 != NULL, "Small allocation failed");

    // ptr3 should be in the first chunk (reuse)
    ASSERT(ptr3 >= arena->first_chunk->memory &&
               ptr3 < arena->first_chunk->memory + arena->first_chunk->size,
           "Small allocation not placed in first chunk");

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Test edge cases and error conditions
int test_edge_cases() {
    TEST_START("Edge Cases");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    // Test zero-size allocation
    void *ptr_zero = arena_alloc(arena, 0);
    ASSERT(ptr_zero != NULL, "Zero-size allocation should succeed");

    // Test very small allocations
    void *ptr1 = arena_alloc(arena, 1);
    void *ptr2 = arena_alloc(arena, 1);
    ASSERT(ptr1 != NULL && ptr2 != NULL, "Small allocations failed");
    ASSERT(ptr1 != ptr2, "Small allocations returned same pointer");

    // Test alignment with odd sizes
    void *ptr_odd1 = arena_alloc(arena, 13);
    void *ptr_odd2 = arena_alloc(arena, 17);
    ASSERT(((uintptr_t)ptr_odd1) % 8 == 0, "Odd size allocation not aligned");
    ASSERT(((uintptr_t)ptr_odd2) % 8 == 0, "Odd size allocation not aligned");

    arena_destroy(arena);

    // Test destroying NULL arena (should not crash)
    arena_destroy(NULL);

    TEST_PASS();
    return 1;
}

// Test stress scenario with many allocations
int test_stress() {
    TEST_START("Stress Test");

    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    const int num_allocs = 1000;
    void *ptrs[num_allocs];

    // Make many small allocations
    for (int i = 0; i < num_allocs; i++) {
        ptrs[i] = arena_alloc(arena, (i % 100) + 1);
        ASSERT(ptrs[i] != NULL, "Stress allocation failed");

        // Write pattern to verify memory integrity
        memset(ptrs[i], i % 256, (i % 100) + 1);
    }

    // Verify all allocations are still valid
    for (int i = 0; i < num_allocs; i++) {
        char expected = i % 256;
        ASSERT(((char *)ptrs[i])[0] == expected,
               "Memory corrupted during stress test");
    }

    arena_destroy(arena);

    TEST_PASS();
    return 1;
}

// Main test runner
int main() {
    printf("Starting Arena Memory Allocator Tests\n");
    printf("=====================================\n\n");

    // Run all tests
    test_arena_creation();
    test_basic_allocation();
    test_alignment();
    test_large_allocation();
    test_huge_allocation();
    test_arena_reset();
    test_arena_calloc();
    test_arena_available();
    test_chunk_reuse();
    test_edge_cases();
    test_stress();

    // Print summary
    printf("\n=====================================\n");
    printf("Test Results: %d/%d tests passed\n", tests_passed, tests_run);

    if (tests_passed == tests_run) {
        printf("All tests PASSED!\n");
        return 0;
    } else {
        printf("%d tests FAILED\n", tests_run - tests_passed);
        return 1;
    }
}
