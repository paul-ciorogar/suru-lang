; Suru array runtime — compiled to suru_array.o and linked with every Suru program.
;
; Every Suru Array is a ptr to a heap-allocated
;   %suru.Array = { i64 type_tag, i64 elem_tag, i64 len, i64 cap, ptr data }  (40 bytes)
; type_tag=5 (TYPE_ARRAY) at field 0: any heap ptr inspected at offset 0 identifies this as Array.
; data is a typed buffer; element byte size depends on elem_tag:
;   Bool(0) → i8[] (1 byte), Int32(1) → i32[] (4 bytes), everything else → i64[] (8 bytes).
; Scalars are stored directly (no boxing); heap ptrs are stored as ptrtoint i64.

; ModuleID = 'suru_array.ll'
source_filename = "suru_array.ll"

%suru.Array = type { i64, i64, i64, i64, ptr }

declare ptr  @malloc(i64)
declare ptr  @realloc(ptr, i64)
declare ptr  @memcpy(ptr, ptr, i64)
declare void @free(ptr)

; Cross-module dynamic dispatch for element clone/drop.
; suru_clone_dyn / suru_drop_dyn handle all type_tags (0-7) via vtable for structs/variants.
declare ptr  @suru_clone_dyn(ptr)
declare void @suru_drop_dyn(ptr)

; ─── suru_array_elem_size ──────────────────────────────────────────────────────
;
; Byte size of one element for a given elem_tag.
; Bool(0)→1, Int32(1)→4, everything else (Int64/Float64/heap)→8.
define internal i64 @suru_array_elem_size(i64 %etag) {
entry:
  %isbool = icmp eq i64 %etag, 0
  %isi32  = icmp eq i64 %etag, 1
  %s8     = select i1 %isbool, i64 1, i64 8
  %sz     = select i1 %isi32,  i64 4, i64 %s8
  ret i64 %sz
}

; ─── suru_array_add_i8 ─────────────────────────────────────────────────────────
;
; Append an i8 value (Bool element) to the array with amortised growth.
; Mutates the %suru.Array header in-place: cap and data may change in the grow path.
;
; Growth policy:
;   cap == 0       → new_cap = 4
;   0 < cap < 1024 → new_cap = cap * 2
;   cap >= 1024    → new_cap = cap + 1024
define void @suru_array_add_i8(ptr %arr, i8 %val) {
entry:
  %lgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 2
  %len  = load i64, ptr %lgep
  %cgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 3
  %cap  = load i64, ptr %cgep
  %dgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 4
  %full = icmp eq i64 %len, %cap
  br i1 %full, label %grow, label %store
grow:
  %dbl  = mul i64 %cap, 2
  %lin  = add i64 %cap, 1024
  %udbl = icmp ult i64 %cap, 1024
  %grwn = select i1 %udbl, i64 %dbl, i64 %lin
  %isz  = icmp eq i64 %cap, 0
  %ncap = select i1 %isz, i64 4, i64 %grwn
  %nbyt = mul i64 %ncap, 1
  %data = load ptr, ptr %dgep
  %ndat = call ptr @realloc(ptr %data, i64 %nbyt)
  store i64 %ncap, ptr %cgep
  store ptr %ndat, ptr %dgep
  br label %store
store:
  %cdat = load ptr, ptr %dgep
  %slot = getelementptr i8, ptr %cdat, i64 %len
  store i8 %val, ptr %slot
  %nlen = add i64 %len, 1
  store i64 %nlen, ptr %lgep
  ret void
}

; ─── suru_array_add_i32 ────────────────────────────────────────────────────────
;
; Append an i32 value (Int32 element) to the array with amortised growth.
define void @suru_array_add_i32(ptr %arr, i32 %val) {
entry:
  %lgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 2
  %len  = load i64, ptr %lgep
  %cgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 3
  %cap  = load i64, ptr %cgep
  %dgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 4
  %full = icmp eq i64 %len, %cap
  br i1 %full, label %grow, label %store
grow:
  %dbl  = mul i64 %cap, 2
  %lin  = add i64 %cap, 1024
  %udbl = icmp ult i64 %cap, 1024
  %grwn = select i1 %udbl, i64 %dbl, i64 %lin
  %isz  = icmp eq i64 %cap, 0
  %ncap = select i1 %isz, i64 4, i64 %grwn
  %nbyt = mul i64 %ncap, 4
  %data = load ptr, ptr %dgep
  %ndat = call ptr @realloc(ptr %data, i64 %nbyt)
  store i64 %ncap, ptr %cgep
  store ptr %ndat, ptr %dgep
  br label %store
store:
  %cdat = load ptr, ptr %dgep
  %slot = getelementptr i32, ptr %cdat, i64 %len
  store i32 %val, ptr %slot
  %nlen = add i64 %len, 1
  store i64 %nlen, ptr %lgep
  ret void
}

