/* string.c — Suru string runtime. Replaces string.ll.
 *
 * Every Suru String is a `ptr` to a SuruString header (24 bytes):
 *   { i64 type_tag, i64 len, ptr data }
 * type_tag = 6: heap-allocated (malloc'd header + malloc'd, NUL-terminated data buffer).
 * type_tag = 8: static literal (a global constant in .rodata — never freed).
 * Placing type_tag first means any heap ptr can be inspected at offset 0 to learn
 * its Suru kind at runtime. `len` is the byte length, excluding the NUL terminator;
 * every data buffer below is NUL-terminated so the libc str* functions are safe.
 *
 * This is a behaviour-preserving port of string.ll: the same exported symbols and
 * the same allocation/byte semantics, so it links as a drop-in with no codegen change.
 */
#include "suru_runtime.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* ── construction ───────────────────────────────────────────────────────────
 *
 * Allocate a fresh 24-byte tag-6 header wrapping `data`/`len`. The header takes
 * ownership of `data`; callers pass a buffer they have already allocated. */
void *suru_string_create(char *data, int64_t len) {
    SuruString *s = malloc(sizeof(SuruString));
    s->type_tag = 6;
    s->len = len;
    s->data = data;
    return s;
}

/* Deep-copy a String: duplicate the data buffer (including its NUL terminator)
 * into a fresh tag-6 header. */
void *suru_string_clone(void *sp) {
    SuruString *s = (SuruString *)sp;
    int64_t len = s->len;
    char *buf = malloc((size_t)(len + 1));
    memcpy(buf, s->data, (size_t)(len + 1)); /* +1 copies the NUL too */

    SuruString *out = malloc(sizeof(SuruString));
    out->type_tag = 6;
    out->len = len;
    out->data = buf;
    return out;
}

/* Free a String. Static literals (type_tag = 8) live in .rodata — skip entirely;
 * freeing them would corrupt the heap. */
void suru_string_drop(void *sp) {
    SuruString *s = (SuruString *)sp;
    if (s->type_tag == 8) {
        return;
    }
    free(s->data);
    free(s);
}

/* ── concatenation ──────────────────────────────────────────────────────────
 *
 * Allocate a new tag-6 String holding lhs followed by rhs. The result buffer is
 * llen + rlen + 1 bytes, NUL-terminated. */
void *suru_string_append(void *lhsp, void *rhsp) {
    SuruString *lhs = (SuruString *)lhsp;
    SuruString *rhs = (SuruString *)rhsp;
    int64_t llen = lhs->len;
    int64_t rlen = rhs->len;
    int64_t tot = llen + rlen;

    char *buf = malloc((size_t)(tot + 1));
    memcpy(buf, lhs->data, (size_t)llen);
    memcpy(buf + llen, rhs->data, (size_t)rlen);
    buf[tot] = 0;

    SuruString *out = malloc(sizeof(SuruString));
    out->type_tag = 6;
    out->len = tot;
    out->data = buf;
    return out;
}

/* Append a raw byte buffer (e.g. a string-literal global) to lhs. `data` points
 * at raw bytes (NOT a SuruString); `rlen` is the byte count. */
void *suru_string_append_lit(void *lhsp, char *data, int64_t rlen) {
    SuruString *lhs = (SuruString *)lhsp;
    int64_t llen = lhs->len;
    int64_t tot = llen + rlen;

    char *buf = malloc((size_t)(tot + 1));
    memcpy(buf, lhs->data, (size_t)llen);
    memcpy(buf + llen, data, (size_t)rlen);
    buf[tot] = 0;

    SuruString *out = malloc(sizeof(SuruString));
    out->type_tag = 6;
    out->len = tot;
    out->data = buf;
    return out;
}

/* Append one byte to lhs → new heap String (like append_lit with rlen = 1). */
void *suru_string_append_char(void *lhsp, char c) {
    SuruString *lhs = (SuruString *)lhsp;
    int64_t llen = lhs->len;
    int64_t tot = llen + 1;

    char *buf = malloc((size_t)(tot + 1));
    memcpy(buf, lhs->data, (size_t)llen);
    buf[llen] = c;
    buf[tot] = 0;

    SuruString *out = malloc(sizeof(SuruString));
    out->type_tag = 6;
    out->len = tot;
    out->data = buf;
    return out;
}

