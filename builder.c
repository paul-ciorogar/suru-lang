#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>

// Configuration
#define MAX_PATH 512

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
int file_exists(const char *filename) {
    struct stat buffer;
    return (stat(filename, &buffer) == 0);
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
    snprintf(output_path, sizeof(output_path), "%s/%s", config->build_directory,
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

// Calculate required command buffer size
size_t calculate_command_size(BuildConfig *config) {
    size_t total_size = 0;

    // Compiler name + space
    total_size += strlen(config->compiler) + 1;

    // Flags + space
    if (config->is_production) {
        total_size += strlen("-Wall -Wextra -std=c99 -O2 -DNDEBUG") + 1;
    } else {
        total_size += strlen("-Wall -Wextra -std=c99 -g -O0") + 1;
    }

    // Source files + spaces
    SourceFile *current = config->source_files;
    while (current) {
        total_size += strlen(current->filename) + 1; // +1 for space
        current = current->next;
    }

    // Output flag and filename
    total_size += strlen("-o /") + strlen(config->build_directory) +
                  strlen(config->output_name);

    // Null terminator
    total_size += 1;

    // Add small safety margin (10 bytes)
    total_size += 10;

    return total_size;
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
    if (create_directory("tmpbuild") != 0) {
        printf("Error: Failed to create build directory\n");
        return 1;
    }

    if (!needs_rebuild(config)) {
        return 0; // Already up to date
    }

    // Calculate required buffer size and allocate dynamically
    size_t cmd_size = calculate_command_size(config);
    char *command = calloc(cmd_size, sizeof(char));
    if (!command) {
        printf("Error: Memory allocation failed for command buffer\n");
        return 1;
    }

    // printf("Allocated command buffer: %zu bytes\n", cmd_size);

    // Start building the command
    strcat(command, config->compiler);
    strcat(command, " ");

    // Add flags based on build type
    if (config->is_production) {
        strcat(command, "-Wall -Wextra -std=c99 -O2 -DNDEBUG");
    } else {
        strcat(command, "-Wall -Wextra -std=c99 -g -O0");
    }

    strcat(command, " ");

    // Add source files
    SourceFile *current = config->source_files;
    while (current) {
        strcat(command, current->filename);
        strcat(command, " ");
        current = current->next;
    }

    // Add output (in build directory)
    strcat(command, "-o ");
    strcat(command, config->build_directory);
    strcat(command, "/");
    strcat(command, config->output_name);

    int result = execute_command(command);
    free(command);
    return result;
}

// Clean build artifacts
int clean_project(BuildConfig *config) {
    printf("Cleaning project...\n");
    char output_path[MAX_PATH];
    snprintf(output_path, sizeof(output_path), "build/%s", config->output_name);

    if (file_exists(output_path)) {
        char cmd[MAX_PATH + 20];
        snprintf(cmd, sizeof(cmd), "rm -f %s", output_path);
        return execute_command(cmd);
    }
    printf("Nothing to clean.\n");
    return 0;
}

// Auto-discover C files in src directory
void discover_source_files(BuildConfig *config) {
    DIR *dir = opendir("src");
    struct dirent *entry;

    if (dir == NULL) {
        printf("Error: Cannot open src directory\n");
        printf("Make sure you have a 'src' folder with your C source files.\n");
        return;
    }

    printf("Auto-discovering C source files in src/...\n");

    while ((entry = readdir(dir)) != NULL) {
        char *ext = strrchr(entry->d_name, '.');
        if (ext && strcmp(ext, ".c") == 0) {
            // Build full path: src/filename.c
            char full_path[MAX_PATH];
            snprintf(full_path, sizeof(full_path), "src/%s", entry->d_name);

            add_source_file(config, full_path);
            printf("  Found: %s\n", full_path);
        }
    }

    closedir(dir);
}

// Initialize default configuration
void init_config(BuildConfig *config, int is_production) {
    memset(config, 0, sizeof(BuildConfig));
    config->compiler = "gcc";
    config->output_name = "suru";
    config->build_directory = "tmpbuild";
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

int main(int argc, char *argv[]) {
    BuildConfig config;

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
    printf("  Flags: %s\n", is_production
                                ? "-Wall -Wextra -std=c99 -O2 -DNDEBUG"
                                : "-Wall -Wextra -std=c99 -g -O0");
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

    if (result == 0) {
        printf("Build completed successfully!\n");
    } else {
        printf("Build failed with exit code %d\n", result);
    }

    return result;
}
