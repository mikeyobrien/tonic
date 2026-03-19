pub(super) fn emit_stubs_constructors(out: &mut String) {
    out.push_str(
        r###"static void tn_runtime_init_singletons(void) {
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

static TnVal tn_runtime_make_bitstring_varargs(TnVal count, ...) {
  if (count < 0) {
    return tn_stub_abort("tn_runtime_make_bitstring");
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

"###,
    );
}
