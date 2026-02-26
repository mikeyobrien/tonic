use crate::ir::{CmpKind, IrCallTarget, IrForGenerator, IrOp, IrPattern};
use crate::llvm_backend::mangle_function_name;
use crate::mir::{MirInstruction, MirProgram};
use std::collections::BTreeMap;

use super::{
    error::CBackendError,
    hash::{
        closure_capture_names, hash_closure_descriptor_i64, hash_ir_op_i64, hash_pattern_i64,
        hash_text_i64,
    },
    runtime_patterns::emit_runtime_pattern_helpers,
};

/// Emit the C file preamble: include directives and typedef.
pub(super) fn emit_header(out: &mut String) {
    out.push_str("/* tonic c backend - generated file */\n");
    out.push_str("#include <stdio.h>\n");
    out.push_str("#include <stdlib.h>\n");
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <inttypes.h>\n");
    out.push_str("#include <string.h>\n");
    out.push_str("#include <stdarg.h>\n");
    out.push_str("#include <errno.h>\n");
    out.push_str("#include <limits.h>\n");
    out.push_str("#include <sys/stat.h>\n");
    out.push_str("#include <sys/wait.h>\n");
    out.push_str("#include <unistd.h>\n");
    out.push('\n');
    out.push_str("typedef int64_t TnVal;\n");
    out.push('\n');
}

