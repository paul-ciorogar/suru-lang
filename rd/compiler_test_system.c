#ifndef INTEGRATION_TESTS_H
#define INTEGRATION_TESTS_H

#include <dirent.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

// String with length for efficient operations
typedef struct {
    char *data;
    size_t length;
} String;

// Test types based on convention
typedef enum {
    TEST_COMPILE_ONLY,  // expected.txt exists
    TEST_COMPILE_ERROR, // expected_error.txt exists
    TEST_RUN_OUTPUT,    // expected_output.txt exists
    TEST_CHECK_FILES,   // expected_files.txt exists
    TEST_UNKNOWN
} TestType;

// Test case structure
typedef struct {
    String test_folder;
    String expected_file;
    TestType type;
} TestCase;

// Linked list node for test cases
typedef struct TestNode {
    TestCase test;
    struct TestNode *next;
} TestNode;

// Test list
typedef struct {
    TestNode *head;
    TestNode *tail;
    size_t count;
} TestList;

// === String Operations ===

String string_create(const char *str) {
    String s;
    if (str == NULL) {
        s.data = NULL;
        s.length = 0;
    } else {
        s.length = strlen(str);
        s.data = malloc(s.length + 1);
        if (s.data) {
            memcpy(s.data, str, s.length + 1);
        }
    }
    return s;
}

String string_create_with_length(const char *str, size_t len) {
    String s;
    s.length = len;
    s.data = malloc(len + 1);
    if (s.data) {
        memcpy(s.data, str, len);
        s.data[len] = '\0';
    }
    return s;
}

void string_free(String *s) {
    if (s && s->data) {
        free(s->data);
        s->data = NULL;
        s->length = 0;
    }
}

String string_concat(const String *s1, const String *s2) {
    String result;
    result.length = s1->length + s2->length;
    result.data = malloc(result.length + 1);
    if (result.data) {
        memcpy(result.data, s1->data, s1->length);
        memcpy(result.data + s1->length, s2->data, s2->length);
        result.data[result.length] = '\0';
    }
    return result;
}

String string_concat_cstr(const String *s, const char *cstr) {
    String s2 = string_create(cstr);
    String result = string_concat(s, &s2);
    string_free(&s2);
    return result;
}

// === Test List Operations ===

TestList *test_list_create(void) {
    TestList *list = malloc(sizeof(TestList));
    if (list) {
        list->head = NULL;
        list->tail = NULL;
        list->count = 0;
    }
    return list;
}

void test_list_append(TestList *list, TestCase test) {
    TestNode *node = malloc(sizeof(TestNode));
    if (!node)
        return;

    node->test = test;
    node->next = NULL;

    if (list->tail) {
        list->tail->next = node;
        list->tail = node;
    } else {
        list->head = list->tail = node;
    }
    list->count++;
}

void test_list_free(TestList *list) {
    if (!list)
        return;

    TestNode *current = list->head;
    while (current) {
        TestNode *next = current->next;
        string_free(&current->test.test_folder);
        string_free(&current->test.expected_file);
        free(current);
        current = next;
    }
    free(list);
}

// === Helper Functions ===

bool file_exists(const char *path) {
    struct stat st;
    return stat(path, &st) == 0 && S_ISREG(st.st_mode);
}

bool is_directory(const char *path) {
    struct stat st;
    return stat(path, &st) == 0 && S_ISDIR(st.st_mode);
}

// Determine test type based on which expected file exists
TestType detect_test_type(const char *test_folder, String *expected_file) {
    char path[1024];

    // Check in priority order
    snprintf(path, sizeof(path), "%s/expected_error.txt", test_folder);
    if (file_exists(path)) {
        *expected_file = string_create("expected_error.txt");
        return TEST_COMPILE_ERROR;
    }

    snprintf(path, sizeof(path), "%s/expected_output.txt", test_folder);
    if (file_exists(path)) {
        *expected_file = string_create("expected_output.txt");
        return TEST_RUN_OUTPUT;
    }

    snprintf(path, sizeof(path), "%s/expected_files.txt", test_folder);
    if (file_exists(path)) {
        *expected_file = string_create("expected_files.txt");
        return TEST_CHECK_FILES;
    }

    snprintf(path, sizeof(path), "%s/expected.txt", test_folder);
    if (file_exists(path)) {
        *expected_file = string_create("expected.txt");
        return TEST_COMPILE_ONLY;
    }

    *expected_file = string_create("");
    return TEST_UNKNOWN;
}

