#include "integration_tests.h"

int main(int argc, char **argv) {
    const char *compiler_path = "./your_compiler";  // Path to your compiler
    const char *tests_dir = "integration_tests";     // Tests directory
    
    // Allow overriding compiler path via command line
    if (argc > 1) {
        compiler_path = argv[1];
    }
    
    printf("Integration Test Runner\n");
    printf("Compiler: %s\n", compiler_path);
    printf("Tests directory: %s\n\n", tests_dir);
    
    // Check if compiler exists
    if (!file_exists(compiler_path)) {
        fprintf(stderr, "Error: Compiler not found at %s\n", compiler_path);
        fprintf(stderr, "Please build the compiler first.\n");
        return 1;
    }
    
    // Discover all tests
    TestList *tests = discover_tests(tests_dir);
    
    if (!tests || tests->count == 0) {
        fprintf(stderr, "No tests found in %s\n", tests_dir);
        if (tests) test_list_free(tests);
        return 1;
    }
    
    printf("Found %zu test(s)\n\n", tests->count);
    
    // Run all tests
    TestStatistics stats = run_all_tests(tests, compiler_path);
    
    // Cleanup
    test_list_free(tests);
    
    // Return non-zero if any tests failed
    return (stats.failed > 0 || stats.errors > 0) ? 1 : 0;
}

/*
 * Example directory structure:
 * 
 * integration_tests/
 * ├── test_hello_world/
 * │   ├── hello.c
 * │   └── expected_output.txt
 * ├── test_syntax_error/
 * │   ├── bad_syntax.c
 * │   └── expected_error.txt
 * ├── test_codegen/
 * │   ├── arithmetic.c
 * │   └── expected_output.txt
 * └── test_file_generation/
 *     ├── program.c
 *     └── expected_files.txt
 * 
 * Running:
 *   ./test_runner ./my_compiler
 * 
 * Or integrate into your builder:
 *   - Add discover_tests() call
 *   - Add run_all_tests() call
 *   - Check return value to fail build if tests fail
 */