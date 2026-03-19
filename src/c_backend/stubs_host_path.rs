pub(super) fn emit_stubs_host_path(out: &mut String) {
    out.push_str(
        r###"  if (strcmp(key, "path_join") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Path.join expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *left_obj = tn_get_obj(args[1]);
    TnObj *right_obj = tn_get_obj(args[2]);
    if (left_obj == NULL || left_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: Path.join expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (right_obj == NULL || right_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: Path.join expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    const char *left = left_obj->as.text.text;
    const char *right = right_obj->as.text.text;
    if (right[0] == '/') {
      free(args);
      return tn_runtime_const_string((TnVal)(intptr_t)right);
    }
    if (left[0] == '\0') {
      free(args);
      return tn_runtime_const_string((TnVal)(intptr_t)right);
    }
    size_t left_len = strlen(left);
    size_t right_len = strlen(right);
    int needs_sep = left_len > 0 && left[left_len - 1] != '/';
    size_t result_len = left_len + right_len + (needs_sep ? 1 : 0);
    char *joined = (char *)malloc(result_len + 1);
    if (joined == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    snprintf(joined,
             result_len + 1,
             needs_sep ? "%s/%s" : "%s%s",
             left,
             right);
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)joined);
    free(joined);
    free(args);
    return result;
  }

  if (strcmp(key, "path_dirname") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Path.dirname expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: Path.dirname expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    size_t len = strlen(path);
    while (len > 1 && path[len - 1] == '/') {
      len -= 1;
    }
    while (len > 0 && path[len - 1] != '/') {
      len -= 1;
    }
    if (len == 0) {
      free(args);
      return tn_runtime_const_string((TnVal)(intptr_t)".");
    }
    while (len > 1 && path[len - 1] == '/') {
      len -= 1;
    }
    char *dir = (char *)malloc(len + 1);
    if (dir == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    memcpy(dir, path, len);
    dir[len] = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)dir);
    free(dir);
    free(args);
    return result;
  }

  if (strcmp(key, "path_basename") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Path.basename expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: Path.basename expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    size_t len = strlen(path);
    while (len > 1 && path[len - 1] == '/') {
      len -= 1;
    }
    const char *start = path;
    for (size_t i = len; i > 0; i -= 1) {
      if (path[i - 1] == '/') {
        start = path + i;
        break;
      }
    }
    size_t base_len = len - (size_t)(start - path);
    char *base = (char *)malloc(base_len + 1);
    if (base == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    memcpy(base, start, base_len);
    base[base_len] = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)base);
    free(base);
    free(args);
    return result;
  }

  if (strcmp(key, "path_extname") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Path.extname expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: Path.extname expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    size_t len = strlen(path);
    while (len > 1 && path[len - 1] == '/') {
      len -= 1;
    }
    size_t start = 0;
    for (size_t i = len; i > 0; i -= 1) {
      if (path[i - 1] == '/') {
        start = i;
        break;
      }
    }
    size_t dot_index = len;
    for (size_t i = len; i > start; i -= 1) {
      if (path[i - 1] == '.') {
        dot_index = i - 1;
        break;
      }
    }
    if (dot_index == len || dot_index == start) {
      free(args);
      return tn_runtime_const_string((TnVal)(intptr_t)"");
    }
    size_t ext_len = len - dot_index;
    char *ext = (char *)malloc(ext_len + 1);
    if (ext == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    memcpy(ext, path + dot_index, ext_len);
    ext[ext_len] = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)ext);
    free(ext);
    free(args);
    return result;
  }

  if (strcmp(key, "path_expand") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Path.expand expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: Path.expand expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    char *expanded = NULL;
    const char *effective = path;
    if (path[0] == '~' && (path[1] == '/' || path[1] == '\0')) {
      const char *home = getenv("HOME");
      if (home == NULL) {
        home = "/";
      }
      size_t home_len = strlen(home);
      size_t suffix_len = strlen(path + 1);
      expanded = (char *)malloc(home_len + suffix_len + 1);
      if (expanded == NULL) {
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }
      snprintf(expanded, home_len + suffix_len + 1, "%s%s", home, path + 1);
      effective = expanded;
    }
    if (effective[0] == '/') {
      TnVal result = tn_runtime_const_string((TnVal)(intptr_t)effective);
      free(expanded);
      free(args);
      return result;
    }
    char cwd_buffer[PATH_MAX];
    if (getcwd(cwd_buffer, sizeof(cwd_buffer)) == NULL) {
      return tn_runtime_fail("host error: Path.expand could not get cwd");
    }
    size_t cwd_len = strlen(cwd_buffer);
    size_t path_len = strlen(effective);
    int needs_sep = cwd_len > 0 && cwd_buffer[cwd_len - 1] != '/';
    size_t result_len = cwd_len + path_len + (path_len > 0 && needs_sep ? 1 : 0);
    char *absolute = (char *)malloc(result_len + 1);
    if (absolute == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    if (path_len == 0) {
      snprintf(absolute, result_len + 1, "%s", cwd_buffer);
    } else {
      snprintf(absolute,
               result_len + 1,
               needs_sep ? "%s/%s" : "%s%s",
               cwd_buffer,
               effective);
    }
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)absolute);
    free(absolute);
    free(expanded);
    free(args);
    return result;
  }

  if (strcmp(key, "path_relative_to") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Path.relative_to expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    TnObj *base_obj = tn_get_obj(args[2]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: Path.relative_to expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (base_obj == NULL || base_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: Path.relative_to expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    const char *path = path_obj->as.text.text;
    const char *base = base_obj->as.text.text;
    size_t base_len = strlen(base);
    if (strncmp(path, base, base_len) == 0) {
      const char *relative = path + base_len;
      if (relative[0] == '/') {
        relative += 1;
      }
      if (relative[0] == '\0' || base_len == 0 || base[base_len - 1] == '/' || path[base_len] == '\0' || path[base_len] == '/') {
        free(args);
        return tn_runtime_const_string((TnVal)(intptr_t)relative);
      }
    }
    free(args);
    return tn_runtime_const_string((TnVal)(intptr_t)path);
  }

  if (strcmp(key, "io_puts") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: IO.puts expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal result = tn_host_io_puts(args[1]);
    free(args);
    return result;
  }

  if (strcmp(key, "io_inspect") == 0) {
    if (argc < 2) {
      return tn_runtime_failf("host error: IO.inspect expects at least 1 argument, found %zu", argc - 1);
    }
    TnVal result = tn_host_io_inspect(args[1]);
    free(args);
    return result;
  }

  if (strcmp(key, "io_gets") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: IO.gets expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal result = tn_host_io_gets(args[1]);
    free(args);
    return result;
  }

  if (strcmp(key, "io_ansi_red") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: IO.ansi_red expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal result = tn_host_io_ansi_red(args[1]);
    free(args);
    return result;
  }

  if (strcmp(key, "io_ansi_green") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: IO.ansi_green expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal result = tn_host_io_ansi_green(args[1]);
    free(args);
    return result;
  }

  if (strcmp(key, "io_ansi_yellow") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: IO.ansi_yellow expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal result = tn_host_io_ansi_yellow(args[1]);
    free(args);
    return result;
  }

  if (strcmp(key, "io_ansi_blue") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: IO.ansi_blue expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal result = tn_host_io_ansi_blue(args[1]);
    free(args);
    return result;
  }

  if (strcmp(key, "io_ansi_reset") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: IO.ansi_reset expects exactly 0 arguments, found %zu", argc - 1);
    }
    TnVal result = tn_host_io_ansi_reset();
    free(args);
    return result;
  }

  if (strcmp(key, "map_keys") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Map.keys expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal result = tn_host_map_keys(args[1]);
    free(args);
    return result;
  }

  if (strcmp(key, "map_values") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Map.values expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal result = tn_host_map_values(args[1]);
    free(args);
    return result;
  }

  if (strcmp(key, "map_merge") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Map.merge expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal result = tn_host_map_merge(args[1], args[2]);
    free(args);
    return result;
  }

  if (strcmp(key, "map_drop") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Map.drop expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal result = tn_host_map_filter_keys(args[1], args[2], 0);
    free(args);
    return result;
  }

  if (strcmp(key, "map_take") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Map.take expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal result = tn_host_map_filter_keys(args[1], args[2], 1);
    free(args);
    return result;
  }

  if (strcmp(key, "map_get") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: Map.get expects exactly 3 arguments, found %zu", argc - 1);
    }
    TnVal result = tn_host_map_get(args[1], args[2], args[3]);
    free(args);
    return result;
  }

  if (strcmp(key, "map_put") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: Map.put expects exactly 3 arguments, found %zu", argc - 1);
    }
    tn_expect_host_map_arg("Map.put", args[1], 1);
    TnVal result = tn_runtime_map_put(args[1], args[2], args[3]);
    free(args);
    return result;
  }

  if (strcmp(key, "map_delete") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Map.delete expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal result = tn_host_map_delete(args[1], args[2]);
    free(args);
    return result;
  }

"###,
    );
}