/// Emit runtime helper definitions for the generated C program.
///
/// Task 05 helpers are implemented inline; unsupported helpers remain explicit
/// abort stubs so failures stay deterministic.
pub(super) fn emit_runtime_stubs(mir: &MirProgram, out: &mut String) -> Result<(), CBackendError> {
    out.push_str(
        r###"/* runtime helpers */
static TnVal tn_stub_abort(const char *name) {
  fprintf(stderr, "error: native runtime not available for '%s'\n", name);
  exit(1);
}

typedef enum {
  TN_OBJ_BOOL = 1,
  TN_OBJ_NIL,
  TN_OBJ_ATOM,
  TN_OBJ_STRING,
  TN_OBJ_FLOAT,
  TN_OBJ_TUPLE,
  TN_OBJ_LIST,
  TN_OBJ_MAP,
  TN_OBJ_KEYWORD,
  TN_OBJ_RANGE,
  TN_OBJ_RESULT,
  TN_OBJ_CLOSURE
} TnObjKind;

typedef struct {
  TnVal key;
  TnVal value;
} TnPair;

typedef struct TnObj {
  TnObjKind kind;
  uint64_t alloc_id;
  uint32_t gc_flags;
  uint32_t refcount;
  union {
    int bool_value;
    struct {
      char *text;
    } text;
    struct {
      TnVal left;
      TnVal right;
    } tuple;
    struct {
      size_t len;
      TnVal *items;
    } list;
    struct {
      size_t len;
      TnPair *items;
    } map_like;
    struct {
      TnVal start;
      TnVal end;
    } range;
    struct {
      int is_ok;
      TnVal value;
    } result;
    struct {
      TnVal descriptor_hash;
      TnVal param_count;
      TnVal capture_count;
    } closure;
  } as;
} TnObj;

static TnObj **tn_heap = NULL;
static size_t tn_heap_len = 0;
static size_t tn_heap_cap = 0;
static size_t *tn_heap_free_ids = NULL;
static size_t tn_heap_free_len = 0;
static size_t tn_heap_free_cap = 0;

static TnVal *tn_root_stack = NULL;
static size_t tn_root_stack_len = 0;
static size_t tn_root_stack_cap = 0;

static uint64_t tn_memory_objects_total = 0;
static uint64_t tn_memory_reclaims_total = 0;
static uint64_t tn_memory_object_alloc_id_high_water = 0;
static uint64_t tn_memory_heap_slots_high_water = 0;
static uint64_t tn_memory_heap_capacity_high_water = 0;
static uint64_t tn_memory_heap_live_slots = 0;
static uint64_t tn_memory_heap_live_slots_high_water = 0;
static uint64_t tn_memory_roots_registered_total = 0;
static uint64_t tn_memory_root_frames_active = 0;
static uint64_t tn_memory_root_frames_high_water = 0;
static uint64_t tn_memory_root_slots_high_water = 0;
static uint64_t tn_memory_next_alloc_id = 1;
static uint64_t tn_memory_gc_collections_total = 0;
static int tn_memory_stats_enabled = -1;
static int tn_memory_rc_enabled = -1;
static int tn_memory_trace_enabled = -1;

static const uint32_t TN_GC_FLAG_MARK = UINT32_C(1);

static int tn_is_boxed(TnVal value);
static void tn_runtime_retain(TnVal value);
static void tn_runtime_release(TnVal value);
static int tn_runtime_memory_trace_enabled(void);
static const char *tn_runtime_memory_mode_label(void);
static const char *tn_runtime_cycle_collection_label(void);
static void tn_runtime_gc_mark_value(TnVal value);
static void tn_runtime_gc_collect(void);

static int tn_runtime_memory_stats_enabled(void) {
  if (tn_memory_stats_enabled >= 0) {
    return tn_memory_stats_enabled;
  }

  const char *value = getenv("TONIC_MEMORY_STATS");
  tn_memory_stats_enabled =
      (value != NULL && strcmp(value, "1") == 0) ? 1 : 0;
  return tn_memory_stats_enabled;
}

static int tn_runtime_memory_rc_enabled(void) {
  if (tn_memory_rc_enabled >= 0) {
    return tn_memory_rc_enabled;
  }

  const char *mode = getenv("TONIC_MEMORY_MODE");
  tn_memory_rc_enabled =
      (mode != NULL && strcmp(mode, "rc") == 0) ? 1 : 0;
  return tn_memory_rc_enabled;
}

static int tn_runtime_memory_trace_enabled(void) {
  if (tn_memory_trace_enabled >= 0) {
    return tn_memory_trace_enabled;
  }

  const char *mode = getenv("TONIC_MEMORY_MODE");
  tn_memory_trace_enabled =
      (mode == NULL || mode[0] == '\0' || strcmp(mode, "trace") == 0) ? 1 : 0;
  return tn_memory_trace_enabled;
}

static const char *tn_runtime_memory_mode_label(void) {
  if (tn_runtime_memory_trace_enabled()) {
    return "trace";
  }
  if (tn_runtime_memory_rc_enabled()) {
    return "rc";
  }
  return "append_only";
}

static const char *tn_runtime_cycle_collection_label(void) {
  return tn_runtime_memory_trace_enabled() ? "mark_sweep" : "off";
}

static void tn_runtime_memory_stats_print(void) {
  if (!tn_runtime_memory_stats_enabled()) {
    return;
  }

  if (tn_runtime_memory_trace_enabled()) {
    tn_runtime_gc_collect();
  }

  const char *memory_mode = tn_runtime_memory_mode_label();
  const char *cycle_collection = tn_runtime_cycle_collection_label();

  fprintf(
      stderr,
      "memory.stats c_runtime memory_mode=%s"
      " cycle_collection=%s"
      " objects_total=%" PRIu64
      " reclaims_total=%" PRIu64
      " gc_collections_total=%" PRIu64
      " heap_slots=%zu heap_slots_hwm=%" PRIu64
      " heap_live_slots=%" PRIu64 " heap_live_slots_hwm=%" PRIu64
      " heap_capacity=%zu heap_capacity_hwm=%" PRIu64
      " object_alloc_id_hwm=%" PRIu64
      " roots_registered_total=%" PRIu64
      " root_frames_active=%" PRIu64
      " root_frames_hwm=%" PRIu64
      " root_slots=%zu root_slots_hwm=%" PRIu64 "\n",
      memory_mode,
      cycle_collection,
      tn_memory_objects_total,
      tn_memory_reclaims_total,
      tn_memory_gc_collections_total,
      tn_heap_len,
      tn_memory_heap_slots_high_water,
      tn_memory_heap_live_slots,
      tn_memory_heap_live_slots_high_water,
      tn_heap_cap,
      tn_memory_heap_capacity_high_water,
      tn_memory_object_alloc_id_high_water,
      tn_memory_roots_registered_total,
      tn_memory_root_frames_active,
      tn_memory_root_frames_high_water,
      tn_root_stack_len,
      tn_memory_root_slots_high_water);
}

static size_t tn_runtime_root_frame_push(void) {
  tn_memory_root_frames_active += 1;
  if (tn_memory_root_frames_high_water < tn_memory_root_frames_active) {
    tn_memory_root_frames_high_water = tn_memory_root_frames_active;
  }
  return tn_root_stack_len;
}

static void tn_runtime_root_register(TnVal value) {
  if (!tn_is_boxed(value)) {
    return;
  }

  if (tn_root_stack_len == tn_root_stack_cap) {
    size_t next_cap = tn_root_stack_cap == 0 ? 64 : tn_root_stack_cap * 2;
    TnVal *next_stack = (TnVal *)realloc(tn_root_stack, next_cap * sizeof(TnVal));
    if (next_stack == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }

    tn_root_stack = next_stack;
    tn_root_stack_cap = next_cap;
  }

  tn_root_stack[tn_root_stack_len] = value;
  tn_root_stack_len += 1;
  tn_memory_roots_registered_total += 1;
  if (tn_memory_root_slots_high_water < tn_root_stack_len) {
    tn_memory_root_slots_high_water = (uint64_t)tn_root_stack_len;
  }

  tn_runtime_retain(value);
}

static void tn_runtime_root_frame_pop(size_t frame_start) {
  if (frame_start > tn_root_stack_len) {
    fprintf(stderr, "error: native runtime root frame corruption\n");
    exit(1);
  }

  if (tn_memory_root_frames_active == 0) {
    fprintf(stderr, "error: native runtime root frame underflow\n");
    exit(1);
  }

  if (tn_runtime_memory_rc_enabled()) {
    for (size_t i = tn_root_stack_len; i > frame_start; i -= 1) {
      tn_runtime_release(tn_root_stack[i - 1]);
    }
  }

  tn_root_stack_len = frame_start;
  tn_memory_root_frames_active -= 1;
}

static TnVal tn_true_value = 0;
static TnVal tn_false_value = 0;
static TnVal tn_nil_value = 0;

static const uint64_t TN_BOX_TAG = UINT64_C(0x7ff0000000000000);
static const uint64_t TN_BOX_PAYLOAD_MASK = UINT64_C(0x0000ffffffffffff);

static int tn_is_boxed(TnVal value) {
  return ((((uint64_t)value) & TN_BOX_TAG) == TN_BOX_TAG) != 0;
}

static size_t tn_box_id(TnVal value) {
  return (size_t)(((uint64_t)value) & TN_BOX_PAYLOAD_MASK);
}

static TnVal tn_make_box(size_t id) {
  return (TnVal)(TN_BOX_TAG | (uint64_t)id);
}

static int tn_runtime_is_truthy(TnVal value);
static int tn_runtime_value_equal(TnVal left, TnVal right);
static const char *tn_runtime_value_kind(TnVal value);
static TnVal tn_runtime_guard_is_integer(TnVal value);
static TnVal tn_runtime_guard_is_float(TnVal value);
static TnVal tn_runtime_guard_is_number(TnVal value);
static TnVal tn_runtime_guard_is_atom(TnVal value);
static TnVal tn_runtime_guard_is_binary(TnVal value);
static TnVal tn_runtime_guard_is_list(TnVal value);
static TnVal tn_runtime_guard_is_tuple(TnVal value);
static TnVal tn_runtime_guard_is_map(TnVal value);
static TnVal tn_runtime_guard_is_nil(TnVal value);
static void tn_render_value(FILE *out, TnVal value);

static TnVal tn_runtime_fail(const char *message) {
  fprintf(stderr, "error: %s\n", message);
  exit(1);
}

static TnVal tn_runtime_failf(const char *format, ...) {
  fprintf(stderr, "error: ");
  va_list args;
  va_start(args, format);
  vfprintf(stderr, format, args);
  va_end(args);
  fputc('\n', stderr);
  exit(1);
}

static char *tn_strdup_or_die(const char *value) {
  size_t len = strlen(value);
  char *copy = (char *)malloc(len + 1);
  if (copy == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  memcpy(copy, value, len + 1);
  return copy;
}

static TnObj *tn_new_obj(TnObjKind kind) {
  TnObj *obj = (TnObj *)calloc(1, sizeof(TnObj));
  if (obj == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  obj->kind = kind;
  obj->gc_flags = 0;
  obj->refcount = 0;
  obj->alloc_id = tn_memory_next_alloc_id;
  if (tn_memory_next_alloc_id < UINT64_MAX) {
    tn_memory_next_alloc_id += 1;
  }
  if (tn_memory_object_alloc_id_high_water < obj->alloc_id) {
    tn_memory_object_alloc_id_high_water = obj->alloc_id;
  }

  tn_memory_objects_total += 1;
  return obj;
}

static void tn_heap_push_free_id(size_t id) {
  if (tn_heap_free_len == tn_heap_free_cap) {
    size_t next_cap = tn_heap_free_cap == 0 ? 64 : tn_heap_free_cap * 2;
    size_t *next_free_ids =
        (size_t *)realloc(tn_heap_free_ids, next_cap * sizeof(size_t));
    if (next_free_ids == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }

    tn_heap_free_ids = next_free_ids;
    tn_heap_free_cap = next_cap;
  }

  tn_heap_free_ids[tn_heap_free_len] = id;
  tn_heap_free_len += 1;
}

static TnVal tn_heap_store(TnObj *obj) {
  size_t id = 0;

  if (tn_heap_free_len > 0) {
    id = tn_heap_free_ids[tn_heap_free_len - 1];
    tn_heap_free_len -= 1;
    tn_heap[id - 1] = obj;
  } else {
    if (tn_heap_len == tn_heap_cap) {
      size_t next_cap = tn_heap_cap == 0 ? 64 : tn_heap_cap * 2;
      TnObj **next_heap = (TnObj **)realloc(tn_heap, next_cap * sizeof(TnObj *));
      if (next_heap == NULL) {
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }

      tn_heap = next_heap;
      tn_heap_cap = next_cap;
      if (tn_memory_heap_capacity_high_water < tn_heap_cap) {
        tn_memory_heap_capacity_high_water = (uint64_t)tn_heap_cap;
      }
    }

    tn_heap[tn_heap_len] = obj;
    tn_heap_len += 1;
    if (tn_memory_heap_slots_high_water < tn_heap_len) {
      tn_memory_heap_slots_high_water = (uint64_t)tn_heap_len;
    }
    id = tn_heap_len;
  }

  tn_memory_heap_live_slots += 1;
  if (tn_memory_heap_live_slots_high_water < tn_memory_heap_live_slots) {
    tn_memory_heap_live_slots_high_water = tn_memory_heap_live_slots;
  }

  return tn_make_box(id);
}

static TnObj *tn_get_obj(TnVal value) {
  if (!tn_is_boxed(value)) {
    return NULL;
  }

  size_t id = tn_box_id(value);
  if (id == 0 || id > tn_heap_len) {
    return NULL;
  }

  return tn_heap[id - 1];
}

static void tn_runtime_gc_free_payload(TnObj *obj) {
  switch (obj->kind) {
    case TN_OBJ_ATOM:
    case TN_OBJ_STRING:
    case TN_OBJ_FLOAT:
      free(obj->as.text.text);
      return;
    case TN_OBJ_LIST:
      free(obj->as.list.items);
      return;
    case TN_OBJ_MAP:
    case TN_OBJ_KEYWORD:
      free(obj->as.map_like.items);
      return;
    case TN_OBJ_BOOL:
    case TN_OBJ_NIL:
    case TN_OBJ_TUPLE:
    case TN_OBJ_RANGE:
    case TN_OBJ_RESULT:
    case TN_OBJ_CLOSURE:
      return;
  }
}

static void tn_runtime_gc_mark_value(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  if (obj == NULL || (obj->gc_flags & TN_GC_FLAG_MARK) != 0) {
    return;
  }

  obj->gc_flags |= TN_GC_FLAG_MARK;

  switch (obj->kind) {
    case TN_OBJ_TUPLE:
      tn_runtime_gc_mark_value(obj->as.tuple.left);
      tn_runtime_gc_mark_value(obj->as.tuple.right);
      return;
    case TN_OBJ_LIST:
      for (size_t i = 0; i < obj->as.list.len; i += 1) {
        tn_runtime_gc_mark_value(obj->as.list.items[i]);
      }
      return;
    case TN_OBJ_MAP:
    case TN_OBJ_KEYWORD:
      for (size_t i = 0; i < obj->as.map_like.len; i += 1) {
        tn_runtime_gc_mark_value(obj->as.map_like.items[i].key);
        tn_runtime_gc_mark_value(obj->as.map_like.items[i].value);
      }
      return;
    case TN_OBJ_RANGE:
      tn_runtime_gc_mark_value(obj->as.range.start);
      tn_runtime_gc_mark_value(obj->as.range.end);
      return;
    case TN_OBJ_RESULT:
      tn_runtime_gc_mark_value(obj->as.result.value);
      return;
    case TN_OBJ_BOOL:
    case TN_OBJ_NIL:
    case TN_OBJ_ATOM:
    case TN_OBJ_STRING:
    case TN_OBJ_FLOAT:
    case TN_OBJ_CLOSURE:
      return;
  }
}

static void tn_runtime_gc_collect(void) {
  if (!tn_runtime_memory_trace_enabled()) {
    return;
  }

  if (tn_true_value != 0) {
    tn_runtime_gc_mark_value(tn_true_value);
  }
  if (tn_false_value != 0) {
    tn_runtime_gc_mark_value(tn_false_value);
  }
  if (tn_nil_value != 0) {
    tn_runtime_gc_mark_value(tn_nil_value);
  }
  for (size_t i = 0; i < tn_root_stack_len; i += 1) {
    tn_runtime_gc_mark_value(tn_root_stack[i]);
  }

  for (size_t i = 0; i < tn_heap_len; i += 1) {
    TnObj *obj = tn_heap[i];
    if (obj == NULL) {
      continue;
    }

    if ((obj->gc_flags & TN_GC_FLAG_MARK) != 0) {
      obj->gc_flags &= ~TN_GC_FLAG_MARK;
      continue;
    }

    tn_heap[i] = NULL;
    tn_heap_push_free_id(i + 1);
    if (tn_memory_heap_live_slots > 0) {
      tn_memory_heap_live_slots -= 1;
    }
    tn_memory_reclaims_total += 1;

    tn_runtime_gc_free_payload(obj);
    free(obj);
  }

  tn_memory_gc_collections_total += 1;
}

static TnObj *tn_get_obj_for_rc(TnVal value, const char *action, size_t *id_out) {
  if (!tn_is_boxed(value)) {
    return NULL;
  }

  size_t id = tn_box_id(value);
  if (id == 0 || id > tn_heap_len) {
    fprintf(stderr,
            "error: native runtime ownership misuse during %s: unknown object id %zu\n",
            action,
            id);
    exit(1);
  }

  TnObj *obj = tn_heap[id - 1];
  if (obj == NULL) {
    fprintf(stderr,
            "error: native runtime ownership misuse during %s: object id %zu already reclaimed\n",
            action,
            id);
    exit(1);
  }

  if (id_out != NULL) {
    *id_out = id;
  }
  return obj;
}

static void tn_runtime_retain(TnVal value) {
  if (!tn_runtime_memory_rc_enabled() || !tn_is_boxed(value)) {
    return;
  }

  TnObj *obj = tn_get_obj_for_rc(value, "retain", NULL);
  if (obj->refcount < UINT32_MAX) {
    obj->refcount += 1;
  }
}

static void tn_runtime_release(TnVal value) {
  if (!tn_runtime_memory_rc_enabled() || !tn_is_boxed(value)) {
    return;
  }

  size_t id = 0;
  TnObj *obj = tn_get_obj_for_rc(value, "release", &id);
  if (obj->refcount == 0) {
    fprintf(stderr,
            "error: native runtime ownership misuse during release: object id %zu already released\n",
            id);
    exit(1);
  }

  obj->refcount -= 1;
  if (obj->refcount > 0) {
    return;
  }

  tn_heap[id - 1] = NULL;
  tn_heap_push_free_id(id);
  if (tn_memory_heap_live_slots > 0) {
    tn_memory_heap_live_slots -= 1;
  }
  tn_memory_reclaims_total += 1;

  TnVal self_value = tn_make_box(id);

  switch (obj->kind) {
    case TN_OBJ_ATOM:
    case TN_OBJ_STRING:
    case TN_OBJ_FLOAT:
      free(obj->as.text.text);
      break;
    case TN_OBJ_TUPLE:
      if (obj->as.tuple.left != self_value) {
        tn_runtime_release(obj->as.tuple.left);
      }
      if (obj->as.tuple.right != self_value) {
        tn_runtime_release(obj->as.tuple.right);
      }
      break;
    case TN_OBJ_LIST:
      for (size_t i = 0; i < obj->as.list.len; i += 1) {
        if (obj->as.list.items[i] != self_value) {
          tn_runtime_release(obj->as.list.items[i]);
        }
      }
      free(obj->as.list.items);
      break;
    case TN_OBJ_MAP:
    case TN_OBJ_KEYWORD:
      for (size_t i = 0; i < obj->as.map_like.len; i += 1) {
        if (obj->as.map_like.items[i].key != self_value) {
          tn_runtime_release(obj->as.map_like.items[i].key);
        }
        if (obj->as.map_like.items[i].value != self_value) {
          tn_runtime_release(obj->as.map_like.items[i].value);
        }
      }
      free(obj->as.map_like.items);
      break;
    case TN_OBJ_RANGE:
      if (obj->as.range.start != self_value) {
        tn_runtime_release(obj->as.range.start);
      }
      if (obj->as.range.end != self_value) {
        tn_runtime_release(obj->as.range.end);
      }
      break;
    case TN_OBJ_RESULT:
      if (obj->as.result.value != self_value) {
        tn_runtime_release(obj->as.result.value);
      }
      break;
    case TN_OBJ_BOOL:
    case TN_OBJ_NIL:
    case TN_OBJ_CLOSURE:
      break;
  }

  free(obj);
}

static void tn_runtime_init_singletons(void) {
  if (tn_true_value != 0) {
    return;
  }

  TnObj *true_obj = tn_new_obj(TN_OBJ_BOOL);
  true_obj->as.bool_value = 1;
  tn_true_value = tn_heap_store(true_obj);
  tn_runtime_retain(tn_true_value);

  TnObj *false_obj = tn_new_obj(TN_OBJ_BOOL);
  false_obj->as.bool_value = 0;
  tn_false_value = tn_heap_store(false_obj);
  tn_runtime_retain(tn_false_value);

  TnObj *nil_obj = tn_new_obj(TN_OBJ_NIL);
  tn_nil_value = tn_heap_store(nil_obj);
  tn_runtime_retain(tn_nil_value);
}

static TnVal tn_runtime_const_bool(TnVal raw) {
  tn_runtime_init_singletons();
  return raw != 0 ? tn_true_value : tn_false_value;
}

static TnVal tn_runtime_const_nil(void) {
  tn_runtime_init_singletons();
  return tn_nil_value;
}

static TnVal tn_runtime_const_atom(TnVal raw) {
  const char *text = (const char *)(intptr_t)raw;
  TnObj *obj = tn_new_obj(TN_OBJ_ATOM);
  obj->as.text.text = tn_strdup_or_die(text);
  return tn_heap_store(obj);
}

static TnVal tn_runtime_const_string(TnVal raw) {
  const char *text = (const char *)(intptr_t)raw;
  TnObj *obj = tn_new_obj(TN_OBJ_STRING);
  obj->as.text.text = tn_strdup_or_die(text);
  return tn_heap_store(obj);
}

static TnVal tn_runtime_const_float(TnVal raw) {
  const char *text = (const char *)(intptr_t)raw;
  TnObj *obj = tn_new_obj(TN_OBJ_FLOAT);
  obj->as.text.text = tn_strdup_or_die(text);
  return tn_heap_store(obj);
}

static TnVal tn_runtime_make_tuple(TnVal left, TnVal right) {
  TnObj *obj = tn_new_obj(TN_OBJ_TUPLE);
  obj->as.tuple.left = left;
  obj->as.tuple.right = right;
  tn_runtime_retain(left);
  tn_runtime_retain(right);
  return tn_heap_store(obj);
}

static TnVal tn_runtime_make_list_varargs(TnVal count, ...) {
  if (count < 0) {
    return tn_stub_abort("tn_runtime_make_list");
  }

  size_t len = (size_t)count;
  TnObj *obj = tn_new_obj(TN_OBJ_LIST);
  obj->as.list.len = len;
  obj->as.list.items = len == 0 ? NULL : (TnVal *)calloc(len, sizeof(TnVal));
  if (len > 0 && obj->as.list.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  va_list args;
  va_start(args, count);
  for (size_t i = 0; i < len; i += 1) {
    obj->as.list.items[i] = va_arg(args, TnVal);
    tn_runtime_retain(obj->as.list.items[i]);
  }
  va_end(args);

  return tn_heap_store(obj);
}

static TnVal tn_runtime_range(TnVal left, TnVal right) {
  if (tn_is_boxed(left) || tn_is_boxed(right)) {
    return tn_stub_abort("tn_runtime_range");
  }

  TnObj *obj = tn_new_obj(TN_OBJ_RANGE);
  obj->as.range.start = left;
  obj->as.range.end = right;
  return tn_heap_store(obj);
}

static int tn_runtime_is_truthy(TnVal value) {
  tn_runtime_init_singletons();
  return value != tn_false_value && value != tn_nil_value;
}

static int tn_runtime_value_equal(TnVal left, TnVal right) {
  if (left == right) {
    return 1;
  }

  int left_boxed = tn_is_boxed(left);
  int right_boxed = tn_is_boxed(right);
  if (!left_boxed && !right_boxed) {
    return left == right;
  }

  if (left_boxed != right_boxed) {
    return 0;
  }

  TnObj *left_obj = tn_get_obj(left);
  TnObj *right_obj = tn_get_obj(right);
  if (left_obj == NULL || right_obj == NULL || left_obj->kind != right_obj->kind) {
    return 0;
  }

  switch (left_obj->kind) {
    case TN_OBJ_BOOL:
      return left_obj->as.bool_value == right_obj->as.bool_value;
    case TN_OBJ_NIL:
      return 1;
    case TN_OBJ_ATOM:
    case TN_OBJ_STRING:
    case TN_OBJ_FLOAT:
      return strcmp(left_obj->as.text.text, right_obj->as.text.text) == 0;
    case TN_OBJ_TUPLE:
      return tn_runtime_value_equal(left_obj->as.tuple.left, right_obj->as.tuple.left) &&
             tn_runtime_value_equal(left_obj->as.tuple.right, right_obj->as.tuple.right);
    case TN_OBJ_LIST:
      if (left_obj->as.list.len != right_obj->as.list.len) {
        return 0;
      }
      for (size_t i = 0; i < left_obj->as.list.len; i += 1) {
        if (!tn_runtime_value_equal(left_obj->as.list.items[i], right_obj->as.list.items[i])) {
          return 0;
        }
      }
      return 1;
    case TN_OBJ_MAP:
    case TN_OBJ_KEYWORD:
      if (left_obj->as.map_like.len != right_obj->as.map_like.len) {
        return 0;
      }
      for (size_t i = 0; i < left_obj->as.map_like.len; i += 1) {
        if (!tn_runtime_value_equal(left_obj->as.map_like.items[i].key,
                                    right_obj->as.map_like.items[i].key) ||
            !tn_runtime_value_equal(left_obj->as.map_like.items[i].value,
                                    right_obj->as.map_like.items[i].value)) {
          return 0;
        }
      }
      return 1;
    case TN_OBJ_RANGE:
      return left_obj->as.range.start == right_obj->as.range.start &&
             left_obj->as.range.end == right_obj->as.range.end;
    case TN_OBJ_RESULT:
      return left_obj->as.result.is_ok == right_obj->as.result.is_ok &&
             tn_runtime_value_equal(left_obj->as.result.value, right_obj->as.result.value);
    default:
      return 0;
  }
}

static const char *tn_runtime_value_kind(TnVal value) {
  if (!tn_is_boxed(value)) {
    return "int";
  }

  TnObj *obj = tn_get_obj(value);
  if (obj == NULL) {
    return "unknown";
  }

  switch (obj->kind) {
    case TN_OBJ_BOOL:
      return "bool";
    case TN_OBJ_NIL:
      return "nil";
    case TN_OBJ_ATOM:
      return "atom";
    case TN_OBJ_STRING:
      return "string";
    case TN_OBJ_FLOAT:
      return "float";
    case TN_OBJ_TUPLE:
      return "tuple";
    case TN_OBJ_LIST:
      return "list";
    case TN_OBJ_MAP:
      return "map";
    case TN_OBJ_KEYWORD:
      return "keyword";
    case TN_OBJ_RANGE:
      return "range";
    case TN_OBJ_RESULT:
      return "result";
    case TN_OBJ_CLOSURE:
      return "function";
    default:
      return "unknown";
  }
}

static TnVal tn_runtime_guard_is_integer(TnVal value) {
  return tn_is_boxed(value) ? 0 : 1;
}

static TnVal tn_runtime_guard_is_float(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  return (obj != NULL && obj->kind == TN_OBJ_FLOAT) ? 1 : 0;
}

static TnVal tn_runtime_guard_is_number(TnVal value) {
  if (!tn_is_boxed(value)) {
    return 1;
  }

  TnObj *obj = tn_get_obj(value);
  return (obj != NULL && obj->kind == TN_OBJ_FLOAT) ? 1 : 0;
}

static TnVal tn_runtime_guard_is_atom(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  return (obj != NULL && obj->kind == TN_OBJ_ATOM) ? 1 : 0;
}

static TnVal tn_runtime_guard_is_binary(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  return (obj != NULL && obj->kind == TN_OBJ_STRING) ? 1 : 0;
}

static TnVal tn_runtime_guard_is_list(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  return (obj != NULL && (obj->kind == TN_OBJ_LIST || obj->kind == TN_OBJ_KEYWORD)) ? 1 : 0;
}

static TnVal tn_runtime_guard_is_tuple(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  return (obj != NULL && obj->kind == TN_OBJ_TUPLE) ? 1 : 0;
}

static TnVal tn_runtime_guard_is_map(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  return (obj != NULL && obj->kind == TN_OBJ_MAP) ? 1 : 0;
}

static TnVal tn_runtime_guard_is_nil(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  return (obj != NULL && obj->kind == TN_OBJ_NIL) ? 1 : 0;
}

static long tn_map_like_find_index(const TnObj *map_like, TnVal key) {
  for (size_t i = 0; i < map_like->as.map_like.len; i += 1) {
    if (tn_runtime_value_equal(map_like->as.map_like.items[i].key, key)) {
      return (long)i;
    }
  }

  return -1;
}

static TnVal tn_clone_map_like_with_capacity(const TnObj *source, TnObjKind kind, size_t extra) {
  TnObj *obj = tn_new_obj(kind);
  obj->as.map_like.len = source->as.map_like.len;
  size_t cap = source->as.map_like.len + extra;
  obj->as.map_like.items = cap == 0 ? NULL : (TnPair *)calloc(cap, sizeof(TnPair));
  if (cap > 0 && obj->as.map_like.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  for (size_t i = 0; i < source->as.map_like.len; i += 1) {
    obj->as.map_like.items[i] = source->as.map_like.items[i];
    tn_runtime_retain(obj->as.map_like.items[i].key);
    tn_runtime_retain(obj->as.map_like.items[i].value);
  }

  return tn_heap_store(obj);
}

static TnVal tn_runtime_map_empty(void) {
  TnObj *obj = tn_new_obj(TN_OBJ_MAP);
  obj->as.map_like.len = 0;
  obj->as.map_like.items = NULL;
  return tn_heap_store(obj);
}

static TnVal tn_runtime_make_map(TnVal key, TnVal value) {
  TnObj *obj = tn_new_obj(TN_OBJ_MAP);
  obj->as.map_like.len = 1;
  obj->as.map_like.items = (TnPair *)calloc(1, sizeof(TnPair));
  if (obj->as.map_like.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  obj->as.map_like.items[0].key = key;
  obj->as.map_like.items[0].value = value;
  tn_runtime_retain(key);
  tn_runtime_retain(value);
  return tn_heap_store(obj);
}

static TnVal tn_runtime_map_put(TnVal base, TnVal key, TnVal value) {
  TnObj *map = tn_get_obj(base);
  if (map == NULL || map->kind != TN_OBJ_MAP) {
    return tn_stub_abort("tn_runtime_map_put");
  }

  long existing_index = tn_map_like_find_index(map, key);
  size_t extra = existing_index < 0 ? 1 : 0;
  TnVal cloned = tn_clone_map_like_with_capacity(map, TN_OBJ_MAP, extra);
  TnObj *next = tn_get_obj(cloned);

  if (existing_index >= 0) {
    tn_runtime_retain(value);
    tn_runtime_release(next->as.map_like.items[existing_index].value);
    next->as.map_like.items[existing_index].value = value;
  } else {
    size_t write_index = next->as.map_like.len;
    next->as.map_like.items[write_index].key = key;
    next->as.map_like.items[write_index].value = value;
    tn_runtime_retain(key);
    tn_runtime_retain(value);
    next->as.map_like.len += 1;
  }

  return cloned;
}

static TnVal tn_runtime_map_update(TnVal base, TnVal key, TnVal value) {
  TnObj *map = tn_get_obj(base);
  if (map == NULL || map->kind != TN_OBJ_MAP) {
    return tn_stub_abort("tn_runtime_map_update");
  }

  long existing_index = tn_map_like_find_index(map, key);
  if (existing_index < 0) {
    return tn_stub_abort("tn_runtime_map_update");
  }

  TnVal cloned = tn_clone_map_like_with_capacity(map, TN_OBJ_MAP, 0);
  TnObj *next = tn_get_obj(cloned);
  tn_runtime_retain(value);
  tn_runtime_release(next->as.map_like.items[existing_index].value);
  next->as.map_like.items[existing_index].value = value;
  return cloned;
}

static TnVal tn_runtime_map_access(TnVal base, TnVal key) {
  tn_runtime_init_singletons();

  TnObj *map = tn_get_obj(base);
  if (map == NULL || map->kind != TN_OBJ_MAP) {
    return tn_stub_abort("tn_runtime_map_access");
  }

  long existing_index = tn_map_like_find_index(map, key);
  if (existing_index < 0) {
    return tn_nil_value;
  }

  TnVal value = map->as.map_like.items[existing_index].value;
  tn_runtime_retain(value);
  return value;
}

static TnVal tn_runtime_make_keyword(TnVal key, TnVal value) {
  TnObj *obj = tn_new_obj(TN_OBJ_KEYWORD);
  obj->as.map_like.len = 1;
  obj->as.map_like.items = (TnPair *)calloc(1, sizeof(TnPair));
  if (obj->as.map_like.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  obj->as.map_like.items[0].key = key;
  obj->as.map_like.items[0].value = value;
  tn_runtime_retain(key);
  tn_runtime_retain(value);
  return tn_heap_store(obj);
}

static TnVal tn_runtime_keyword_append(TnVal base, TnVal key, TnVal value) {
  TnObj *keyword = tn_get_obj(base);
  if (keyword == NULL || keyword->kind != TN_OBJ_KEYWORD) {
    return tn_stub_abort("tn_runtime_keyword_append");
  }

  TnVal cloned = tn_clone_map_like_with_capacity(keyword, TN_OBJ_KEYWORD, 1);
  TnObj *next = tn_get_obj(cloned);
  size_t write_index = next->as.map_like.len;
  next->as.map_like.items[write_index].key = key;
  next->as.map_like.items[write_index].value = value;
  tn_runtime_retain(key);
  tn_runtime_retain(value);
  next->as.map_like.len += 1;
  return cloned;
}

static TnVal tn_runtime_in(TnVal left, TnVal right) {
  TnObj *right_obj = tn_get_obj(right);
  if (right_obj == NULL) {
    return tn_stub_abort("tn_runtime_in");
  }

  if (right_obj->kind == TN_OBJ_LIST) {
    for (size_t i = 0; i < right_obj->as.list.len; i += 1) {
      if (tn_runtime_value_equal(left, right_obj->as.list.items[i])) {
        return tn_runtime_const_bool(1);
      }
    }
    return tn_runtime_const_bool(0);
  }

  if (right_obj->kind == TN_OBJ_RANGE) {
    if (tn_is_boxed(left)) {
      return tn_runtime_const_bool(0);
    }

    return tn_runtime_const_bool(
        (left >= right_obj->as.range.start && left <= right_obj->as.range.end) ? 1 : 0);
  }

  return tn_stub_abort("tn_runtime_in");
}

static void tn_render_value(FILE *out, TnVal value);

static void tn_render_keyword_key(FILE *out, TnVal key) {
  TnObj *key_obj = tn_get_obj(key);
  if (key_obj != NULL && key_obj->kind == TN_OBJ_ATOM) {
    fputs(key_obj->as.text.text, out);
    return;
  }

  tn_render_value(out, key);
}

static void tn_render_value(FILE *out, TnVal value) {
  if (!tn_is_boxed(value)) {
    fprintf(out, "%" PRId64, (int64_t)value);
    return;
  }

  TnObj *obj = tn_get_obj(value);
  if (obj == NULL) {
    fprintf(out, "%" PRId64, (int64_t)value);
    return;
  }

  switch (obj->kind) {
    case TN_OBJ_BOOL:
      fputs(obj->as.bool_value ? "true" : "false", out);
      return;
    case TN_OBJ_NIL:
      fputs("nil", out);
      return;
    case TN_OBJ_ATOM:
      fputc(':', out);
      fputs(obj->as.text.text, out);
      return;
    case TN_OBJ_STRING:
      fputc('"', out);
      fputs(obj->as.text.text, out);
      fputc('"', out);
      return;
    case TN_OBJ_FLOAT:
      fputs(obj->as.text.text, out);
      return;
    case TN_OBJ_TUPLE:
      fputc('{', out);
      tn_render_value(out, obj->as.tuple.left);
      fputs(", ", out);
      tn_render_value(out, obj->as.tuple.right);
      fputc('}', out);
      return;
    case TN_OBJ_LIST:
      fputc('[', out);
      for (size_t i = 0; i < obj->as.list.len; i += 1) {
        if (i > 0) {
          fputs(", ", out);
        }
        tn_render_value(out, obj->as.list.items[i]);
      }
      fputc(']', out);
      return;
    case TN_OBJ_MAP:
      fputs("%{", out);
      for (size_t i = 0; i < obj->as.map_like.len; i += 1) {
        if (i > 0) {
          fputs(", ", out);
        }
        tn_render_value(out, obj->as.map_like.items[i].key);
        fputs(" => ", out);
        tn_render_value(out, obj->as.map_like.items[i].value);
      }
      fputc('}', out);
      return;
    case TN_OBJ_KEYWORD:
      fputc('[', out);
      for (size_t i = 0; i < obj->as.map_like.len; i += 1) {
        if (i > 0) {
          fputs(", ", out);
        }
        tn_render_keyword_key(out, obj->as.map_like.items[i].key);
        fputs(": ", out);
        tn_render_value(out, obj->as.map_like.items[i].value);
      }
      fputc(']', out);
      return;
    case TN_OBJ_RANGE:
      tn_render_value(out, obj->as.range.start);
      fputs("..", out);
      tn_render_value(out, obj->as.range.end);
      return;
    case TN_OBJ_RESULT:
      fputs(obj->as.result.is_ok ? "ok(" : "err(", out);
      tn_render_value(out, obj->as.result.value);
      fputc(')', out);
      return;
    case TN_OBJ_CLOSURE:
      fprintf(out, "#Function<%" PRId64 ">", (int64_t)obj->as.closure.param_count);
      return;
    default:
      fputs("<unknown>", out);
      return;
  }
}

static void tn_runtime_println(TnVal value) {
  tn_render_value(stdout, value);
  fputc('\n', stdout);
}

"###,
    );

    out.push_str(
        "static TnVal tn_runtime_call_compiled_closure(TnVal descriptor_hash, const TnVal *argv, size_t argc);\n\n",
    );

    out.push_str(
        "static TnVal tn_runtime_make_closure(TnVal descriptor_hash, TnVal param_count, TnVal capture_count) {\n",
    );
    out.push_str("  TnObj *obj = tn_new_obj(TN_OBJ_CLOSURE);\n");
    out.push_str("  obj->as.closure.descriptor_hash = descriptor_hash;\n");
    out.push_str("  obj->as.closure.param_count = param_count;\n");
    out.push_str("  obj->as.closure.capture_count = capture_count;\n");
    out.push_str("  return tn_heap_store(obj);\n");
    out.push_str("}\n\n");

    out.push_str("static TnVal tn_runtime_protocol_dispatch(TnVal value) {\n");
    out.push_str("  TnObj *obj = tn_get_obj(value);\n");
    out.push_str("  if (obj != NULL && obj->kind == TN_OBJ_TUPLE) {\n");
    out.push_str("    return (TnVal)1;\n");
    out.push_str("  }\n");
    out.push_str("  if (obj != NULL && obj->kind == TN_OBJ_MAP) {\n");
    out.push_str("    return (TnVal)2;\n");
    out.push_str("  }\n");
    out.push_str(
        "  return tn_runtime_failf(\"protocol_dispatch has no implementation for %s\", tn_runtime_value_kind(value));\n",
    );
    out.push_str("}\n\n");

    out.push_str(
        "// Globals for sys_argv
int tn_global_argc = 0;
char **tn_global_argv = NULL;

static TnVal tn_runtime_host_call_varargs(TnVal count, ...) {\n",
    );
    out.push_str("  if (count < 1) {\n");
    out.push_str(
        "    return tn_runtime_fail(\"host_call requires at least 1 argument (host function key)\");\n",
    );
    out.push_str("  }\n\n");
    out.push_str("  size_t argc = (size_t)count;\n");
    out.push_str("  TnVal *args = (TnVal *)calloc(argc, sizeof(TnVal));\n");
    out.push_str("  if (args == NULL) {\n");
    out.push_str("    fprintf(stderr, \"error: native runtime allocation failure\\n\");\n");
    out.push_str("    exit(1);\n");
    out.push_str("  }\n\n");
    out.push_str("  va_list vargs;\n");
    out.push_str("  va_start(vargs, count);\n");
    out.push_str("  for (size_t i = 0; i < argc; i += 1) {\n");
    out.push_str("    args[i] = va_arg(vargs, TnVal);\n");
    out.push_str("  }\n");
    out.push_str("  va_end(vargs);\n\n");
    out.push_str("  TnObj *key_obj = tn_get_obj(args[0]);\n");
    out.push_str("  if (key_obj == NULL || key_obj->kind != TN_OBJ_ATOM) {\n");
    out.push_str(
        "    return tn_runtime_failf(\"host_call first argument must be an atom (host key), found %s\", tn_runtime_value_kind(args[0]));\n",
    );
    out.push_str("  }\n\n");
    out.push_str("  const char *key = key_obj->as.text.text;\n");
    out.push_str("  if (strcmp(key, \"identity\") == 0) {\n");
    out.push_str("    if (argc != 2) {\n");
    out.push_str(
        "      return tn_runtime_failf(\"host error: identity expects exactly 1 argument, found %zu\", argc - 1);\n",
    );
    out.push_str("    }\n");
    out.push_str("    TnVal result = args[1];\n");
    out.push_str("    free(args);\n");
    out.push_str("    return result;\n");
    out.push_str("  }\n\n");
    out.push_str("  if (strcmp(key, \"sum_ints\") == 0) {\n");
    out.push_str("    if (argc <= 1) {\n");
    out.push_str(
        "      return tn_runtime_fail(\"host error: sum_ints expects at least 1 argument\");\n",
    );
    out.push_str("    }\n");
    out.push_str("    int64_t total = 0;\n");
    out.push_str("    for (size_t i = 1; i < argc; i += 1) {\n");
    out.push_str("      if (tn_is_boxed(args[i])) {\n");
    out.push_str(
        "        return tn_runtime_failf(\"host error: sum_ints expects int arguments only; argument %zu was %s\", i, tn_runtime_value_kind(args[i]));\n",
    );
    out.push_str("      }\n");
    out.push_str("      total += (int64_t)args[i];\n");
    out.push_str("    }\n");
    out.push_str("    free(args);\n");
    out.push_str("    return (TnVal)total;\n");
    out.push_str("  }\n\n");
    out.push_str("  if (strcmp(key, \"memory_cycle_churn\") == 0) {\n");
    out.push_str("    if (argc != 2) {\n");
    out.push_str(
        "      return tn_runtime_failf(\"host error: memory_cycle_churn expects exactly 1 argument, found %zu\", argc - 1);\n",
    );
    out.push_str("    }\n");
    out.push_str("    if (tn_is_boxed(args[1])) {\n");
    out.push_str(
        "      return tn_runtime_failf(\"host error: memory_cycle_churn expects int argument 1; found %s\", tn_runtime_value_kind(args[1]));\n",
    );
    out.push_str("    }\n");
    out.push_str("    int64_t churn = (int64_t)args[1];\n");
    out.push_str("    if (churn < 0) {\n");
    out.push_str(
        "      return tn_runtime_fail(\"host error: memory_cycle_churn expects non-negative churn count\");\n",
    );
    out.push_str("    }\n");
    out.push_str("    for (int64_t i = 0; i < churn; i += 1) {\n");
    out.push_str("      TnObj *left_obj = tn_new_obj(TN_OBJ_LIST);\n");
    out.push_str("      left_obj->as.list.len = 1;\n");
    out.push_str("      left_obj->as.list.items = (TnVal *)calloc(1, sizeof(TnVal));\n");
    out.push_str("      if (left_obj->as.list.items == NULL) {\n");
    out.push_str("        fprintf(stderr, \"error: native runtime allocation failure\\n\");\n");
    out.push_str("        exit(1);\n");
    out.push_str("      }\n");
    out.push_str("      TnVal left_value = tn_heap_store(left_obj);\n\n");
    out.push_str("      TnObj *right_obj = tn_new_obj(TN_OBJ_LIST);\n");
    out.push_str("      right_obj->as.list.len = 1;\n");
    out.push_str("      right_obj->as.list.items = (TnVal *)calloc(1, sizeof(TnVal));\n");
    out.push_str("      if (right_obj->as.list.items == NULL) {\n");
    out.push_str("        fprintf(stderr, \"error: native runtime allocation failure\\n\");\n");
    out.push_str("        exit(1);\n");
    out.push_str("      }\n");
    out.push_str("      TnVal right_value = tn_heap_store(right_obj);\n\n");
    out.push_str("      left_obj->as.list.items[0] = right_value;\n");
    out.push_str("      right_obj->as.list.items[0] = left_value;\n");
    out.push_str("      tn_runtime_retain(right_value);\n");
    out.push_str("      tn_runtime_retain(left_value);\n");
    out.push_str("    }\n");
    out.push_str("    free(args);\n");
    out.push_str("    return (TnVal)churn;\n");
    out.push_str("  }\n\n");
    out.push_str(
        r###"  if (strcmp(key, "sys_path_exists") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_path_exists expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_path_exists expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    int exists = access(path_obj->as.text.text, F_OK) == 0;
    free(args);
    return tn_runtime_const_bool((TnVal)(exists != 0));
  }

  if (strcmp(key, "sys_ensure_dir") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_ensure_dir expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_ensure_dir expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    if (path[0] == '\0') {
      return tn_runtime_fail("host error: sys_ensure_dir path must not be empty");
    }

    size_t path_len = strlen(path);
    if (path_len >= PATH_MAX) {
      return tn_runtime_fail("host error: sys_ensure_dir path is too long");
    }

    char mkdir_path[PATH_MAX];
    memcpy(mkdir_path, path, path_len + 1);

    for (size_t i = 1; i < path_len; i += 1) {
      if (mkdir_path[i] != '/') {
        continue;
      }

      mkdir_path[i] = '\0';
      if (mkdir_path[0] != '\0' && mkdir(mkdir_path, 0777) != 0 && errno != EEXIST) {
        return tn_runtime_failf("host error: sys_ensure_dir failed for '%s'", path);
      }
      mkdir_path[i] = '/';
    }

    if (mkdir(mkdir_path, 0777) != 0 && errno != EEXIST) {
      return tn_runtime_failf("host error: sys_ensure_dir failed for '%s'", path);
    }

    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  if (strcmp(key, "sys_write_text") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: sys_write_text expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    TnObj *content_obj = tn_get_obj(args[2]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_write_text expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (content_obj == NULL || content_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_write_text expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    FILE *handle = fopen(path_obj->as.text.text, "w");
    if (handle == NULL) {
      return tn_runtime_failf("host error: sys_write_text failed for '%s'", path_obj->as.text.text);
    }
    if (fputs(content_obj->as.text.text, handle) < 0 || fclose(handle) != 0) {
      return tn_runtime_failf("host error: sys_write_text failed for '%s'", path_obj->as.text.text);
    }
    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  if (strcmp(key, "sys_env") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_env expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *name_obj = tn_get_obj(args[1]);
    if (name_obj == NULL || name_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_env expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *value = getenv(name_obj->as.text.text);
    free(args);
    if (value == NULL) {
      return tn_runtime_const_nil();
    }
    return tn_runtime_const_string((TnVal)(intptr_t)value);
  }

  if (strcmp(key, "sys_which") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_which expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *name_obj = tn_get_obj(args[1]);
    if (name_obj == NULL || name_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_which expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *name = name_obj->as.text.text;
    if (strchr(name, ' ') != NULL || strchr(name, '\t') != NULL || strchr(name, '\n') != NULL || strchr(name, '\r') != NULL) {
      return tn_runtime_fail("host error: sys_which command name contains unsupported whitespace");
    }
    char command[4096];
    int written = snprintf(command, sizeof(command), "command -v %s 2>/dev/null", name);
    if (written <= 0 || (size_t)written >= sizeof(command)) {
      return tn_runtime_fail("host error: sys_which command too long");
    }
    FILE *pipe = popen(command, "r");
    if (pipe == NULL) {
      return tn_runtime_fail("host error: sys_which failed to spawn shell");
    }
    char found[4096];
    if (fgets(found, sizeof(found), pipe) == NULL) {
      pclose(pipe);
      free(args);
      return tn_runtime_const_nil();
    }
    pclose(pipe);
    size_t len = strlen(found);
    while (len > 0 && (found[len - 1] == '\n' || found[len - 1] == '\r')) {
      found[len - 1] = '\0';
      len -= 1;
    }
    free(args);
    return tn_runtime_const_string((TnVal)(intptr_t)found);
  }

  if (strcmp(key, "sys_cwd") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: sys_cwd expects exactly 0 arguments, found %zu", argc - 1);
    }
    char cwd_buffer[PATH_MAX];
    if (getcwd(cwd_buffer, sizeof(cwd_buffer)) == NULL) {
      return tn_runtime_fail("host error: sys_cwd failed to read current directory");
    }
    free(args);
    return tn_runtime_const_string((TnVal)(intptr_t)cwd_buffer);
  }

  if (strcmp(key, "sys_argv") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: sys_argv expects exactly 0 arguments, found %zu", argc - 1);
    }
    
    // allocate list of strings for sys_argv
    TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
    list_obj->as.list.len = tn_global_argc;
    list_obj->as.list.items = tn_global_argc == 0 ? NULL : (TnVal *)calloc(tn_global_argc, sizeof(TnVal));
    if (tn_global_argc > 0 && list_obj->as.list.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    for (int i = 0; i < tn_global_argc; i++) {
      list_obj->as.list.items[i] = tn_runtime_const_string((TnVal)(intptr_t)tn_global_argv[i]);
      tn_runtime_retain(list_obj->as.list.items[i]);
    }
    free(args);
    return tn_heap_store(list_obj);
  }

  if (strcmp(key, "sys_run") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_run expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *command_obj = tn_get_obj(args[1]);
    if (command_obj == NULL || command_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_run expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }

    const char *command_text = command_obj->as.text.text;
    size_t command_len = strlen(command_text);
    char *shell_command = (char *)malloc(command_len + 6);
    if (shell_command == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    snprintf(shell_command, command_len + 6, "%s 2>&1", command_text);

    FILE *pipe = popen(shell_command, "r");
    free(shell_command);
    if (pipe == NULL) {
      return tn_runtime_fail("host error: sys_run failed to spawn shell");
    }

    size_t cap = 256;
    size_t len = 0;
    char *buffer = (char *)malloc(cap);
    if (buffer == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    buffer[0] = '\0';

    int next_ch = 0;
    while ((next_ch = fgetc(pipe)) != EOF) {
      if (len + 2 > cap) {
        cap *= 2;
        char *next = (char *)realloc(buffer, cap);
        if (next == NULL) {
          free(buffer);
          fprintf(stderr, "error: native runtime allocation failure\n");
          exit(1);
        }
        buffer = next;
      }

      buffer[len] = (char)next_ch;
      len += 1;
      buffer[len] = '\0';
    }

    int status = pclose(pipe);
    int exit_code = -1;
    if (status >= 0) {
#ifdef WIFEXITED
      if (WIFEXITED(status)) {
        exit_code = WEXITSTATUS(status);
      }
#else
      exit_code = status;
#endif
    }

    TnVal result = tn_runtime_make_map(
      tn_runtime_const_atom((TnVal)(intptr_t)"exit_code"),
      (TnVal)exit_code
    );
    result = tn_runtime_map_put(
      result,
      tn_runtime_const_atom((TnVal)(intptr_t)"output"),
      tn_runtime_const_string((TnVal)(intptr_t)buffer)
    );

    free(buffer);
    free(args);
    return result;
  }

  return tn_runtime_failf("host error: unknown host function: %s", key);
"###,
    );
    out.push_str("}\n\n");

    out.push_str(
        "static TnVal tn_runtime_call_closure_varargs(TnVal closure, TnVal count, ...) {\n",
    );
    out.push_str("  if (count < 0) {\n");
    out.push_str("    return tn_runtime_fail(\"arity mismatch for anonymous function: expected 0 args, found -1\");\n");
    out.push_str("  }\n\n");
    out.push_str("  TnObj *closure_obj = tn_get_obj(closure);\n");
    out.push_str("  if (closure_obj == NULL || closure_obj->kind != TN_OBJ_CLOSURE) {\n");
    out.push_str(
        "    return tn_runtime_failf(\"attempted to call non-function value: %s\", tn_runtime_value_kind(closure));\n",
    );
    out.push_str("  }\n\n");
    out.push_str("  if (closure_obj->as.closure.param_count != count) {\n");
    out.push_str(
        "    return tn_runtime_failf(\"arity mismatch for anonymous function: expected %lld args, found %lld\", (long long)closure_obj->as.closure.param_count, (long long)count);\n",
    );
    out.push_str("  }\n\n");
    out.push_str("  size_t root_frame = tn_runtime_root_frame_push();\n");
    out.push_str("  tn_runtime_root_register(closure);\n");
    out.push_str("  size_t argc = (size_t)count;\n");
    out.push_str("  TnVal *args = argc == 0 ? NULL : (TnVal *)calloc(argc, sizeof(TnVal));\n");
    out.push_str("  if (argc > 0 && args == NULL) {\n");
    out.push_str("    fprintf(stderr, \"error: native runtime allocation failure\\n\");\n");
    out.push_str("    exit(1);\n");
    out.push_str("  }\n\n");
    out.push_str("  va_list vargs;\n");
    out.push_str("  va_start(vargs, count);\n");
    out.push_str("  for (size_t i = 0; i < argc; i += 1) {\n");
    out.push_str("    args[i] = va_arg(vargs, TnVal);\n");
    out.push_str("    tn_runtime_root_register(args[i]);\n");
    out.push_str("  }\n");
    out.push_str("  va_end(vargs);\n\n");
    out.push_str(
        "  TnVal result = tn_runtime_call_compiled_closure(closure_obj->as.closure.descriptor_hash, args, argc);\n",
    );
    out.push_str("  tn_runtime_retain(result);\n");
    out.push_str("  free(args);\n");
    out.push_str("  tn_runtime_root_frame_pop(root_frame);\n");
    out.push_str("  return result;\n");
    out.push_str("}\n\n");

    out.push_str("static TnVal tn_runtime_make_ok(TnVal value) {\n");
    out.push_str("  TnObj *obj = tn_new_obj(TN_OBJ_RESULT);\n");
    out.push_str("  obj->as.result.is_ok = 1;\n");
    out.push_str("  obj->as.result.value = value;\n");
    out.push_str("  tn_runtime_retain(value);\n");
    out.push_str("  return tn_heap_store(obj);\n");
    out.push_str("}\n\n");

    out.push_str("static TnVal tn_runtime_make_err(TnVal value) {\n");
    out.push_str("  TnObj *obj = tn_new_obj(TN_OBJ_RESULT);\n");
    out.push_str("  obj->as.result.is_ok = 0;\n");
    out.push_str("  obj->as.result.value = value;\n");
    out.push_str("  tn_runtime_retain(value);\n");
    out.push_str("  return tn_heap_store(obj);\n");
    out.push_str("}\n\n");

    out.push_str("static TnVal tn_runtime_question(TnVal value) {\n");
    out.push_str("  TnObj *obj = tn_get_obj(value);\n");
    out.push_str("  if (obj == NULL || obj->kind != TN_OBJ_RESULT) {\n");
    out.push_str("    return tn_runtime_failf(\"question expects result value, found %s\", tn_runtime_value_kind(value));\n");
    out.push_str("  }\n");
    out.push_str("  if (obj->as.result.is_ok != 0) {\n");
    out.push_str("    tn_runtime_retain(obj->as.result.value);\n");
    out.push_str("    return obj->as.result.value;\n");
    out.push_str("  }\n\n");
    out.push_str("  fprintf(stderr, \"error: runtime returned \" );\n");
    out.push_str("  tn_render_value(stderr, value);\n");
    out.push_str("  fputc('\\n', stderr);\n");
    out.push_str("  exit(1);\n");
    out.push_str("}\n\n");

    out.push_str("static TnVal tn_runtime_raise(TnVal error_value) {\n");
    out.push_str("  TnObj *obj = tn_get_obj(error_value);\n");
    out.push_str("  if (obj != NULL && obj->kind == TN_OBJ_STRING) {\n");
    out.push_str("    return tn_runtime_fail(obj->as.text.text);\n");
    out.push_str("  }\n");
    out.push_str("  if (obj != NULL && obj->kind == TN_OBJ_ATOM) {\n");
    out.push_str("    return tn_runtime_fail(obj->as.text.text);\n");
    out.push_str("  }\n");
    out.push_str("  return tn_runtime_fail(\"exception raised\");\n");
    out.push_str("}\n\n");

    // Zero-arg stubs
    for name in &[
        "tn_runtime_error_no_matching_clause",
        "tn_runtime_error_bad_match",
        "tn_runtime_error_arity_mismatch",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(void) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    out.push_str(
        r###"static TnVal tn_runtime_to_string(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  if (obj == NULL) {
    char buffer[32];
    snprintf(buffer, sizeof(buffer), "%lld", (long long)value);
    return tn_runtime_const_string((TnVal)(intptr_t)buffer);
  }

  switch (obj->kind) {
    case TN_OBJ_BOOL:
      return tn_runtime_const_string((TnVal)(intptr_t)(obj->as.bool_value ? "true" : "false"));
    case TN_OBJ_NIL:
      return tn_runtime_const_string((TnVal)(intptr_t)"nil");
    case TN_OBJ_ATOM:
    case TN_OBJ_STRING:
    case TN_OBJ_FLOAT:
      return tn_runtime_const_string((TnVal)(intptr_t)obj->as.text.text);
    default:
      return tn_runtime_failf("to_string expects scalar value, found %s", tn_runtime_value_kind(value));
  }
}

static TnVal tn_runtime_not(TnVal _a) { return tn_stub_abort("tn_runtime_not"); }
static TnVal tn_runtime_bang(TnVal _a) { return tn_stub_abort("tn_runtime_bang"); }

static TnVal tn_runtime_concat(TnVal left, TnVal right) {
  TnObj *left_obj = tn_get_obj(left);
  TnObj *right_obj = tn_get_obj(right);
  if (left_obj == NULL || left_obj->kind != TN_OBJ_STRING ||
      right_obj == NULL || right_obj->kind != TN_OBJ_STRING) {
    return tn_runtime_failf("concat expects string <> string, found %s <> %s", tn_runtime_value_kind(left), tn_runtime_value_kind(right));
  }

  const char *left_text = left_obj->as.text.text;
  const char *right_text = right_obj->as.text.text;
  size_t left_len = strlen(left_text);
  size_t right_len = strlen(right_text);
  char *joined = (char *)malloc(left_len + right_len + 1);
  if (joined == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  memcpy(joined, left_text, left_len);
  memcpy(joined + left_len, right_text, right_len + 1);

  TnObj *obj = tn_new_obj(TN_OBJ_STRING);
  obj->as.text.text = joined;
  return tn_heap_store(obj);
}

static TnVal tn_runtime_list_concat(TnVal left, TnVal right) {
  TnObj *left_obj = tn_get_obj(left);
  TnObj *right_obj = tn_get_obj(right);
  if (left_obj == NULL || left_obj->kind != TN_OBJ_LIST ||
      right_obj == NULL || right_obj->kind != TN_OBJ_LIST) {
    return tn_runtime_failf("list concat expects list ++ list, found %s ++ %s", tn_runtime_value_kind(left), tn_runtime_value_kind(right));
  }

  size_t left_len = left_obj->as.list.len;
  size_t right_len = right_obj->as.list.len;
  size_t combined_len = left_len + right_len;

  TnObj *obj = tn_new_obj(TN_OBJ_LIST);
  obj->as.list.len = combined_len;
  obj->as.list.items = combined_len == 0 ? NULL : (TnVal *)calloc(combined_len, sizeof(TnVal));
  if (combined_len > 0 && obj->as.list.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  for (size_t i = 0; i < left_len; i += 1) {
    obj->as.list.items[i] = left_obj->as.list.items[i];
    tn_runtime_retain(obj->as.list.items[i]);
  }
  for (size_t i = 0; i < right_len; i += 1) {
    obj->as.list.items[left_len + i] = right_obj->as.list.items[i];
    tn_runtime_retain(obj->as.list.items[left_len + i]);
  }

  return tn_heap_store(obj);
}

static TnVal tn_runtime_list_subtract(TnVal left, TnVal right) {
  TnObj *left_obj = tn_get_obj(left);
  TnObj *right_obj = tn_get_obj(right);
  if (left_obj == NULL || left_obj->kind != TN_OBJ_LIST ||
      right_obj == NULL || right_obj->kind != TN_OBJ_LIST) {
    return tn_runtime_failf("list subtract expects list -- list, found %s -- %s", tn_runtime_value_kind(left), tn_runtime_value_kind(right));
  }

  size_t left_len = left_obj->as.list.len;
  size_t right_len = right_obj->as.list.len;

  int *consumed = right_len == 0 ? NULL : (int *)calloc(right_len, sizeof(int));
  if (right_len > 0 && consumed == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  TnObj *obj = tn_new_obj(TN_OBJ_LIST);
  obj->as.list.len = 0;
  obj->as.list.items = left_len == 0 ? NULL : (TnVal *)calloc(left_len, sizeof(TnVal));
  if (left_len > 0 && obj->as.list.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  for (size_t i = 0; i < left_len; i += 1) {
    TnVal candidate = left_obj->as.list.items[i];
    int removed = 0;
    for (size_t j = 0; j < right_len; j += 1) {
      if (consumed[j] == 0 && tn_runtime_value_equal(candidate, right_obj->as.list.items[j])) {
        consumed[j] = 1;
        removed = 1;
        break;
      }
    }

    if (!removed) {
      obj->as.list.items[obj->as.list.len] = candidate;
      tn_runtime_retain(candidate);
      obj->as.list.len += 1;
    }
  }

  free(consumed);
  return tn_heap_store(obj);
}

"###,
    );
    emit_runtime_pattern_helpers(mir, out)?;
    emit_runtime_try_helpers(mir, out)?;
    emit_runtime_for_helpers(mir, out)?;
    emit_compiled_closure_helpers(mir, out)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClosureSpec {
    hash: i64,
    params: Vec<String>,
    ops: Vec<IrOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrySpec {
    hash: i64,
    op: IrOp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ForSpec {
    hash: i64,
    op: IrOp,
}

const FOR_REDUCE_ACC_BINDING: &str = "__tonic_for_acc";

#[derive(Debug, Clone, PartialEq, Eq)]
enum StaticForValue {
    Int(i64),
    Bool(bool),
    Nil,
    Atom(String),
    String(String),
    Float(String),
    Tuple(Box<StaticForValue>, Box<StaticForValue>),
    List(Vec<StaticForValue>),
    Map(Vec<(StaticForValue, StaticForValue)>),
    Keyword(Vec<(StaticForValue, StaticForValue)>),
}

impl StaticForValue {
    fn kind_label(&self) -> &'static str {
        match self {
            StaticForValue::Int(_) => "int",
            StaticForValue::Bool(_) => "bool",
            StaticForValue::Nil => "nil",
            StaticForValue::Atom(_) => "atom",
            StaticForValue::String(_) => "string",
            StaticForValue::Float(_) => "float",
            StaticForValue::Tuple(_, _) => "tuple",
            StaticForValue::List(_) => "list",
            StaticForValue::Map(_) => "map",
            StaticForValue::Keyword(_) => "keyword",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StaticForEvalIssue {
    Runtime(String),
    Unsupported(String),
}

fn emit_runtime_try_helpers(mir: &MirProgram, out: &mut String) -> Result<(), CBackendError> {
    let try_specs = collect_try_specs(mir)?;

    out.push_str("/* compiled try helpers */\n");
    for (index, try_spec) in try_specs.iter().enumerate() {
        emit_runtime_try_case(index, try_spec, out)?;
    }

    out.push_str("static TnVal tn_runtime_try(TnVal op_hash) {\n");
    if try_specs.is_empty() {
        out.push_str("  return tn_stub_abort(\"tn_runtime_try\");\n");
    } else {
        out.push_str("  switch (op_hash) {\n");
        for (index, try_spec) in try_specs.iter().enumerate() {
            out.push_str(&format!(
                "    case (TnVal){}LL: return tn_runtime_try_case_{index}();\n",
                try_spec.hash
            ));
        }
        out.push_str("    default:\n");
        out.push_str("      return tn_stub_abort(\"tn_runtime_try\");\n");
        out.push_str("  }\n");
    }
    out.push_str("}\n\n");

    Ok(())
}

fn collect_try_specs(mir: &MirProgram) -> Result<Vec<TrySpec>, CBackendError> {
    let mut by_hash = BTreeMap::<i64, IrOp>::new();

    for function in &mir.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let MirInstruction::Legacy { source, .. } = instruction else {
                    continue;
                };

                if !matches!(source, IrOp::Try { .. }) {
                    continue;
                }

                let hash = hash_ir_op_i64(source)?;
                if let Some(existing) = by_hash.get(&hash) {
                    if existing != source {
                        return Err(CBackendError::new(format!(
                            "c backend try hash collision for hash {hash}"
                        )));
                    }
                } else {
                    by_hash.insert(hash, source.clone());
                }
            }
        }
    }

    Ok(by_hash
        .into_iter()
        .map(|(hash, op)| TrySpec { hash, op })
        .collect())
}

fn emit_runtime_for_helpers(mir: &MirProgram, out: &mut String) -> Result<(), CBackendError> {
    let for_specs = collect_for_specs(mir)?;

    out.push_str("/* compiled for helpers */\n");
    for (index, for_spec) in for_specs.iter().enumerate() {
        emit_runtime_for_case(index, for_spec, out)?;
    }

    out.push_str("static TnVal tn_runtime_for(TnVal op_hash) {\n");
    if for_specs.is_empty() {
        out.push_str("  return tn_stub_abort(\"tn_runtime_for\");\n");
    } else {
        out.push_str("  switch (op_hash) {\n");
        for (index, for_spec) in for_specs.iter().enumerate() {
            out.push_str(&format!(
                "    case (TnVal){}LL: return tn_runtime_for_case_{index}();\n",
                for_spec.hash
            ));
        }
        out.push_str("    default:\n");
        out.push_str("      return tn_stub_abort(\"tn_runtime_for\");\n");
        out.push_str("  }\n");
    }
    out.push_str("}\n\n");

    Ok(())
}

fn collect_for_specs(mir: &MirProgram) -> Result<Vec<ForSpec>, CBackendError> {
    let mut by_hash = BTreeMap::<i64, IrOp>::new();

    for function in &mir.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let MirInstruction::Legacy { source, .. } = instruction else {
                    continue;
                };

                if !matches!(source, IrOp::For { .. }) {
                    continue;
                }

                let hash = hash_ir_op_i64(source)?;
                if let Some(existing) = by_hash.get(&hash) {
                    if existing != source {
                        return Err(CBackendError::new(format!(
                            "c backend for hash collision for hash {hash}"
                        )));
                    }
                } else {
                    by_hash.insert(hash, source.clone());
                }
            }
        }
    }

    Ok(by_hash
        .into_iter()
        .map(|(hash, op)| ForSpec { hash, op })
        .collect())
}

fn emit_runtime_for_case(
    index: usize,
    for_spec: &ForSpec,
    out: &mut String,
) -> Result<(), CBackendError> {
    out.push_str(&format!(
        "static TnVal tn_runtime_for_case_{index}(void) {{\n"
    ));

    match evaluate_for_spec(&for_spec.op) {
        Ok(value) => {
            let mut temp_index = 0usize;
            let rendered = emit_static_for_value(&value, out, &mut temp_index);
            out.push_str(&format!("  return {rendered};\n"));
        }
        Err(StaticForEvalIssue::Runtime(message)) => {
            let escaped = c_string_literal(&message);
            out.push_str(&format!("  return tn_runtime_fail({escaped});\n"));
        }
        Err(StaticForEvalIssue::Unsupported(_)) => {
            out.push_str("  return tn_stub_abort(\"tn_runtime_for\");\n");
        }
    }

    out.push_str("}\n\n");
    Ok(())
}

fn emit_static_for_value(
    value: &StaticForValue,
    out: &mut String,
    temp_index: &mut usize,
) -> String {
    match value {
        StaticForValue::Int(value) => format!("(TnVal){value}LL"),
        StaticForValue::Bool(value) => {
            format!(
                "tn_runtime_const_bool((TnVal){})",
                if *value { 1 } else { 0 }
            )
        }
        StaticForValue::Nil => "tn_runtime_const_nil()".to_string(),
        StaticForValue::Atom(value) => {
            let escaped = c_string_literal(value);
            format!("tn_runtime_const_atom((TnVal)(intptr_t){escaped})")
        }
        StaticForValue::String(value) => {
            let escaped = c_string_literal(value);
            format!("tn_runtime_const_string((TnVal)(intptr_t){escaped})")
        }
        StaticForValue::Float(value) => {
            let escaped = c_string_literal(value);
            format!("tn_runtime_const_float((TnVal)(intptr_t){escaped})")
        }
        StaticForValue::Tuple(left, right) => {
            let left_value = emit_static_for_value(left, out, temp_index);
            let right_value = emit_static_for_value(right, out, temp_index);
            format!("tn_runtime_make_tuple({left_value}, {right_value})")
        }
        StaticForValue::List(values) => {
            let rendered_items = values
                .iter()
                .map(|item| emit_static_for_value(item, out, temp_index))
                .collect::<Vec<_>>();
            let args = std::iter::once(format!("(TnVal){}", values.len()))
                .chain(rendered_items)
                .collect::<Vec<_>>()
                .join(", ");
            format!("tn_runtime_make_list_varargs({args})")
        }
        StaticForValue::Map(entries) => {
            if entries.is_empty() {
                "tn_runtime_map_empty()".to_string()
            } else {
                let temp = format!("tn_for_value_{}", *temp_index);
                *temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_map_empty();\n"));
                for (key, value) in entries {
                    let rendered_key = emit_static_for_value(key, out, temp_index);
                    let rendered_value = emit_static_for_value(value, out, temp_index);
                    out.push_str(&format!(
                        "  {temp} = tn_runtime_map_put({temp}, {rendered_key}, {rendered_value});\n"
                    ));
                }
                temp
            }
        }
        StaticForValue::Keyword(entries) => {
            if entries.is_empty() {
                "tn_runtime_make_list_varargs((TnVal)0)".to_string()
            } else {
                let temp = format!("tn_for_value_{}", *temp_index);
                *temp_index += 1;
                let (first_key, first_value) = &entries[0];
                let rendered_first_key = emit_static_for_value(first_key, out, temp_index);
                let rendered_first_value = emit_static_for_value(first_value, out, temp_index);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_keyword({rendered_first_key}, {rendered_first_value});\n"
                ));

                for (key, value) in entries.iter().skip(1) {
                    let rendered_key = emit_static_for_value(key, out, temp_index);
                    let rendered_value = emit_static_for_value(value, out, temp_index);
                    out.push_str(&format!(
                        "  {temp} = tn_runtime_keyword_append({temp}, {rendered_key}, {rendered_value});\n"
                    ));
                }

                temp
            }
        }
    }
}

enum StaticForCollector {
    List(Vec<StaticForValue>),
    Map(Vec<(StaticForValue, StaticForValue)>),
    Keyword(Vec<(StaticForValue, StaticForValue)>),
    Reduce(StaticForValue),
}

fn evaluate_for_spec(for_op: &IrOp) -> Result<StaticForValue, StaticForEvalIssue> {
    let IrOp::For {
        generators,
        into_ops,
        reduce_ops,
        body_ops,
        ..
    } = for_op
    else {
        return Err(StaticForEvalIssue::Unsupported(
            "for helper source was not IrOp::For".to_string(),
        ));
    };

    if into_ops.is_some() && reduce_ops.is_some() {
        return Err(StaticForEvalIssue::Runtime(
            "for options 'reduce' and 'into' cannot be combined".to_string(),
        ));
    }

    let mut collector = if let Some(reduce_ops) = reduce_ops {
        StaticForCollector::Reduce(evaluate_static_for_ops(reduce_ops, &BTreeMap::new())?)
    } else if let Some(into_ops) = into_ops {
        match evaluate_static_for_ops(into_ops, &BTreeMap::new())? {
            StaticForValue::List(values) => StaticForCollector::List(values),
            StaticForValue::Map(entries) => StaticForCollector::Map(entries),
            StaticForValue::Keyword(entries) => StaticForCollector::Keyword(entries),
            other => {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for into destination must be a list, map, or keyword, found {}",
                    other.kind_label()
                )));
            }
        }
    } else {
        StaticForCollector::List(Vec::new())
    };

    evaluate_for_generators(generators, 0, &BTreeMap::new(), body_ops, &mut collector)?;

    Ok(match collector {
        StaticForCollector::List(values) => StaticForValue::List(values),
        StaticForCollector::Map(entries) => StaticForValue::Map(entries),
        StaticForCollector::Keyword(entries) => StaticForValue::Keyword(entries),
        StaticForCollector::Reduce(value) => value,
    })
}

fn evaluate_for_generators(
    generators: &[IrForGenerator],
    index: usize,
    env: &BTreeMap<String, StaticForValue>,
    body_ops: &[IrOp],
    collector: &mut StaticForCollector,
) -> Result<(), StaticForEvalIssue> {
    if index >= generators.len() {
        match collector {
            StaticForCollector::Reduce(accumulator) => {
                let mut reduce_env = env.clone();
                reduce_env.insert(FOR_REDUCE_ACC_BINDING.to_string(), accumulator.clone());
                *accumulator = evaluate_static_for_ops(body_ops, &reduce_env)?;
            }
            _ => {
                let body_value = evaluate_static_for_ops(body_ops, env)?;
                collect_for_value(collector, body_value)?;
            }
        }
        return Ok(());
    }

    let generator = &generators[index];
    let enumerable = evaluate_static_for_ops(&generator.source_ops, env)?;
    let values = match enumerable {
        StaticForValue::List(values) => values,
        other => {
            return Err(StaticForEvalIssue::Runtime(format!(
                "for expects list generator, found {}",
                other.kind_label()
            )));
        }
    };

    for value in values {
        let mut iteration_env = env.clone();
        if !apply_pattern_bindings(&generator.pattern, &value, &mut iteration_env)? {
            continue;
        }

        if let Some(guard_ops) = &generator.guard_ops {
            let guard_value = evaluate_static_for_ops(guard_ops, &iteration_env)?;
            let StaticForValue::Bool(guard_result) = guard_value else {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for generator guard must evaluate to bool, found {}",
                    guard_value.kind_label()
                )));
            };

            if !guard_result {
                continue;
            }
        }

        evaluate_for_generators(generators, index + 1, &iteration_env, body_ops, collector)?;
    }

    Ok(())
}

fn collect_for_value(
    collector: &mut StaticForCollector,
    value: StaticForValue,
) -> Result<(), StaticForEvalIssue> {
    match collector {
        StaticForCollector::List(values) => values.push(value),
        StaticForCollector::Map(entries) => {
            let StaticForValue::Tuple(key, entry_value) = value else {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for into map expects tuple {{key, value}}, found {}",
                    value.kind_label()
                )));
            };

            let key = *key;
            let entry_value = *entry_value;
            if let Some(existing) = entries.iter_mut().find(|(entry_key, _)| *entry_key == key) {
                existing.1 = entry_value;
            } else {
                entries.push((key, entry_value));
            }
        }
        StaticForCollector::Keyword(entries) => {
            let StaticForValue::Tuple(key, entry_value) = value else {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for into keyword expects tuple {{key, value}}, found {}",
                    value.kind_label()
                )));
            };

            let key = *key;
            if !matches!(key, StaticForValue::Atom(_)) {
                return Err(StaticForEvalIssue::Runtime(format!(
                    "for into keyword expects atom key, found {}",
                    key.kind_label()
                )));
            }

            entries.push((key, *entry_value));
        }
        StaticForCollector::Reduce(_) => {
            return Err(StaticForEvalIssue::Runtime(
                "for internal error: reduce collector cannot accept yielded values".to_string(),
            ));
        }
    }

    Ok(())
}

fn evaluate_static_for_ops(
    ops: &[IrOp],
    env: &BTreeMap<String, StaticForValue>,
) -> Result<StaticForValue, StaticForEvalIssue> {
    let mut stack = Vec::<StaticForValue>::new();

    for op in ops {
        match op {
            IrOp::ConstInt { value, .. } => stack.push(StaticForValue::Int(*value)),
            IrOp::ConstBool { value, .. } => stack.push(StaticForValue::Bool(*value)),
            IrOp::ConstNil { .. } => stack.push(StaticForValue::Nil),
            IrOp::ConstAtom { value, .. } => stack.push(StaticForValue::Atom(value.clone())),
            IrOp::ConstString { value, .. } => stack.push(StaticForValue::String(value.clone())),
            IrOp::ConstFloat { value, .. } => stack.push(StaticForValue::Float(value.clone())),
            IrOp::LoadVariable { name, .. } => {
                if let Some(value) = env.get(name) {
                    stack.push(value.clone());
                } else {
                    return Err(StaticForEvalIssue::Unsupported(format!(
                        "for helper unknown binding '{name}'"
                    )));
                }
            }
            IrOp::AddInt { .. } => {
                let right = pop_static_for_int(&mut stack, "add right")?;
                let left = pop_static_for_int(&mut stack, "add left")?;
                stack.push(StaticForValue::Int(left + right));
            }
            IrOp::SubInt { .. } => {
                let right = pop_static_for_int(&mut stack, "sub right")?;
                let left = pop_static_for_int(&mut stack, "sub left")?;
                stack.push(StaticForValue::Int(left - right));
            }
            IrOp::MulInt { .. } => {
                let right = pop_static_for_int(&mut stack, "mul right")?;
                let left = pop_static_for_int(&mut stack, "mul left")?;
                stack.push(StaticForValue::Int(left * right));
            }
            IrOp::DivInt { .. } => {
                let right = pop_static_for_int(&mut stack, "div right")?;
                let left = pop_static_for_int(&mut stack, "div left")?;
                stack.push(StaticForValue::Int(left / right));
            }
            IrOp::Case { branches, .. } => {
                let subject = pop_static_for_value(&mut stack, "case subject")?;
                let mut matched_value = None;

                for branch in branches {
                    let mut branch_env = env.clone();
                    if !apply_pattern_bindings(&branch.pattern, &subject, &mut branch_env)? {
                        continue;
                    }

                    if let Some(guard_ops) = &branch.guard_ops {
                        let guard_value = evaluate_static_for_ops(guard_ops, &branch_env)?;
                        let StaticForValue::Bool(guard_result) = guard_value else {
                            return Err(StaticForEvalIssue::Runtime(format!(
                                "for helper case guard must evaluate to bool, found {}",
                                guard_value.kind_label()
                            )));
                        };

                        if !guard_result {
                            continue;
                        }
                    }

                    matched_value = Some(evaluate_static_for_ops(&branch.ops, &branch_env)?);
                    break;
                }

                if let Some(value) = matched_value {
                    stack.push(value);
                } else {
                    return Err(StaticForEvalIssue::Runtime(
                        "no case clause matching".to_string(),
                    ));
                }
            }
            IrOp::CmpInt { kind, .. } => {
                let right = pop_static_for_int(&mut stack, "cmp right")?;
                let left = pop_static_for_int(&mut stack, "cmp left")?;
                let result = match kind {
                    CmpKind::Eq => left == right,
                    CmpKind::NotEq => left != right,
                    CmpKind::Lt => left < right,
                    CmpKind::Lte => left <= right,
                    CmpKind::Gt => left > right,
                    CmpKind::Gte => left >= right,
                };
                stack.push(StaticForValue::Bool(result));
            }
            IrOp::Call { callee, argc, .. } => {
                let mut args = Vec::with_capacity(*argc);
                for _ in 0..*argc {
                    args.push(pop_static_for_value(&mut stack, "call argument")?);
                }
                args.reverse();

                match callee {
                    IrCallTarget::Builtin { name } => match name.as_str() {
                        "tuple" => {
                            if args.len() != 2 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper tuple expected 2 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Tuple(
                                Box::new(args[0].clone()),
                                Box::new(args[1].clone()),
                            ));
                        }
                        "list" => stack.push(StaticForValue::List(args)),
                        "map_empty" => {
                            if !args.is_empty() {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper map_empty expected 0 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Map(Vec::new()));
                        }
                        "map" => {
                            if args.len() != 2 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper map expected 2 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Map(vec![(
                                args[0].clone(),
                                args[1].clone(),
                            )]));
                        }
                        "map_put" => {
                            if args.len() != 3 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper map_put expected 3 args, found {}",
                                    args.len()
                                )));
                            }

                            let mut entries = match args[0].clone() {
                                StaticForValue::Map(entries) => entries,
                                other => {
                                    return Err(StaticForEvalIssue::Runtime(format!(
                                        "for helper map_put expected map base, found {}",
                                        other.kind_label()
                                    )));
                                }
                            };

                            let key = args[1].clone();
                            let value = args[2].clone();
                            if let Some(existing) =
                                entries.iter_mut().find(|(entry_key, _)| *entry_key == key)
                            {
                                existing.1 = value;
                            } else {
                                entries.push((key, value));
                            }
                            stack.push(StaticForValue::Map(entries));
                        }
                        "keyword" => {
                            if args.len() != 2 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper keyword expected 2 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Keyword(vec![(
                                args[0].clone(),
                                args[1].clone(),
                            )]));
                        }
                        "keyword_append" => {
                            if args.len() != 3 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper keyword_append expected 3 args, found {}",
                                    args.len()
                                )));
                            }

                            let mut entries = match args[0].clone() {
                                StaticForValue::Keyword(entries) => entries,
                                other => {
                                    return Err(StaticForEvalIssue::Runtime(format!(
                                        "for helper keyword_append expected keyword base, found {}",
                                        other.kind_label()
                                    )));
                                }
                            };
                            entries.push((args[1].clone(), args[2].clone()));
                            stack.push(StaticForValue::Keyword(entries));
                        }
                        "is_integer" => {
                            if args.len() != 1 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper is_integer expected 1 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Bool(matches!(
                                args.first(),
                                Some(StaticForValue::Int(_))
                            )));
                        }
                        "is_number" => {
                            if args.len() != 1 {
                                return Err(StaticForEvalIssue::Unsupported(format!(
                                    "for helper is_number expected 1 args, found {}",
                                    args.len()
                                )));
                            }
                            stack.push(StaticForValue::Bool(matches!(
                                args.first(),
                                Some(StaticForValue::Int(_) | StaticForValue::Float(_))
                            )));
                        }
                        other => {
                            return Err(StaticForEvalIssue::Unsupported(format!(
                                "for helper unsupported builtin call target: {other}"
                            )));
                        }
                    },
                    IrCallTarget::Function { name } => {
                        return Err(StaticForEvalIssue::Unsupported(format!(
                            "for helper unsupported function call target: {name}"
                        )));
                    }
                }
            }
            IrOp::Return { .. } => {
                if let Some(value) = stack.pop() {
                    return Ok(value);
                }
                return Ok(StaticForValue::Nil);
            }
            other => {
                return Err(StaticForEvalIssue::Unsupported(format!(
                    "for helper unsupported op: {other:?}"
                )));
            }
        }
    }

    if let Some(value) = stack.pop() {
        Ok(value)
    } else {
        Ok(StaticForValue::Nil)
    }
}