// Find source file in test folder (you can customize extensions)
String find_source_file(const char *test_folder) {
    const char *extensions[] = {".c", ".src", ".txt", NULL};
    char path[1024];

    DIR *dir = opendir(test_folder);
    if (!dir)
        return string_create("");

    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_name[0] == '.')
            continue;

        for (int i = 0; extensions[i] != NULL; i++) {
            if (strstr(entry->d_name, extensions[i])) {
                // Check it's not an expected file
                if (strncmp(entry->d_name, "expected", 8) != 0) {
                    closedir(dir);
                    return string_create(entry->d_name);
                }
            }
        }
    }

    closedir(dir);
    return string_create("");
}

// === Test Discovery ===

TestList *discover_tests(const char *integration_tests_dir) {
    TestList *list = test_list_create();
    if (!list)
        return NULL;

    DIR *dir = opendir(integration_tests_dir);
    if (!dir) {
        fprintf(stderr, "Failed to open directory: %s\n",
                integration_tests_dir);
        return list;
    }

    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
        // Skip hidden and parent directories
        if (entry->d_name[0] == '.')
            continue;

        char test_path[1024];
        snprintf(test_path, sizeof(test_path), "%s/%s", integration_tests_dir,
                 entry->d_name);

        if (!is_directory(test_path))
            continue;

        TestCase test;
        test.test_folder = string_create(test_path);
        test.type = detect_test_type(test_path, &test.expected_file);

        if (test.type != TEST_UNKNOWN) {
            test_list_append(list, test);
            printf("Discovered test: %s (type: %d)\n", test.test_folder.data,
                   test.type);
        } else {
            printf("Skipping %s: no expected file found\n", test_path);
            string_free(&test.test_folder);
            string_free(&test.expected_file);
        }
    }

    closedir(dir);
    return list;
}

// === Command Builder ===

typedef struct {
    String command;
    size_t capacity;
} CommandBuilder;

CommandBuilder *command_builder_create(void) {
    CommandBuilder *builder = malloc(sizeof(CommandBuilder));
    if (builder) {
        builder->capacity = 1024;
        builder->command.data = malloc(builder->capacity);
        builder->command.length = 0;
        if (builder->command.data) {
            builder->command.data[0] = '\0';
        }
    }
    return builder;
}

void command_builder_append(CommandBuilder *builder, const char *str) {
    size_t len = strlen(str);
    size_t new_length = builder->command.length + len;

    if (new_length >= builder->capacity) {
        builder->capacity = (new_length + 1) * 2;
        builder->command.data =
            realloc(builder->command.data, builder->capacity);
    }

    memcpy(builder->command.data + builder->command.length, str, len);
    builder->command.length = new_length;
    builder->command.data[new_length] = '\0';
}

void command_builder_append_string(CommandBuilder *builder, const String *str) {
    size_t new_length = builder->command.length + str->length;

    if (new_length >= builder->capacity) {
        builder->capacity = (new_length + 1) * 2;
        builder->command.data =
            realloc(builder->command.data, builder->capacity);
    }

    memcpy(builder->command.data + builder->command.length, str->data,
           str->length);
    builder->command.length = new_length;
    builder->command.data[new_length] = '\0';
}

void command_builder_reset(CommandBuilder *builder) {
    builder->command.length = 0;
    if (builder->command.data) {
        builder->command.data[0] = '\0';
    }
}

String command_builder_build(CommandBuilder *builder) {
    return string_create(builder->command.data);
}

void command_builder_free(CommandBuilder *builder) {
    if (builder) {
        if (builder->command.data) {
            free(builder->command.data);
        }
        free(builder);
    }
}

