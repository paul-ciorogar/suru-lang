/* struct.c — Suru struct/variant runtime. Replaces struct.ll.
 *
 * Structs (tag 4) and sum types (tag 7) carry a 32-byte header:
 *   { i64 type_tag, i64 variant_idx, ptr clone_fn, ptr drop_fn }
 * with field data following at offset 32. clone_fn/drop_fn are per-type
 * functions emitted into the user module.
 *
 * Clone/drop is statically dispatched by the codegen: for a struct/variant value
 * it emits a call to one of the trampolines below, which dispatch straight
 * through the value's own clone_fn/drop_fn pointer — no type_tag read.
 *
 * type_tag: 0=Bool 1=Int32 2=Int64 3=Float64 4=Struct 5=Array 6=String 7=SumType 8=StaticString
 */
#include "suru_runtime.h"

#include <stdlib.h>

/* ── static-dispatch vtable trampolines ──────────────────────────────────────
 *
 * Used when the codegen statically knows a value is a struct or sum type (the
 * only two kinds that carry the 32-byte header with clone_fn/drop_fn). These
 * read NO type_tag: they dispatch straight through the per-value clone_fn/drop_fn
 * pointer the allocator stored. null-safe so a zero-initialized field (the `{}`
 * release idiom) clones to null / drops to a no-op. */
void *suru_clone_via_vtable(void *val) {
    if (val == NULL) {
        return NULL;
    }
    return ((SuruHeader *)val)->clone_fn(val);
}

void suru_drop_via_vtable(void *val) {
    if (val == NULL) {
        return;
    }
    ((SuruHeader *)val)->drop_fn(val);
}