fn apply_pattern_bindings(
    pattern: &IrPattern,
    value: &StaticForValue,
    env: &mut BTreeMap<String, StaticForValue>,
) -> Result<bool, StaticForEvalIssue> {
    let snapshot = env.clone();

    let matched = match pattern {
        IrPattern::Bind { name } => {
            if let Some(existing) = env.get(name) {
                existing == value
            } else {
                env.insert(name.clone(), value.clone());
                true
            }
        }
        IrPattern::Pin { name } => env.get(name).map(|bound| bound == value).unwrap_or(false),
        IrPattern::Wildcard => true,
        IrPattern::Integer { value: expected } => {
            matches!(value, StaticForValue::Int(actual) if actual == expected)
        }
        IrPattern::Bool { value: expected } => {
            matches!(value, StaticForValue::Bool(actual) if actual == expected)
        }
        IrPattern::Nil => matches!(value, StaticForValue::Nil),
        IrPattern::String { value: expected } => {
            matches!(value, StaticForValue::String(actual) if actual == expected)
        }
        IrPattern::Atom { value: expected } => {
            matches!(value, StaticForValue::Atom(actual) if actual == expected)
        }
        IrPattern::Tuple { items } => {
            if let StaticForValue::Tuple(left, right) = value {
                if items.len() != 2 {
                    false
                } else {
                    apply_pattern_bindings(&items[0], left, env)?
                        && apply_pattern_bindings(&items[1], right, env)?
                }
            } else {
                false
            }
        }
        IrPattern::List { items, tail } => {
            if let StaticForValue::List(values) = value {
                if values.len() < items.len() || (tail.is_none() && values.len() != items.len()) {
                    false
                } else {
                    let mut matches = true;
                    for (idx, item_pattern) in items.iter().enumerate() {
                        if !apply_pattern_bindings(item_pattern, &values[idx], env)? {
                            matches = false;
                            break;
                        }
                    }

                    if matches {
                        if let Some(tail_pattern) = tail {
                            let tail_values = values[items.len()..].to_vec();
                            apply_pattern_bindings(
                                tail_pattern,
                                &StaticForValue::List(tail_values),
                                env,
                            )?
                        } else {
                            true
                        }
                    } else {
                        false
                    }
                }
            } else {
                false
            }
        }
        IrPattern::Map { .. } => {
            return Err(StaticForEvalIssue::Unsupported(
                "for helper does not support map patterns".to_string(),
            ));
        }
    };

    if matched {
        Ok(true)
    } else {
        *env = snapshot;
        Ok(false)
    }
}