// Build compile command based on test type
String build_compile_command(const TestCase *test, const char *compiler_path) {
    CommandBuilder *builder = command_builder_create();

    // Find source file
    String source_file = find_source_file(test->test_folder.data);
    if (source_file.length == 0) {
        fprintf(stderr, "No source file found in %s\n", test->test_folder.data);
        command_builder_free(builder);
        return string_create("");
    }

    // Build command: compiler_path source_file
    command_builder_append(builder, compiler_path);
    command_builder_append(builder, " ");
    command_builder_append_string(builder, &test->test_folder);
    command_builder_append(builder, "/");
    command_builder_append_string(builder, &source_file);

    // Redirect output based on test type
    if (test->type == TEST_COMPILE_ERROR || test->type == TEST_COMPILE_ONLY) {
        command_builder_append(builder, " > ");
        command_builder_append_string(builder, &test->test_folder);
        command_builder_append(builder, "/compiler_output.txt 2>&1");
    } else if (test->type == TEST_RUN_OUTPUT ||
               test->type == TEST_CHECK_FILES) {
        command_builder_append(builder, " -o ");
        command_builder_append_string(builder, &test->test_folder);
        command_builder_append(builder, "/test_executable");
        command_builder_append(builder, " > ");
        command_builder_append_string(builder, &test->test_folder);
        command_builder_append(builder, "/compiler_output.txt 2>&1");
    }

    String result = command_builder_build(builder);
    string_free(&source_file);
    command_builder_free(builder);

    return result;
}

// Build run command for executable
String build_run_command(const TestCase *test) {
    CommandBuilder *builder = command_builder_create();

    command_builder_append_string(builder, &test->test_folder);
    command_builder_append(builder, "/test_executable > ");
    command_builder_append_string(builder, &test->test_folder);
    command_builder_append(builder, "/actual_output.txt 2>&1");

    String result = command_builder_build(builder);
    command_builder_free(builder);

    return result;
}

// === File Comparison ===

bool compare_files(const char *file1, const char *file2) {
    FILE *f1 = fopen(file1, "r");
    FILE *f2 = fopen(file2, "r");

    if (!f1 || !f2) {
        if (f1)
            fclose(f1);
        if (f2)
            fclose(f2);
        return false;
    }

    bool same = true;
    int ch1, ch2;

    while ((ch1 = fgetc(f1)) != EOF && (ch2 = fgetc(f2)) != EOF) {
        if (ch1 != ch2) {
            same = false;
            break;
        }
    }

    // Check if both reached EOF
    if (same && (fgetc(f1) != EOF || fgetc(f2) != EOF)) {
        same = false;
    }

    fclose(f1);
    fclose(f2);
    return same;
}

// Read expected files list from expected_files.txt
typedef struct {
    String *files;
    size_t count;
} FileList;

FileList read_expected_files(const char *expected_files_path) {
    FileList list = {NULL, 0};

    FILE *f = fopen(expected_files_path, "r");
    if (!f)
        return list;

    // Count lines first
    size_t capacity = 10;
    list.files = malloc(sizeof(String) * capacity);

    char line[512];
    while (fgets(line, sizeof(line), f)) {
        // Remove newline
        size_t len = strlen(line);
        while (len > 0 && (line[len - 1] == '\n' || line[len - 1] == '\r')) {
            line[--len] = '\0';
        }

        // Skip empty lines and comments
        if (len == 0 || line[0] == '#')
            continue;

        // Expand array if needed
        if (list.count >= capacity) {
            capacity *= 2;
            list.files = realloc(list.files, sizeof(String) * capacity);
        }

        list.files[list.count++] = string_create(line);
    }

    fclose(f);
    return list;
}

void file_list_free(FileList *list) {
    for (size_t i = 0; i < list->count; i++) {
        string_free(&list->files[i]);
    }
    free(list->files);
    list->files = NULL;
    list->count = 0;
}

// === Test Results ===

typedef enum { RESULT_PASS, RESULT_FAIL, RESULT_ERROR } TestResult;

typedef struct {
    TestResult result;
    String message;
    double duration; // in seconds
} TestRunResult;

TestRunResult test_run_result_create(TestResult result, const char *message) {
    TestRunResult r;
    r.result = result;
    r.message = string_create(message);
    r.duration = 0.0;
    return r;
}

