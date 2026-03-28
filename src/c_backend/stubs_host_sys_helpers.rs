pub(super) fn emit_stubs_host_sys_helpers(out: &mut String) {
    out.push_str(
        r###"static long long tn_expect_host_int_arg(const char *function, TnVal value, size_t index) {
  if (tn_get_obj(value) != NULL) {
    tn_runtime_failf(
        "host error: %s expects int argument %zu; found %s",
        function,
        index,
        tn_runtime_value_kind(value));
  }

  return (long long)value;
}

static int tn_sys_log_is_space(char ch) {
  return isspace((unsigned char)ch) != 0;
}

static int tn_sys_string_is_blank(const char *text) {
  while (*text != '\0') {
    if (!tn_sys_log_is_space(*text)) {
      return 0;
    }
    text += 1;
  }

  return 1;
}

static const char *tn_sys_log_level_label(const char *raw) {
  while (*raw != '\0' && tn_sys_log_is_space(*raw)) {
    raw += 1;
  }

  size_t len = strlen(raw);
  while (len > 0 && tn_sys_log_is_space(raw[len - 1])) {
    len -= 1;
  }

  char normalized[6];
  if (len == 0 || len >= sizeof(normalized)) {
    return NULL;
  }

  for (size_t i = 0; i < len; i += 1) {
    normalized[i] = (char)tolower((unsigned char)raw[i]);
  }
  normalized[len] = '\0';

  if (strcmp(normalized, "debug") == 0) {
    return "debug";
  }
  if (strcmp(normalized, "info") == 0) {
    return "info";
  }
  if (strcmp(normalized, "warn") == 0) {
    return "warn";
  }
  if (strcmp(normalized, "error") == 0) {
    return "error";
  }

  return NULL;
}

static long long tn_sys_unix_timestamp_ms(void) {
  struct timeval now;
  if (gettimeofday(&now, NULL) != 0) {
    return 0;
  }

  return ((long long)now.tv_sec * 1000LL) + ((long long)now.tv_usec / 1000LL);
}

static void tn_sys_json_write_string(FILE *sink, const char *text) {
  fputc('"', sink);
  for (const unsigned char *cursor = (const unsigned char *)text; *cursor != '\0'; cursor += 1) {
    unsigned char ch = *cursor;
    switch (ch) {
      case '"':
        fputs("\\\"", sink);
        break;
      case '\\':
        fputs("\\\\", sink);
        break;
      case '\b':
        fputs("\\b", sink);
        break;
      case '\f':
        fputs("\\f", sink);
        break;
      case '\n':
        fputs("\\n", sink);
        break;
      case '\r':
        fputs("\\r", sink);
        break;
      case '\t':
        fputs("\\t", sink);
        break;
      default:
        if (ch < 0x20) {
          fprintf(sink, "\\u%04x", (unsigned int)ch);
        } else {
          fputc((int)ch, sink);
        }
        break;
    }
  }
  fputc('"', sink);
}

static char *tn_sys_log_child_path(const char *path, const char *segment) {
  size_t path_len = strlen(path);
  size_t segment_len = strlen(segment);
  size_t total_len = path_len == 0 ? segment_len + 1 : path_len + 1 + segment_len + 1;
  char *child = (char *)malloc(total_len);
  if (child == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  if (path_len == 0) {
    snprintf(child, total_len, "%s", segment);
  } else {
    snprintf(child, total_len, "%s.%s", path, segment);
  }

  return child;
}

static void tn_sys_log_write_json_value(FILE *sink, const char *path, TnVal value);

static void tn_sys_log_write_json_map_like(FILE *sink, const char *path, const TnObj *map_like) {
  fputc('{', sink);
  for (size_t i = 0; i < map_like->as.map_like.len; i += 1) {
    if (i > 0) {
      fputc(',', sink);
    }

    TnVal key = map_like->as.map_like.items[i].key;
    TnObj *key_obj = tn_get_obj(key);
    if (key_obj == NULL || (key_obj->kind != TN_OBJ_ATOM && key_obj->kind != TN_OBJ_STRING)) {
      tn_runtime_failf(
          "host error: sys_log %s key at entry %zu must be atom or string; found %s",
          path,
          i + 1,
          tn_runtime_value_kind(key));
    }

    const char *key_name = key_obj->as.text.text;
    if (tn_sys_string_is_blank(key_name)) {
      tn_runtime_failf(
          "host error: sys_log %s key at entry %zu must not be empty",
          path,
          i + 1);
    }

    tn_sys_json_write_string(sink, key_name);
    fputc(':', sink);
    char *child_path = tn_sys_log_child_path(path, key_name);
    tn_sys_log_write_json_value(sink, child_path, map_like->as.map_like.items[i].value);
    free(child_path);
  }
  fputc('}', sink);
}

static void tn_sys_log_write_json_list(FILE *sink, const char *path, const TnObj *list) {
  fputc('[', sink);
  for (size_t i = 0; i < list->as.list.len; i += 1) {
    if (i > 0) {
      fputc(',', sink);
    }

    char index_buf[32];
    snprintf(index_buf, sizeof(index_buf), "%zu", i);
    char *child_path = tn_sys_log_child_path(path, index_buf);
    tn_sys_log_write_json_value(sink, child_path, list->as.list.items[i]);
    free(child_path);
  }
  fputc(']', sink);
}

static void tn_sys_log_write_json_value(FILE *sink, const char *path, TnVal value) {
  TnObj *obj = tn_get_obj(value);
  if (obj == NULL) {
    fprintf(sink, "%lld", (long long)value);
    return;
  }

  switch (obj->kind) {
    case TN_OBJ_BOOL:
      fputs(obj->as.bool_value ? "true" : "false", sink);
      return;
    case TN_OBJ_NIL:
      fputs("null", sink);
      return;
    case TN_OBJ_ATOM:
    case TN_OBJ_STRING:
      tn_sys_json_write_string(sink, obj->as.text.text);
      return;
    case TN_OBJ_FLOAT: {
      char *end = NULL;
      errno = 0;
      double parsed = strtod(obj->as.text.text, &end);
      if (errno != 0 || end == obj->as.text.text || (end != NULL && *end != '\0') || !isfinite(parsed)) {
        tn_runtime_failf(
            "host error: sys_log %s float must parse as finite number; found %s",
            path,
            obj->as.text.text);
      }
      fputs(obj->as.text.text, sink);
      return;
    }
    case TN_OBJ_TUPLE: {
      fputc('[', sink);
      char *left_path = tn_sys_log_child_path(path, "0");
      tn_sys_log_write_json_value(sink, left_path, obj->as.tuple.left);
      free(left_path);
      fputc(',', sink);
      char *right_path = tn_sys_log_child_path(path, "1");
      tn_sys_log_write_json_value(sink, right_path, obj->as.tuple.right);
      free(right_path);
      fputc(']', sink);
      return;
    }
    case TN_OBJ_LIST:
    case TN_OBJ_BINARY:
      tn_sys_log_write_json_list(sink, path, obj);
      return;
    case TN_OBJ_MAP:
    case TN_OBJ_KEYWORD:
      tn_sys_log_write_json_map_like(sink, path, obj);
      return;
    case TN_OBJ_RANGE:
      fprintf(sink, "{\"start\":%lld,\"end\":%lld}", (long long)obj->as.range.start, (long long)obj->as.range.end);
      return;
    case TN_OBJ_RESULT: {
      fputc('{', sink);
      if (obj->as.result.is_ok) {
        tn_sys_json_write_string(sink, "ok");
        fputc(':', sink);
        char *ok_path = tn_sys_log_child_path(path, "ok");
        tn_sys_log_write_json_value(sink, ok_path, obj->as.result.value);
        free(ok_path);
      } else {
        tn_sys_json_write_string(sink, "err");
        fputc(':', sink);
        char *err_path = tn_sys_log_child_path(path, "err");
        tn_sys_log_write_json_value(sink, err_path, obj->as.result.value);
        free(err_path);
      }
      fputc('}', sink);
      return;
    }
    case TN_OBJ_CLOSURE:
      tn_runtime_failf("host error: sys_log %s does not support function values", path);
      return;
    default:
      tn_runtime_failf("host error: sys_log %s does not support value kind %s", path, tn_runtime_value_kind(value));
      return;
  }
}

static void tn_sys_ensure_parent_dir_for_file(const char *function, const char *path) {
  const char *last_slash = strrchr(path, '/');
  if (last_slash == NULL) {
    return;
  }

  size_t parent_len = (size_t)(last_slash - path);
  if (parent_len == 0) {
    return;
  }

  if (parent_len >= PATH_MAX) {
    tn_runtime_failf("host error: %s failed to create sink directory '%.*s': path too long", function, (int)parent_len, path);
  }

  char parent[PATH_MAX];
  memcpy(parent, path, parent_len);
  parent[parent_len] = '\0';

  for (size_t i = 1; i < parent_len; i += 1) {
    if (parent[i] != '/') {
      continue;
    }

    parent[i] = '\0';
    if (parent[0] != '\0' && mkdir(parent, 0777) != 0 && errno != EEXIST) {
      int mkdir_errno = errno;
      parent[i] = '/';
      tn_runtime_failf(
          "host error: %s failed to create sink directory '%s': %s",
          function,
          parent,
          strerror(mkdir_errno));
    }
    parent[i] = '/';
  }

  if (mkdir(parent, 0777) != 0 && errno != EEXIST) {
    int mkdir_errno = errno;
    tn_runtime_failf(
        "host error: %s failed to create sink directory '%s': %s",
        function,
        parent,
        strerror(mkdir_errno));
  }
}

static void tn_sys_fill_random_bytes(unsigned char *buffer, size_t len) {
  FILE *urandom = fopen("/dev/urandom", "rb");
  if (urandom == NULL) {
    int io_errno = errno != 0 ? errno : EIO;
    tn_runtime_failf("host error: sys_random_token failed: %s", strerror(io_errno));
  }

  size_t offset = 0;
  while (offset < len) {
    size_t bytes_read = fread(buffer + offset, 1, len - offset, urandom);
    if (bytes_read > 0) {
      offset += bytes_read;
      continue;
    }

    int io_errno = ferror(urandom) ? (errno != 0 ? errno : EIO) : EIO;
    fclose(urandom);
    tn_runtime_failf("host error: sys_random_token failed: %s", strerror(io_errno));
  }

  if (fclose(urandom) != 0) {
    int io_errno = errno != 0 ? errno : EIO;
    tn_runtime_failf("host error: sys_random_token failed: %s", strerror(io_errno));
  }
}

static char *tn_sys_base64url_encode(const unsigned char *buffer, size_t len) {
  static const char table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
  size_t full_groups = len / 3;
  size_t remainder = len % 3;
  size_t encoded_len = full_groups * 4 + (remainder == 0 ? 0 : (remainder + 1));
  char *encoded = (char *)malloc(encoded_len + 1);
  if (encoded == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  size_t in_index = 0;
  size_t out_index = 0;
  while (in_index + 3 <= len) {
    unsigned int chunk = ((unsigned int)buffer[in_index] << 16) |
                         ((unsigned int)buffer[in_index + 1] << 8) |
                         (unsigned int)buffer[in_index + 2];
    encoded[out_index++] = table[(chunk >> 18) & 0x3f];
    encoded[out_index++] = table[(chunk >> 12) & 0x3f];
    encoded[out_index++] = table[(chunk >> 6) & 0x3f];
    encoded[out_index++] = table[chunk & 0x3f];
    in_index += 3;
  }

  if (remainder == 1) {
    unsigned int chunk = ((unsigned int)buffer[in_index] << 16);
    encoded[out_index++] = table[(chunk >> 18) & 0x3f];
    encoded[out_index++] = table[(chunk >> 12) & 0x3f];
  } else if (remainder == 2) {
    unsigned int chunk = ((unsigned int)buffer[in_index] << 16) |
                         ((unsigned int)buffer[in_index + 1] << 8);
    encoded[out_index++] = table[(chunk >> 18) & 0x3f];
    encoded[out_index++] = table[(chunk >> 12) & 0x3f];
    encoded[out_index++] = table[(chunk >> 6) & 0x3f];
  }

  encoded[out_index] = '\0';
  return encoded;
}

"###,
    );
}
