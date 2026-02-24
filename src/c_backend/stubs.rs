/// Emit the C file preamble: include directives and typedef.
pub(super) fn emit_header(out: &mut String) {
    out.push_str("/* tonic c backend - generated file */\n");
    out.push_str("#include <stdio.h>\n");
    out.push_str("#include <stdlib.h>\n");
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <inttypes.h>\n");
    out.push_str("#include <string.h>\n");
    out.push_str("#include <stdarg.h>\n");
    out.push('\n');
    out.push_str("typedef int64_t TnVal;\n");
    out.push('\n');
}

/// Emit runtime helper definitions for the generated C program.
///
/// Task 05 helpers are implemented inline; unsupported helpers remain explicit
/// abort stubs so failures stay deterministic.
pub(super) fn emit_runtime_stubs(out: &mut String) {
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
  TN_OBJ_RANGE
} TnObjKind;

typedef struct {
  TnVal key;
  TnVal value;
} TnPair;

typedef struct TnObj {
  TnObjKind kind;
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
  } as;
} TnObj;

static TnObj **tn_heap = NULL;
static size_t tn_heap_len = 0;
static size_t tn_heap_cap = 0;

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
  return obj;
}

static TnVal tn_heap_store(TnObj *obj) {
  if (tn_heap_len == tn_heap_cap) {
    size_t next_cap = tn_heap_cap == 0 ? 64 : tn_heap_cap * 2;
    TnObj **next_heap = (TnObj **)realloc(tn_heap, next_cap * sizeof(TnObj *));
    if (next_heap == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }

    tn_heap = next_heap;
    tn_heap_cap = next_cap;
  }

  tn_heap[tn_heap_len] = obj;
  tn_heap_len += 1;
  return tn_make_box(tn_heap_len);
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

static void tn_runtime_init_singletons(void) {
  if (tn_true_value != 0) {
    return;
  }

  TnObj *true_obj = tn_new_obj(TN_OBJ_BOOL);
  true_obj->as.bool_value = 1;
  tn_true_value = tn_heap_store(true_obj);

  TnObj *false_obj = tn_new_obj(TN_OBJ_BOOL);
  false_obj->as.bool_value = 0;
  tn_false_value = tn_heap_store(false_obj);

  TnObj *nil_obj = tn_new_obj(TN_OBJ_NIL);
  tn_nil_value = tn_heap_store(nil_obj);
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
    default:
      return 0;
  }
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
    next->as.map_like.items[existing_index].value = value;
  } else {
    size_t write_index = next->as.map_like.len;
    next->as.map_like.items[write_index].key = key;
    next->as.map_like.items[write_index].value = value;
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

  return map->as.map_like.items[existing_index].value;
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

    // Single-arg stubs
    for name in &[
        "tn_runtime_make_ok",
        "tn_runtime_make_err",
        "tn_runtime_question",
        "tn_runtime_raise",
        "tn_runtime_try",
        "tn_runtime_for",
        "tn_runtime_to_string",
        "tn_runtime_not",
        "tn_runtime_bang",
        "tn_runtime_load_binding",
        "tn_runtime_protocol_dispatch",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(TnVal _a) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Two-arg stubs
    for name in &[
        "tn_runtime_match_operator",
        "tn_runtime_concat",
        "tn_runtime_list_concat",
        "tn_runtime_list_subtract",
    ] {
        out.push_str(&format!(
            "static TnVal {name}(TnVal _a, TnVal _b) {{ return tn_stub_abort(\"{name}\"); }}\n"
        ));
    }
    out.push('\n');

    // Three-arg stubs
    out.push_str("static TnVal tn_runtime_make_closure(TnVal _a, TnVal _b, TnVal _c) { return tn_stub_abort(\"tn_runtime_make_closure\"); }\n");
    out.push('\n');

    // Pattern + varargs stubs
    out.push_str("static int tn_runtime_pattern_matches(TnVal _v, TnVal _p) { (void)tn_stub_abort(\"tn_runtime_pattern_matches\"); return 0; }\n");
    out.push_str("static TnVal tn_runtime_host_call_varargs(TnVal _count, ...) { return tn_stub_abort(\"tn_runtime_host_call\"); }\n");
    out.push_str("static TnVal tn_runtime_call_closure_varargs(TnVal _closure, TnVal _count, ...) { return tn_stub_abort(\"tn_runtime_call_closure\"); }\n");
    out.push('\n');
}
