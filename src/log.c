#include "log.h"
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static FILE *log_file = NULL;
static LogLevel log_level = LOG_LEVEL_NONE;

// Initialize logging system
void log_init(void) {
    // Check for log file path environment variable
    const char *log_path = getenv("SURU_LOG_FILE");
    if (log_path) {
        log_file = fopen(log_path, "a");
        if (!log_file) {
            fprintf(stderr, "Warning: Failed to open log file: %s\n", log_path);
        }
    }

    // Check for log level environment variable
    const char *level_str = getenv("SURU_LOG_LEVEL");
    if (level_str) {
        if (strcmp(level_str, "DEBUG") == 0) {
            log_level = LOG_LEVEL_DEBUG;
        } else if (strcmp(level_str, "INFO") == 0) {
            log_level = LOG_LEVEL_INFO;
        } else if (strcmp(level_str, "ERROR") == 0) {
            log_level = LOG_LEVEL_ERROR;
        } else {
            log_level = LOG_LEVEL_NONE;
        }
    }
}

// Close logging system
void log_close(void) {
    if (log_file) {
        fclose(log_file);
        log_file = NULL;
    }
}

// Helper function to get current timestamp
static void get_timestamp(char *buffer, size_t size) {
    time_t now = time(NULL);
    struct tm *t = localtime(&now);
    strftime(buffer, size, "%Y-%m-%d %H:%M:%S", t);
}

// Helper function to log with level
static void log_with_level(LogLevel level, const char *level_str, const char *format, va_list args) {
    if (level < log_level || !log_file) {
        return;
    }

    char timestamp[64];
    get_timestamp(timestamp, sizeof(timestamp));

    fprintf(log_file, "[%s] [%s] ", timestamp, level_str);
    vfprintf(log_file, format, args);
    fprintf(log_file, "\n");
    fflush(log_file);
}

// Log functions
void log_debug(const char *format, ...) {
    va_list args;
    va_start(args, format);
    log_with_level(LOG_LEVEL_DEBUG, "DEBUG", format, args);
    va_end(args);
}

void log_info(const char *format, ...) {
    va_list args;
    va_start(args, format);
    log_with_level(LOG_LEVEL_INFO, "INFO", format, args);
    va_end(args);
}

void log_error(const char *format, ...) {
    va_list args;
    va_start(args, format);
    log_with_level(LOG_LEVEL_ERROR, "ERROR", format, args);
    va_end(args);
}
