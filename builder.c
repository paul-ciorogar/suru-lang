#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

// Configuration

// used for self rebuild
#define SOURCE_FILE "builder.c"
#define TARGET_EXECUTABLE "builder"
#define BUILD_DIRECTORY "tmpbuild"
#define OUTPUT_NAME "suru"

#define INTEGRATION_TEST_DIR "integration_tests"

#define EXPECTED_FILE "expected.txt"
#define COMMAND_FILE "command.txt"

#define DEBUG_FLAGS "-Wall -Wextra -std=c99 -O2 -DNDEBUG"
#define PRODUCTION_FLAGS "-Wall -Wextra -std=c99 -g -O0"

#define MAX_PATH 512
#define MAX_CMD 512

typedef struct {
    char *data;
    size_t length;
    size_t capacity;
} StringBuilder;

// Initialize a new string builder
StringBuilder *sb_create(size_t initial_capacity) {
    StringBuilder *sb = (StringBuilder *)malloc(sizeof(StringBuilder));
    if (!sb)
        return NULL;

    sb->capacity = initial_capacity > 0 ? initial_capacity : 16;
    sb->data = (char *)malloc(sb->capacity);
    if (!sb->data) {
        free(sb);
        return NULL;
    }

    sb->data[0] = '\0';
    sb->length = 0;
    return sb;
}

void sb_reset(StringBuilder *sb) {
    sb->data[0] = '\0';
    sb->length = 0;
}

// Append a string to the string builder
int sb_append(StringBuilder *sb, const char *str) {
    if (!sb || !str)
        return -1;

    size_t str_len = strlen(str);
    size_t required = sb->length + str_len + 1;

    // Resize if needed
    if (required > sb->capacity) {
        size_t new_capacity = sb->capacity;
        while (new_capacity < required) {
            new_capacity *= 2;
        }

        char *new_data = (char *)realloc(sb->data, new_capacity);
        if (!new_data)
            return -1;

        sb->data = new_data;
        sb->capacity = new_capacity;
    }

    memcpy(sb->data + sb->length, str, str_len);
    sb->length += str_len;
    sb->data[sb->length] = '\0';

    return 0;
}

// Free the string builder
void sb_free(StringBuilder *sb) {
    if (sb) {
        free(sb->data);
        free(sb);
    }
}

// Copy contents to a buffer
int sb_copy_to_buffer(const StringBuilder *sb, char *buffer,
                      size_t buffer_size) {
    if (!sb || !buffer || buffer_size == 0)
        return -1;

    size_t copy_len = buffer_size < sb->length ? buffer_size : sb->length;
    memcpy(buffer, sb->data, copy_len);
    buffer[copy_len] = '\0';

    return copy_len;
}

// Linked list node for source files
typedef struct SourceFile {
    char *filename;
    struct SourceFile *next;
} SourceFile;

typedef struct {
    SourceFile *source_files;
    int file_count;
    char *output_name;
    char *build_directory;
    char *compiler;
    int is_production;
} BuildConfig;

// Function to check if file exists
int file_exists(const char *path) {
    struct stat st;
    return stat(path, &st) == 0 && S_ISREG(st.st_mode);
}

int is_directory(const char *path) {
    struct stat st;
    return stat(path, &st) == 0 && S_ISDIR(st.st_mode);
}

// Get file modification time
time_t get_file_time(const char *filename) {
    struct stat buffer;
    if (stat(filename, &buffer) == 0) {
        return buffer.st_mtime;
    }
    return 0;
}

// Add source file to linked list
void add_source_file(BuildConfig *config, const char *filename) {
    SourceFile *new_file = malloc(sizeof(SourceFile));
    if (!new_file) {
        printf("Error: Memory allocation failed\n");
        return;
    }

    new_file->filename = malloc(strlen(filename) + 1);
    if (!new_file->filename) {
        printf("Error: Memory allocation failed\n");
        free(new_file);
        return;
    }

    strcpy(new_file->filename, filename);
    new_file->next = config->source_files;
    config->source_files = new_file;
    config->file_count++;
}

// Free source files linked list
void free_source_files(BuildConfig *config) {
    SourceFile *current = config->source_files;
    while (current) {
        SourceFile *next = current->next;
        free(current->filename);
        free(current);
        current = next;
    }
    config->source_files = NULL;
    config->file_count = 0;
}