; ─── suru_array_add_i64 ────────────────────────────────────────────────────────
;
; Append an i64 value to the array with amortised growth.
; Used for Int64 elements, Float64 elements (caller bitcasts double→i64),
; and heap-type elements (caller ptrtoint ptr→i64).
define void @suru_array_add_i64(ptr %arr, i64 %val) {
entry:
  %lgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 2
  %len  = load i64, ptr %lgep
  %cgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 3
  %cap  = load i64, ptr %cgep
  %dgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 4
  %full = icmp eq i64 %len, %cap
  br i1 %full, label %grow, label %store
grow:
  %dbl  = mul i64 %cap, 2
  %lin  = add i64 %cap, 1024
  %udbl = icmp ult i64 %cap, 1024
  %grwn = select i1 %udbl, i64 %dbl, i64 %lin
  %isz  = icmp eq i64 %cap, 0
  %ncap = select i1 %isz, i64 4, i64 %grwn
  %nbyt = mul i64 %ncap, 8
  %data = load ptr, ptr %dgep
  %ndat = call ptr @realloc(ptr %data, i64 %nbyt)
  store i64 %ncap, ptr %cgep
  store ptr %ndat, ptr %dgep
  br label %store
store:
  %cdat = load ptr, ptr %dgep
  %slot = getelementptr i64, ptr %cdat, i64 %len
  store i64 %val, ptr %slot
  %nlen = add i64 %len, 1
  store i64 %nlen, ptr %lgep
  ret void
}

; ─── Backward-compatibility shims ─────────────────────────────────────────────
;
; The bootstrap binary (compiled before the typed-element ABI) calls these
; functions.  Remove after the next successful bootstrap.

; suru_array_at(ptr arr, i64 idx) → ptr
; Loads the raw i64 from the data slot and reinterprets it as a ptr.
; Old binary always stored heap ptrs via ptrtoint, so inttoptr is correct.
define ptr @suru_array_at(ptr %arr, i64 %idx) {
entry:
  %dgep   = getelementptr %suru.Array, ptr %arr, i32 0, i32 4
  %data   = load ptr, ptr %dgep
  %slot   = getelementptr i64, ptr %data, i64 %idx
  %raw    = load i64, ptr %slot
  %result = inttoptr i64 %raw to ptr
  ret ptr %result
}

; suru_array_set(ptr arr, i64 idx, ptr val) → void
; Encodes val as i64 via ptrtoint and stores it at the indexed slot.
define void @suru_array_set(ptr %arr, i64 %idx, ptr %val) {
entry:
  %dgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 4
  %data = load ptr, ptr %dgep
  %slot = getelementptr i64, ptr %data, i64 %idx
  %raw  = ptrtoint ptr %val to i64
  store i64 %raw, ptr %slot
  ret void
}

; suru_array_add(ptr arr, ptr val) → void
; Encodes val as i64 via ptrtoint and delegates to suru_array_add_i64.
define void @suru_array_add(ptr %arr, ptr %val) {
entry:
  %raw = ptrtoint ptr %val to i64
  call void @suru_array_add_i64(ptr %arr, i64 %raw)
  ret void
}

; ─── suru_array_slice ──────────────────────────────────────────────────────────
;
; Return a new array header containing a bitwise copy of elements [from, to).
; The raw element bytes are copied as-is. elem_tag is propagated from source.
define ptr @suru_array_slice(ptr %arr, i64 %from, i64 %to) {
entry:
  %egep = getelementptr %suru.Array, ptr %arr, i32 0, i32 1
  %etag = load i64, ptr %egep
  %dgep = getelementptr %suru.Array, ptr %arr, i32 0, i32 4
  %data = load ptr, ptr %dgep
  %slen      = sub i64 %to, %from
  %esz       = call i64 @suru_array_elem_size(i64 %etag)
  %bc        = mul i64 %slen, %esz
  %from_bytes = mul i64 %from, %esz
  %srcp      = getelementptr i8, ptr %data, i64 %from_bytes
  %nd   = call ptr @malloc(i64 %bc)
  call ptr @memcpy(ptr %nd, ptr %srcp, i64 %bc)
  %nh   = call ptr @malloc(i64 40)
  %tgg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 0
  store i64 5, ptr %tgg
  %egg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 1
  store i64 %etag, ptr %egg
  %lgg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 2
  store i64 %slen, ptr %lgg
  %cgg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 3
  store i64 %slen, ptr %cgg
  %dgg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 4
  store ptr %nd, ptr %dgg
  ret ptr %nh
}

