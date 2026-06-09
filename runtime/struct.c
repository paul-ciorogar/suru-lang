/* struct.c — Suru struct/variant runtime. Replaces struct.ll.
 *
 * Structs (tag 4) and sum types (tag 7) carry a 32-byte header:
 *   { i64 type_tag, i64 variant_idx, ptr clone_fn, ptr drop_fn }
 * with field data following at offset 32. clone_fn/drop_fn are per-type
 * functions emitted into the user module; the dynamic dispatchers below call
 * them through those pointers.
 *
 * The dispatchers read type_tag at offset 0 and route every heap value to the
 * right clone/drop routine, so codegen can clone or drop a value without
 * statically knowing its type.
 *
 * type_tag: 0=Bool 1=Int32 2=Int64 3=Float64 4=Struct 5=Array 6=String 7=SumType 8=StaticString
 */
#include "suru_runtime.h"

#include <stdlib.h>

/* Cross-module refs — defined in box.c, string.c and array.c. */
extern void *suru_box_clone(void *);
extern void *suru_string_clone(void *);
extern void  suru_string_drop(void *);
extern void *suru_array_clone_dyn(void *);
extern void  suru_array_drop_dyn(void *);

/* Clone any Suru heap value by reading type_tag at offset 0.
 * null → null so a zero-initialized (null) struct field clones to null. */
void *suru_clone_dyn(void *val) {
    // TODO all types should have their own clone method in their vtable
    if (val == NULL) {
        return NULL;
    }
    int64_t tag = ((SuruHeader *)val)->type_tag;
    switch (tag) {
    case 4: /* Struct */
    case 7: /* SumType */
        return ((SuruHeader *)val)->clone_fn(val);
    case 5: /* Array */
        return suru_array_clone_dyn(val);
    case 6: /* String */
    case 8: /* StaticString */
        return suru_string_clone(val);
    default: /* tags 0–3: boxed scalar */
        return suru_box_clone(val);
    }
}

/* Drop any Suru heap value by reading type_tag at offset 0.
 * null → no-op so partial/empty struct literals don't crash on drop. */
void suru_drop_dyn(void *val) {
    // TODO all types should have their own drop method in their vtable
    if (val == NULL) {
        return;
    }
    int64_t tag = ((SuruHeader *)val)->type_tag;
    switch (tag) {
    case 4: /* Struct */
    case 7: /* SumType */
        ((SuruHeader *)val)->drop_fn(val);
        return;
    case 5: /* Array */
        suru_array_drop_dyn(val);
        return;
    case 6: /* String */
    case 8: /* StaticString */
        suru_string_drop(val);
        return;
    default: /* tags 0–3: boxed scalar */
        free(val);
        return;
    }
}