// Check if source files are newer than output
int needs_rebuild(BuildConfig *config) {
    char output_path[MAX_PATH];
    snprintf(output_path, MAX_PATH, "%s/%s", config->build_directory,
             config->output_name);

    if (!file_exists(output_path)) {
        printf("Output file doesn't exist, building...\n");
        return 1;
    }

    time_t output_time = get_file_time(output_path);

    SourceFile *current = config->source_files;
    while (current) {
        time_t source_time = get_file_time(current->filename);
        if (source_time > output_time) {
            printf("Source file %s is newer, rebuilding...\n",
                   current->filename);
            return 1;
        }
        current = current->next;
    }

    printf("Output is up to date.\n");
    return 0;
}

// Execute command and return exit code
int execute_command(const char *cmd) {
    printf("Executing: %s\n", cmd);
    return system(cmd);
}

// Create directory if it doesn't exist
int create_directory(const char *path) {
    struct stat st = {0};
    if (stat(path, &st) == -1) {
        printf("Creating directory: %s\n", path);
#ifdef _WIN32
        return mkdir(path);
#else
        return mkdir(path, 0755);
#endif
    }
    return 0; // Directory already exists
}

// Build the project
int build_project(BuildConfig *config) {
    // Create build directory if it doesn't exist
    if (create_directory(BUILD_DIRECTORY) != 0) {
        printf("Error: Failed to create build directory\n");
        return 1;
    }

    if (!needs_rebuild(config)) {
        return 0; // Already up to date
    }

    // Start building the command
    StringBuilder *s = sb_create(MAX_CMD);

    sb_append(s, config->compiler);
    sb_append(s, " ");

    // Add flags based on build type
    if (config->is_production) {
        sb_append(s, DEBUG_FLAGS);
    } else {
        sb_append(s, PRODUCTION_FLAGS);
    }

    sb_append(s, " ");

    // Add source files
    SourceFile *current = config->source_files;
    while (current) {
        sb_append(s, current->filename);
        sb_append(s, " ");
        current = current->next;
    }

    // Add output (in build directory)
    sb_append(s, "-o ");
    sb_append(s, config->build_directory);
    sb_append(s, "/");
    sb_append(s, config->output_name);

    int result = execute_command(s->data);
    sb_free(s);

    if (result > 0) {
        printf("Build completed successfully!\n");
    }

    return result;
}

// Clean build artifacts
int clean_project(BuildConfig *config) {
    printf("Cleaning project...\n");
    char output_path[MAX_PATH];
    snprintf(output_path, sizeof(output_path), "%s/%s", BUILD_DIRECTORY,
             config->output_name);

    if (file_exists(output_path)) {
        char cmd[MAX_CMD + MAX_PATH];
        snprintf(cmd, MAX_CMD + MAX_PATH, "rm -f %s", output_path);
        return execute_command(cmd);
    }
    printf("Nothing to clean.\n");
    return 0;
}

// Discover C files in a given directory
void discover_files_in_directory(BuildConfig *config, const char *dir_path) {
    DIR *dir = opendir(dir_path);
    struct dirent *entry;

    if (dir == NULL) {
        printf("Warning: Cannot open directory %s (skipping)\n", dir_path);
        return;
    }

    printf("Auto-discovering C source files in %s/...\n", dir_path);

    while ((entry = readdir(dir)) != NULL) {
        char *ext = strrchr(entry->d_name, '.');
        if (ext && strcmp(ext, ".c") == 0) {
            // Build full path: dir_path/filename.c
            char full_path[MAX_PATH];
            snprintf(full_path, MAX_PATH, "%s/%s", dir_path, entry->d_name);

            add_source_file(config, full_path);
            printf("  Found: %s\n", full_path);
        }
    }

    closedir(dir);
}

// Auto-discover C files in src and src/lsp directories
void discover_source_files(BuildConfig *config) {
    discover_files_in_directory(config, "src");
    discover_files_in_directory(config, "src/lsp");
}

// Initialize default configuration
void init_config(BuildConfig *config, int is_production) {
    memset(config, 0, sizeof(BuildConfig));
    config->compiler = "gcc";
    config->output_name = "suru";
    config->build_directory = BUILD_DIRECTORY;
    config->is_production = is_production;
    config->source_files = NULL;
    config->file_count = 0;
}

// Print usage information
void print_usage(const char *program_name) {
    printf("Usage: %s [options]\n", program_name);
    printf("Options:\n");
    printf("  build       - Build for development (debug mode, asserts "
           "enabled)\n");
    printf(
        "  build-prod  - Build for production (optimized, asserts disabled)\n");
    printf("  clean       - Clean build artifacts\n");
    printf("  rebuild     - Clean and build (development)\n");
    printf("  rebuild-prod- Clean and build (production)\n");
    printf("  --help      - Show this help\n");
}

