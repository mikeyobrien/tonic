use super::stubs::c_string_literal;

pub(super) fn emit_stubs_types(source_path: &str, source: &str, out: &mut String) {
    let escaped_source_path = c_string_literal(source_path);
    let escaped_source = c_string_literal(source);
    out.push_str(&format!(
        r###"/* runtime helpers */
static const char *tn_runtime_source_path = {escaped_source_path};
static const char *tn_runtime_source = {escaped_source};

typedef struct {{
  size_t offset;
  int active;
}} TnErrorContext;

static size_t tn_runtime_error_offset = 0;
static int tn_runtime_error_offset_active = 0;

static size_t tn_runtime_utf8_advance(const char *text, size_t len, size_t index) {{
  unsigned char lead = (unsigned char)text[index];

  if ((lead & 0x80u) == 0) {{
    return 1;
  }}

  if ((lead & 0xE0u) == 0xC0u && index + 1 < len && (((unsigned char)text[index + 1]) & 0xC0u) == 0x80u) {{
    return 2;
  }}

  if ((lead & 0xF0u) == 0xE0u && index + 2 < len && (((unsigned char)text[index + 1]) & 0xC0u) == 0x80u && (((unsigned char)text[index + 2]) & 0xC0u) == 0x80u) {{
    return 3;
  }}

  if ((lead & 0xF8u) == 0xF0u && index + 3 < len && (((unsigned char)text[index + 1]) & 0xC0u) == 0x80u && (((unsigned char)text[index + 2]) & 0xC0u) == 0x80u && (((unsigned char)text[index + 3]) & 0xC0u) == 0x80u) {{
    return 4;
  }}

  return 1;
}}

static size_t tn_runtime_utf8_codepoint_count(const char *text, size_t len) {{
  size_t count = 0;
  size_t index = 0;

  while (index < len) {{
    index += tn_runtime_utf8_advance(text, len, index);
    count += 1;
  }}

  return count;
}}

static int tn_runtime_is_char_boundary(const char *text, size_t len, size_t offset) {{
  if (offset > len) {{
    return 0;
  }}

  if (offset == len) {{
    return 1;
  }}

  return ((((unsigned char)text[offset]) & 0xC0u) != 0x80u) ? 1 : 0;
}}

static void tn_runtime_emit_source_context(void) {{
  if (!tn_runtime_error_offset_active || tn_runtime_source == NULL) {{
    return;
  }}

  size_t source_len = strlen(tn_runtime_source);
  size_t offset = tn_runtime_error_offset;
  if (offset > source_len || !tn_runtime_is_char_boundary(tn_runtime_source, source_len, offset)) {{
    return;
  }}

  size_t line_start = 0;
  for (size_t i = 0; i < offset; i += 1) {{
    if (tn_runtime_source[i] == '\n') {{
      line_start = i + 1;
    }}
  }}

  size_t line_end = source_len;
  for (size_t i = offset; i < source_len; i += 1) {{
    if (tn_runtime_source[i] == '\n') {{
      line_end = i;
      break;
    }}
  }}

  size_t line_number = 1;
  for (size_t i = 0; i < line_start; i += 1) {{
    if (tn_runtime_source[i] == '\n') {{
      line_number += 1;
    }}
  }}

  size_t column_number =
      tn_runtime_utf8_codepoint_count(tn_runtime_source + line_start, offset - line_start) + 1;

  if (tn_runtime_source_path != NULL && tn_runtime_source_path[0] != '\0') {{
    fprintf(stderr, " --> %s:%zu:%zu\n", tn_runtime_source_path, line_number, column_number);
  }} else {{
    fprintf(stderr, " --> line %zu, column %zu\n", line_number, column_number);
  }}

  fprintf(
      stderr,
      "%4zu | %.*s\n",
      line_number,
      (int)(line_end - line_start),
      tn_runtime_source + line_start);
  fputs("     | ", stderr);
  for (size_t index = line_start; index < offset;) {{
    if (tn_runtime_source[index] == '\t') {{
      fputc('\t', stderr);
      index += 1;
      continue;
    }}

    fputc(' ', stderr);
    index += tn_runtime_utf8_advance(tn_runtime_source, source_len, index);
  }}
  fputs("^\n", stderr);
}}

static void tn_runtime_emit_failure(const char *message) {{
  if (tn_runtime_error_offset_active) {{
    fprintf(stderr, "error: %s at offset %zu\n", message, tn_runtime_error_offset);
    tn_runtime_emit_source_context();
    return;
  }}

  fprintf(stderr, "error: %s\n", message);
}}

static TnErrorContext tn_runtime_push_error_context(size_t offset) {{
  TnErrorContext previous = {{tn_runtime_error_offset, tn_runtime_error_offset_active}};
  tn_runtime_error_offset = offset;
  tn_runtime_error_offset_active = 1;
  return previous;
}}

static void tn_runtime_pop_error_context(TnErrorContext previous) {{
  tn_runtime_error_offset = previous.offset;
  tn_runtime_error_offset_active = previous.active;
}}

"###,
    ));
    out.push_str(
        r###"static TnVal tn_stub_abort(const char *name) {
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
  TN_OBJ_CLOSURE,
  TN_OBJ_BINARY
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
static int tn_stdout_observed = 0;

static const uint32_t TN_GC_FLAG_MARK = UINT32_C(1);

static int tn_is_boxed(TnVal value);
static void tn_runtime_retain(TnVal value);
static void tn_runtime_release(TnVal value);
static void tn_runtime_reset_stdout_observed(void);
static void tn_runtime_observe_stdout(void);
static int tn_runtime_stdout_was_observed(void);
static int tn_runtime_memory_trace_enabled(void);
static const char *tn_runtime_memory_mode_label(void);
static const char *tn_runtime_cycle_collection_label(void);
static void tn_runtime_gc_mark_value(TnVal value);
static void tn_runtime_gc_collect(void);
static void tn_runtime_gc_finalize(void);

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

static void tn_runtime_reset_stdout_observed(void) {
  tn_stdout_observed = 0;
}

static void tn_runtime_observe_stdout(void) {
  tn_stdout_observed = 1;
}

static int tn_runtime_stdout_was_observed(void) {
  return tn_stdout_observed;
}

static void tn_runtime_gc_finalize(void) {
  if (tn_runtime_memory_trace_enabled()) {
    tn_runtime_gc_collect();
  }
}

static void tn_runtime_memory_stats_print(void) {
  if (!tn_runtime_memory_stats_enabled()) {
    return;
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
static const uint64_t TN_BOX_MASK = UINT64_C(0xfff0000000000000);

static int tn_is_boxed(TnVal value) {
  return ((((uint64_t)value) & TN_BOX_MASK) == TN_BOX_TAG) != 0;
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
  tn_runtime_emit_failure(message);
  exit(1);
}

static TnVal tn_runtime_failf(const char *format, ...) {
  va_list args;
  va_start(args, format);

  va_list count_args;
  va_copy(count_args, args);
  int needed = vsnprintf(NULL, 0, format, count_args);
  va_end(count_args);

  if (needed < 0) {
    va_end(args);
    tn_runtime_emit_failure("native runtime formatting failure");
    exit(1);
  }

  char *message = (char *)malloc((size_t)needed + 1);
  if (message == NULL) {
    va_end(args);
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  vsnprintf(message, (size_t)needed + 1, format, args);
  va_end(args);
  tn_runtime_emit_failure(message);
  free(message);
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
"###,
    );
}