fn pop_static_for_value(
    stack: &mut Vec<StaticForValue>,
    context: &str,
) -> Result<StaticForValue, StaticForEvalIssue> {
    stack.pop().ok_or_else(|| {
        StaticForEvalIssue::Unsupported(format!("for helper stack underflow for {context}"))
    })
}

fn pop_static_for_int(
    stack: &mut Vec<StaticForValue>,
    context: &str,
) -> Result<i64, StaticForEvalIssue> {
    match pop_static_for_value(stack, context)? {
        StaticForValue::Int(value) => Ok(value),
        other => Err(StaticForEvalIssue::Runtime(format!(
            "for arithmetic expects int {context}, found {}",
            other.kind_label()
        ))),
    }
}

fn emit_runtime_try_case(
    index: usize,
    try_spec: &TrySpec,
    out: &mut String,
) -> Result<(), CBackendError> {
    let IrOp::Try {
        body_ops,
        rescue_branches,
        catch_branches,
        after_ops,
        ..
    } = &try_spec.op
    else {
        return Err(CBackendError::new(
            "c backend internal error: try case source was not IrOp::Try",
        ));
    };

    out.push_str(&format!(
        "static TnVal tn_runtime_try_case_{index}(void) {{\n"
    ));
    out.push_str("  int tn_try_raised = 0;\n");
    out.push_str("  TnVal tn_try_error = tn_runtime_const_nil();\n");
    out.push_str("  TnVal tn_try_result = tn_runtime_const_nil();\n");

    emit_try_ops(
        body_ops,
        "tn_try_result",
        "tn_try_raised",
        "tn_try_error",
        &format!("tn_try_case_{index}_body"),
        "  ",
        out,
    )?;

    out.push_str("  if (tn_try_raised != 0) {\n");
    out.push_str("    int tn_try_handled = 0;\n");

    for (branch_index, branch) in rescue_branches.iter().enumerate() {
        if branch.guard_ops.is_some() {
            return Err(CBackendError::new(format!(
                "c backend try helper does not support rescue guard ops (case {index}, branch {branch_index})"
            )));
        }
        let pattern_hash = hash_pattern_i64(&branch.pattern)?;
        out.push_str(&format!(
            "    if (tn_try_handled == 0 && tn_runtime_pattern_matches(tn_try_error, (TnVal){pattern_hash}LL)) {{\n"
        ));
        out.push_str("      int tn_branch_raised = 0;\n");
        out.push_str("      TnVal tn_branch_error = tn_runtime_const_nil();\n");
        out.push_str("      TnVal tn_branch_result = tn_runtime_const_nil();\n");
        emit_try_ops(
            &branch.ops,
            "tn_branch_result",
            "tn_branch_raised",
            "tn_branch_error",
            &format!("tn_try_case_{index}_rescue_{branch_index}"),
            "      ",
            out,
        )?;
        out.push_str("      if (tn_branch_raised != 0) {\n");
        out.push_str("        tn_try_raised = 1;\n");
        out.push_str("        tn_try_error = tn_branch_error;\n");
        out.push_str("      } else {\n");
        out.push_str("        tn_try_raised = 0;\n");
        out.push_str("        tn_try_result = tn_branch_result;\n");
        out.push_str("        tn_try_handled = 1;\n");
        out.push_str("      }\n");
        out.push_str("    }\n");
    }

    for (branch_index, branch) in catch_branches.iter().enumerate() {
        if branch.guard_ops.is_some() {
            return Err(CBackendError::new(format!(
                "c backend try helper does not support catch guard ops (case {index}, branch {branch_index})"
            )));
        }
        let pattern_hash = hash_pattern_i64(&branch.pattern)?;
        out.push_str(&format!(
            "    if (tn_try_raised != 0 && tn_try_handled == 0 && tn_runtime_pattern_matches(tn_try_error, (TnVal){pattern_hash}LL)) {{\n"
        ));
        out.push_str("      int tn_branch_raised = 0;\n");
        out.push_str("      TnVal tn_branch_error = tn_runtime_const_nil();\n");
        out.push_str("      TnVal tn_branch_result = tn_runtime_const_nil();\n");
        emit_try_ops(
            &branch.ops,
            "tn_branch_result",
            "tn_branch_raised",
            "tn_branch_error",
            &format!("tn_try_case_{index}_catch_{branch_index}"),
            "      ",
            out,
        )?;
        out.push_str("      if (tn_branch_raised != 0) {\n");
        out.push_str("        tn_try_raised = 1;\n");
        out.push_str("        tn_try_error = tn_branch_error;\n");
        out.push_str("      } else {\n");
        out.push_str("        tn_try_raised = 0;\n");
        out.push_str("        tn_try_result = tn_branch_result;\n");
        out.push_str("        tn_try_handled = 1;\n");
        out.push_str("      }\n");
        out.push_str("    }\n");
    }

    out.push_str("  }\n");

    if let Some(after_ops) = after_ops {
        out.push_str("  int tn_after_raised = 0;\n");
        out.push_str("  TnVal tn_after_error = tn_runtime_const_nil();\n");
        out.push_str("  TnVal tn_after_result = tn_runtime_const_nil();\n");
        emit_try_ops(
            after_ops,
            "tn_after_result",
            "tn_after_raised",
            "tn_after_error",
            &format!("tn_try_case_{index}_after"),
            "  ",
            out,
        )?;
        out.push_str("  if (tn_after_raised != 0) {\n");
        out.push_str("    tn_try_raised = 1;\n");
        out.push_str("    tn_try_error = tn_after_error;\n");
        out.push_str("  }\n");
    }

    out.push_str("  if (tn_try_raised != 0) {\n");
    out.push_str("    TnObj *err_obj = tn_get_obj(tn_try_error);\n");
    out.push_str("    if (err_obj != NULL && err_obj->kind == TN_OBJ_STRING) {\n");
    out.push_str("      return tn_runtime_fail(err_obj->as.text.text);\n");
    out.push_str("    }\n");
    out.push_str("    if (err_obj != NULL && err_obj->kind == TN_OBJ_ATOM) {\n");
    out.push_str("      return tn_runtime_fail(err_obj->as.text.text);\n");
    out.push_str("    }\n");
    out.push_str("    return tn_runtime_fail(\"exception raised\");\n");
    out.push_str("  }\n");

    out.push_str("  return tn_try_result;\n");
    out.push_str("}\n\n");

    Ok(())
}

