#include "../src/arena.c"
#include "../src/string_storage.c"
#include <assert.h>
#include <stdio.h>
#include <string.h>

// Test macros
#define TEST_FAIL(msg)                                                         \
    printf("FAILED: %s\n", msg);                                               \
    abort();

#define ASSERT(condition, msg)                                                 \
    if (!(condition)) {                                                        \
        TEST_FAIL(msg);                                                        \
    }

#define ASSERT_STR_EQ(actual, expected, message)                               \
    if (strcmp(actual, expected) != 0) {                                       \
        printf("ASSERTION FAILED: %s\n", message);                             \
        printf("  Expected: \"%s\", Actual: \"%s\"\n", expected, actual);      \
        printf("  at %s:%d\n", __FILE__, __LINE__);                            \
    }

// Test basic string storage functionality
void test_basic_storage() {
    Arena *arena = arena_create(1024);
    ASSERT(arena != NULL, "Arena creation failed");

    StringStorage *storage = string_storage_init(arena);
    ASSERT(storage != NULL, "String storage initialization failed");
    ASSERT(storage->string_count == 0, "Initial string count should be 0");
    ASSERT(storage->head == NULL, "Initial head should be NULL");
    ASSERT(storage->tail == NULL, "Initial tail should be NULL");

    arena_destroy(arena);
}

// Test storing strings from buffer
void test_store_from_buffer() {
    Arena *arena = arena_create(1024);
    StringStorage *storage = string_storage_init(arena);

    const char *buffer = "Hello, World! This is a test buffer.";

    // Store "Hello"
    String *str1 = store_from_buffer(storage, buffer, 0, 5);
    ASSERT(str1 != NULL, "Failed to store first string");
    printf("%s", str1->data);
    printf("%ld", str1->length);
    ASSERT(str1->length == 5, "First string length incorrect");
    ASSERT_STR_EQ(str1->data, "Hello", "First string data incorrect");
    ASSERT(storage->string_count == 1, "String count should be 1");

    // Store "World"
    String *str2 = store_from_buffer(storage, buffer, 7, 5);
    ASSERT(str2 != NULL, "Failed to store second string");
    ASSERT(str2->length == 5, "Second string length incorrect");
    ASSERT_STR_EQ(str2->data, "World", "Second string data incorrect");
    ASSERT(storage->string_count == 2, "String count should be 2");

    // Store "test"
    String *str3 = store_from_buffer(storage, buffer, 24, 4);
    ASSERT(str3 != NULL, "Failed to store third string");
    ASSERT(str3->length == 4, "Third string length incorrect");
    ASSERT_STR_EQ(str3->data, "test", "Third string data incorrect");
    ASSERT(storage->string_count == 3, "String count should be 3");

    arena_destroy(arena);
}

// Test string deduplication
void test_string_deduplication() {
    Arena *arena = arena_create(1024);
    StringStorage *storage = string_storage_init(arena);

    // Store "test" first time
    String *str1 = store_cstring(storage, "test");
    ASSERT(str1 != NULL, "Failed to store first instance");
    ASSERT(storage->string_count == 1, "String count should be 1");

    // Store "test" again - should return same pointer
    String *str2 = store_cstring(storage, "test");
    ASSERT(str2 != NULL, "Failed to store second instance");
    ASSERT(str1 == str2, "Deduplication failed - different pointers returned");
    ASSERT(storage->string_count == 1, "String count should still be 1");

    // Store different string
    String *str3 = store_cstring(storage, "different");
    ASSERT(str3 != NULL, "Failed to store different string");
    ASSERT(str3 != str1, "Different strings should have different pointers");
    ASSERT(storage->string_count == 2, "String count should be 2");

    // Store "test" again from buffer - should still deduplicate
    const char *buffer = "testing";
    String *str4 = store_from_buffer(storage, buffer, 0, 4);
    ASSERT(str4 == str1, "Buffer deduplication failed");
    ASSERT(storage->string_count == 2, "String count should still be 2");

    arena_destroy(arena);
}

