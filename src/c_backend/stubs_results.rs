pub(super) fn emit_stubs_results(out: &mut String) {
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
    // Callee already returns with rc=1 (from its terminator retain).
    // Do not add an extra retain here: the call site handles registration.
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

static TnVal tn_runtime_byte_size(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  if (obj == NULL || (obj->kind != TN_OBJ_LIST && obj->kind != TN_OBJ_KEYWORD)) {
    return tn_runtime_failf("byte_size expects a bitstring (list), found %s", tn_runtime_value_kind(value));
  }
  return (TnVal)obj->as.list.len;
}

static TnVal tn_runtime_bit_size(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  if (obj == NULL || (obj->kind != TN_OBJ_LIST && obj->kind != TN_OBJ_KEYWORD)) {
    return tn_runtime_failf("bit_size expects a bitstring (list), found %s", tn_runtime_value_kind(value));
  }
  return (TnVal)(obj->as.list.len * 8);
}

"###,
    );
}