fn emit_try_ops(
    ops: &[IrOp],
    result_var: &str,
    raised_flag_var: &str,
    raised_value_var: &str,
    label: &str,
    indent: &str,
    out: &mut String,
) -> Result<(), CBackendError> {
    out.push_str(&format!("{indent}do {{\n"));

    let mut stack = Vec::<String>::new();
    let mut temp_index = 0usize;
    let mut terminated = false;

    for op in ops {
        match op {
            IrOp::ConstInt { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("{indent}  TnVal {temp} = (TnVal){value}LL;\n"));
                stack.push(temp);
            }
            IrOp::ConstBool { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_bool((TnVal){});\n",
                    if *value { 1 } else { 0 }
                ));
                stack.push(temp);
            }
            IrOp::ConstNil { .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_nil();\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstString { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_string((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstAtom { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_atom((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstFloat { value, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_const_float((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::LoadVariable { name, .. } => {
                let temp = format!("{label}_tmp_{temp_index}");
                temp_index += 1;
                let binding_hash = hash_text_i64(name);
                out.push_str(&format!(
                    "{indent}  TnVal {temp} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                ));
                stack.push(temp);
            }
            IrOp::Raise { .. } => {
                let error_value = pop_stack_value(&mut stack, "try raise input")?;
                out.push_str(&format!("{indent}  {raised_flag_var} = 1;\n"));
                out.push_str(&format!("{indent}  {raised_value_var} = {error_value};\n"));
                out.push_str(&format!("{indent}  break;\n"));
                terminated = true;
                break;
            }
            IrOp::Return { .. } => {
                let return_value = pop_stack_value(&mut stack, "try return value")?;
                out.push_str(&format!("{indent}  {result_var} = {return_value};\n"));
                out.push_str(&format!("{indent}  break;\n"));
                terminated = true;
                break;
            }
            other => {
                return Err(CBackendError::new(format!(
                    "c backend try helper unsupported op: {other:?}"
                )));
            }
        }
    }

    if !terminated {
        if let Some(value) = stack.pop() {
            out.push_str(&format!("{indent}  {result_var} = {value};\n"));
        } else {
            out.push_str(&format!(
                "{indent}  {result_var} = tn_runtime_const_nil();\n"
            ));
        }
    }

    out.push_str(&format!("{indent}}} while (0);\n"));
    Ok(())
}

fn emit_compiled_closure_helpers(mir: &MirProgram, out: &mut String) -> Result<(), CBackendError> {
    let closures = collect_closure_specs(mir)?;

    out.push_str("/* compiled closure helpers */\n");

    for (index, closure) in closures.iter().enumerate() {
        emit_compiled_closure_body(index, closure, out)?;
    }

    out.push_str(
        "static TnVal tn_runtime_call_compiled_closure(TnVal descriptor_hash, const TnVal *argv, size_t argc) {\n",
    );
    if closures.is_empty() {
        out.push_str(
            "  return tn_runtime_failf(\"unsupported closure descriptor %lld\", (long long)descriptor_hash);\n",
        );
    } else {
        out.push_str("  switch (descriptor_hash) {\n");
        for (index, closure) in closures.iter().enumerate() {
            out.push_str(&format!(
                "    case (TnVal){}LL: return tn_compiled_closure_{index}(argv, argc);\n",
                closure.hash
            ));
        }
        out.push_str("    default:\n");
        out.push_str(
            "      return tn_runtime_failf(\"unsupported closure descriptor %lld\", (long long)descriptor_hash);\n",
        );
        out.push_str("  }\n");
    }
    out.push_str("}\n");

    Ok(())
}

fn collect_closure_specs(mir: &MirProgram) -> Result<Vec<ClosureSpec>, CBackendError> {
    let mut by_hash = BTreeMap::<i64, ClosureSpec>::new();

    for function in &mir.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let MirInstruction::MakeClosure { params, ops, .. } = instruction else {
                    continue;
                };

                let capture_names = closure_capture_names(params, ops);
                let hash = hash_closure_descriptor_i64(params, ops, &capture_names)?;

                let spec = ClosureSpec {
                    hash,
                    params: params.clone(),
                    ops: ops.clone(),
                };

                if let Some(existing) = by_hash.get(&hash) {
                    if existing != &spec {
                        return Err(CBackendError::new(format!(
                            "c backend closure descriptor hash collision for hash {hash}"
                        )));
                    }
                } else {
                    by_hash.insert(hash, spec);
                }
            }
        }
    }

    Ok(by_hash.into_values().collect())
}

fn emit_compiled_closure_body(
    index: usize,
    closure: &ClosureSpec,
    out: &mut String,
) -> Result<(), CBackendError> {
    out.push_str(&format!(
        "static TnVal tn_compiled_closure_{index}(const TnVal *argv, size_t argc) {{\n"
    ));
    out.push_str(&format!("  if (argc != {}) {{\n", closure.params.len()));
    out.push_str(&format!(
        "    return tn_runtime_failf(\"arity mismatch for anonymous function: expected %d args, found %zu\", {}, argc);\n",
        closure.params.len()
    ));
    out.push_str("  }\n\n");
    out.push_str("  size_t tn_closure_root_frame = tn_runtime_root_frame_push();\n\n");

    let mut params = BTreeMap::<String, usize>::new();
    for (position, name) in closure.params.iter().enumerate() {
        params.insert(name.clone(), position);
    }

    let mut stack = Vec::<String>::new();
    let mut temp_index = 0usize;
    let mut emitted_return = false;

    for op in &closure.ops {
        match op {
            IrOp::LoadVariable { name, .. } => {
                if let Some(position) = params.get(name) {
                    stack.push(format!("argv[{position}]"));
                } else {
                    let binding_hash = hash_text_i64(name);
                    let temp = format!("tmp_{temp_index}");
                    temp_index += 1;
                    out.push_str(&format!(
                        "  TnVal {temp} = tn_runtime_load_binding((TnVal){binding_hash}LL);\n"
                    ));
                    stack.push(temp);
                }
            }
            IrOp::ConstInt { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = (TnVal){value}LL;\n"));
                stack.push(temp);
            }
            IrOp::ConstBool { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_bool((TnVal){});\n",
                    if *value { 1 } else { 0 }
                ));
                stack.push(temp);
            }
            IrOp::ConstNil { .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_const_nil();\n"));
                stack.push(temp);
            }
            IrOp::ConstString { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_string((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstAtom { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_atom((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::ConstFloat { value, .. } => {
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                let escaped = c_string_literal(value);
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_float((TnVal)(intptr_t){escaped});\n"
                ));
                stack.push(temp);
            }
            IrOp::AddInt { .. } => {
                emit_closure_binary("+", &mut stack, &mut temp_index, out)?;
            }
            IrOp::SubInt { .. } => {
                emit_closure_binary("-", &mut stack, &mut temp_index, out)?;
            }
            IrOp::MulInt { .. } => {
                emit_closure_binary("*", &mut stack, &mut temp_index, out)?;
            }
            IrOp::DivInt { .. } => {
                emit_closure_binary("/", &mut stack, &mut temp_index, out)?;
            }
            IrOp::CmpInt { kind, .. } => {
                let right = pop_stack_value(&mut stack, "cmp_int right operand")?;
                let left = pop_stack_value(&mut stack, "cmp_int left operand")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                let operator = match kind {
                    CmpKind::Eq => "==",
                    CmpKind::NotEq => "!=",
                    CmpKind::Lt => "<",
                    CmpKind::Lte => "<=",
                    CmpKind::Gt => ">",
                    CmpKind::Gte => ">=",
                };
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_const_bool(({left} {operator} {right}) ? 1 : 0);\n"
                ));
                stack.push(temp);
            }
            IrOp::ToString { .. } => {
                let input = pop_stack_value(&mut stack, "to_string input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_to_string({input});\n"
                ));
                stack.push(temp);
            }
            IrOp::Not { .. } => {
                let input = pop_stack_value(&mut stack, "not input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_not({input});\n"));
                stack.push(temp);
            }
            IrOp::Bang { .. } => {
                let input = pop_stack_value(&mut stack, "bang input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_bang({input});\n"));
                stack.push(temp);
            }
            IrOp::Question { .. } => {
                let input = pop_stack_value(&mut stack, "question input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_question({input});\n"));
                stack.push(temp);
            }
            IrOp::Raise { .. } => {
                let input = pop_stack_value(&mut stack, "raise input")?;
                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!("  TnVal {temp} = tn_runtime_raise({input});\n"));
                stack.push(temp);
            }
            IrOp::Call { callee, argc, .. } => {
                emit_closure_call(callee, *argc, &mut stack, &mut temp_index, out)?;
            }
            IrOp::CallValue { argc, .. } => {
                let mut args = Vec::with_capacity(*argc);
                for _ in 0..*argc {
                    args.push(pop_stack_value(&mut stack, "closure argument")?);
                }
                args.reverse();
                let callee = pop_stack_value(&mut stack, "closure callee")?;

                let root_frame = format!("root_frame_{temp_index}");
                out.push_str(&format!(
                    "  size_t {root_frame} = tn_runtime_root_frame_push();\n"
                ));
                out.push_str(&format!("  tn_runtime_root_register({callee});\n"));
                for argument in &args {
                    out.push_str(&format!("  tn_runtime_root_register({argument});\n"));
                }

                let call_args = std::iter::once(callee)
                    .chain(std::iter::once(format!("(TnVal){argc}")))
                    .chain(args.into_iter())
                    .collect::<Vec<_>>()
                    .join(", ");

                let temp = format!("tmp_{temp_index}");
                temp_index += 1;
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_call_closure_varargs({call_args});\n"
                ));
                out.push_str(&format!("  tn_runtime_retain({temp});\n"));
                out.push_str(&format!("  tn_runtime_root_frame_pop({root_frame});\n"));
                out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
                out.push_str(&format!("  tn_runtime_release({temp});\n"));
                stack.push(temp);
            }
            IrOp::Return { .. } => {
                let value = pop_stack_value(&mut stack, "return value")?;
                out.push_str(&format!("  tn_runtime_retain({value});\n"));
                out.push_str("  tn_runtime_root_frame_pop(tn_closure_root_frame);\n");
                out.push_str(&format!("  return {value};\n"));
                emitted_return = true;
                break;
            }
            _ => {
                out.push_str(
                    "  return tn_runtime_fail(\"unsupported closure operation in native runtime\");\n",
                );
                emitted_return = true;
                break;
            }
        }
    }

    if !emitted_return {
        if let Some(value) = stack.pop() {
            out.push_str(&format!("  tn_runtime_retain({value});\n"));
            out.push_str("  tn_runtime_root_frame_pop(tn_closure_root_frame);\n");
            out.push_str(&format!("  return {value};\n"));
        } else {
            out.push_str(
                "  return tn_runtime_fail(\"anonymous function ended without return\");\n",
            );
        }
    }

    out.push_str("}\n\n");
    Ok(())
}

