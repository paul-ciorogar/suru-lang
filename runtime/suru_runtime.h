/* suru_runtime.h — shared layout definitions for the Suru C runtime.
 *
 * These structs mirror, byte-for-byte, the LLVM struct types the compiler's
 * codegen emits GEPs against. The field offsets here are an ABI contract with
 * the code generator: changing them silently breaks every compiled program.
 *
 * type_tag encoding (offset 0 of every heap value):
 *   0=Bool 1=Int32 2=Int64 3=Float64 4=Struct 5=Array 6=String 7=SumType 8=StaticString
 *
 * Migration note (FFI plan, Phase A): this header backs box.c and struct.c.
 * array.c, string.c and variant.c (Tasks A2–A4) will share it too.
 */
#ifndef SURU_RUNTIME_H
#define SURU_RUNTIME_H

#include <stdint.h>
#include <stdbool.h>

/* A boxed scalar. The payload at offset 8 holds the value as a raw 64-bit word:
 * bool zero-extended, i32 sign-extended, f64 stored by its bit pattern. 16 bytes. */
typedef struct {
    int64_t type_tag;
    int64_t value;
} SuruBox;

/* A heap string. `data` points at a NUL-terminated byte buffer; `len` is the
 * byte length excluding the terminator. data sits at offset 16, len at offset 8. */
typedef struct {
    int64_t type_tag;
    int64_t len;
    char   *data;
} SuruString;

/* A dynamic array. `len` (offset 16) is the element count; `elem_tag` records
 * the element's type_tag; `cap` is the allocated element capacity. */
typedef struct {
    int64_t type_tag;
    int64_t elem_tag;
    int64_t len;
    int64_t cap;
    void   *data;
} SuruArray;

/* The 32-byte header shared by structs (tag 4) and sum types (tag 7). clone_fn
 * (offset 16) and drop_fn (offset 24) are per-type functions emitted into the
 * user module; the dynamic dispatchers call them through these pointers. Field
 * data follows the header at offset 32. */
typedef struct {
    int64_t  type_tag;
    int64_t  variant_idx;
    void   *(*clone_fn)(void *);
    void    (*drop_fn)(void *);
} SuruHeader;

#endif /* SURU_RUNTIME_H */
