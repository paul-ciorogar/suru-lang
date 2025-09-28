#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

// Test result enumeration
typedef enum {
    TEST_PASSED,
    TEST_FAILED,
    TEST_COMPILE_ERROR,
    TEST_RUNTIME_ERROR,
    TEST_TIMEOUT
} test_result_t;

// Test structure
typedef struct test {
    char *filename;
    char *other_files;
    char *executable_name;
    test_result_t result;
    double compile_time;
    double run_time;
    int exit_code;
    struct test *next;
} test_t;

// Test statistics
typedef struct {
    int total_tests;
    int passed;
    int failed;
    int compile_errors;
    int runtime_errors;
    int timeouts;
    double total_time;
} test_stats_t;

// Function prototypes
test_t *create_test_list(void);
void add_test(test_t **head, const char *filename, const char *other_files);
int compile_test(test_t *test);
int run_test(test_t *test);
void run_all_tests(test_t *head);
void print_summary(test_t *head);
void cleanup_tests(test_t *head);
double get_time_diff(struct timespec start, struct timespec end);
char *get_executable_name(const char *filename);

// Create an empty test list
test_t *create_test_list(void) { return NULL; }

// Add a test to the linked list
void add_test(test_t **head, const char *filename, const char *other_files) {
    test_t *new_test = malloc(sizeof(test_t));
    if (!new_test) {
        fprintf(stderr, "Memory allocation failed\n");
        return;
    }

    // Copy filename
    new_test->filename = malloc(strlen(filename) + 1);
    strcpy(new_test->filename, filename);

    // Copy other files (can be NULL)
    if (other_files && strlen(other_files) > 0) {
        new_test->other_files = malloc(strlen(other_files) + 1);
        strcpy(new_test->other_files, other_files);
    } else {
        new_test->other_files = NULL;
    }

    // Generate executable name
    new_test->executable_name = get_executable_name(filename);

    // Initialize other fields
    new_test->result = TEST_FAILED;
    new_test->compile_time = 0.0;
    new_test->run_time = 0.0;
    new_test->exit_code = -1;
    new_test->next = *head;

    *head = new_test;

    printf("Added test: %s", filename);
    if (other_files) {
        printf(" (with: %s)", other_files);
    }
    printf("\n");
}

// Generate executable name from source filename
char *get_executable_name(const char *filename) {
    char *exec_name = malloc(strlen(filename) + 10);
    strcpy(exec_name, filename);

    // Remove .c extension if present
    char *dot = strrchr(exec_name, '.');
    if (dot && strcmp(dot, ".c") == 0) {
        *dot = '\0';
    }

    // Add .out extension
    strcat(exec_name, ".out");
    return exec_name;
}

// Compile a single test
int compile_test(test_t *test) {
    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);

    // Build compile command
    char compile_cmd[1024];
    snprintf(compile_cmd, sizeof(compile_cmd), "gcc -o %s %s",
             test->executable_name, test->filename);

    if (test->other_files) {
        strcat(compile_cmd, " ");
        strcat(compile_cmd, test->other_files);
    }

    // Add some common flags
    strcat(compile_cmd, " -Wall -Wextra -std=c99");

    printf("Compiling: %s\n", compile_cmd);

    int result = system(compile_cmd);

    clock_gettime(CLOCK_MONOTONIC, &end);
    test->compile_time = get_time_diff(start, end);

    if (result == 0) {
        printf("✓ Compilation successful (%.3fs)\n", test->compile_time);
        return 1;
    } else {
        printf("✗ Compilation failed (%.3fs)\n", test->compile_time);
        test->result = TEST_COMPILE_ERROR;
        return 0;
    }
}

// Run a single test
int run_test(test_t *test) {
    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);

    char run_cmd[512];
    snprintf(run_cmd, sizeof(run_cmd), "./%s", test->executable_name);

    printf("Running: %s\n", run_cmd);

    pid_t pid = fork();
    if (pid == 0) {
        // Child process
        execl("/bin/sh", "sh", "-c", run_cmd, NULL);
        exit(127); // execl failed
    } else if (pid > 0) {
        // Parent process
        int status;
        int result = waitpid(pid, &status, 0);

        clock_gettime(CLOCK_MONOTONIC, &end);
        test->run_time = get_time_diff(start, end);

        if (result == -1) {
            test->result = TEST_RUNTIME_ERROR;
            printf("✗ Runtime error (%.3fs)\n", test->run_time);
            return 0;
        }

        test->exit_code = WEXITSTATUS(status);

        if (test->exit_code == 0) {
            test->result = TEST_PASSED;
            printf("✓ Test passed (%.3fs)\n", test->run_time);
            return 1;
        } else {
            test->result = TEST_FAILED;
            printf("✗ Test failed with exit code %d (%.3fs)\n", test->exit_code,
                   test->run_time);
            return 0;
        }
    } else {
        // Fork failed
        test->result = TEST_RUNTIME_ERROR;
        printf("✗ Failed to create process\n");
        return 0;
    }
}