void test_run_result_free(TestRunResult *result) {
    string_free(&result->message);
}

// === Test Execution ===

TestRunResult run_test_compile_only(const TestCase *test,
                                    const char *compiler_path) {
    // Build and execute compile command
    String compile_cmd = build_compile_command(test, compiler_path);
    printf("  Executing: %s\n", compile_cmd.data);

    int ret = system(compile_cmd.data);
    string_free(&compile_cmd);

    if (ret != 0) {
        return test_run_result_create(
            RESULT_FAIL, "Compilation failed (non-zero exit code)");
    }

    // Compare compiler output with expected
    char actual_path[1024], expected_path[1024];
    snprintf(actual_path, sizeof(actual_path), "%s/compiler_output.txt",
             test->test_folder.data);
    snprintf(expected_path, sizeof(expected_path), "%s/%s",
             test->test_folder.data, test->expected_file.data);

    if (!compare_files(actual_path, expected_path)) {
        return test_run_result_create(
            RESULT_FAIL, "Compiler output does not match expected output");
    }

    return test_run_result_create(RESULT_PASS, "Compilation output matches");
}

TestRunResult run_test_compile_error(const TestCase *test,
                                     const char *compiler_path) {
    String compile_cmd = build_compile_command(test, compiler_path);
    printf("  Executing: %s\n", compile_cmd.data);

    int ret = system(compile_cmd.data);
    string_free(&compile_cmd);

    if (ret == 0) {
        return test_run_result_create(
            RESULT_FAIL, "Expected compilation to fail, but it succeeded");
    }

    // Compare error output with expected
    char actual_path[1024], expected_path[1024];
    snprintf(actual_path, sizeof(actual_path), "%s/compiler_output.txt",
             test->test_folder.data);
    snprintf(expected_path, sizeof(expected_path), "%s/%s",
             test->test_folder.data, test->expected_file.data);

    if (!compare_files(actual_path, expected_path)) {
        return test_run_result_create(
            RESULT_FAIL, "Error message does not match expected error");
    }

    return test_run_result_create(RESULT_PASS, "Error output matches");
}

TestRunResult run_test_run_output(const TestCase *test,
                                  const char *compiler_path) {
    // Compile first
    String compile_cmd = build_compile_command(test, compiler_path);
    printf("  Compiling: %s\n", compile_cmd.data);

    int ret = system(compile_cmd.data);
    string_free(&compile_cmd);

    if (ret != 0) {
        return test_run_result_create(
            RESULT_FAIL, "Compilation failed (non-zero exit code)");
    }

    // Check if executable was created
    char exe_path[1024];
    snprintf(exe_path, sizeof(exe_path), "%s/test_executable",
             test->test_folder.data);

    if (!file_exists(exe_path)) {
        return test_run_result_create(RESULT_FAIL,
                                      "Executable was not created");
    }

    // Run the executable
    String run_cmd = build_run_command(test);
    printf("  Running: %s\n", run_cmd.data);

    ret = system(run_cmd.data);
    string_free(&run_cmd);

    if (ret != 0) {
        return test_run_result_create(RESULT_FAIL,
                                      "Executable failed (non-zero exit code)");
    }

    // Compare output
    char actual_path[1024], expected_path[1024];
    snprintf(actual_path, sizeof(actual_path), "%s/actual_output.txt",
             test->test_folder.data);
    snprintf(expected_path, sizeof(expected_path), "%s/%s",
             test->test_folder.data, test->expected_file.data);

    if (!compare_files(actual_path, expected_path)) {
        return test_run_result_create(
            RESULT_FAIL, "Executable output does not match expected output");
    }

    return test_run_result_create(RESULT_PASS,
                                  "Compilation succeeded and output matches");
}