// Test empty and edge case strings
void test_edge_cases() {
    Arena *arena = arena_create(1024);
    StringStorage *storage = string_storage_init(arena);

    // Test empty string
    String *empty1 = store_cstring(storage, "");
    ASSERT(empty1 != NULL, "Failed to store empty string");
    ASSERT(empty1->length == 0, "Empty string length should be 0");
    ASSERT_STR_EQ(empty1->data, "", "Empty string data should be empty");

    // Test empty string deduplication
    String *empty2 = store_from_buffer(storage, "hello", 0, 0);
    ASSERT(empty2 == empty1, "Empty string deduplication failed");
    ASSERT(storage->string_count == 1, "Should only have one empty string");

    // Test single character string
    String *single = store_cstring(storage, "a");
    ASSERT(single != NULL, "Failed to store single character");
    ASSERT(single->length == 1, "Single char length should be 1");
    ASSERT_STR_EQ(single->data, "a", "Single char data incorrect");

    // Test very long string (within arena limits)
    char long_str[500];
    memset(long_str, 'x', 499);
    long_str[499] = '\0';

    String *long_string = store_cstring(storage, long_str);
    ASSERT(long_string != NULL, "Failed to store long string");
    ASSERT(long_string->length == 499, "Long string length incorrect");
    ASSERT_STR_EQ(long_string->data, long_str, "Long string data incorrect");

    arena_destroy(arena);
}

// Test string storage with special characters
void test_special_characters() {
    Arena *arena = arena_create(1024);
    StringStorage *storage = string_storage_init(arena);

    // Test string with null bytes (stored as buffer)
    const char buffer_with_null[] = {'h', 'e', 'l', 'l', 'o', '\0',
                                     'w', 'o', 'r', 'l', 'd'};
    String *str_with_null = store_from_buffer(storage, buffer_with_null, 0, 11);
    ASSERT(str_with_null != NULL, "Failed to store string with null byte");
    ASSERT(str_with_null->length == 11,
           "String with null byte length incorrect");

    // Test strings with newlines and tabs
    String *str_newline = store_cstring(storage, "hello\nworld");
    ASSERT(str_newline != NULL, "Failed to store string with newline");
    ASSERT_STR_EQ(str_newline->data, "hello\nworld",
                  "Newline string incorrect");

    String *str_tab = store_cstring(storage, "hello\tworld");
    ASSERT(str_tab != NULL, "Failed to store string with tab");
    ASSERT_STR_EQ(str_tab->data, "hello\tworld", "Tab string incorrect");

    // Test unicode/high ASCII characters
    String *str_unicode = store_cstring(storage, "héllø wørld");
    ASSERT(str_unicode != NULL, "Failed to store unicode string");
    ASSERT_STR_EQ(str_unicode->data, "héllø wørld", "Unicode string incorrect");

    arena_destroy(arena);
}

// Test storage statistics
void test_storage_stats() {
    Arena *arena = arena_create(1024);
    StringStorage *storage = string_storage_init(arena);

    StringStorageStats stats = get_storage_stats(storage);
    ASSERT(stats.total_strings == 0, "Initial stats should show 0 strings");
    ASSERT(stats.memory_used_strings == 0, "Initial memory usage should be 0");

    // Add some strings
    store_cstring(storage, "hello");
    store_cstring(storage, "world");
    store_cstring(storage, "test");

    stats = get_storage_stats(storage);
    ASSERT(stats.total_strings == 3, "Should have 3 strings");
    ASSERT(stats.memory_used_strings > 0, "Memory usage should be > 0");

    // Add duplicate - stats shouldn't change
    store_cstring(storage, "hello");
    stats = get_storage_stats(storage);
    ASSERT(stats.total_strings == 3, "Duplicate shouldn't increase count");

    arena_destroy(arena);
}

// Test linked list integrity
void test_linked_list_integrity() {
    Arena *arena = arena_create(1024);
    StringStorage *storage = string_storage_init(arena);

    // Add strings and verify list structure
    const char *test_strings[] = {"first", "second", "third", "fourth"};
    String *stored_strings[4];

    for (int i = 0; i < 4; i++) {
        stored_strings[i] = store_cstring(storage, test_strings[i]);
        ASSERT(stored_strings[i] != NULL,
               "Failed to store string in list test");
    }

    // Verify we can traverse the entire list
    StringNode *current = storage->head;
    int count = 0;
    while (current) {
        ASSERT(count < 4, "List traversal found more nodes than expected");
        ASSERT_STR_EQ(current->string.data, test_strings[count],
                      "List order incorrect");
        current = current->next;
        count++;
    }
    ASSERT(count == 4, "List traversal count incorrect");

    // Verify tail points to last node
    ASSERT(storage->tail != NULL, "Tail should not be NULL");
    ASSERT_STR_EQ(storage->tail->string.data, "fourth",
                  "Tail should point to last string");
    ASSERT(storage->tail->next == NULL, "Tail's next should be NULL");

    arena_destroy(arena);
}

int main() {
    test_basic_storage();
    test_store_from_buffer();
    test_string_deduplication();
    test_edge_cases();
    test_special_characters();
    test_storage_stats();
    test_linked_list_integrity();

    return 0;
}
