#ifndef LOG_H
#define LOG_H

#include <stdio.h>

// Log levels
typedef enum {
    LOG_LEVEL_DEBUG,
    LOG_LEVEL_INFO,
    LOG_LEVEL_ERROR,
    LOG_LEVEL_NONE
} LogLevel;

// Initialize logging system (reads environment variables)
void log_init(void);

// Close logging system
void log_close(void);

// Log functions
void log_debug(const char *format, ...);
void log_info(const char *format, ...);
void log_error(const char *format, ...);

#endif // LOG_H