// Rebuild the test runner executable and restart
void rebuild_self(const char *source_file, const char *target_exe,
                  char *argv[]) {
    printf("Rebuilding test runner...\n");

    char compile_cmd[MAX_CMD];
    snprintf(compile_cmd, MAX_CMD, "gcc -o %s %s -Wall -Wextra -std=c99 ",
             target_exe, source_file);

    printf("Executing: %s\n", compile_cmd);

    int result = system(compile_cmd);

    if (result == 0) {
        printf("Test runner rebuilt successfully\n");
        printf("Restarting with new executable...\n\n");

        // Execute the new version
        execv(target_exe, argv);

        // If we get here, execv failed
        perror("Failed to restart with new executable");
        return;
    } else {
        printf("Failed to rebuild test runner\n");
        return;
    }
}

// Get file modification time
time_t get_file_mtime(const char *filename) {
    struct stat file_stat;
    if (stat(filename, &file_stat) == 0) {
        return file_stat.st_mtime;
    }
    return 0;
}

// Check if self needs rebuilding and do it if necessary
void check_and_rebuild_self(const char *source_file, const char *target,
                            char *argv[]) {

    // Check if source file exists
    if (!file_exists(source_file)) {
        printf("Source file '%s' not found, skipping self-rebuild check\n",
               source_file);
        return; // Continue anyway
    }

    // Check if target executable exists
    if (!file_exists(target)) {
        printf("Target executable '%s' not found, rebuilding...\n", target);
        rebuild_self(source_file, target, argv);
        return;
    }

    // Compare modification times
    time_t source_mtime = get_file_mtime(source_file);
    time_t target_mtime = get_file_mtime(target);

    if (source_mtime > target_mtime) {
        printf("Source file is newer than executable, rebuilding...\n");
        return rebuild_self(source_file, target, argv);
    }

    printf("Test runner is up to date\n");
    return;
}

// Integration tests

// Test case structure
typedef struct {
    char *test_folder;
    char *command;
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

// Test list operations
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

// Read command from command.txt file
char *read_command_file(const char *test_folder) {
    char path[MAX_PATH];
    snprintf(path, MAX_PATH, "%s/%s", test_folder, COMMAND_FILE);

    FILE *f = fopen(path, "r");
    if (!f) {
        return NULL;
    }

    // Read the first line (command)
    char buffer[MAX_CMD];
    if (!fgets(buffer, MAX_CMD, f)) {
        fclose(f);
        return NULL;
    }

    fclose(f);

    // Remove trailing newline if present
    size_t len = strlen(buffer);
    if (len > 0 && buffer[len - 1] == '\n') {
        buffer[len - 1] = '\0';
    }

    // Allocate and return the command string
    char *command = malloc(strlen(buffer) + 1);
    if (command) {
        strcpy(command, buffer);
    }

    return command;
}

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

    StringBuilder *s = sb_create(MAX_PATH);
    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
        sb_reset(s);

        // Skip hidden and parent directories
        if (entry->d_name[0] == '.')
            continue;

        sb_append(s, integration_tests_dir);
        sb_append(s, "/");
        sb_append(s, entry->d_name);

        if (!is_directory(s->data))
            continue;

        TestCase test;
        test.test_folder = malloc(s->length + 1);
        printf("%s\n", s->data);
        sb_copy_to_buffer(s, test.test_folder, s->length);
        printf("%s\n", test.test_folder);

        test.command = read_command_file(test.test_folder);

        if (test.command != NULL) {
            test_list_append(list, test);
            printf("Discovered test: %s (command: %s)\n", test.test_folder,
                   test.command);
        } else {
            printf("Skipping %s: no command.txt found\n", test.test_folder);
        }
    }

    closedir(dir);
    sb_free(s);
    return list;
}

char *build_compile_command(const TestCase *test) {
    StringBuilder *s = sb_create(512);

    // Build command from test.command
    sb_append(s, BUILD_DIRECTORY);
    sb_append(s, "/");
    sb_append(s, OUTPUT_NAME);
    sb_append(s, " ");
    sb_append(s, test->command);
    sb_append(s, " > ");
    sb_append(s, test->test_folder);
    sb_append(s, "/output.txt 2>&1");

    char *result = malloc(s->length + 1);
    sb_copy_to_buffer(s, result, s->length);
    sb_free(s);

    return result;
}