// Run all tests in the list
void run_all_tests(test_t *head) {
    printf("\n=== Starting Test Suite ===\n\n");

    test_t *current = head;
    int test_number = 1;

    while (current != NULL) {
        printf("Test %d: %s\n", test_number, current->filename);
        printf("----------------------------------------\n");

        // Try to compile
        if (compile_test(current)) {
            // If compilation successful, run the test
            run_test(current);

            // Clean up executable
            char rm_cmd[256];
            snprintf(rm_cmd, sizeof(rm_cmd), "rm -f %s",
                     current->executable_name);
            system(rm_cmd);
        }

        printf("\n");
        current = current->next;
        test_number++;
    }
}

// Print test summary
void print_summary(test_t *head) {
    test_stats_t stats = {0};
    test_t *current = head;

    // Calculate statistics
    while (current != NULL) {
        stats.total_tests++;
        stats.total_time += current->compile_time + current->run_time;

        switch (current->result) {
        case TEST_PASSED:
            stats.passed++;
            break;
        case TEST_FAILED:
            stats.failed++;
            break;
        case TEST_COMPILE_ERROR:
            stats.compile_errors++;
            break;
        case TEST_RUNTIME_ERROR:
            stats.runtime_errors++;
            break;
        case TEST_TIMEOUT:
            stats.timeouts++;
            break;
        }
        current = current->next;
    }

    printf("=== Test Summary ===\n");
    printf("Total tests:      %d\n", stats.total_tests);
    printf("Passed:           %d (%.1f%%)\n", stats.passed,
           stats.total_tests ? (100.0 * stats.passed / stats.total_tests)
                             : 0.0);
    printf("Failed:           %d (%.1f%%)\n", stats.failed,
           stats.total_tests ? (100.0 * stats.failed / stats.total_tests)
                             : 0.0);
    printf("Compile errors:   %d (%.1f%%)\n", stats.compile_errors,
           stats.total_tests
               ? (100.0 * stats.compile_errors / stats.total_tests)
               : 0.0);
    printf("Runtime errors:   %d (%.1f%%)\n", stats.runtime_errors,
           stats.total_tests
               ? (100.0 * stats.runtime_errors / stats.total_tests)
               : 0.0);
    printf("Total time:       %.3fs\n", stats.total_time);
    printf("Average per test: %.3fs\n",
           stats.total_tests ? (stats.total_time / stats.total_tests) : 0.0);

    // Detailed results
    printf("\n=== Detailed Results ===\n");
    current = head;
    int test_num = 1;
    while (current != NULL) {
        const char *result_str;
        switch (current->result) {
        case TEST_PASSED:
            result_str = "PASSED";
            break;
        case TEST_FAILED:
            result_str = "FAILED";
            break;
        case TEST_COMPILE_ERROR:
            result_str = "COMPILE_ERROR";
            break;
        case TEST_RUNTIME_ERROR:
            result_str = "RUNTIME_ERROR";
            break;
        case TEST_TIMEOUT:
            result_str = "TIMEOUT";
            break;
        default:
            result_str = "UNKNOWN";
            break;
        }

        printf("Test %d: %-20s %s (compile: %.3fs, run: %.3fs)\n", test_num,
               current->filename, result_str, current->compile_time,
               current->run_time);

        current = current->next;
        test_num++;
    }
}

// Clean up memory
void cleanup_tests(test_t *head) {
    test_t *current = head;
    while (current != NULL) {
        test_t *next = current->next;
        free(current->filename);
        free(current->other_files);
        free(current->executable_name);
        free(current);
        current = next;
    }
}

// Calculate time difference in seconds
double get_time_diff(struct timespec start, struct timespec end) {
    return (end.tv_sec - start.tv_sec) +
           (end.tv_nsec - start.tv_nsec) / 1000000000.0;
}

// Example usage
int main() {
    // Create test list
    test_t *tests = create_test_list();

    // Add some example tests
    add_test(&tests, "tests/arena_test.c", NULL);

    // Run all tests
    run_all_tests(tests);

    // Print summary
    print_summary(tests);

    // Clean up
    cleanup_tests(tests);

    return 0;
}