TestRunResult run_test_check_files(const TestCase *test,
                                   const char *compiler_path) {
    // Compile first
    String compile_cmd = build_compile_command(test, compiler_path);
    printf("  Compiling: %s\n", compile_cmd.data);

    int ret = system(compile_cmd.data);
    string_free(&compile_cmd);

    if (ret != 0) {
        return test_run_result_create(
            RESULT_FAIL, "Compilation failed (non-zero exit code)");
    }

    // Read expected files list
    char expected_files_path[1024];
    snprintf(expected_files_path, sizeof(expected_files_path), "%s/%s",
             test->test_folder.data, test->expected_file.data);

    FileList expected_files = read_expected_files(expected_files_path);

    if (expected_files.count == 0) {
        file_list_free(&expected_files);
        return test_run_result_create(
            RESULT_ERROR, "No expected files listed in expected_files.txt");
    }

    // Check each expected file exists
    bool all_exist = true;
    char missing_file[512] = "";

    for (size_t i = 0; i < expected_files.count; i++) {
        char file_path[1024];
        snprintf(file_path, sizeof(file_path), "%s/%s", test->test_folder.data,
                 expected_files.files[i].data);

        if (!file_exists(file_path)) {
            all_exist = false;
            snprintf(missing_file, sizeof(missing_file), "Missing file: %s",
                     expected_files.files[i].data);
            break;
        }
    }

    file_list_free(&expected_files);

    if (!all_exist) {
        return test_run_result_create(RESULT_FAIL, missing_file);
    }

    return test_run_result_create(RESULT_PASS,
                                  "All expected files were created");
}

TestRunResult run_single_test(const TestCase *test, const char *compiler_path) {
    switch (test->type) {
    case TEST_COMPILE_ONLY:
        return run_test_compile_only(test, compiler_path);
    case TEST_COMPILE_ERROR:
        return run_test_compile_error(test, compiler_path);
    case TEST_RUN_OUTPUT:
        return run_test_run_output(test, compiler_path);
    case TEST_CHECK_FILES:
        return run_test_check_files(test, compiler_path);
    default:
        return test_run_result_create(RESULT_ERROR, "Unknown test type");
    }
}

// === Test Runner ===

typedef struct {
    size_t total;
    size_t passed;
    size_t failed;
    size_t errors;
} TestStatistics;

void print_test_result(const char *test_name, const TestRunResult *result) {
    const char *status_str;
    const char *color_code;

    switch (result->result) {
    case RESULT_PASS:
        status_str = "PASS";
        color_code = "\033[32m"; // Green
        break;
    case RESULT_FAIL:
        status_str = "FAIL";
        color_code = "\033[31m"; // Red
        break;
    case RESULT_ERROR:
        status_str = "ERROR";
        color_code = "\033[33m"; // Yellow
        break;
    default:
        status_str = "UNKNOWN";
        color_code = "\033[0m";
    }

    printf("%s[%s]\033[0m %s\n", color_code, status_str, test_name);

    if (result->message.length > 0 && result->result != RESULT_PASS) {
        printf("  %s\n", result->message.data);
    }
}

TestStatistics run_all_tests(TestList *tests, const char *compiler_path) {
    TestStatistics stats = {0, 0, 0, 0};

    printf("\n=== Running Integration Tests ===\n\n");

    TestNode *current = tests->head;
    while (current) {
        const TestCase *test = &current->test;

        // Extract test name from path
        const char *test_name = strrchr(test->test_folder.data, '/');
        test_name = test_name ? test_name + 1 : test->test_folder.data;

        printf("Running: %s\n", test_name);

        TestRunResult result = run_single_test(test, compiler_path);
        print_test_result(test_name, &result);

        stats.total++;
        switch (result.result) {
        case RESULT_PASS:
            stats.passed++;
            break;
        case RESULT_FAIL:
            stats.failed++;
            break;
        case RESULT_ERROR:
            stats.errors++;
            break;
        }

        test_run_result_free(&result);
        printf("\n");

        current = current->next;
    }

    // Print summary
    printf("=== Test Summary ===\n");
    printf("Total:  %zu\n", stats.total);
    printf("\033[32mPassed: %zu\033[0m\n", stats.passed);
    printf("\033[31mFailed: %zu\033[0m\n", stats.failed);
    printf("\033[33mErrors: %zu\033[0m\n", stats.errors);

    if (stats.total > 0) {
        double pass_rate = (double)stats.passed / stats.total * 100.0;
        printf("Pass rate: %.1f%%\n", pass_rate);
    }

    return stats;
}

#endif // INTEGRATION_TESTS_H