; ─── suru_array_clone_dyn ──────────────────────────────────────────────────────
;
; Clone an array. For scalar element types (elem_tag < 4), bitcopies the data
; buffer directly. For heap types (elem_tag >= 4), delegates each element to
; @suru_clone_dyn (which handles all heap type_tags via vtable dispatch).
define ptr @suru_array_clone_dyn(ptr %arr) {
entry:
  %tgep  = getelementptr %suru.Array, ptr %arr, i32 0, i32 0
  %ttag  = load i64, ptr %tgep
  %egep  = getelementptr %suru.Array, ptr %arr, i32 0, i32 1
  %etag  = load i64, ptr %egep
  %lgep  = getelementptr %suru.Array, ptr %arr, i32 0, i32 2
  %len   = load i64, ptr %lgep
  %dgep  = getelementptr %suru.Array, ptr %arr, i32 0, i32 4
  %sdat  = load ptr, ptr %dgep
  %esz   = call i64 @suru_array_elem_size(i64 %etag)
  %bc    = mul i64 %len, %esz
  %nh    = call ptr @malloc(i64 40)
  %nd    = call ptr @malloc(i64 %bc)
  %iptr  = alloca i64
  %isscl = icmp ult i64 %etag, 4
  br i1 %isscl, label %scalar_copy, label %heap_loop
scalar_copy:
  call ptr @memcpy(ptr %nd, ptr %sdat, i64 %bc)
  br label %after
heap_loop:
  store i64 0, ptr %iptr
  br label %cond
cond:
  %i    = load i64, ptr %iptr
  %done = icmp eq i64 %i, %len
  br i1 %done, label %after, label %body
body:
  %ss   = getelementptr i64, ptr %sdat, i64 %i
  %ri64 = load i64, ptr %ss
  %ep   = inttoptr i64 %ri64 to ptr
  %ec   = call ptr @suru_clone_dyn(ptr %ep)
  %eci  = ptrtoint ptr %ec to i64
  %ds   = getelementptr i64, ptr %nd, i64 %i
  store i64 %eci, ptr %ds
  br label %next
next:
  %ni   = add i64 %i, 1
  store i64 %ni, ptr %iptr
  br label %cond
after:
  %tgg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 0
  store i64 %ttag, ptr %tgg
  %egg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 1
  store i64 %etag, ptr %egg
  %lgg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 2
  store i64 %len, ptr %lgg
  %cgg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 3
  store i64 %len, ptr %cgg
  %dgg  = getelementptr %suru.Array, ptr %nh, i32 0, i32 4
  store ptr %nd, ptr %dgg
  ret ptr %nh
}

; ─── suru_array_drop_dyn ───────────────────────────────────────────────────────
;
; Drop an array. For scalar element types (elem_tag < 4), skips element drops
; and frees the buffer directly. For heap types (elem_tag >= 4), delegates each
; element to @suru_drop_dyn before freeing. Then frees the array header.
define void @suru_array_drop_dyn(ptr %arr) {
entry:
  %egep  = getelementptr %suru.Array, ptr %arr, i32 0, i32 1
  %etag  = load i64, ptr %egep
  %lgep  = getelementptr %suru.Array, ptr %arr, i32 0, i32 2
  %len   = load i64, ptr %lgep
  %dgep  = getelementptr %suru.Array, ptr %arr, i32 0, i32 4
  %data  = load ptr, ptr %dgep
  %iptr  = alloca i64
  %isscl = icmp ult i64 %etag, 4
  br i1 %isscl, label %after, label %heap_loop
heap_loop:
  store i64 0, ptr %iptr
  br label %cond
cond:
  %i    = load i64, ptr %iptr
  %done = icmp eq i64 %i, %len
  br i1 %done, label %after, label %body
body:
  %slot = getelementptr i64, ptr %data, i64 %i
  %ri64 = load i64, ptr %slot
  %ep   = inttoptr i64 %ri64 to ptr
  call void @suru_drop_dyn(ptr %ep)
  br label %next
next:
  %ni   = add i64 %i, 1
  store i64 %ni, ptr %iptr
  br label %cond
after:
  call void @free(ptr %data)
  call void @free(ptr %arr)
  ret void
}
