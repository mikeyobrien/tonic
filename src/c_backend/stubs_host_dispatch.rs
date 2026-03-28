pub(super) fn emit_stubs_host_dispatch(out: &mut String) {
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
        "typedef struct {\n  char **items;\n  size_t len;\n  size_t cap;\n} TnPathStringList;\n\nstatic int tn_path_string_compare(const void *left, const void *right) {\n  const char *const *left_item = (const char *const *)left;\n  const char *const *right_item = (const char *const *)right;\n  return strcmp(*left_item, *right_item);\n}\n\nstatic void tn_path_string_list_free(TnPathStringList *list) {\n  if (list == NULL) {\n    return;\n  }\n  for (size_t i = 0; i < list->len; i += 1) {\n    free(list->items[i]);\n  }\n  free(list->items);\n  list->items = NULL;\n  list->len = 0;\n  list->cap = 0;\n}\n\nstatic int tn_path_string_list_push(TnPathStringList *list, const char *value) {\n  if (list->len == list->cap) {\n    size_t next_cap = list->cap == 0 ? 8 : list->cap * 2;\n    char **next_items = (char **)realloc(list->items, next_cap * sizeof(char *));\n    if (next_items == NULL) {\n      return 0;\n    }\n    list->items = next_items;\n    list->cap = next_cap;\n  }\n\n  size_t value_len = strlen(value);\n  char *copy = (char *)malloc(value_len + 1);\n  if (copy == NULL) {\n    return 0;\n  }\n  memcpy(copy, value, value_len + 1);\n  list->items[list->len] = copy;\n  list->len += 1;\n  return 1;\n}\n\nstatic int tn_collect_relative_files_recursive(const char *root_path, const char *relative_path, TnPathStringList *files, char *error_message, size_t error_cap) {\n  char directory_path[PATH_MAX];\n  if (relative_path[0] == '\\0') {\n    if (snprintf(directory_path, sizeof(directory_path), \"%s\", root_path) >= (int)sizeof(directory_path)) {\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n      return 0;\n    }\n  } else {\n    if (snprintf(directory_path, sizeof(directory_path), \"%s/%s\", root_path, relative_path) >= (int)sizeof(directory_path)) {\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n      return 0;\n    }\n  }\n\n  DIR *directory = opendir(directory_path);\n  if (directory == NULL) {\n    snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': %s\", root_path, strerror(errno));\n    return 0;\n  }\n\n  TnPathStringList entry_names = {0};\n  struct dirent *entry = NULL;\n  while ((entry = readdir(directory)) != NULL) {\n    if (strcmp(entry->d_name, \".\") == 0 || strcmp(entry->d_name, \"..\") == 0) {\n      continue;\n    }\n    if (!tn_path_string_list_push(&entry_names, entry->d_name)) {\n      tn_path_string_list_free(&entry_names);\n      closedir(directory);\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': out of memory\", root_path);\n      return 0;\n    }\n  }\n  closedir(directory);\n\n  qsort(entry_names.items, entry_names.len, sizeof(char *), tn_path_string_compare);\n\n  for (size_t i = 0; i < entry_names.len; i += 1) {\n    char child_relative[PATH_MAX];\n    if (relative_path[0] == '\\0') {\n      if (snprintf(child_relative, sizeof(child_relative), \"%s\", entry_names.items[i]) >= (int)sizeof(child_relative)) {\n        tn_path_string_list_free(&entry_names);\n        snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n        return 0;\n      }\n    } else {\n      if (snprintf(child_relative, sizeof(child_relative), \"%s/%s\", relative_path, entry_names.items[i]) >= (int)sizeof(child_relative)) {\n        tn_path_string_list_free(&entry_names);\n        snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n        return 0;\n      }\n    }\n\n    char child_path[PATH_MAX];\n    if (snprintf(child_path, sizeof(child_path), \"%s/%s\", root_path, child_relative) >= (int)sizeof(child_path)) {\n      tn_path_string_list_free(&entry_names);\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n      return 0;\n    }\n\n    struct stat child_stat;\n    if (lstat(child_path, &child_stat) != 0) {\n      int stat_errno = errno;\n      tn_path_string_list_free(&entry_names);\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': %s\", root_path, strerror(stat_errno));\n      return 0;\n    }\n\n    if (S_ISDIR(child_stat.st_mode)) {\n      if (!tn_collect_relative_files_recursive(root_path, child_relative, files, error_message, error_cap)) {\n        tn_path_string_list_free(&entry_names);\n        return 0;\n      }\n    } else if (S_ISREG(child_stat.st_mode)) {\n      if (!tn_path_string_list_push(files, child_relative)) {\n        tn_path_string_list_free(&entry_names);\n        snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': out of memory\", root_path);\n        return 0;\n      }\n    }\n  }\n\n  tn_path_string_list_free(&entry_names);\n  return 1;\n}\n\nstatic int tn_remove_path_recursive(const char *path, char *error_message, size_t error_cap) {\n  struct stat path_stat;\n  if (lstat(path, &path_stat) != 0) {\n    if (errno == ENOENT) {\n      return 2;\n    }\n    snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': %s\", path, strerror(errno));\n    return 0;\n  }\n\n  if (!S_ISDIR(path_stat.st_mode)) {\n    if (unlink(path) != 0) {\n      snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': %s\", path, strerror(errno));\n      return 0;\n    }\n    return 1;\n  }\n\n  DIR *directory = opendir(path);\n  if (directory == NULL) {\n    snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': %s\", path, strerror(errno));\n    return 0;\n  }\n\n  TnPathStringList entry_names = {0};\n  struct dirent *entry = NULL;\n  while ((entry = readdir(directory)) != NULL) {\n    if (strcmp(entry->d_name, \".\") == 0 || strcmp(entry->d_name, \"..\") == 0) {\n      continue;\n    }\n    if (!tn_path_string_list_push(&entry_names, entry->d_name)) {\n      tn_path_string_list_free(&entry_names);\n      closedir(directory);\n      snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': out of memory\", path);\n      return 0;\n    }\n  }\n  closedir(directory);\n\n  qsort(entry_names.items, entry_names.len, sizeof(char *), tn_path_string_compare);\n\n  for (size_t i = 0; i < entry_names.len; i += 1) {\n    char child_path[PATH_MAX];\n    if (snprintf(child_path, sizeof(child_path), \"%s/%s\", path, entry_names.items[i]) >= (int)sizeof(child_path)) {\n      tn_path_string_list_free(&entry_names);\n      snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': path is too long\", path);\n      return 0;\n    }\n\n    int child_result = tn_remove_path_recursive(child_path, error_message, error_cap);\n    if (child_result == 0) {\n      tn_path_string_list_free(&entry_names);\n      return 0;\n    }\n  }\n\n  tn_path_string_list_free(&entry_names);\n\n  if (rmdir(path) != 0) {\n    snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': %s\", path, strerror(errno));\n    return 0;\n  }\n\n  return 1;\n}\n\nstatic size_t tn_utf8_codepoint_count(const char *text) {\n  size_t count = 0;\n  size_t index = 0;\n  size_t len = strlen(text);\n\n  while (index < len) {\n    unsigned char lead = (unsigned char)text[index];\n    size_t advance = 1;\n\n    if ((lead & 0x80u) == 0) {\n      advance = 1;\n    } else if ((lead & 0xE0u) == 0xC0u && index + 1 < len && (((unsigned char)text[index + 1]) & 0xC0u) == 0x80u) {\n      advance = 2;\n    } else if ((lead & 0xF0u) == 0xE0u && index + 2 < len && (((unsigned char)text[index + 1]) & 0xC0u) == 0x80u && (((unsigned char)text[index + 2]) & 0xC0u) == 0x80u) {\n      advance = 3;\n    } else if ((lead & 0xF8u) == 0xF0u && index + 3 < len && (((unsigned char)text[index + 1]) & 0xC0u) == 0x80u && (((unsigned char)text[index + 2]) & 0xC0u) == 0x80u && (((unsigned char)text[index + 3]) & 0xC0u) == 0x80u) {\n      advance = 4;\n    }\n\n    index += advance;\n    count += 1;\n  }\n\n  return count;\n}\n\nstatic uint32_t tn_utf8_decode_next(const char *text, size_t len, size_t *index) {\n  unsigned char lead = (unsigned char)text[*index];\n\n  if ((lead & 0x80u) == 0) {\n    *index += 1;\n    return (uint32_t)lead;\n  }\n\n  if ((lead & 0xE0u) == 0xC0u && *index + 1 < len) {\n    unsigned char b1 = (unsigned char)text[*index + 1];\n    if ((b1 & 0xC0u) == 0x80u) {\n      *index += 2;\n      return ((uint32_t)(lead & 0x1Fu) << 6) | (uint32_t)(b1 & 0x3Fu);\n    }\n  }\n\n  if ((lead & 0xF0u) == 0xE0u && *index + 2 < len) {\n    unsigned char b1 = (unsigned char)text[*index + 1];\n    unsigned char b2 = (unsigned char)text[*index + 2];\n    if ((b1 & 0xC0u) == 0x80u && (b2 & 0xC0u) == 0x80u) {\n      *index += 3;\n      return ((uint32_t)(lead & 0x0Fu) << 12) | ((uint32_t)(b1 & 0x3Fu) << 6) | (uint32_t)(b2 & 0x3Fu);\n    }\n  }\n\n  if ((lead & 0xF8u) == 0xF0u && *index + 3 < len) {\n    unsigned char b1 = (unsigned char)text[*index + 1];\n    unsigned char b2 = (unsigned char)text[*index + 2];\n    unsigned char b3 = (unsigned char)text[*index + 3];\n    if ((b1 & 0xC0u) == 0x80u && (b2 & 0xC0u) == 0x80u && (b3 & 0xC0u) == 0x80u) {\n      *index += 4;\n      return ((uint32_t)(lead & 0x07u) << 18) | ((uint32_t)(b1 & 0x3Fu) << 12) | ((uint32_t)(b2 & 0x3Fu) << 6) | (uint32_t)(b3 & 0x3Fu);\n    }\n  }\n\n  *index += 1;\n  return (uint32_t)lead;\n}\n\n// Globals for sys_argv
int tn_global_argc = 0;
char **tn_global_argv = NULL;

// Globals for HTTP server
static int tn_http_listeners[1024];
static int tn_http_listeners_count = 0;
static int tn_http_connections[1024];
static int tn_http_connections_count = 0;

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
        r###"  if (strcmp(key, "str_split") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: String.split expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    TnObj *delimiter_obj = tn_get_obj(args[2]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.split expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (delimiter_obj == NULL || delimiter_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.split expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    const char *text = text_obj->as.text.text;
    const char *delimiter = delimiter_obj->as.text.text;
    size_t text_len = strlen(text);
    size_t delimiter_len = strlen(delimiter);

    if (delimiter_len == 0) {
      size_t part_count = text_len + 2;
      TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
      list_obj->as.list.len = part_count;
      list_obj->as.list.items = part_count == 0 ? NULL : (TnVal *)calloc(part_count, sizeof(TnVal));
      if (part_count > 0 && list_obj->as.list.items == NULL) {
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }
      list_obj->as.list.items[0] = tn_runtime_const_string((TnVal)(intptr_t)"");
      tn_runtime_retain(list_obj->as.list.items[0]);
      for (size_t i = 0; i < text_len; i += 1) {
        char part[2];
        part[0] = text[i];
        part[1] = '\0';
        list_obj->as.list.items[i + 1] = tn_runtime_const_string((TnVal)(intptr_t)part);
        tn_runtime_retain(list_obj->as.list.items[i + 1]);
      }
      list_obj->as.list.items[part_count - 1] = tn_runtime_const_string((TnVal)(intptr_t)"");
      tn_runtime_retain(list_obj->as.list.items[part_count - 1]);
      free(args);
      return tn_heap_store(list_obj);
    }

    size_t part_count = 1;
    const char *scan = text;
    for (;;) {
      const char *match = strstr(scan, delimiter);
      if (match == NULL) {
        break;
      }
      part_count += 1;
      scan = match + delimiter_len;
    }

    TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
    list_obj->as.list.len = part_count;
    list_obj->as.list.items = part_count == 0 ? NULL : (TnVal *)calloc(part_count, sizeof(TnVal));
    if (part_count > 0 && list_obj->as.list.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }

    const char *cursor = text;
    for (size_t index = 0; index < part_count; index += 1) {
      const char *match = strstr(cursor, delimiter);
      size_t part_len = match == NULL ? strlen(cursor) : (size_t)(match - cursor);
      char *part = (char *)malloc(part_len + 1);
      if (part == NULL) {
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }
      memcpy(part, cursor, part_len);
      part[part_len] = '\0';
      list_obj->as.list.items[index] = tn_runtime_const_string((TnVal)(intptr_t)part);
      tn_runtime_retain(list_obj->as.list.items[index]);
      free(part);
      if (match == NULL) {
        break;
      }
      cursor = match + delimiter_len;
    }

    free(args);
    return tn_heap_store(list_obj);
  }

  if (strcmp(key, "str_replace") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: String.replace expects exactly 3 arguments, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    TnObj *pattern_obj = tn_get_obj(args[2]);
    TnObj *replacement_obj = tn_get_obj(args[3]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.replace expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (pattern_obj == NULL || pattern_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.replace expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    if (replacement_obj == NULL || replacement_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.replace expects string argument 3; found %s", tn_runtime_value_kind(args[3]));
    }

    const char *text = text_obj->as.text.text;
    const char *pattern = pattern_obj->as.text.text;
    const char *replacement = replacement_obj->as.text.text;
    size_t text_len = strlen(text);
    size_t pattern_len = strlen(pattern);
    size_t replacement_len = strlen(replacement);

    if (pattern_len == 0) {
      size_t insertion_count = tn_utf8_codepoint_count(text) + 1;
      size_t replaced_len = text_len + (insertion_count * replacement_len);
      char *replaced = (char *)malloc(replaced_len + 1);
      if (replaced == NULL) {
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }

      char *cursor = replaced;
      memcpy(cursor, replacement, replacement_len);
      cursor += replacement_len;

      size_t index = 0;
      while (index < text_len) {
        size_t start = index;
        tn_utf8_decode_next(text, text_len, &index);
        size_t segment_len = index - start;
        memcpy(cursor, text + start, segment_len);
        cursor += segment_len;
        memcpy(cursor, replacement, replacement_len);
        cursor += replacement_len;
      }
      *cursor = '\0';

      TnVal result = tn_runtime_const_string((TnVal)(intptr_t)replaced);
      free(replaced);
      free(args);
      return result;
    }

    size_t match_count = 0;
    const char *scan = text;
    for (;;) {
      const char *match = strstr(scan, pattern);
      if (match == NULL) {
        break;
      }
      match_count += 1;
      scan = match + pattern_len;
    }

    size_t replaced_len = text_len;
    if (replacement_len >= pattern_len) {
      replaced_len += match_count * (replacement_len - pattern_len);
    } else {
      replaced_len -= match_count * (pattern_len - replacement_len);
    }

    char *replaced = (char *)malloc(replaced_len + 1);
    if (replaced == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }

    const char *cursor = text;
    char *write_cursor = replaced;
    for (;;) {
      const char *match = strstr(cursor, pattern);
      if (match == NULL) {
        size_t tail_len = strlen(cursor);
        memcpy(write_cursor, cursor, tail_len);
        write_cursor += tail_len;
        break;
      }

      size_t prefix_len = (size_t)(match - cursor);
      memcpy(write_cursor, cursor, prefix_len);
      write_cursor += prefix_len;
      memcpy(write_cursor, replacement, replacement_len);
      write_cursor += replacement_len;
      cursor = match + pattern_len;
    }
    *write_cursor = '\0';

    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)replaced);
    free(replaced);
    free(args);
    return result;
  }

  if (strcmp(key, "str_trim") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.trim expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.trim expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *text = text_obj->as.text.text;
    size_t start = 0;
    size_t end = strlen(text);
    while (start < end && (text[start] == ' ' || text[start] == '\n' || text[start] == '\r' || text[start] == '\t' || text[start] == '\f' || text[start] == '\v')) {
      start += 1;
    }
    while (end > start && (text[end - 1] == ' ' || text[end - 1] == '\n' || text[end - 1] == '\r' || text[end - 1] == '\t' || text[end - 1] == '\f' || text[end - 1] == '\v')) {
      end -= 1;
    }
    size_t trimmed_len = end - start;
    char *trimmed = (char *)malloc(trimmed_len + 1);
    if (trimmed == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    memcpy(trimmed, text + start, trimmed_len);
    trimmed[trimmed_len] = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)trimmed);
    free(trimmed);
    free(args);
    return result;
  }

  if (strcmp(key, "str_trim_leading") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.trim_leading expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.trim_leading expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *text = text_obj->as.text.text;
    size_t start = 0;
    size_t end = strlen(text);
    while (start < end && (text[start] == ' ' || text[start] == '\n' || text[start] == '\r' || text[start] == '\t' || text[start] == '\f' || text[start] == '\v')) {
      start += 1;
    }
    size_t trimmed_len = end - start;
    char *trimmed = (char *)malloc(trimmed_len + 1);
    if (trimmed == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    memcpy(trimmed, text + start, trimmed_len);
    trimmed[trimmed_len] = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)trimmed);
    free(trimmed);
    free(args);
    return result;
  }

  if (strcmp(key, "str_trim_trailing") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.trim_trailing expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.trim_trailing expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *text = text_obj->as.text.text;
    size_t end = strlen(text);
    while (end > 0 && (text[end - 1] == ' ' || text[end - 1] == '\n' || text[end - 1] == '\r' || text[end - 1] == '\t' || text[end - 1] == '\f' || text[end - 1] == '\v')) {
      end -= 1;
    }
    char *trimmed = (char *)malloc(end + 1);
    if (trimmed == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    memcpy(trimmed, text, end);
    trimmed[end] = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)trimmed);
    free(trimmed);
    free(args);
    return result;
  }

  if (strcmp(key, "str_starts_with") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: String.starts_with expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    TnObj *prefix_obj = tn_get_obj(args[2]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.starts_with expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (prefix_obj == NULL || prefix_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.starts_with expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    const char *text = text_obj->as.text.text;
    const char *prefix = prefix_obj->as.text.text;
    size_t text_len = strlen(text);
    size_t prefix_len = strlen(prefix);
    free(args);
    return tn_runtime_const_bool((TnVal)(text_len >= prefix_len && strncmp(text, prefix, prefix_len) == 0));
  }

  if (strcmp(key, "str_ends_with") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: String.ends_with expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    TnObj *suffix_obj = tn_get_obj(args[2]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.ends_with expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (suffix_obj == NULL || suffix_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.ends_with expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    const char *text = text_obj->as.text.text;
    const char *suffix = suffix_obj->as.text.text;
    size_t text_len = strlen(text);
    size_t suffix_len = strlen(suffix);
    free(args);
    return tn_runtime_const_bool((TnVal)(text_len >= suffix_len && strcmp(text + text_len - suffix_len, suffix) == 0));
  }

  if (strcmp(key, "str_contains") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: String.contains expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    TnObj *substr_obj = tn_get_obj(args[2]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.contains expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (substr_obj == NULL || substr_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.contains expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    free(args);
    return tn_runtime_const_bool((TnVal)(strstr(text_obj->as.text.text, substr_obj->as.text.text) != NULL));
  }

  if (strcmp(key, "str_to_charlist") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.to_charlist expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.to_charlist expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *text = text_obj->as.text.text;
    size_t text_len = strlen(text);
    size_t list_len = tn_utf8_codepoint_count(text);
    TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
    list_obj->as.list.len = list_len;
    list_obj->as.list.items = list_len == 0 ? NULL : (TnVal *)calloc(list_len, sizeof(TnVal));
    if (list_len > 0 && list_obj->as.list.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    size_t index = 0;
    for (size_t item_index = 0; item_index < list_len; item_index += 1) {
      list_obj->as.list.items[item_index] = (TnVal)(int64_t)tn_utf8_decode_next(text, text_len, &index);
    }
    free(args);
    return tn_heap_store(list_obj);
  }

  if (strcmp(key, "str_slice") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: String.slice expects exactly 3 arguments, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.slice expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (tn_is_boxed(args[2])) {
      return tn_runtime_failf("host error: String.slice expects int argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    if (tn_is_boxed(args[3])) {
      return tn_runtime_failf("host error: String.slice expects int argument 3; found %s", tn_runtime_value_kind(args[3]));
    }
    const char *text = text_obj->as.text.text;
    size_t text_len = strlen(text);
    int64_t start = (int64_t)args[2];
    int64_t len = (int64_t)args[3];
    int64_t resolved_start = start < 0 ? (int64_t)text_len + start : start;
    if (resolved_start < 0) {
      resolved_start = 0;
    }
    if ((size_t)resolved_start > text_len) {
      resolved_start = (int64_t)text_len;
    }
    size_t resolved_len = len < 0 ? 0 : (size_t)len;
    size_t max_len = text_len - (size_t)resolved_start;
    if (resolved_len > max_len) {
      resolved_len = max_len;
    }
    char *slice = (char *)malloc(resolved_len + 1);
    if (slice == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    memcpy(slice, text + resolved_start, resolved_len);
    slice[resolved_len] = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)slice);
    free(slice);
    free(args);
    return result;
  }

  if (strcmp(key, "str_to_integer") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.to_integer expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.to_integer expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *text = text_obj->as.text.text;
    const char *start = text;
    while (*start == ' ' || *start == '\n' || *start == '\r' || *start == '\t' || *start == '\f' || *start == '\v') {
      start += 1;
    }
    errno = 0;
    char *end = NULL;
    long long value = strtoll(start, &end, 10);
    while (end != NULL && (*end == ' ' || *end == '\n' || *end == '\r' || *end == '\t' || *end == '\f' || *end == '\v')) {
      end += 1;
    }
    if (start == end || errno == ERANGE || end == NULL || *end != '\0') {
      return tn_runtime_failf("host error: String.to_integer could not parse \"%s\" as integer", text);
    }
    free(args);
    return (TnVal)((int64_t)value);
  }

"###,
    );
}