int compare_files(const char *file1, const char *file2) {
    FILE *f1 = fopen(file1, "r");
    FILE *f2 = fopen(file2, "r");

    if (!f1 || !f2) {
        if (f1)
            fclose(f1);
        if (f2)
            fclose(f2);
        return 0;
    }

    int same = 1;
    int ch1, ch2;

    while ((ch1 = fgetc(f1)) != EOF && (ch2 = fgetc(f2)) != EOF) {
        if (ch1 != ch2) {
            same = 0;
            break;
        }
    }

    // Check if both reached EOF
    if (same && (fgetc(f1) != EOF || fgetc(f2) != EOF)) {
        same = 0;
    }

    fclose(f1);
    fclose(f2);
    return same;
}

int compare_test_output(const TestCase *test) {
    char actual_path[MAX_PATH], expected_path[MAX_PATH];
    snprintf(actual_path, MAX_PATH, "%s/output.txt", test->test_folder);
    snprintf(expected_path, MAX_PATH, "%s/%s", test->test_folder,
             EXPECTED_FILE);

    if (!compare_files(actual_path, expected_path)) {
        printf("Fail: actual does not match expected\n");
        return 0;
    }

    return 1;
}

int run_single_test(const TestCase *test) {
    char *compile_cmd = build_compile_command(test);
    printf("  Executing: %s\n", compile_cmd);

    int ret = system(compile_cmd);

    free(compile_cmd);

    // Don't fail on non-zero exit code - some tests expect errors
    // Instead, just compare the output
    (void)ret;  // Unused variable

    return compare_test_output(test);
}

int run_all_tests(TestList *tests) {
    printf("\n=== Running Integration Tests ===\n\n");

    TestNode *current = tests->head;
    while (current) {
        const TestCase *test = &current->test;

        // Extract test name from path
        const char *test_name = strrchr(test->test_folder, '/');
        test_name = test_name ? test_name + 1 : test->test_folder;

        printf("Running: %s\n", test_name);

        int success = run_single_test(test);

        if (!success) {
            return 0;
        }

        current = current->next;
    }

    return 1;
}

int run_integration_tests() {
    printf("Running integration tests!\n");

    // Discover all tests
    TestList *tests = discover_tests(INTEGRATION_TEST_DIR);

    if (!tests || tests->count == 0) {
        fprintf(stderr, "No tests found in %s\n", INTEGRATION_TEST_DIR);
        return 0;
    }

    printf("Found %zu test(s)\n\n", tests->count);

    // Run all tests
    return run_all_tests(tests);
}

int main(int argc, char *argv[]) {
    BuildConfig config;

    const char *source_file = SOURCE_FILE;
    const char *target_executable = TARGET_EXECUTABLE;

    check_and_rebuild_self(source_file, target_executable, argv);

    // Parse command line arguments
    char *action = "build";
    int is_production = 0;

    if (argc > 1) {
        action = argv[1];
    }

    if (strcmp(action, "--help") == 0) {
        print_usage(argv[0]);
        return 0;
    }

    // Determine build type
    if (strcmp(action, "build-prod") == 0 ||
        strcmp(action, "rebuild-prod") == 0) {
        is_production = 1;
    }

    init_config(&config, is_production);

    printf("Build mode: %s\n", is_production ? "Production" : "Development");

    // Auto-discover source files
    discover_source_files(&config);

    if (config.file_count == 0) {
        printf("Error: No C source files found in src/ directory!\n");
        return 1;
    }

    printf("Configuration:\n");
    printf("  Compiler: %s\n", config.compiler);
    printf("  Flags: %s\n", is_production ? DEBUG_FLAGS : PRODUCTION_FLAGS);
    printf("  Output: %s/%s\n", config.build_directory, config.output_name);
    printf("  Source files: %d\n", config.file_count);

    int result = 0;

    if (strcmp(action, "clean") == 0) {
        result = clean_project(&config);
    } else if (strcmp(action, "rebuild") == 0) {
        clean_project(&config);
        result = build_project(&config);
    } else if (strcmp(action, "rebuild-prod") == 0) {
        clean_project(&config);
        result = build_project(&config);
    } else {
        result = build_project(&config);
    }

    // Cleanup allocated memory
    free_source_files(&config);

    if (result > 0) {
        printf("Build failed with exit code %d\n", result);
        return result;
    }

    result = run_integration_tests();

    return result;
}
