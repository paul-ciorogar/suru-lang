/* array.c — Suru array runtime. Replaces array.ll.
 *
 * Every Suru Array is a `ptr` to a heap-allocated SuruArray header (40 bytes):
 *   { i64 type_tag, i64 elem_tag, i64 len, i64 cap, ptr data }
 * type_tag is 5 (Array) at offset 0, so any heap ptr inspected at offset 0
 * identifies itself as an Array. `data` is a typed buffer whose element byte
 * size depends on elem_tag:
 *   Bool(0) → i8[] (1 byte), Int32(1) → i32[] (4 bytes), everything else → i64[] (8 bytes).
 * Scalars are stored directly (no boxing); heap ptrs are stored as the pointer
 * reinterpreted as an int64 (ptrtoint), recovered on read via the reverse cast.
 *
 * This is a behaviour-preserving port of array.ll: the same exported symbols,
 * the same amortised growth policy, and the same bitwise element semantics, so
 * it links as a drop-in replacement with no codegen change.
 */
#include "suru_runtime.h"

#include <stdlib.h>
#include <string.h>

/* Cross-module dynamic dispatch for per-element clone/drop. Defined in
 * struct.c (suru_clone_dyn / suru_drop_dyn dispatch all heap type_tags via the
 * vtable for structs/variants, and forward strings/arrays/scalars). */
extern void *suru_clone_dyn(void *);
extern void  suru_drop_dyn(void *);

/* ── element sizing ─────────────────────────────────────────────────────────
 *
 * Byte size of one element for a given elem_tag.
 * Bool(0)→1, Int32(1)→4, everything else (Int64/Float64/heap)→8. */
static int64_t suru_array_elem_size(int64_t etag) {
    if (etag == 0) {
        return 1;
    }
    if (etag == 1) {
        return 4;
    }
    return 8;
}

/* Ensure room for one more element, growing in place when len == cap.
 *
 * Growth policy (identical to the IR, shared by every add path):
 *   cap == 0       → new_cap = 4
 *   0 < cap < 1024 → new_cap = cap * 2
 *   cap >= 1024    → new_cap = cap + 1024
 * On grow, realloc the typed data buffer to new_cap * elem_size bytes and
 * write the new cap/data back into the header. */
static void suru_array_ensure_capacity(SuruArray *arr, int64_t elem_size) {
    if (arr->len != arr->cap) {
        return;
    }
    int64_t ncap;
    if (arr->cap == 0) {
        ncap = 4;
    } else if (arr->cap < 1024) {
        ncap = arr->cap * 2;
    } else {
        ncap = arr->cap + 1024;
    }
    arr->data = realloc(arr->data, (size_t)(ncap * elem_size));
    arr->cap = ncap;
}

/* ── append ─────────────────────────────────────────────────────────────────
 *
 * Append a value with amortised growth, mutating the header in place. There is
 * one entry point per element width; the caller picks the width from elem_tag.
 * add_i64 also receives Float64 elements (bitcast double→i64 by the caller) and
 * heap-type elements (the pointer reinterpreted as int64 by the caller). */

void suru_array_add_i8(SuruArray *arr, int8_t val) {
    suru_array_ensure_capacity(arr, 1);
    ((int8_t *)arr->data)[arr->len] = val;
    arr->len++;
}

void suru_array_add_i32(SuruArray *arr, int32_t val) {
    suru_array_ensure_capacity(arr, 4);
    ((int32_t *)arr->data)[arr->len] = val;
    arr->len++;
}

void suru_array_add_i64(SuruArray *arr, int64_t val) {
    suru_array_ensure_capacity(arr, 8);
    ((int64_t *)arr->data)[arr->len] = val;
    arr->len++;
}

/* ── backward-compatibility shims ────────────────────────────────────────────
 *
 * The bootstrap binary (compiled before the typed-element ABI) calls these.
 * They treat every slot as an i64 and reinterpret it as a pointer, matching the
 * old binary which always stored heap ptrs via ptrtoint. No bounds checks, to
 * stay bit-identical to the IR. Remove after the next successful bootstrap. */

void *suru_array_at(SuruArray *arr, int64_t idx) {
    return (void *)(intptr_t)((int64_t *)arr->data)[idx];
}

void suru_array_set(SuruArray *arr, int64_t idx, void *val) {
    ((int64_t *)arr->data)[idx] = (int64_t)(intptr_t)val;
}

void suru_array_add(SuruArray *arr, void *val) {
    suru_array_add_i64(arr, (int64_t)(intptr_t)val);
}

/* ── slice ───────────────────────────────────────────────────────────────────
 *
 * Return a fresh array header holding a bitwise copy of elements [from, to).
 * The raw element bytes are copied as-is; elem_tag is propagated from the
 * source. No bounds checks (matches the IR). */
void *suru_array_slice(SuruArray *arr, int64_t from, int64_t to) {
    int64_t etag = arr->elem_tag;
    int64_t esz = suru_array_elem_size(etag);
    int64_t count = to - from;
    int64_t nbytes = count * esz;

    void *nd = malloc((size_t)nbytes);
    memcpy(nd, (char *)arr->data + from * esz, (size_t)nbytes);

    SuruArray *nh = malloc(sizeof(SuruArray));
    nh->type_tag = 5;
    nh->elem_tag = etag;
    nh->len = count;
    nh->cap = count;
    nh->data = nd;
    return nh;
}

/* ── clone ───────────────────────────────────────────────────────────────────
 *
 * Deep-clone an array. Scalar element types (elem_tag < 4) bitcopy the data
 * buffer directly. Heap types (elem_tag >= 4) clone each element through
 * suru_clone_dyn — the slot holds the element ptr reinterpreted as int64, so we
 * cast back to a pointer, clone, and store the clone's pointer the same way. */
void *suru_array_clone_dyn(SuruArray *arr) {
    int64_t etag = arr->elem_tag;
    int64_t len = arr->len;
    int64_t esz = suru_array_elem_size(etag);
    int64_t nbytes = len * esz;

    void *nd = malloc((size_t)nbytes);
    if (etag < 4) {
        memcpy(nd, arr->data, (size_t)nbytes);
    } else {
        int64_t *src = (int64_t *)arr->data;
        int64_t *dst = (int64_t *)nd;
        for (int64_t i = 0; i < len; i++) {
            void *cloned = suru_clone_dyn((void *)(intptr_t)src[i]);
            dst[i] = (int64_t)(intptr_t)cloned;
        }
    }

    SuruArray *nh = malloc(sizeof(SuruArray));
    nh->type_tag = arr->type_tag;
    nh->elem_tag = etag;
    nh->len = len;
    nh->cap = len;
    nh->data = nd;
    return nh;
}

/* ── drop ────────────────────────────────────────────────────────────────────
 *
 * Free an array. Scalar element types (elem_tag < 4) just free the buffer; heap
 * types (elem_tag >= 4) drop each element through suru_drop_dyn first. Then free
 * the data buffer and the header. */
void suru_array_drop_dyn(SuruArray *arr) {
    int64_t etag = arr->elem_tag;
    if (etag >= 4) {
        int64_t len = arr->len;
        int64_t *data = (int64_t *)arr->data;
        for (int64_t i = 0; i < len; i++) {
            suru_drop_dyn((void *)(intptr_t)data[i]);
        }
    }
    free(arr->data);
    free(arr);
}