fn emit_closure_binary(
    operator: &str,
    stack: &mut Vec<String>,
    temp_index: &mut usize,
    out: &mut String,
) -> Result<(), CBackendError> {
    let right = pop_stack_value(stack, "binary right operand")?;
    let left = pop_stack_value(stack, "binary left operand")?;
    let temp = format!("tmp_{}", *temp_index);
    *temp_index += 1;
    out.push_str(&format!("  TnVal {temp} = {left} {operator} {right};\n"));
    stack.push(temp);
    Ok(())
}

fn emit_closure_call(
    callee: &IrCallTarget,
    argc: usize,
    stack: &mut Vec<String>,
    temp_index: &mut usize,
    out: &mut String,
) -> Result<(), CBackendError> {
    let mut args = Vec::with_capacity(argc);
    for _ in 0..argc {
        args.push(pop_stack_value(stack, "call argument")?);
    }
    args.reverse();

    let rendered_args = args.join(", ");
    let temp = format!("tmp_{}", *temp_index);
    let root_frame = format!("root_frame_{}", *temp_index);
    *temp_index += 1;

    out.push_str(&format!(
        "  size_t {root_frame} = tn_runtime_root_frame_push();\n"
    ));
    for argument in &args {
        out.push_str(&format!("  tn_runtime_root_register({argument});\n"));
    }

    match callee {
        IrCallTarget::Builtin { name } => match name.as_str() {
            "tuple" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_tuple({rendered_args});\n"
                ));
            }
            "list" => {
                let count_then_args = std::iter::once(format!("(TnVal){argc}"))
                    .chain(args)
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_list_varargs({count_then_args});\n"
                ));
            }
            "map_empty" => {
                out.push_str(&format!("  TnVal {temp} = tn_runtime_map_empty();\n"));
            }
            "map" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_map({rendered_args});\n"
                ));
            }
            "map_put" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_map_put({rendered_args});\n"
                ));
            }
            "map_update" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_map_update({rendered_args});\n"
                ));
            }
            "map_access" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_map_access({rendered_args});\n"
                ));
            }
            "keyword" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_keyword({rendered_args});\n"
                ));
            }
            "keyword_append" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_keyword_append({rendered_args});\n"
                ));
            }
            "host_call" => {
                let count_then_args = std::iter::once(format!("(TnVal){argc}"))
                    .chain(args)
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_host_call_varargs({count_then_args});\n"
                ));
            }
            "protocol_dispatch" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_protocol_dispatch({rendered_args});\n"
                ));
            }
            "ok" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_ok({rendered_args});\n"
                ));
            }
            "err" => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_make_err({rendered_args});\n"
                ));
            }
            other => {
                out.push_str(&format!(
                    "  TnVal {temp} = tn_runtime_fail(\"unsupported closure builtin call target: {other}\");\n"
                ));
            }
        },
        IrCallTarget::Function { name } => {
            let symbol = mangle_function_name(name, argc);
            out.push_str(&format!("  TnVal {temp} = {symbol}({rendered_args});\n"));
        }
    }

    out.push_str(&format!("  tn_runtime_retain({temp});\n"));
    out.push_str(&format!("  tn_runtime_root_frame_pop({root_frame});\n"));
    out.push_str(&format!("  tn_runtime_root_register({temp});\n"));
    out.push_str(&format!("  tn_runtime_release({temp});\n"));
    stack.push(temp);
    Ok(())
}

fn pop_stack_value(stack: &mut Vec<String>, context: &str) -> Result<String, CBackendError> {
    stack.pop().ok_or_else(|| {
        CBackendError::new(format!("c backend closure stack underflow for {context}"))
    })
}

fn c_string_literal(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
