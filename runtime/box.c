/* box.c — Suru box runtime. Replaces box.ll.
 *
 * %suru.Box wraps a scalar so every Suru value is a `ptr` carrying its type_tag
 * at offset 0. The dispatch printers (suru_println / suru_printerror) and
 * suru_dyn_len read that tag to handle any heap value uniformly.
 *
 * type_tag: 0=Bool 1=Int32 2=Int64 3=Float64 4=Struct 5=Array 6=String 7=SumType 8=StaticString
 *
 * Output formats are kept byte-for-byte identical to the prior IR: integers via
 * printf("%lld\n"/"%d\n"), floats via "%g\n", bools/struct/array via puts() of a
 * literal (puts appends the newline). The stderr variants emit the body then a
 * separate "\n", matching the IR which did not fold the newline into the format.
 */
#include "suru_runtime.h"

#include <stdlib.h>
#include <stdio.h>
#include <string.h>

/* ── boxing ─────────────────────────────────────────────────────────────── */

void *suru_box_bool(bool v) {
    SuruBox *b = malloc(sizeof(SuruBox));
    b->type_tag = 0;
    b->value = (int64_t)(v ? 1 : 0); /* zero-extend i1 → i64 */
    return b;
}

void *suru_box_int32(int32_t v) {
    SuruBox *b = malloc(sizeof(SuruBox));
    b->type_tag = 1;
    b->value = (int64_t)v; /* sign-extend i32 → i64 */
    return b;
}

void *suru_box_int64(int64_t v) {
    SuruBox *b = malloc(sizeof(SuruBox));
    b->type_tag = 2;
    b->value = v;
    return b;
}

void *suru_box_float64(double v) {
    SuruBox *b = malloc(sizeof(SuruBox));
    b->type_tag = 3;
    memcpy(&b->value, &v, sizeof(double)); /* store the bit pattern, not a cast */
    return b;
}

/* ── unboxing ───────────────────────────────────────────────────────────── */

bool suru_unbox_bool(void *box) {
    return (bool)(((SuruBox *)box)->value & 1);
}

int32_t suru_unbox_int32(void *box) {
    return (int32_t)((SuruBox *)box)->value; /* truncate i64 → i32 */
}

int64_t suru_unbox_int64(void *box) {
    return ((SuruBox *)box)->value;
}

double suru_unbox_float64(void *box) {
    double d;
    memcpy(&d, &((SuruBox *)box)->value, sizeof(double));
    return d;
}

/* ── clone ──────────────────────────────────────────────────────────────── */

void *suru_box_clone(void *src) {
    SuruBox *s = (SuruBox *)src;
    SuruBox *b = malloc(sizeof(SuruBox));
    b->type_tag = s->type_tag;
    b->value = s->value;
    return b;
}

/* ── dynamic printers ───────────────────────────────────────────────────── */

/* Print any Suru value to stdout followed by a newline; dispatch on type_tag. */
void suru_println(void *v) {
    int64_t tag = ((SuruBox *)v)->type_tag;
    switch (tag) {
    case 0: /* Bool */
        puts((((SuruBox *)v)->value & 1) ? "true" : "false");
        break;
    case 1: /* Int32 */
        printf("%d\n", (int)(int32_t)((SuruBox *)v)->value);
        break;
    case 2: /* Int64 */
        printf("%lld\n", (long long)((SuruBox *)v)->value);
        break;
    case 3: { /* Float64 */
        double d;
        memcpy(&d, &((SuruBox *)v)->value, sizeof(double));
        printf("%g\n", d);
        break;
    }
    case 4: /* Struct */
        puts("<struct>");
        break;
    case 5: /* Array */
        puts("<array>");
        break;
    case 6: /* String */
    case 8: /* StaticString */
        puts(((SuruString *)v)->data);
        break;
    default:
        break; /* unknown tag: no-op */
    }
}

/* Same dispatch as suru_println but to stderr. */
void suru_printerror(void *v) {
    int64_t tag = ((SuruBox *)v)->type_tag;
    switch (tag) {
    case 0: /* Bool */
        fputs((((SuruBox *)v)->value & 1) ? "true" : "false", stderr);
        fputs("\n", stderr);
        break;
    case 1: /* Int32 */
        fprintf(stderr, "%d\n", (int)(int32_t)((SuruBox *)v)->value);
        break;
    case 2: /* Int64 */
        fprintf(stderr, "%lld\n", (long long)((SuruBox *)v)->value);
        break;
    case 3: { /* Float64 */
        double d;
        memcpy(&d, &((SuruBox *)v)->value, sizeof(double));
        fprintf(stderr, "%g\n", d);
        break;
    }
    case 4: /* Struct */
        fputs("<struct>", stderr);
        fputs("\n", stderr);
        break;
    case 5: /* Array */
        fputs("<array>", stderr);
        fputs("\n", stderr);
        break;
    case 6: /* String */
    case 8: /* StaticString */
        fputs(((SuruString *)v)->data, stderr);
        fputs("\n", stderr);
        break;
    default:
        break;
    }
}

/* ── scalar fast-path printers (no heap allocation) ─────────────────────── */

void suru_println_bool(bool v) { puts(v ? "true" : "false"); }
void suru_println_i32(int32_t v) { printf("%d\n", (int)v); }
void suru_println_i64(int64_t v) { printf("%lld\n", (long long)v); }
void suru_println_f64(double v) { printf("%g\n", v); }

void suru_printerror_bool(bool v) {
    fputs(v ? "true" : "false", stderr);
    fputs("\n", stderr);
}
void suru_printerror_i32(int32_t v) { fprintf(stderr, "%d\n", (int)v); }
void suru_printerror_i64(int64_t v) { fprintf(stderr, "%lld\n", (long long)v); }
void suru_printerror_f64(double v) { fprintf(stderr, "%g\n", v); }

/* Print a raw NUL-terminated byte buffer (e.g. a string literal) to stderr. */
void suru_printerror_lit(char *data) {
    fputs(data, stderr);
    fputs("\n", stderr);
}

/* ── length ─────────────────────────────────────────────────────────────── */

/* Length of any String or Array; 0 for null or any other tag. */
int64_t suru_dyn_len(void *v) {
    if (v == NULL) {
        return 0;
    }
    int64_t tag = ((SuruBox *)v)->type_tag;
    switch (tag) {
    case 5: /* Array */
        return ((SuruArray *)v)->len;
    case 6: /* String */
    case 8: /* StaticString */
        return ((SuruString *)v)->len;
    default:
        return 0;
    }
}