/* ── indexing ───────────────────────────────────────────────────────────────
 *
 * Return a fresh length-1 heap String containing the byte at index `i`. */
void *suru_string_at(void *sp, int64_t i) {
    SuruString *s = (SuruString *)sp;
    char *buf = malloc(2);
    buf[0] = s->data[i];
    buf[1] = 0;

    SuruString *out = malloc(sizeof(SuruString));
    out->type_tag = 6;
    out->len = 1;
    out->data = buf;
    return out;
}

/* The byte at index `i` as a raw Char — NO allocation, NO header. */
char suru_string_char_at(void *sp, int64_t i) {
    SuruString *s = (SuruString *)sp;
    return s->data[i];
}

/* The first byte as an i64 code point, zero-extended (unsigned). */
int64_t suru_string_ord(void *sp) {
    SuruString *s = (SuruString *)sp;
    return (int64_t)(unsigned char)s->data[0];
}

/* Materialize a Char into an owned heap String (tag 6). '\0' (null byte) is
 * treated as end-of-string and yields an empty String. */
void *suru_string_from_char(char c) {
    if (c == 0) {
        char *buf = malloc(1);
        buf[0] = 0;
        SuruString *out = malloc(sizeof(SuruString));
        out->type_tag = 6;
        out->len = 0;
        out->data = buf;
        return out;
    }
    char *buf = malloc(2);
    buf[0] = c;
    buf[1] = 0;
    SuruString *out = malloc(sizeof(SuruString));
    out->type_tag = 6;
    out->len = 1;
    out->data = buf;
    return out;
}

/* ── comparison / slicing ───────────────────────────────────────────────────
 *
 * Byte-wise equality of the two NUL-terminated data buffers. */
bool suru_string_equals(void *lhsp, void *rhsp) {
    SuruString *lhs = (SuruString *)lhsp;
    SuruString *rhs = (SuruString *)rhsp;
    return strcmp(lhs->data, rhs->data) == 0;
}

/* New tag-6 String holding the byte range [from, to), NUL-terminated. */
void *suru_string_slice(void *sp, int64_t from, int64_t to) {
    SuruString *s = (SuruString *)sp;
    int64_t slen = to - from;

    char *buf = malloc((size_t)(slen + 1));
    memcpy(buf, s->data + from, (size_t)slen);
    buf[slen] = 0;

    SuruString *out = malloc(sizeof(SuruString));
    out->type_tag = 6;
    out->len = slen;
    out->data = buf;
    return out;
}

/* ── numeric conversion ─────────────────────────────────────────────────────
 *
 * Parse a base-10 integer from the String's bytes. */
int64_t suru_int64_from_string(void *sp) {
    SuruString *s = (SuruString *)sp;
    return (int64_t)strtol(s->data, NULL, 10);
}

/* Format an i64 as a decimal String (tag 6). snprintf with a NULL buffer returns
 * the digit count, which sizes the allocation and becomes len. */
void *suru_int64_to_string(int64_t v) {
    int count = snprintf(NULL, 0, "%lld", (long long)v);
    int64_t len = (int64_t)count;
    char *buf = malloc((size_t)(len + 1));
    snprintf(buf, (size_t)(len + 1), "%lld", (long long)v);

    SuruString *out = malloc(sizeof(SuruString));
    out->type_tag = 6;
    out->len = len;
    out->data = buf;
    return out;
}

/* Parse a decimal String (e.g. "-2.5") into a raw double via strtod. */
double suru_float64_from_string(void *sp) {
    SuruString *s = (SuruString *)sp;
    return strtod(s->data, NULL);
}

/* Convert a double to its LLVM IR hex literal string, e.g. "0xC004000000000000".
 * The output is always 18 chars: "0x" + 16 uppercase hex digits of the bit
 * pattern. memcpy reads the bits without violating strict aliasing. */
void *suru_float64_to_llvm_hex(double f) {
    uint64_t bits;
    memcpy(&bits, &f, sizeof(bits));

    char *buf = malloc(19); /* 18 chars + NUL */
    snprintf(buf, 19, "0x%016llX", (unsigned long long)bits);

    SuruString *out = malloc(sizeof(SuruString));
    out->type_tag = 6;
    out->len = 18;
    out->data = buf;
    return out;
}
