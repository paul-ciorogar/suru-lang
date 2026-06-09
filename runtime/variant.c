/* variant.c — Suru sum type (variant) runtime. Replaces variant.ll.
 *
 * Phase 2: a sum type value is a flat fixed-layout struct sharing the 32-byte
 * struct header (see SuruHeader in suru_runtime.h):
 *   { i64 type_tag, i64 variant_idx, ptr clone_fn, ptr drop_fn }
 * A type_tag of 7 (SumType) at offset 0 identifies the value as a variant, and
 * variant_idx at offset 8 records the active variant. clone_fn/drop_fn at
 * offsets 16/24 are the per-type vtable functions emitted into the user module;
 * suru_variant_drop dispatches through drop_fn so a variant can be dropped
 * without statically knowing its concrete arm.
 *
 * type_tag: 0=Bool 1=Int32 2=Int64 3=Float64 4=Struct 5=Array 6=String 7=SumType 8=StaticString
 */
#include "suru_runtime.h"

/* ── suru_variant_create ─────────────────────────────────────────────────── */

/* Marks an existing flat struct as a variant: writes type_tag=7 and the active
 * variant_idx into the header. Returns the same pointer — no new allocation. */
void *suru_variant_create(int64_t idx, void *struct_ptr) {
    SuruHeader *h = (SuruHeader *)struct_ptr;
    h->type_tag = 7;       /* SumType */
    h->variant_idx = idx;
    return struct_ptr;
}

/* ── suru_variant_tag ────────────────────────────────────────────────────── */

/* Returns the active variant index stored at offset 8 of the variant struct. */
int64_t suru_variant_tag(void *v) {
    return ((SuruHeader *)v)->variant_idx;
}

/* ── suru_variant_inner ──────────────────────────────────────────────────── */

/* Returns the pointer unchanged — the variant IS the flat struct, so the inner
 * payload begins at offset 32 of this same allocation; no unwrapping needed. */
void *suru_variant_inner(void *v) {
    return v;
}

/* ── suru_variant_drop ───────────────────────────────────────────────────── */

/* Drops a variant by calling its drop_fn (vtable offset 24) through the header.
 * The per-type drop routine frees the arm's fields and the allocation itself. */
void suru_variant_drop(void *v) {
    ((SuruHeader *)v)->drop_fn(v);
}
