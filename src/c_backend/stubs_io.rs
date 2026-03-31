pub(super) fn emit_stubs_io(out: &mut String) {
    out.push_str(
        r###"static TnVal tn_host_io_puts(TnVal value) {
  const char *text = tn_expect_host_string_arg("IO.puts", value, 1);
  tn_runtime_observe_stdout();
  fputs(text, stdout);
  fputc('\n', stdout);
  return tn_runtime_const_nil();
}

static TnVal tn_host_io_inspect(TnVal value) {
  tn_render_value(stderr, value);
  fputc('\n', stderr);
  tn_runtime_retain(value);
  return value;
}

/* alias so ops.rs builtin "inspect" can call tn_runtime_inspect */
static TnVal tn_runtime_inspect(TnVal value) {
  return tn_host_io_inspect(value);
}

static TnVal tn_host_io_gets(TnVal prompt_value) {
  const char *prompt = tn_expect_host_string_arg("IO.gets", prompt_value, 1);
  tn_runtime_observe_stdout();
  fputs(prompt, stdout);
  if (fflush(stdout) != 0) {
    int io_errno = errno != 0 ? errno : EIO;
    tn_runtime_failf("host error: IO.gets failed to flush stdout: %s", strerror(io_errno));
  }

  size_t buffer_cap = 128;
  size_t buffer_len = 0;
  char *buffer = (char *)malloc(buffer_cap + 1);
  if (buffer == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  for (;;) {
    int ch = fgetc(stdin);
    if (ch == EOF) {
      if (ferror(stdin)) {
        int io_errno = errno != 0 ? errno : EIO;
        free(buffer);
        tn_runtime_failf("host error: IO.gets failed to read line: %s", strerror(io_errno));
      }
      break;
    }

    if ((char)ch == '\n') {
      break;
    }

    if (buffer_len == buffer_cap) {
      size_t next_cap = buffer_cap > SIZE_MAX / 2 ? buffer_cap + 1 : buffer_cap * 2;
      if (next_cap <= buffer_cap) {
        next_cap = buffer_cap + 1;
      }
      char *next_buffer = (char *)realloc(buffer, next_cap + 1);
      if (next_buffer == NULL) {
        free(buffer);
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }
      buffer = next_buffer;
      buffer_cap = next_cap;
    }

    buffer[buffer_len] = (char)ch;
    buffer_len += 1;
  }

  if (buffer_len > 0 && buffer[buffer_len - 1] == '\r') {
    buffer_len -= 1;
  }
  buffer[buffer_len] = '\0';

  TnVal result = tn_runtime_const_string((TnVal)(intptr_t)buffer);
  free(buffer);
  return result;
}

static TnVal tn_host_io_wrap_ansi(const char *function, TnVal value, const char *prefix) {
  const char *text = tn_expect_host_string_arg(function, value, 1);
  static const char *suffix = "\x1b[0m";
  size_t prefix_len = strlen(prefix);
  size_t text_len = strlen(text);
  size_t suffix_len = strlen(suffix);
  char *buffer = (char *)malloc(prefix_len + text_len + suffix_len + 1);
  if (buffer == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  memcpy(buffer, prefix, prefix_len);
  memcpy(buffer + prefix_len, text, text_len);
  memcpy(buffer + prefix_len + text_len, suffix, suffix_len + 1);

  TnVal result = tn_runtime_const_string((TnVal)(intptr_t)buffer);
  free(buffer);
  return result;
}

static TnVal tn_host_io_ansi_red(TnVal value) {
  return tn_host_io_wrap_ansi("IO.ansi_red", value, "\x1b[31m");
}

static TnVal tn_host_io_ansi_green(TnVal value) {
  return tn_host_io_wrap_ansi("IO.ansi_green", value, "\x1b[32m");
}

static TnVal tn_host_io_ansi_yellow(TnVal value) {
  return tn_host_io_wrap_ansi("IO.ansi_yellow", value, "\x1b[33m");
}

static TnVal tn_host_io_ansi_blue(TnVal value) {
  return tn_host_io_wrap_ansi("IO.ansi_blue", value, "\x1b[34m");
}

static TnVal tn_host_io_ansi_reset(void) {
  return tn_runtime_const_string((TnVal)(intptr_t)"\x1b[0m");
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

static TnVal tn_runtime_not_in(TnVal left, TnVal right) {
  TnVal in_result = tn_runtime_in(left, right);
  /* negate the boolean result */
  if (in_result == tn_runtime_const_bool(1)) {
    return tn_runtime_const_bool(0);
  }
  return tn_runtime_const_bool(1);
}

static TnVal tn_runtime_stepped_range(TnVal start_val, TnVal step_val) {
  /* stub: stepped ranges are not yet fully supported in native compilation */
  (void)step_val;
  return start_val;
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
    case TN_OBJ_BINARY:
      fputs("<<", out);
      for (size_t i = 0; i < obj->as.list.len; i += 1) {
        if (i > 0) {
          fputs(", ", out);
        }
        tn_render_value(out, obj->as.list.items[i]);
      }
      fputs(">>", out);
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
}
