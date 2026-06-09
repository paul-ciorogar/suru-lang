/* fileio.c — File I/O runtime helpers for Suru.
 *
 * Replaces the hand-written LLVM IR emission in irFileioCodegen.suru (Phase C).
 * Each function accepts SuruString* values directly; `path->data` is a valid
 * NUL-terminated char* for both tag=6 heap strings and tag=8 static strings.
 *
 * Declared in user modules via:
 *   declare ptr  @suru_readfile(ptr)
 *   declare void @suru_writefile(ptr, ptr)
 *   declare void @suru_appendtofile(ptr, ptr)
 */

#include "suru_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern SuruString *suru_string_create(char *data, int64_t len);

/* suru_readfile — read an entire file into a new Suru String.
 *
 * Algorithm: fopen("r") → fseek(SEEK_END) → ftell → rewind →
 *            malloc(size+1) → fread → NUL-terminate → fclose →
 *            suru_string_create(buf, size).
 */
SuruString *suru_readfile(SuruString *path) {
    FILE *f = fopen(path->data, "r");
    fseek(f, 0, SEEK_END);
    int64_t size = (int64_t)ftell(f);
    rewind(f);
    char *buf = malloc((size_t)(size + 1));
    fread(buf, 1, (size_t)size, f);
    buf[size] = '\0';
    fclose(f);
    return suru_string_create(buf, size);
}

/* suru_writefile — write (truncating) content to a file.
 *
 * Algorithm: fopen("w") → fwrite(content->data, 1, content->len) → fclose.
 */
void suru_writefile(SuruString *path, SuruString *content) {
    FILE *f = fopen(path->data, "w");
    fwrite(content->data, 1, (size_t)content->len, f);
    fclose(f);
}

/* suru_appendtofile — append content to a file (no truncation).
 *
 * Algorithm: fopen("a") → fwrite(content->data, 1, content->len) → fclose.
 */
void suru_appendtofile(SuruString *path, SuruString *content) {
    FILE *f = fopen(path->data, "a");
    fwrite(content->data, 1, (size_t)content->len, f);
    fclose(f);
}
