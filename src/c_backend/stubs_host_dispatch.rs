use std::fmt::Write as _;
use std::sync::OnceLock;

struct UnicodeCaseMappingEntry {
    codepoint: u32,
    mapped: Vec<u32>,
}

fn collect_unicode_case_mappings<F>(map_case: F) -> Vec<UnicodeCaseMappingEntry>
where
    F: Fn(char) -> Vec<char>,
{
    let mut mappings = Vec::new();
    for codepoint in 0..=0x10FFFF {
        if (0xD800..=0xDFFF).contains(&codepoint) {
            continue;
        }
        let Some(ch) = char::from_u32(codepoint) else {
            continue;
        };
        let mapped_chars = map_case(ch);
        if mapped_chars.len() == 1 && mapped_chars[0] == ch {
            continue;
        }
        mappings.push(UnicodeCaseMappingEntry {
            codepoint,
            mapped: mapped_chars.into_iter().map(u32::from).collect(),
        });
    }
    mappings
}

fn emit_unicode_case_mapping_table(
    out: &mut String,
    name: &str,
    mappings: &[UnicodeCaseMappingEntry],
    mapping_slots: usize,
) {
    writeln!(out, "static const TnUnicodeCaseMapping {name}[] = {{").unwrap();
    for mapping in mappings {
        let mut slots = vec!["0u".to_string(); mapping_slots];
        for (index, codepoint) in mapping.mapped.iter().enumerate() {
            slots[index] = format!("0x{codepoint:06X}u");
        }
        writeln!(
            out,
            "  {{ 0x{:06X}u, {}, {{{}}} }},",
            mapping.codepoint,
            mapping.mapped.len(),
            slots.join(", ")
        )
        .unwrap();
    }
    out.push_str("};\n");
    writeln!(
        out,
        "static const size_t {name}_len = sizeof({name}) / sizeof({name}[0]);\n"
    )
    .unwrap();
}

fn unicode_case_helpers() -> &'static str {
    static HELPERS: OnceLock<String> = OnceLock::new();
    HELPERS
        .get_or_init(|| {
            let uppercase_mappings =
                collect_unicode_case_mappings(|ch| ch.to_uppercase().collect());
            let lowercase_mappings =
                collect_unicode_case_mappings(|ch| ch.to_lowercase().collect());
            let mapping_slots = uppercase_mappings
                .iter()
                .chain(lowercase_mappings.iter())
                .map(|mapping| mapping.mapped.len())
                .max()
                .unwrap_or(1);

            let mut out = String::new();
            writeln!(
                &mut out,
                "#define TN_UNICODE_CASE_MAPPING_SLOTS {mapping_slots}\n"
            )
            .unwrap();
            out.push_str(
                r#"typedef struct {
  uint32_t codepoint;
  size_t len;
  uint32_t mapped[TN_UNICODE_CASE_MAPPING_SLOTS];
} TnUnicodeCaseMapping;

static size_t tn_utf8_encoded_len(uint32_t codepoint) {
  if (codepoint > 0x10FFFFu || (codepoint >= 0xD800u && codepoint <= 0xDFFFu)) {
    codepoint = 0xFFFDu;
  }
  if (codepoint <= 0x7Fu) {
    return 1;
  }
  if (codepoint <= 0x7FFu) {
    return 2;
  }
  if (codepoint <= 0xFFFFu) {
    return 3;
  }
  return 4;
}

static size_t tn_utf8_encode(uint32_t codepoint, char *out) {
  if (codepoint > 0x10FFFFu || (codepoint >= 0xD800u && codepoint <= 0xDFFFu)) {
    codepoint = 0xFFFDu;
  }
  if (codepoint <= 0x7Fu) {
    out[0] = (char)codepoint;
    return 1;
  }
  if (codepoint <= 0x7FFu) {
    out[0] = (char)(0xC0u | (codepoint >> 6));
    out[1] = (char)(0x80u | (codepoint & 0x3Fu));
    return 2;
  }
  if (codepoint <= 0xFFFFu) {
    out[0] = (char)(0xE0u | (codepoint >> 12));
    out[1] = (char)(0x80u | ((codepoint >> 6) & 0x3Fu));
    out[2] = (char)(0x80u | (codepoint & 0x3Fu));
    return 3;
  }
  out[0] = (char)(0xF0u | (codepoint >> 18));
  out[1] = (char)(0x80u | ((codepoint >> 12) & 0x3Fu));
  out[2] = (char)(0x80u | ((codepoint >> 6) & 0x3Fu));
  out[3] = (char)(0x80u | (codepoint & 0x3Fu));
  return 4;
}

static const TnUnicodeCaseMapping *tn_unicode_case_mapping_lookup(
    const TnUnicodeCaseMapping *mappings,
    size_t mapping_len,
    uint32_t codepoint
) {
  size_t left = 0;
  size_t right = mapping_len;
  while (left < right) {
    size_t mid = left + ((right - left) / 2);
    uint32_t mid_codepoint = mappings[mid].codepoint;
    if (codepoint < mid_codepoint) {
      right = mid;
    } else if (codepoint > mid_codepoint) {
      left = mid + 1;
    } else {
      return &mappings[mid];
    }
  }
  return NULL;
}

static char *tn_utf8_map_case(
    const char *text,
    const TnUnicodeCaseMapping *mappings,
    size_t mapping_len
) {
  size_t text_len = strlen(text);
  size_t mapped_len = 0;
  size_t index = 0;
  while (index < text_len) {
    uint32_t codepoint = tn_utf8_decode_next(text, text_len, &index);
    const TnUnicodeCaseMapping *mapping =
        tn_unicode_case_mapping_lookup(mappings, mapping_len, codepoint);
    if (mapping == NULL) {
      mapped_len += tn_utf8_encoded_len(codepoint);
      continue;
    }
    for (size_t mapping_index = 0; mapping_index < mapping->len; mapping_index += 1) {
      mapped_len += tn_utf8_encoded_len(mapping->mapped[mapping_index]);
    }
  }

  char *mapped = (char *)malloc(mapped_len + 1);
  if (mapped == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  char *cursor = mapped;
  index = 0;
  while (index < text_len) {
    uint32_t codepoint = tn_utf8_decode_next(text, text_len, &index);
    const TnUnicodeCaseMapping *mapping =
        tn_unicode_case_mapping_lookup(mappings, mapping_len, codepoint);
    if (mapping == NULL) {
      cursor += tn_utf8_encode(codepoint, cursor);
      continue;
    }
    for (size_t mapping_index = 0; mapping_index < mapping->len; mapping_index += 1) {
      cursor += tn_utf8_encode(mapping->mapped[mapping_index], cursor);
    }
  }

  *cursor = '\0';
  return mapped;
}

"#,
            );
            emit_unicode_case_mapping_table(
                &mut out,
                "tn_uppercase_mappings",
                &uppercase_mappings,
                mapping_slots,
            );
            emit_unicode_case_mapping_table(
                &mut out,
                "tn_lowercase_mappings",
                &lowercase_mappings,
                mapping_slots,
            );
            out
        })
        .as_str()
}

pub(super) fn emit_stubs_host_dispatch(out: &mut String) {
    out.push_str(
        "static TnVal tn_runtime_call_compiled_closure(TnVal descriptor_hash, const TnVal *argv, size_t argc);\n",
    );
    out.push_str("static int tn_runtime_number_to_f64(TnVal value, double *out);\n");
    out.push_str("static TnVal tn_runtime_float_from_f64(double value);\n\n");

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
        "typedef struct {\n  char **items;\n  size_t len;\n  size_t cap;\n} TnPathStringList;\n\nstatic int tn_path_string_compare(const void *left, const void *right) {\n  const char *const *left_item = (const char *const *)left;\n  const char *const *right_item = (const char *const *)right;\n  return strcmp(*left_item, *right_item);\n}\n\nstatic void tn_path_string_list_free(TnPathStringList *list) {\n  if (list == NULL) {\n    return;\n  }\n  for (size_t i = 0; i < list->len; i += 1) {\n    free(list->items[i]);\n  }\n  free(list->items);\n  list->items = NULL;\n  list->len = 0;\n  list->cap = 0;\n}\n\nstatic int tn_path_string_list_push(TnPathStringList *list, const char *value) {\n  if (list->len == list->cap) {\n    size_t next_cap = list->cap == 0 ? 8 : list->cap * 2;\n    char **next_items = (char **)realloc(list->items, next_cap * sizeof(char *));\n    if (next_items == NULL) {\n      return 0;\n    }\n    list->items = next_items;\n    list->cap = next_cap;\n  }\n\n  size_t value_len = strlen(value);\n  char *copy = (char *)malloc(value_len + 1);\n  if (copy == NULL) {\n    return 0;\n  }\n  memcpy(copy, value, value_len + 1);\n  list->items[list->len] = copy;\n  list->len += 1;\n  return 1;\n}\n\nstatic int tn_collect_relative_files_recursive(const char *root_path, const char *relative_path, TnPathStringList *files, char *error_message, size_t error_cap) {\n  char directory_path[PATH_MAX];\n  if (relative_path[0] == '\\0') {\n    if (snprintf(directory_path, sizeof(directory_path), \"%s\", root_path) >= (int)sizeof(directory_path)) {\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n      return 0;\n    }\n  } else {\n    if (snprintf(directory_path, sizeof(directory_path), \"%s/%s\", root_path, relative_path) >= (int)sizeof(directory_path)) {\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n      return 0;\n    }\n  }\n\n  DIR *directory = opendir(directory_path);\n  if (directory == NULL) {\n    snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': %s\", root_path, strerror(errno));\n    return 0;\n  }\n\n  TnPathStringList entry_names = {0};\n  struct dirent *entry = NULL;\n  while ((entry = readdir(directory)) != NULL) {\n    if (strcmp(entry->d_name, \".\") == 0 || strcmp(entry->d_name, \"..\") == 0) {\n      continue;\n    }\n    if (!tn_path_string_list_push(&entry_names, entry->d_name)) {\n      tn_path_string_list_free(&entry_names);\n      closedir(directory);\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': out of memory\", root_path);\n      return 0;\n    }\n  }\n  closedir(directory);\n\n  qsort(entry_names.items, entry_names.len, sizeof(char *), tn_path_string_compare);\n\n  for (size_t i = 0; i < entry_names.len; i += 1) {\n    char child_relative[PATH_MAX];\n    if (relative_path[0] == '\\0') {\n      if (snprintf(child_relative, sizeof(child_relative), \"%s\", entry_names.items[i]) >= (int)sizeof(child_relative)) {\n        tn_path_string_list_free(&entry_names);\n        snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n        return 0;\n      }\n    } else {\n      if (snprintf(child_relative, sizeof(child_relative), \"%s/%s\", relative_path, entry_names.items[i]) >= (int)sizeof(child_relative)) {\n        tn_path_string_list_free(&entry_names);\n        snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n        return 0;\n      }\n    }\n\n    char child_path[PATH_MAX];\n    if (snprintf(child_path, sizeof(child_path), \"%s/%s\", root_path, child_relative) >= (int)sizeof(child_path)) {\n      tn_path_string_list_free(&entry_names);\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': path is too long\", root_path);\n      return 0;\n    }\n\n    struct stat child_stat;\n    if (lstat(child_path, &child_stat) != 0) {\n      int stat_errno = errno;\n      tn_path_string_list_free(&entry_names);\n      snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': %s\", root_path, strerror(stat_errno));\n      return 0;\n    }\n\n    if (S_ISDIR(child_stat.st_mode)) {\n      if (!tn_collect_relative_files_recursive(root_path, child_relative, files, error_message, error_cap)) {\n        tn_path_string_list_free(&entry_names);\n        return 0;\n      }\n    } else if (S_ISREG(child_stat.st_mode)) {\n      if (!tn_path_string_list_push(files, child_relative)) {\n        tn_path_string_list_free(&entry_names);\n        snprintf(error_message, error_cap, \"host error: sys_list_files_recursive failed for '%s': out of memory\", root_path);\n        return 0;\n      }\n    }\n  }\n\n  tn_path_string_list_free(&entry_names);\n  return 1;\n}\n\nstatic int tn_remove_path_recursive(const char *path, char *error_message, size_t error_cap) {\n  struct stat path_stat;\n  if (lstat(path, &path_stat) != 0) {\n    if (errno == ENOENT) {\n      return 2;\n    }\n    snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': %s\", path, strerror(errno));\n    return 0;\n  }\n\n  if (!S_ISDIR(path_stat.st_mode)) {\n    if (unlink(path) != 0) {\n      snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': %s\", path, strerror(errno));\n      return 0;\n    }\n    return 1;\n  }\n\n  DIR *directory = opendir(path);\n  if (directory == NULL) {\n    snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': %s\", path, strerror(errno));\n    return 0;\n  }\n\n  TnPathStringList entry_names = {0};\n  struct dirent *entry = NULL;\n  while ((entry = readdir(directory)) != NULL) {\n    if (strcmp(entry->d_name, \".\") == 0 || strcmp(entry->d_name, \"..\") == 0) {\n      continue;\n    }\n    if (!tn_path_string_list_push(&entry_names, entry->d_name)) {\n      tn_path_string_list_free(&entry_names);\n      closedir(directory);\n      snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': out of memory\", path);\n      return 0;\n    }\n  }\n  closedir(directory);\n\n  qsort(entry_names.items, entry_names.len, sizeof(char *), tn_path_string_compare);\n\n  for (size_t i = 0; i < entry_names.len; i += 1) {\n    char child_path[PATH_MAX];\n    if (snprintf(child_path, sizeof(child_path), \"%s/%s\", path, entry_names.items[i]) >= (int)sizeof(child_path)) {\n      tn_path_string_list_free(&entry_names);\n      snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': path is too long\", path);\n      return 0;\n    }\n\n    int child_result = tn_remove_path_recursive(child_path, error_message, error_cap);\n    if (child_result == 0) {\n      tn_path_string_list_free(&entry_names);\n      return 0;\n    }\n  }\n\n  tn_path_string_list_free(&entry_names);\n\n  if (rmdir(path) != 0) {\n    snprintf(error_message, error_cap, \"host error: sys_remove_tree failed for '%s': %s\", path, strerror(errno));\n    return 0;\n  }\n\n  return 1;\n}\n\nstatic size_t tn_utf8_codepoint_count(const char *text) {\n  size_t count = 0;\n  size_t index = 0;\n  size_t len = strlen(text);\n\n  while (index < len) {\n    unsigned char lead = (unsigned char)text[index];\n    size_t advance = 1;\n\n    if ((lead & 0x80u) == 0) {\n      advance = 1;\n    } else if ((lead & 0xE0u) == 0xC0u && index + 1 < len && (((unsigned char)text[index + 1]) & 0xC0u) == 0x80u) {\n      advance = 2;\n    } else if ((lead & 0xF0u) == 0xE0u && index + 2 < len && (((unsigned char)text[index + 1]) & 0xC0u) == 0x80u && (((unsigned char)text[index + 2]) & 0xC0u) == 0x80u) {\n      advance = 3;\n    } else if ((lead & 0xF8u) == 0xF0u && index + 3 < len && (((unsigned char)text[index + 1]) & 0xC0u) == 0x80u && (((unsigned char)text[index + 2]) & 0xC0u) == 0x80u && (((unsigned char)text[index + 3]) & 0xC0u) == 0x80u) {\n      advance = 4;\n    }\n\n    index += advance;\n    count += 1;\n  }\n\n  return count;\n}\n\nstatic uint32_t tn_utf8_decode_next(const char *text, size_t len, size_t *index) {\n  unsigned char lead = (unsigned char)text[*index];\n\n  if ((lead & 0x80u) == 0) {\n    *index += 1;\n    return (uint32_t)lead;\n  }\n\n  if ((lead & 0xE0u) == 0xC0u && *index + 1 < len) {\n    unsigned char b1 = (unsigned char)text[*index + 1];\n    if ((b1 & 0xC0u) == 0x80u) {\n      *index += 2;\n      return ((uint32_t)(lead & 0x1Fu) << 6) | (uint32_t)(b1 & 0x3Fu);\n    }\n  }\n\n  if ((lead & 0xF0u) == 0xE0u && *index + 2 < len) {\n    unsigned char b1 = (unsigned char)text[*index + 1];\n    unsigned char b2 = (unsigned char)text[*index + 2];\n    if ((b1 & 0xC0u) == 0x80u && (b2 & 0xC0u) == 0x80u) {\n      *index += 3;\n      return ((uint32_t)(lead & 0x0Fu) << 12) | ((uint32_t)(b1 & 0x3Fu) << 6) | (uint32_t)(b2 & 0x3Fu);\n    }\n  }\n\n  if ((lead & 0xF8u) == 0xF0u && *index + 3 < len) {\n    unsigned char b1 = (unsigned char)text[*index + 1];\n    unsigned char b2 = (unsigned char)text[*index + 2];\n    unsigned char b3 = (unsigned char)text[*index + 3];\n    if ((b1 & 0xC0u) == 0x80u && (b2 & 0xC0u) == 0x80u && (b3 & 0xC0u) == 0x80u) {\n      *index += 4;\n      return ((uint32_t)(lead & 0x07u) << 18) | ((uint32_t)(b1 & 0x3Fu) << 12) | ((uint32_t)(b2 & 0x3Fu) << 6) | (uint32_t)(b3 & 0x3Fu);\n    }\n  }\n\n  *index += 1;\n  return (uint32_t)lead;\n}\n\n",
    );
    out.push_str(unicode_case_helpers());
    out.push_str(
        "// Globals for sys_argv
int tn_global_argc = 0;
char **tn_global_argv = NULL;

// Globals for HTTP server
static int tn_http_listeners[1024];
static int tn_http_listeners_count = 0;
static int tn_http_connections[1024];
static int tn_http_connections_count = 0;

static TnVal tn_runtime_host_call_varargs_impl(TnVal count, va_list vargs) {\n",
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
    out.push_str("  for (size_t i = 0; i < argc; i += 1) {\n");
    out.push_str("    args[i] = va_arg(vargs, TnVal);\n");
    out.push_str("  }\n\n");
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
        r###"  if (strcmp(key, "enum_join") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Enum.join expects exactly 2 arguments, found %zu", argc - 1);
    }

    TnObj *list_obj = tn_expect_host_list_arg("Enum.join", args[1], 1);
    const char *separator = tn_expect_host_string_arg("Enum.join", args[2], 2);
    size_t separator_len = strlen(separator);
    size_t buffer_cap = 64;
    size_t buffer_len = 0;
    char *buffer = (char *)malloc(buffer_cap + 1);
    if (buffer == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    buffer[0] = '\0';

    for (size_t i = 0; i < list_obj->as.list.len; i += 1) {
      TnVal item = list_obj->as.list.items[i];
      const char *part = NULL;
      size_t part_len = 0;
      char int_buffer[32];
      char atom_buffer[256];
      char bool_buffer[6];
      char nil_buffer[4];
      TnObj *item_obj = tn_get_obj(item);

      if (!tn_is_boxed(item)) {
        int written = snprintf(int_buffer, sizeof(int_buffer), "%lld", (long long)item);
        if (written < 0 || (size_t)written >= sizeof(int_buffer)) {
          free(buffer);
          return tn_runtime_fail("host error: Enum.join integer formatting failed");
        }
        part = int_buffer;
        part_len = (size_t)written;
      } else if (item_obj != NULL && item_obj->kind == TN_OBJ_STRING) {
        part = item_obj->as.text.text;
        part_len = strlen(part);
      } else if (item_obj != NULL && item_obj->kind == TN_OBJ_FLOAT) {
        part = item_obj->as.text.text;
        part_len = strlen(part);
      } else if (item_obj != NULL && item_obj->kind == TN_OBJ_ATOM) {
        int written = snprintf(atom_buffer, sizeof(atom_buffer), ":%s", item_obj->as.text.text);
        if (written < 0 || (size_t)written >= sizeof(atom_buffer)) {
          free(buffer);
          return tn_runtime_fail("host error: Enum.join atom formatting failed");
        }
        part = atom_buffer;
        part_len = (size_t)written;
      } else if (item_obj != NULL && item_obj->kind == TN_OBJ_BOOL) {
        const char *text = item_obj->as.bool_value ? "true" : "false";
        int written = snprintf(bool_buffer, sizeof(bool_buffer), "%s", text);
        if (written < 0 || (size_t)written >= sizeof(bool_buffer)) {
          free(buffer);
          return tn_runtime_fail("host error: Enum.join bool formatting failed");
        }
        part = bool_buffer;
        part_len = (size_t)written;
      } else if (item_obj != NULL && item_obj->kind == TN_OBJ_NIL) {
        int written = snprintf(nil_buffer, sizeof(nil_buffer), "nil");
        if (written < 0 || (size_t)written >= sizeof(nil_buffer)) {
          free(buffer);
          return tn_runtime_fail("host error: Enum.join nil formatting failed");
        }
        part = nil_buffer;
        part_len = (size_t)written;
      } else {
        free(buffer);
        return tn_runtime_failf("host error: Enum.join cannot render element %zu of type %s", i + 1, tn_runtime_value_kind(item));
      }

      size_t required = buffer_len + part_len + (i == 0 ? 0 : separator_len);
      if (required > buffer_cap) {
        size_t next_cap = buffer_cap;
        while (required > next_cap) {
          next_cap = next_cap > SIZE_MAX / 2 ? required : next_cap * 2;
          if (next_cap < required) {
            next_cap = required;
          }
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

      if (i > 0) {
        memcpy(buffer + buffer_len, separator, separator_len);
        buffer_len += separator_len;
      }
      memcpy(buffer + buffer_len, part, part_len);
      buffer_len += part_len;
      buffer[buffer_len] = '\0';
    }

    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)buffer);
    free(buffer);
    free(args);
    return result;
  }

  if (strcmp(key, "str_split") == 0) {
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

  if (strcmp(key, "str_upcase") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.upcase expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.upcase expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    char *mapped = tn_utf8_map_case(
      text_obj->as.text.text,
      tn_uppercase_mappings,
      tn_uppercase_mappings_len
    );
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)mapped);
    free(mapped);
    free(args);
    return result;
  }

  if (strcmp(key, "str_downcase") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.downcase expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.downcase expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    char *mapped = tn_utf8_map_case(
      text_obj->as.text.text,
      tn_lowercase_mappings,
      tn_lowercase_mappings_len
    );
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)mapped);
    free(mapped);
    free(args);
    return result;
  }

  if (strcmp(key, "str_length") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.length expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.length expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    free(args);
    return (TnVal)(int64_t)tn_utf8_codepoint_count(text_obj->as.text.text);
  }

  if (strcmp(key, "str_at") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: String.at expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.at expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (tn_get_obj(args[2]) != NULL) {
      return tn_runtime_failf("host error: String.at expects int argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    const char *text = text_obj->as.text.text;
    size_t text_len = strlen(text);
    size_t char_count = tn_utf8_codepoint_count(text);
    int64_t index = (int64_t)args[2];
    int64_t resolved = index < 0 ? (int64_t)char_count + index : index;
    if (resolved < 0 || (size_t)resolved >= char_count) {
      free(args);
      return tn_runtime_const_nil();
    }

    size_t byte_start = 0;
    for (int64_t current = 0; current < resolved; current += 1) {
      tn_utf8_decode_next(text, text_len, &byte_start);
    }
    size_t byte_end = byte_start;
    tn_utf8_decode_next(text, text_len, &byte_end);
    size_t char_len = byte_end - byte_start;
    char *slice = (char *)malloc(char_len + 1);
    if (slice == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    memcpy(slice, text + byte_start, char_len);
    slice[char_len] = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)slice);
    free(slice);
    free(args);
    return result;
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
    if (tn_get_obj(args[2]) != NULL) {
      return tn_runtime_failf("host error: String.slice expects int argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    if (tn_get_obj(args[3]) != NULL) {
      return tn_runtime_failf("host error: String.slice expects int argument 3; found %s", tn_runtime_value_kind(args[3]));
    }
    const char *text = text_obj->as.text.text;
    size_t text_len = strlen(text);
    size_t char_count = tn_utf8_codepoint_count(text);
    int64_t start = (int64_t)args[2];
    int64_t len = (int64_t)args[3];
    int64_t resolved_start = start < 0 ? (int64_t)char_count + start : start;
    if (resolved_start < 0) {
      resolved_start = 0;
    }
    if ((size_t)resolved_start > char_count) {
      resolved_start = (int64_t)char_count;
    }
    size_t resolved_len = len < 0 ? 0 : (size_t)len;
    size_t max_len = char_count - (size_t)resolved_start;
    if (resolved_len > max_len) {
      resolved_len = max_len;
    }

    size_t byte_start = 0;
    for (int64_t current = 0; current < resolved_start; current += 1) {
      tn_utf8_decode_next(text, text_len, &byte_start);
    }
    size_t byte_end = byte_start;
    for (size_t current = 0; current < resolved_len; current += 1) {
      tn_utf8_decode_next(text, text_len, &byte_end);
    }
    size_t slice_len = byte_end - byte_start;
    char *slice = (char *)malloc(slice_len + 1);
    if (slice == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    memcpy(slice, text + byte_start, slice_len);
    slice[slice_len] = '\0';
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

  if (strcmp(key, "str_to_float") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.to_float expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.to_float expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *text = text_obj->as.text.text;
    const char *start = text;
    while (*start == ' ' || *start == '\n' || *start == '\r' || *start == '\t' || *start == '\f' || *start == '\v') {
      start += 1;
    }
    errno = 0;
    char *end = NULL;
    double value = strtod(start, &end);
    while (end != NULL && (*end == ' ' || *end == '\n' || *end == '\r' || *end == '\t' || *end == '\f' || *end == '\v')) {
      end += 1;
    }
    if (start == end || errno == ERANGE || end == NULL || *end != '\0') {
      return tn_runtime_failf("host error: String.to_float could not parse \"%s\" as float", text);
    }
    char formatted[64];
    int formatted_len = snprintf(formatted, sizeof(formatted), "%.17g", value);
    if (formatted_len < 0 || (size_t)formatted_len >= sizeof(formatted)) {
      return tn_runtime_fail("host error: String.to_float formatting failed");
    }
    TnVal result = tn_runtime_const_float((TnVal)(intptr_t)formatted);
    free(args);
    return result;
  }

  if (strcmp(key, "str_pad_leading") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: String.pad_leading expects exactly 3 arguments, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.pad_leading expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (tn_get_obj(args[2]) != NULL) {
      return tn_runtime_failf("host error: String.pad_leading expects int argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    TnObj *padding_obj = tn_get_obj(args[3]);
    if (padding_obj == NULL || padding_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.pad_leading expects string argument 3; found %s", tn_runtime_value_kind(args[3]));
    }
    int64_t target = (int64_t)args[2];
    if (target < 0) {
      return tn_runtime_fail("host error: String.pad_leading count must be non-negative");
    }
    const char *text = text_obj->as.text.text;
    const char *padding = padding_obj->as.text.text;
    size_t text_char_count = tn_utf8_codepoint_count(text);
    if ((size_t)target <= text_char_count) {
      TnVal result = tn_runtime_const_string((TnVal)(intptr_t)text);
      free(args);
      return result;
    }
    size_t padding_len = strlen(padding);
    size_t padding_char_count = tn_utf8_codepoint_count(padding);
    if (padding_char_count == 0) {
      return tn_runtime_fail("host error: String.pad_leading padding must not be empty");
    }
    uint32_t *padding_codepoints = (uint32_t *)calloc(padding_char_count, sizeof(uint32_t));
    if (padding_codepoints == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    size_t padding_index = 0;
    size_t padding_byte_index = 0;
    while (padding_byte_index < padding_len) {
      padding_codepoints[padding_index] = tn_utf8_decode_next(padding, padding_len, &padding_byte_index);
      padding_index += 1;
    }
    size_t needed = (size_t)target - text_char_count;
    size_t pad_bytes = 0;
    for (size_t i = 0; i < needed; i += 1) {
      pad_bytes += tn_utf8_encoded_len(padding_codepoints[i % padding_char_count]);
    }
    size_t text_len = strlen(text);
    char *padded = (char *)malloc(text_len + pad_bytes + 1);
    if (padded == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    char *cursor = padded;
    for (size_t i = 0; i < needed; i += 1) {
      cursor += tn_utf8_encode(padding_codepoints[i % padding_char_count], cursor);
    }
    memcpy(cursor, text, text_len + 1);
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)padded);
    free(padding_codepoints);
    free(padded);
    free(args);
    return result;
  }

  if (strcmp(key, "str_pad_trailing") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: String.pad_trailing expects exactly 3 arguments, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.pad_trailing expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (tn_get_obj(args[2]) != NULL) {
      return tn_runtime_failf("host error: String.pad_trailing expects int argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    TnObj *padding_obj = tn_get_obj(args[3]);
    if (padding_obj == NULL || padding_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.pad_trailing expects string argument 3; found %s", tn_runtime_value_kind(args[3]));
    }
    int64_t target = (int64_t)args[2];
    if (target < 0) {
      return tn_runtime_fail("host error: String.pad_trailing count must be non-negative");
    }
    const char *text = text_obj->as.text.text;
    const char *padding = padding_obj->as.text.text;
    size_t text_char_count = tn_utf8_codepoint_count(text);
    if ((size_t)target <= text_char_count) {
      TnVal result = tn_runtime_const_string((TnVal)(intptr_t)text);
      free(args);
      return result;
    }
    size_t padding_len = strlen(padding);
    size_t padding_char_count = tn_utf8_codepoint_count(padding);
    if (padding_char_count == 0) {
      return tn_runtime_fail("host error: String.pad_trailing padding must not be empty");
    }
    uint32_t *padding_codepoints = (uint32_t *)calloc(padding_char_count, sizeof(uint32_t));
    if (padding_codepoints == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    size_t padding_index = 0;
    size_t padding_byte_index = 0;
    while (padding_byte_index < padding_len) {
      padding_codepoints[padding_index] = tn_utf8_decode_next(padding, padding_len, &padding_byte_index);
      padding_index += 1;
    }
    size_t needed = (size_t)target - text_char_count;
    size_t pad_bytes = 0;
    for (size_t i = 0; i < needed; i += 1) {
      pad_bytes += tn_utf8_encoded_len(padding_codepoints[i % padding_char_count]);
    }
    size_t text_len = strlen(text);
    char *padded = (char *)malloc(text_len + pad_bytes + 1);
    if (padded == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    char *cursor = padded;
    memcpy(cursor, text, text_len);
    cursor += text_len;
    for (size_t i = 0; i < needed; i += 1) {
      cursor += tn_utf8_encode(padding_codepoints[i % padding_char_count], cursor);
    }
    *cursor = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)padded);
    free(padding_codepoints);
    free(padded);
    free(args);
    return result;
  }

  if (strcmp(key, "str_reverse") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.reverse expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.reverse expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *text = text_obj->as.text.text;
    size_t text_len = strlen(text);
    size_t char_count = tn_utf8_codepoint_count(text);
    size_t *starts = char_count == 0 ? NULL : (size_t *)calloc(char_count, sizeof(size_t));
    size_t *lengths = char_count == 0 ? NULL : (size_t *)calloc(char_count, sizeof(size_t));
    if (char_count > 0 && (starts == NULL || lengths == NULL)) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    size_t index = 0;
    for (size_t i = 0; i < char_count; i += 1) {
      size_t start = index;
      tn_utf8_decode_next(text, text_len, &index);
      starts[i] = start;
      lengths[i] = index - start;
    }
    char *reversed = (char *)malloc(text_len + 1);
    if (reversed == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    char *cursor = reversed;
    for (size_t i = char_count; i > 0; i -= 1) {
      memcpy(cursor, text + starts[i - 1], lengths[i - 1]);
      cursor += lengths[i - 1];
    }
    *cursor = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)reversed);
    free(starts);
    free(lengths);
    free(reversed);
    free(args);
    return result;
  }

  if (strcmp(key, "str_to_atom") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.to_atom expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.to_atom expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    TnVal result = tn_runtime_const_atom((TnVal)(intptr_t)text_obj->as.text.text);
    free(args);
    return result;
  }

  if (strcmp(key, "str_graphemes") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: String.graphemes expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *text_obj = tn_get_obj(args[1]);
    if (text_obj == NULL || text_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: String.graphemes expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *text = text_obj->as.text.text;
    size_t text_len = strlen(text);
    size_t char_count = tn_utf8_codepoint_count(text);
    TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
    list_obj->as.list.len = char_count;
    list_obj->as.list.items = char_count == 0 ? NULL : (TnVal *)calloc(char_count, sizeof(TnVal));
    if (char_count > 0 && list_obj->as.list.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    size_t index = 0;
    for (size_t item_index = 0; item_index < char_count; item_index += 1) {
      size_t start = index;
      tn_utf8_decode_next(text, text_len, &index);
      size_t grapheme_len = index - start;
      char *grapheme = (char *)malloc(grapheme_len + 1);
      if (grapheme == NULL) {
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }
      memcpy(grapheme, text + start, grapheme_len);
      grapheme[grapheme_len] = '\0';
      list_obj->as.list.items[item_index] = tn_runtime_const_string((TnVal)(intptr_t)grapheme);
      tn_runtime_retain(list_obj->as.list.items[item_index]);
      free(grapheme);
    }
    free(args);
    return tn_heap_store(list_obj);
  }

  if (strcmp(key, "integer_parse") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Integer.parse expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *str_obj = tn_get_obj(args[1]);
    if (str_obj == NULL || str_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: Integer.parse expects string argument; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *src = str_obj->as.text.text;
    /* skip leading whitespace */
    while (*src == ' ' || *src == '\t' || *src == '\n' || *src == '\r' || *src == '\x0b' || *src == '\x0c') {
      src++;
    }
    if (*src == '\0') {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"error");
    }
    const char *start = src;
    if (*src == '+' || *src == '-') {
      src++;
    }
    /* require at least one digit after optional sign */
    if (*src < '0' || *src > '9') {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"error");
    }
    while (*src >= '0' && *src <= '9') {
      src++;
    }
    /* parse the numeric prefix */
    size_t num_len = (size_t)(src - start);
    char num_buf[32];
    if (num_len >= sizeof(num_buf)) {
      /* too many digits — overflow */
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"error");
    }
    memcpy(num_buf, start, num_len);
    num_buf[num_len] = '\0';
    char *endptr = NULL;
    long long parsed = strtoll(num_buf, &endptr, 10);
    if (endptr != num_buf + num_len) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"error");
    }
    /* check for overflow (strtoll returns LLONG_MIN/MAX and sets errno) */
    if ((parsed == LLONG_MAX || parsed == LLONG_MIN) && errno == ERANGE) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"error");
    }
    TnVal int_val = (TnVal)parsed;
    TnVal rest_val = tn_runtime_const_string((TnVal)(intptr_t)src);
    TnVal tuple_val = tn_runtime_make_tuple(int_val, rest_val);
    free(args);
    return tuple_val;
  }

  /* ── Integer module ── */

  if (strcmp(key, "integer_to_string") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Integer.to_string expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal arg = args[1];
    free(args);
    if (tn_is_boxed(arg)) {
      return tn_runtime_fail("host error: Integer.to_string expects integer argument");
    }
    int64_t n = (int64_t)arg;
    char buf[32];
    snprintf(buf, sizeof(buf), "%lld", (long long)n);
    return tn_runtime_const_string((TnVal)(intptr_t)buf);
  }

  if (strcmp(key, "integer_to_string_base") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Integer.to_string_base expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2];
    free(args);
    if (tn_is_boxed(a0) || tn_is_boxed(a1)) {
      return tn_runtime_fail("host error: Integer.to_string_base expects integer arguments");
    }
    int64_t n = (int64_t)a0;
    int64_t base = (int64_t)a1;
    if (base < 2 || base > 36) {
      return tn_runtime_failf("Integer.to_string: base must be 2..36, got %lld", (long long)base);
    }
    int negative = n < 0;
    uint64_t val = negative ? (uint64_t)(-(n + 1)) + 1u : (uint64_t)n;
    if (val == 0) {
      return tn_runtime_const_string((TnVal)(intptr_t)"0");
    }
    char buf[66]; /* 64 bits + sign + NUL */
    int pos = 65;
    buf[pos] = '\0';
    while (val > 0) {
      uint64_t d = val % (uint64_t)base;
      buf[--pos] = (char)(d < 10 ? '0' + d : 'a' + d - 10);
      val /= (uint64_t)base;
    }
    if (negative) {
      buf[--pos] = '-';
    }
    return tn_runtime_const_string((TnVal)(intptr_t)&buf[pos]);
  }

  if (strcmp(key, "integer_digits") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Integer.digits expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal arg = args[1];
    free(args);
    if (tn_is_boxed(arg)) {
      return tn_runtime_fail("host error: Integer.digits expects integer argument");
    }
    int64_t n = (int64_t)arg;
    uint64_t abs_n = n < 0 ? (uint64_t)(-(n + 1)) + 1u : (uint64_t)n;
    if (abs_n == 0) {
      TnVal zero = (TnVal)0;
      TnObj *list_obj = (TnObj *)malloc(sizeof(TnObj));
      list_obj->kind = TN_OBJ_LIST;
      list_obj->as.list.items = (TnVal *)malloc(sizeof(TnVal));
      list_obj->as.list.items[0] = zero;
      list_obj->as.list.len = 1;
      return tn_heap_store(list_obj);
    }
    TnVal dbuf[20]; /* max 20 digits for u64 */
    int count = 0;
    uint64_t tmp = abs_n;
    while (tmp > 0) {
      dbuf[count++] = (TnVal)(int64_t)(tmp % 10);
      tmp /= 10;
    }
    /* reverse */
    TnObj *list_obj = (TnObj *)malloc(sizeof(TnObj));
    list_obj->kind = TN_OBJ_LIST;
    list_obj->as.list.items = (TnVal *)malloc(sizeof(TnVal) * (size_t)count);
    list_obj->as.list.len = (size_t)count;
    for (int i = 0; i < count; i++) {
      list_obj->as.list.items[i] = dbuf[count - 1 - i];
    }
    return tn_heap_store(list_obj);
  }

  if (strcmp(key, "integer_undigits") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Integer.undigits expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal arg = args[1];
    free(args);
    TnObj *list_obj = tn_get_obj(arg);
    if (list_obj == NULL || list_obj->kind != TN_OBJ_LIST) {
      return tn_runtime_fail("host error: Integer.undigits expects a list");
    }
    int64_t result = 0;
    for (size_t i = 0; i < list_obj->as.list.len; i++) {
      TnVal item = list_obj->as.list.items[i];
      if (tn_is_boxed(item)) {
        return tn_runtime_failf("Integer.undigits: element %zu is not an integer", i);
      }
      int64_t d = (int64_t)item;
      if (d < 0 || d > 9) {
        return tn_runtime_failf("Integer.undigits: digit out of range 0..9, got %lld", (long long)d);
      }
      int64_t next = result * 10 + d;
      if (next / 10 != result) {
        return tn_runtime_fail("Integer.undigits: overflow");
      }
      result = next;
    }
    return (TnVal)result;
  }

  if (strcmp(key, "integer_gcd") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Integer.gcd expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2];
    free(args);
    if (tn_is_boxed(a0) || tn_is_boxed(a1)) {
      return tn_runtime_fail("host error: Integer.gcd expects integer arguments");
    }
    int64_t x = (int64_t)a0;
    int64_t y = (int64_t)a1;
    uint64_t a = x < 0 ? (uint64_t)(-(x + 1)) + 1u : (uint64_t)x;
    uint64_t b = y < 0 ? (uint64_t)(-(y + 1)) + 1u : (uint64_t)y;
    while (b != 0) {
      uint64_t t = b;
      b = a % b;
      a = t;
    }
    return (TnVal)(int64_t)a;
  }

  if (strcmp(key, "integer_is_even") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Integer.is_even expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal arg = args[1];
    free(args);
    if (tn_is_boxed(arg)) {
      return tn_runtime_fail("host error: Integer.is_even expects integer argument");
    }
    int64_t n = (int64_t)arg;
    return tn_runtime_const_bool((TnVal)(n % 2 == 0 ? 1 : 0));
  }

  if (strcmp(key, "integer_is_odd") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Integer.is_odd expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal arg = args[1];
    free(args);
    if (tn_is_boxed(arg)) {
      return tn_runtime_fail("host error: Integer.is_odd expects integer argument");
    }
    int64_t n = (int64_t)arg;
    return tn_runtime_const_bool((TnVal)(n % 2 != 0 ? 1 : 0));
  }

  if (strcmp(key, "integer_pow") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Integer.pow expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2];
    free(args);
    if (tn_is_boxed(a0) || tn_is_boxed(a1)) {
      return tn_runtime_fail("host error: Integer.pow expects integer arguments");
    }
    int64_t base_val = (int64_t)a0;
    int64_t exp_val = (int64_t)a1;
    if (exp_val < 0) {
      return tn_runtime_fail("Integer.pow: exponent must be non-negative");
    }
    int64_t result = 1;
    for (int64_t i = 0; i < exp_val; i++) {
      int64_t prev = result;
      result *= base_val;
      if (base_val != 0 && result / base_val != prev) {
        return tn_runtime_fail("Integer.pow: overflow");
      }
    }
    return (TnVal)result;
  }

  /* ── Math module ── */

  if (strcmp(key, "math_pow") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Math.pow expects exactly 2 arguments, found %zu", argc - 1);
    }
    double base_d, exp_d;
    if (!tn_runtime_number_to_f64(args[1], &base_d) || !tn_runtime_number_to_f64(args[2], &exp_d)) {
      free(args);
      return tn_runtime_fail("host error: Math.pow expects numeric arguments");
    }
    int both_int = !tn_is_boxed(args[1]) && !tn_is_boxed(args[2]);
    free(args);
    double result = pow(base_d, exp_d);
    if (both_int && result == floor(result) && isfinite(result) && fabs(result) <= (double)INT64_MAX) {
      return (TnVal)(int64_t)result;
    }
    return tn_runtime_float_from_f64(result);
  }

  if (strcmp(key, "math_sqrt") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.sqrt expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.sqrt expects numeric argument");
    }
    free(args);
    if (val < 0.0) {
      return tn_runtime_fail("Math.sqrt: cannot take square root of negative number");
    }
    return tn_runtime_float_from_f64(sqrt(val));
  }

  if (strcmp(key, "math_abs") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.abs expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal arg = args[1];
    free(args);
    if (!tn_is_boxed(arg)) {
      int64_t n = (int64_t)arg;
      return (TnVal)(n < 0 ? -n : n);
    }
    double val;
    if (!tn_runtime_number_to_f64(arg, &val)) {
      return tn_runtime_fail("host error: Math.abs expects numeric argument");
    }
    return tn_runtime_float_from_f64(fabs(val));
  }

  if (strcmp(key, "math_min") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Math.min expects exactly 2 arguments, found %zu", argc - 1);
    }
    double a, b;
    if (!tn_runtime_number_to_f64(args[1], &a) || !tn_runtime_number_to_f64(args[2], &b)) {
      free(args);
      return tn_runtime_fail("host error: Math.min expects numeric arguments");
    }
    TnVal result = (a <= b) ? args[1] : args[2];
    tn_runtime_retain(result);
    free(args);
    return result;
  }

  if (strcmp(key, "math_max") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Math.max expects exactly 2 arguments, found %zu", argc - 1);
    }
    double a, b;
    if (!tn_runtime_number_to_f64(args[1], &a) || !tn_runtime_number_to_f64(args[2], &b)) {
      free(args);
      return tn_runtime_fail("host error: Math.max expects numeric arguments");
    }
    TnVal result = (a >= b) ? args[1] : args[2];
    tn_runtime_retain(result);
    free(args);
    return result;
  }

  if (strcmp(key, "math_log") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.log expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.log expects numeric argument");
    }
    free(args);
    if (val <= 0.0) {
      return tn_runtime_fail("Math.log: argument must be positive");
    }
    return tn_runtime_float_from_f64(log(val));
  }

  if (strcmp(key, "math_log2") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.log2 expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.log2 expects numeric argument");
    }
    free(args);
    if (val <= 0.0) {
      return tn_runtime_fail("Math.log2: argument must be positive");
    }
    return tn_runtime_float_from_f64(log2(val));
  }

  if (strcmp(key, "math_log10") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.log10 expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.log10 expects numeric argument");
    }
    free(args);
    if (val <= 0.0) {
      return tn_runtime_fail("Math.log10: argument must be positive");
    }
    return tn_runtime_float_from_f64(log10(val));
  }

  if (strcmp(key, "math_sin") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.sin expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.sin expects numeric argument");
    }
    free(args);
    return tn_runtime_float_from_f64(sin(val));
  }

  if (strcmp(key, "math_cos") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.cos expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.cos expects numeric argument");
    }
    free(args);
    return tn_runtime_float_from_f64(cos(val));
  }

  if (strcmp(key, "math_tan") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.tan expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.tan expects numeric argument");
    }
    free(args);
    return tn_runtime_float_from_f64(tan(val));
  }

  if (strcmp(key, "math_ceil") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.ceil expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.ceil expects numeric argument");
    }
    free(args);
    return (TnVal)(int64_t)ceil(val);
  }

  if (strcmp(key, "math_floor") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.floor expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.floor expects numeric argument");
    }
    free(args);
    return (TnVal)(int64_t)floor(val);
  }

  if (strcmp(key, "math_round") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Math.round expects exactly 1 argument, found %zu", argc - 1);
    }
    double val;
    if (!tn_runtime_number_to_f64(args[1], &val)) {
      free(args);
      return tn_runtime_fail("host error: Math.round expects numeric argument");
    }
    free(args);
    return (TnVal)(int64_t)round(val);
  }

  /* ── Map module ── */

  if (strcmp(key, "map_keys") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Map.keys expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal a0 = args[1];
    free(args);
    return tn_host_map_keys(a0);
  }

  if (strcmp(key, "map_values") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Map.values expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal a0 = args[1];
    free(args);
    return tn_host_map_values(a0);
  }

  if (strcmp(key, "map_merge") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Map.merge expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2];
    free(args);
    return tn_host_map_merge(a0, a1);
  }

  if (strcmp(key, "map_drop") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Map.drop expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2];
    free(args);
    return tn_host_map_filter_keys(a0, a1, 0);
  }

  if (strcmp(key, "map_take") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Map.take expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2];
    free(args);
    return tn_host_map_filter_keys(a0, a1, 1);
  }

  if (strcmp(key, "map_has_key") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Map.has_key? expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2];
    free(args);
    return tn_host_map_has_key(a0, a1);
  }

  if (strcmp(key, "map_get") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: Map.get expects exactly 3 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2], a2 = args[3];
    free(args);
    return tn_host_map_get(a0, a1, a2);
  }

  if (strcmp(key, "map_put") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: Map.put expects exactly 3 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2], a2 = args[3];
    free(args);
    return tn_runtime_map_put(a0, a1, a2);
  }

  if (strcmp(key, "map_delete") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Map.delete expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2];
    free(args);
    return tn_host_map_delete(a0, a1);
  }

  /* ── Float module ── */

  if (strcmp(key, "float_to_string") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Float.to_string expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal a0 = args[1];
    free(args);
    double val;
    if (!tn_is_boxed(a0)) {
      /* integer → float string like "42.0" */
      char buf[64];
      snprintf(buf, sizeof(buf), "%lld.0", (long long)(int64_t)a0);
      return tn_runtime_const_string((TnVal)(intptr_t)buf);
    }
    if (!tn_runtime_number_to_f64(a0, &val)) {
      return tn_runtime_fail("host error: Float.to_string expects numeric argument");
    }
    /* already a float object — return its string representation */
    TnObj *fobj = tn_get_obj(a0);
    if (fobj != NULL && fobj->kind == TN_OBJ_STRING) {
      tn_runtime_retain(a0);
      return a0;
    }
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", val);
    return tn_runtime_const_string((TnVal)(intptr_t)buf);
  }

  if (strcmp(key, "float_round") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Float.round expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnVal a0 = args[1], a1 = args[2];
    free(args);
    double val;
    if (!tn_runtime_number_to_f64(a0, &val)) {
      return tn_runtime_fail("host error: Float.round expects numeric argument 1");
    }
    if (tn_is_boxed(a1)) {
      return tn_runtime_fail("host error: Float.round expects integer precision");
    }
    int64_t precision = (int64_t)a1;
    double factor = pow(10.0, (double)precision);
    double rounded = round(val * factor) / factor;
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", rounded);
    return tn_runtime_const_float((TnVal)(intptr_t)buf);
  }

  if (strcmp(key, "float_ceil") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Float.ceil expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal a0 = args[1];
    free(args);
    double val;
    if (!tn_runtime_number_to_f64(a0, &val)) {
      return tn_runtime_fail("host error: Float.ceil expects numeric argument");
    }
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", ceil(val));
    return tn_runtime_const_float((TnVal)(intptr_t)buf);
  }

  if (strcmp(key, "float_floor") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Float.floor expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal a0 = args[1];
    free(args);
    double val;
    if (!tn_runtime_number_to_f64(a0, &val)) {
      return tn_runtime_fail("host error: Float.floor expects numeric argument");
    }
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", floor(val));
    return tn_runtime_const_float((TnVal)(intptr_t)buf);
  }

  /* ── Bitwise module ── */

  if (strcmp(key, "bitwise_band") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Bitwise.band expects exactly 2 arguments, found %zu", argc - 1);
    }
    if (tn_is_boxed(args[1]) || tn_is_boxed(args[2])) {
      return tn_runtime_fail("host error: Bitwise.band expects integer arguments");
    }
    int64_t result = (int64_t)args[1] & (int64_t)args[2];
    free(args);
    return (TnVal)result;
  }

  if (strcmp(key, "bitwise_bor") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Bitwise.bor expects exactly 2 arguments, found %zu", argc - 1);
    }
    if (tn_is_boxed(args[1]) || tn_is_boxed(args[2])) {
      return tn_runtime_fail("host error: Bitwise.bor expects integer arguments");
    }
    int64_t result = (int64_t)args[1] | (int64_t)args[2];
    free(args);
    return (TnVal)result;
  }

  if (strcmp(key, "bitwise_bxor") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Bitwise.bxor expects exactly 2 arguments, found %zu", argc - 1);
    }
    if (tn_is_boxed(args[1]) || tn_is_boxed(args[2])) {
      return tn_runtime_fail("host error: Bitwise.bxor expects integer arguments");
    }
    int64_t result = (int64_t)args[1] ^ (int64_t)args[2];
    free(args);
    return (TnVal)result;
  }

  if (strcmp(key, "bitwise_bnot") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Bitwise.bnot expects exactly 1 argument, found %zu", argc - 1);
    }
    if (tn_is_boxed(args[1])) {
      return tn_runtime_fail("host error: Bitwise.bnot expects integer argument");
    }
    int64_t result = ~(int64_t)args[1];
    free(args);
    return (TnVal)result;
  }

  if (strcmp(key, "bitwise_bsl") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Bitwise.bsl expects exactly 2 arguments, found %zu", argc - 1);
    }
    if (tn_is_boxed(args[1]) || tn_is_boxed(args[2])) {
      return tn_runtime_fail("host error: Bitwise.bsl expects integer arguments");
    }
    int64_t shift = (int64_t)args[2];
    if (shift < 0 || shift > 63) {
      free(args);
      return tn_runtime_failf("host error: Bitwise.bsl: shift amount must be 0..63, got %lld", (long long)shift);
    }
    int64_t result = (int64_t)args[1] << shift;
    free(args);
    return (TnVal)result;
  }

  if (strcmp(key, "bitwise_bsr") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Bitwise.bsr expects exactly 2 arguments, found %zu", argc - 1);
    }
    if (tn_is_boxed(args[1]) || tn_is_boxed(args[2])) {
      return tn_runtime_fail("host error: Bitwise.bsr expects integer arguments");
    }
    int64_t shift = (int64_t)args[2];
    if (shift < 0 || shift > 63) {
      free(args);
      return tn_runtime_failf("host error: Bitwise.bsr: shift amount must be 0..63, got %lld", (long long)shift);
    }
    int64_t result = (int64_t)args[1] >> shift;
    free(args);
    return (TnVal)result;
  }

  if (strcmp(key, "hex_encode") == 0 || strcmp(key, "hex_encode_upper") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Hex.encode expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Hex.encode expects a string argument");
    }
    TnObj *str_obj = tn_get_obj(args[1]);
    if (str_obj == NULL || str_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Hex.encode expects a string argument");
    }
    const char *input = str_obj->as.text.text;
    size_t input_len = strlen(input);
    int upper = (strcmp(key, "hex_encode_upper") == 0);
    const char *fmt = upper ? "%02X" : "%02x";
    size_t hex_len = input_len * 2;
    char *hex_buf = (char *)malloc(hex_len + 1);
    if (hex_buf == NULL) {
      free(args);
      return tn_runtime_fail("host error: Hex.encode: out of memory");
    }
    for (size_t i = 0; i < input_len; i++) {
      sprintf(hex_buf + i * 2, fmt, (unsigned char)input[i]);
    }
    hex_buf[hex_len] = '\0';
    TnObj *result_obj = tn_new_obj(TN_OBJ_STRING);
    result_obj->as.text.text = hex_buf;
    free(args);
    return tn_heap_store(result_obj);
  }

  if (strcmp(key, "hex_decode") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Hex.decode expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Hex.decode expects a string argument");
    }
    TnObj *str_obj = tn_get_obj(args[1]);
    if (str_obj == NULL || str_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Hex.decode expects a string argument");
    }
    const char *input = str_obj->as.text.text;
    size_t input_len = strlen(input);
    if (input_len % 2 != 0) {
      free(args);
      TnVal err_tag = tn_runtime_const_atom((TnVal)(intptr_t)"error");
      TnVal msg_val = tn_runtime_const_string((TnVal)(intptr_t)"odd-length hex string");
      return tn_runtime_make_tuple(err_tag, msg_val);
    }
    size_t out_len = input_len / 2;
    char *out_buf = (char *)malloc(out_len + 1);
    if (out_buf == NULL) {
      free(args);
      return tn_runtime_fail("host error: Hex.decode: out of memory");
    }
    for (size_t i = 0; i < input_len; i += 2) {
      int hi = -1, lo = -1;
      unsigned char c = (unsigned char)input[i];
      if (c >= '0' && c <= '9') hi = c - '0';
      else if (c >= 'a' && c <= 'f') hi = c - 'a' + 10;
      else if (c >= 'A' && c <= 'F') hi = c - 'A' + 10;
      c = (unsigned char)input[i + 1];
      if (c >= '0' && c <= '9') lo = c - '0';
      else if (c >= 'a' && c <= 'f') lo = c - 'a' + 10;
      else if (c >= 'A' && c <= 'F') lo = c - 'A' + 10;
      if (hi < 0 || lo < 0) {
        free(out_buf);
        free(args);
        char err_msg[128];
        snprintf(err_msg, sizeof(err_msg), "invalid hex character at position %zu", hi < 0 ? i : i + 1);
        TnVal err_tag = tn_runtime_const_atom((TnVal)(intptr_t)"error");
        TnVal msg_val = tn_runtime_const_string((TnVal)(intptr_t)err_msg);
        return tn_runtime_make_tuple(err_tag, msg_val);
      }
      out_buf[i / 2] = (char)((hi << 4) | lo);
    }
    out_buf[out_len] = '\0';
    free(args);
    TnVal ok_tag = tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    TnObj *decoded_obj = tn_new_obj(TN_OBJ_STRING);
    decoded_obj->as.text.text = out_buf;
    TnVal decoded_val = tn_heap_store(decoded_obj);
    return tn_runtime_make_tuple(ok_tag, decoded_val);
  }

  /* ---- Base64 encode (standard, with = padding) ---- */
  if (strcmp(key, "base64_encode") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Base64.encode expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Base64.encode expects a string argument");
    }
    TnObj *str_obj = tn_get_obj(args[1]);
    if (str_obj == NULL || str_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Base64.encode expects a string argument");
    }
    const char *input = str_obj->as.text.text;
    size_t input_len = strlen(input);
    static const char b64_table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    size_t full_groups = input_len / 3;
    size_t remainder = input_len % 3;
    size_t encoded_len = (remainder == 0) ? full_groups * 4 : (full_groups + 1) * 4;
    char *enc = (char *)malloc(encoded_len + 1);
    if (enc == NULL) { free(args); return tn_runtime_fail("host error: Base64.encode: out of memory"); }
    const unsigned char *src = (const unsigned char *)input;
    size_t si = 0, di = 0;
    while (si + 3 <= input_len) {
      unsigned int chunk = ((unsigned int)src[si] << 16) | ((unsigned int)src[si+1] << 8) | (unsigned int)src[si+2];
      enc[di++] = b64_table[(chunk >> 18) & 0x3f];
      enc[di++] = b64_table[(chunk >> 12) & 0x3f];
      enc[di++] = b64_table[(chunk >> 6) & 0x3f];
      enc[di++] = b64_table[chunk & 0x3f];
      si += 3;
    }
    if (remainder == 1) {
      unsigned int chunk = ((unsigned int)src[si] << 16);
      enc[di++] = b64_table[(chunk >> 18) & 0x3f];
      enc[di++] = b64_table[(chunk >> 12) & 0x3f];
      enc[di++] = '=';
      enc[di++] = '=';
    } else if (remainder == 2) {
      unsigned int chunk = ((unsigned int)src[si] << 16) | ((unsigned int)src[si+1] << 8);
      enc[di++] = b64_table[(chunk >> 18) & 0x3f];
      enc[di++] = b64_table[(chunk >> 12) & 0x3f];
      enc[di++] = b64_table[(chunk >> 6) & 0x3f];
      enc[di++] = '=';
    }
    enc[di] = '\0';
    TnObj *result_obj = tn_new_obj(TN_OBJ_STRING);
    result_obj->as.text.text = enc;
    free(args);
    return tn_heap_store(result_obj);
  }

  /* ---- Base64 decode (standard) ---- */
  if (strcmp(key, "base64_decode") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Base64.decode expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Base64.decode expects a string argument");
    }
    TnObj *str_obj = tn_get_obj(args[1]);
    if (str_obj == NULL || str_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Base64.decode expects a string argument");
    }
    const char *input = str_obj->as.text.text;
    size_t input_len = strlen(input);
    free(args);
    if (input_len == 0) {
      TnObj *empty_obj = tn_new_obj(TN_OBJ_STRING);
      empty_obj->as.text.text = strdup("");
      return tn_heap_store(empty_obj);
    }
    /* Build reverse lookup */
    static const char b64_std[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    int b64_rev[256];
    for (int i = 0; i < 256; i++) b64_rev[i] = -1;
    for (int i = 0; i < 64; i++) b64_rev[(unsigned char)b64_std[i]] = i;
    b64_rev[(unsigned char)'='] = 0; /* padding treated as 0 */
    /* Strip trailing padding to count real chars */
    size_t pad = 0;
    if (input_len >= 1 && input[input_len - 1] == '=') pad++;
    if (input_len >= 2 && input[input_len - 2] == '=') pad++;
    size_t out_len = (input_len / 4) * 3 - pad;
    char *out = (char *)malloc(out_len + 1);
    if (out == NULL) return tn_runtime_fail("host error: Base64.decode: out of memory");
    size_t oi = 0;
    for (size_t i = 0; i < input_len; i += 4) {
      int a = b64_rev[(unsigned char)input[i]];
      int b = (i+1 < input_len) ? b64_rev[(unsigned char)input[i+1]] : -1;
      int c = (i+2 < input_len) ? b64_rev[(unsigned char)input[i+2]] : -1;
      int d = (i+3 < input_len) ? b64_rev[(unsigned char)input[i+3]] : -1;
      if (a < 0 || b < 0 || c < 0 || d < 0) {
        free(out);
        return tn_runtime_fail("host error: Base64.decode: invalid base64 input");
      }
      unsigned int triple = ((unsigned int)a << 18) | ((unsigned int)b << 12) | ((unsigned int)c << 6) | (unsigned int)d;
      if (oi < out_len) out[oi++] = (char)((triple >> 16) & 0xff);
      if (oi < out_len) out[oi++] = (char)((triple >> 8) & 0xff);
      if (oi < out_len) out[oi++] = (char)(triple & 0xff);
    }
    out[out_len] = '\0';
    TnObj *dec_obj = tn_new_obj(TN_OBJ_STRING);
    dec_obj->as.text.text = out;
    return tn_heap_store(dec_obj);
  }

  /* ---- Base64 URL-safe encode (no padding) ---- */
  if (strcmp(key, "base64_url_encode") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Base64.url_encode expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Base64.url_encode expects a string argument");
    }
    TnObj *str_obj = tn_get_obj(args[1]);
    if (str_obj == NULL || str_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Base64.url_encode expects a string argument");
    }
    const char *input = str_obj->as.text.text;
    size_t input_len = strlen(input);
    static const char url_table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    size_t full_groups = input_len / 3;
    size_t remainder = input_len % 3;
    size_t encoded_len = full_groups * 4 + (remainder == 0 ? 0 : (remainder + 1));
    char *enc = (char *)malloc(encoded_len + 1);
    if (enc == NULL) { free(args); return tn_runtime_fail("host error: Base64.url_encode: out of memory"); }
    const unsigned char *src = (const unsigned char *)input;
    size_t si = 0, di = 0;
    while (si + 3 <= input_len) {
      unsigned int chunk = ((unsigned int)src[si] << 16) | ((unsigned int)src[si+1] << 8) | (unsigned int)src[si+2];
      enc[di++] = url_table[(chunk >> 18) & 0x3f];
      enc[di++] = url_table[(chunk >> 12) & 0x3f];
      enc[di++] = url_table[(chunk >> 6) & 0x3f];
      enc[di++] = url_table[chunk & 0x3f];
      si += 3;
    }
    if (remainder == 1) {
      unsigned int chunk = ((unsigned int)src[si] << 16);
      enc[di++] = url_table[(chunk >> 18) & 0x3f];
      enc[di++] = url_table[(chunk >> 12) & 0x3f];
    } else if (remainder == 2) {
      unsigned int chunk = ((unsigned int)src[si] << 16) | ((unsigned int)src[si+1] << 8);
      enc[di++] = url_table[(chunk >> 18) & 0x3f];
      enc[di++] = url_table[(chunk >> 12) & 0x3f];
      enc[di++] = url_table[(chunk >> 6) & 0x3f];
    }
    enc[di] = '\0';
    TnObj *result_obj = tn_new_obj(TN_OBJ_STRING);
    result_obj->as.text.text = enc;
    free(args);
    return tn_heap_store(result_obj);
  }

  /* ---- Base64 URL-safe decode (no padding) ---- */
  if (strcmp(key, "base64_url_decode") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Base64.url_decode expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Base64.url_decode expects a string argument");
    }
    TnObj *str_obj = tn_get_obj(args[1]);
    if (str_obj == NULL || str_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Base64.url_decode expects a string argument");
    }
    const char *input = str_obj->as.text.text;
    size_t input_len = strlen(input);
    free(args);
    if (input_len == 0) {
      TnObj *empty_obj = tn_new_obj(TN_OBJ_STRING);
      empty_obj->as.text.text = strdup("");
      return tn_heap_store(empty_obj);
    }
    static const char url_alpha[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    int url_rev[256];
    for (int i = 0; i < 256; i++) url_rev[i] = -1;
    for (int i = 0; i < 64; i++) url_rev[(unsigned char)url_alpha[i]] = i;
    /* Pad input to multiple of 4 for decoding */
    size_t padded_len = input_len;
    if (padded_len % 4 != 0) padded_len += 4 - (padded_len % 4);
    size_t pad = padded_len - input_len;
    size_t out_len = (padded_len / 4) * 3 - pad;
    char *out = (char *)malloc(out_len + 1);
    if (out == NULL) return tn_runtime_fail("host error: Base64.url_decode: out of memory");
    size_t oi = 0;
    for (size_t i = 0; i < input_len; i += 4) {
      int vals[4] = {0, 0, 0, 0};
      size_t chunk_len = 0;
      for (size_t j = 0; j < 4 && (i + j) < input_len; j++) {
        int v = url_rev[(unsigned char)input[i + j]];
        if (v < 0) {
          free(out);
          return tn_runtime_fail("host error: Base64.url_decode: invalid base64url input");
        }
        vals[j] = v;
        chunk_len++;
      }
      unsigned int triple = ((unsigned int)vals[0] << 18) | ((unsigned int)vals[1] << 12) | ((unsigned int)vals[2] << 6) | (unsigned int)vals[3];
      if (oi < out_len) out[oi++] = (char)((triple >> 16) & 0xff);
      if (chunk_len > 2 && oi < out_len) out[oi++] = (char)((triple >> 8) & 0xff);
      if (chunk_len > 3 && oi < out_len) out[oi++] = (char)(triple & 0xff);
    }
    out[out_len] = '\0';
    TnObj *dec_obj = tn_new_obj(TN_OBJ_STRING);
    dec_obj->as.text.text = out;
    return tn_heap_store(dec_obj);
  }

  /* ---- URL percent-encode (RFC 3986) ---- */
  if (strcmp(key, "url_encode") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Url.encode expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Url.encode expects a string argument");
    }
    TnObj *str_obj = tn_get_obj(args[1]);
    if (str_obj == NULL || str_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Url.encode expects a string argument");
    }
    const char *src = str_obj->as.text.text;
    size_t slen = strlen(src);
    /* Worst case: every byte becomes %XX (3x) */
    char *enc = (char *)malloc(slen * 3 + 1);
    if (enc == NULL) { free(args); return tn_runtime_fail("host error: Url.encode: out of memory"); }
    size_t di = 0;
    static const char hex_upper[] = "0123456789ABCDEF";
    for (size_t si = 0; si < slen; si++) {
      unsigned char c = (unsigned char)src[si];
      /* RFC 3986 unreserved: ALPHA / DIGIT / - / . / _ / ~ */
      if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') ||
          c == '-' || c == '.' || c == '_' || c == '~') {
        enc[di++] = (char)c;
      } else {
        enc[di++] = '%';
        enc[di++] = hex_upper[(c >> 4) & 0x0f];
        enc[di++] = hex_upper[c & 0x0f];
      }
    }
    enc[di] = '\0';
    TnObj *result_obj = tn_new_obj(TN_OBJ_STRING);
    result_obj->as.text.text = enc;
    free(args);
    return tn_heap_store(result_obj);
  }

  /* ---- URL percent-decode ---- */
  if (strcmp(key, "url_decode") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Url.decode expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Url.decode expects a string argument");
    }
    TnObj *str_obj = tn_get_obj(args[1]);
    if (str_obj == NULL || str_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Url.decode expects a string argument");
    }
    const char *src = str_obj->as.text.text;
    size_t slen = strlen(src);
    char *dec = (char *)malloc(slen + 1);
    if (dec == NULL) { free(args); return tn_runtime_fail("host error: Url.decode: out of memory"); }
    size_t di = 0;
    for (size_t si = 0; si < slen; ) {
      if (src[si] == '%') {
        if (si + 2 >= slen) {
          free(dec);
          free(args);
          return tn_runtime_failf("host error: Url.decode: incomplete percent-encoding at position %zu", si);
        }
        unsigned char hi = (unsigned char)src[si + 1];
        unsigned char lo = (unsigned char)src[si + 2];
        /* Convert hex chars to nibble values */
        int h = -1, l = -1;
        if (hi >= '0' && hi <= '9') h = hi - '0';
        else if (hi >= 'A' && hi <= 'F') h = hi - 'A' + 10;
        else if (hi >= 'a' && hi <= 'f') h = hi - 'a' + 10;
        if (lo >= '0' && lo <= '9') l = lo - '0';
        else if (lo >= 'A' && lo <= 'F') l = lo - 'A' + 10;
        else if (lo >= 'a' && lo <= 'f') l = lo - 'a' + 10;
        if (h >= 0 && l >= 0) {
          dec[di++] = (char)((h << 4) | l);
          si += 3;
        } else {
          /* Invalid hex — pass through the % literally */
          free(dec);
          free(args);
          return tn_runtime_failf("host error: Url.decode: invalid hex digits at position %zu", si);
        }
      } else if (src[si] == '+') {
        dec[di++] = ' ';
        si++;
      } else {
        dec[di++] = src[si++];
      }
    }
    dec[di] = '\0';
    TnObj *result_obj = tn_new_obj(TN_OBJ_STRING);
    result_obj->as.text.text = dec;
    free(args);
    return tn_heap_store(result_obj);
  }

  /* tuple_to_list: converts a 2-element tuple into a 2-element list */
  if (strcmp(key, "tuple_to_list") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Tuple.to_list expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Tuple.to_list expects tuple argument; found number");
    }
    TnObj *tup = tn_get_obj(args[1]);
    if (tup == NULL || tup->kind != TN_OBJ_TUPLE) {
      free(args);
      return tn_runtime_failf("host error: Tuple.to_list expects tuple argument; found %s", tn_runtime_value_kind(args[1]));
    }
    TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
    list_obj->as.list.len = 2;
    list_obj->as.list.items = (TnVal *)calloc(2, sizeof(TnVal));
    if (list_obj->as.list.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    list_obj->as.list.items[0] = tup->as.tuple.left;
    list_obj->as.list.items[1] = tup->as.tuple.right;
    free(args);
    return tn_heap_store(list_obj);
  }

  /* list_to_tuple: converts a 2-element list into a tuple */
  if (strcmp(key, "list_to_tuple") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: List.to_tuple expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: List.to_tuple expects list argument; found number");
    }
    TnObj *list = tn_get_obj(args[1]);
    if (list == NULL || list->kind != TN_OBJ_LIST) {
      free(args);
      return tn_runtime_failf("host error: List.to_tuple expects list argument; found %s", tn_runtime_value_kind(args[1]));
    }
    if (list->as.list.len != 2) {
      free(args);
      return tn_runtime_failf("host error: List.to_tuple expects a 2-element list, found %zu elements", list->as.list.len);
    }
    TnObj *tup_obj = tn_new_obj(TN_OBJ_TUPLE);
    tup_obj->as.tuple.left = list->as.list.items[0];
    tup_obj->as.tuple.right = list->as.list.items[1];
    free(args);
    return tn_heap_store(tup_obj);
  }

  /* enum_sort: sorts a list (ints by value, strings lexicographically) */
  if (strcmp(key, "enum_sort") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Enum.sort expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Enum.sort expects list argument; found number");
    }
    TnObj *src_list = tn_get_obj(args[1]);
    if (src_list == NULL || src_list->kind != TN_OBJ_LIST) {
      free(args);
      return tn_runtime_failf("host error: Enum.sort expects list argument; found %s", tn_runtime_value_kind(args[1]));
    }
    size_t len = src_list->as.list.len;
    TnObj *sorted_obj = tn_new_obj(TN_OBJ_LIST);
    sorted_obj->as.list.len = len;
    sorted_obj->as.list.items = len == 0 ? NULL : (TnVal *)calloc(len, sizeof(TnVal));
    if (len > 0 && sorted_obj->as.list.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    for (size_t i = 0; i < len; i++) {
      sorted_obj->as.list.items[i] = src_list->as.list.items[i];
    }
    /* insertion sort — stable, simple, fine for typical list sizes */
    for (size_t i = 1; i < len; i++) {
      TnVal key_val = sorted_obj->as.list.items[i];
      size_t j = i;
      while (j > 0) {
        TnVal a = sorted_obj->as.list.items[j - 1];
        TnVal b = key_val;
        int cmp = 0;
        /* both unboxed → integer comparison */
        if (!tn_is_boxed(a) && !tn_is_boxed(b)) {
          int64_t ia = (int64_t)a;
          int64_t ib = (int64_t)b;
          cmp = (ia > ib) - (ia < ib);
        } else if (tn_is_boxed(a) && tn_is_boxed(b)) {
          TnObj *oa = tn_get_obj(a);
          TnObj *ob = tn_get_obj(b);
          if (oa != NULL && ob != NULL && oa->kind == TN_OBJ_STRING && ob->kind == TN_OBJ_STRING) {
            cmp = strcmp(oa->as.text.text, ob->as.text.text);
          }
          /* floats */
          if (oa != NULL && ob != NULL && oa->kind == TN_OBJ_FLOAT && ob->kind == TN_OBJ_FLOAT) {
            double fa = strtod(oa->as.text.text, NULL);
            double fb = strtod(ob->as.text.text, NULL);
            cmp = (fa > fb) - (fa < fb);
          }
        }
        if (cmp <= 0) break;
        sorted_obj->as.list.items[j] = sorted_obj->as.list.items[j - 1];
        j--;
      }
      sorted_obj->as.list.items[j] = key_val;
    }
    free(args);
    return tn_heap_store(sorted_obj);
  }

  /* enum_slice: returns a sub-list from start index with given count */
  if (strcmp(key, "enum_slice") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: Enum.slice expects exactly 3 arguments, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Enum.slice expects list argument 1; found number");
    }
    TnObj *src_list = tn_get_obj(args[1]);
    if (src_list == NULL || src_list->kind != TN_OBJ_LIST) {
      free(args);
      return tn_runtime_failf("host error: Enum.slice expects list argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (tn_is_boxed(args[2])) {
      free(args);
      return tn_runtime_fail("host error: Enum.slice expects int start; found non-integer");
    }
    if (tn_is_boxed(args[3])) {
      free(args);
      return tn_runtime_fail("host error: Enum.slice expects int count; found non-integer");
    }
    int64_t start = (int64_t)args[2];
    int64_t count = (int64_t)args[3];
    size_t src_len = src_list->as.list.len;
    size_t actual_start = (start < 0 || (size_t)start >= src_len) ? src_len : (size_t)start;
    size_t remaining = src_len - actual_start;
    size_t actual_count = (count < 0) ? 0 : ((size_t)count > remaining ? remaining : (size_t)count);
    TnObj *slice_obj = tn_new_obj(TN_OBJ_LIST);
    slice_obj->as.list.len = actual_count;
    slice_obj->as.list.items = actual_count == 0 ? NULL : (TnVal *)calloc(actual_count, sizeof(TnVal));
    if (actual_count > 0 && slice_obj->as.list.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    for (size_t i = 0; i < actual_count; i++) {
      slice_obj->as.list.items[i] = src_list->as.list.items[actual_start + i];
    }
    free(args);
    return tn_heap_store(slice_obj);
  }

  /* datetime_unix_now: returns seconds since Unix epoch as integer */
  if (strcmp(key, "datetime_unix_now") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: DateTime.unix_now expects exactly 0 arguments, found %zu", argc - 1);
    }
    struct timeval tv;
    if (gettimeofday(&tv, NULL) != 0) {
      free(args);
      return tn_runtime_fail("host error: DateTime.unix_now: gettimeofday failed");
    }
    free(args);
    return (TnVal)((int64_t)tv.tv_sec);
  }

  /* datetime_unix_now_ms: returns milliseconds since Unix epoch as integer */
  if (strcmp(key, "datetime_unix_now_ms") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: DateTime.unix_now_ms expects exactly 0 arguments, found %zu", argc - 1);
    }
    struct timeval tv;
    if (gettimeofday(&tv, NULL) != 0) {
      free(args);
      return tn_runtime_fail("host error: DateTime.unix_now_ms: gettimeofday failed");
    }
    int64_t ms = (int64_t)tv.tv_sec * 1000 + (int64_t)tv.tv_usec / 1000;
    free(args);
    return (TnVal)ms;
  }

  /* random_boolean: returns true or false randomly */
  if (strcmp(key, "random_boolean") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: Random.boolean expects exactly 0 arguments, found %zu", argc - 1);
    }
    free(args);
    static int tn_random_seeded = 0;
    if (!tn_random_seeded) { srand((unsigned)time(NULL)); tn_random_seeded = 1; }
    return tn_runtime_const_bool(rand() % 2 == 0 ? 1 : 0);
  }

  /* random_float: returns random float 0.0..1.0 */
  if (strcmp(key, "random_float") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: Random.float expects exactly 0 arguments, found %zu", argc - 1);
    }
    free(args);
    static int tn_rfloat_seeded = 0;
    if (!tn_rfloat_seeded) { srand((unsigned)time(NULL)); tn_rfloat_seeded = 1; }
    double val = (double)rand() / (double)RAND_MAX;
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", val);
    TnObj *rf_obj = tn_new_obj(TN_OBJ_FLOAT);
    rf_obj->as.text.text = strdup(buf);
    return tn_heap_store(rf_obj);
  }

  /* random_integer: returns random int in range [min, max] */
  if (strcmp(key, "random_integer") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Random.integer expects exactly 2 arguments, found %zu", argc - 1);
    }
    int64_t rmin = (int64_t)args[1];
    int64_t rmax = (int64_t)args[2];
    free(args);
    if (rmin > rmax) {
      return tn_runtime_failf("host error: Random.integer: min (%lld) must be <= max (%lld)", (long long)rmin, (long long)rmax);
    }
    static int tn_rint_seeded = 0;
    if (!tn_rint_seeded) { srand((unsigned)time(NULL)); tn_rint_seeded = 1; }
    int64_t range = rmax - rmin + 1;
    int64_t result = rmin + (int64_t)((unsigned long long)rand() % (unsigned long long)range);
    return (TnVal)result;
  }

  /* uuid_v4: returns UUID v4 string from random bytes */
  if (strcmp(key, "uuid_v4") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: Uuid.v4 expects exactly 0 arguments, found %zu", argc - 1);
    }
    free(args);
    static int tn_uuid_seeded = 0;
    if (!tn_uuid_seeded) { srand((unsigned)time(NULL)); tn_uuid_seeded = 1; }
    unsigned char bytes[16];
    for (int i = 0; i < 16; i++) bytes[i] = (unsigned char)(rand() & 0xFF);
    /* version 4 */
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    /* variant RFC 4122 */
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    char uuid[37];
    snprintf(uuid, sizeof(uuid),
      "%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x",
      bytes[0], bytes[1], bytes[2], bytes[3],
      bytes[4], bytes[5], bytes[6], bytes[7],
      bytes[8], bytes[9], bytes[10], bytes[11],
      bytes[12], bytes[13], bytes[14], bytes[15]);
    TnObj *uuid_obj = tn_new_obj(TN_OBJ_STRING);
    uuid_obj->as.text.text = strdup(uuid);
    return tn_heap_store(uuid_obj);
  }

  /* shell_quote: wraps string in single quotes, escapes inner single quotes */
  if (strcmp(key, "shell_quote") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Shell.quote expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal sarg = args[1];
    free(args);
    if (!tn_is_boxed(sarg)) {
      return tn_runtime_fail("host error: Shell.quote expects a string argument");
    }
    TnObj* sobj = tn_get_obj(sarg);
    if (sobj == NULL || sobj->kind != TN_OBJ_STRING) {
      return tn_runtime_fail("host error: Shell.quote expects a string argument");
    }
    const char* s = sobj->as.text.text;
    size_t slen = strlen(s);
    /* worst case: every char is a single quote → 4x + 2 + 1 */
    size_t cap = slen * 5 + 3;
    char* out = (char*)malloc(cap);
    size_t pos = 0;
    out[pos++] = '\'';
    for (size_t i = 0; i < slen; i++) {
      if (s[i] == '\'') {
        out[pos++] = '\'';   /* close quote */
        out[pos++] = '"';    /* open double-quote */
        out[pos++] = '\'';   /* the single quote */
        out[pos++] = '"';    /* close double-quote */
        out[pos++] = '\'';   /* reopen single-quote */
      } else {
        out[pos++] = s[i];
      }
    }
    out[pos++] = '\'';
    out[pos] = '\0';
    TnObj *sq_result_obj = tn_new_obj(TN_OBJ_STRING);
    sq_result_obj->as.text.text = strdup(out);
    free(out);
    return tn_heap_store(sq_result_obj);
  }

  /* shell_join: joins list of strings with shell quoting */
  if (strcmp(key, "shell_join") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Shell.join expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal list_val = args[1];
    free(args);
    if (!tn_is_boxed(list_val)) {
      return tn_runtime_fail("host error: Shell.join expects a list argument");
    }
    TnObj *list_obj = tn_get_obj(list_val);
    if (list_obj == NULL || list_obj->kind != TN_OBJ_LIST) {
      return tn_runtime_fail("host error: Shell.join expects a list argument");
    }
    size_t count = list_obj->as.list.len;
    TnVal* items = list_obj->as.list.items;
    if (count == 0) {
      TnObj *empty_obj = tn_new_obj(TN_OBJ_STRING);
      empty_obj->as.text.text = strdup("");
      return tn_heap_store(empty_obj);
    }
    /* Build result by quoting each element and joining with spaces */
    size_t total_cap = 256;
    char* result = (char*)malloc(total_cap);
    size_t rpos = 0;
    for (size_t i = 0; i < count; i++) {
      if (i > 0) { result[rpos++] = ' '; }
      if (!tn_is_boxed(items[i])) {
        free(result);
        return tn_runtime_failf("host error: Shell.join element %zu is not a string", i);
      }
      TnObj *elem = tn_get_obj(items[i]);
      if (elem == NULL || elem->kind != TN_OBJ_STRING) {
        free(result);
        return tn_runtime_failf("host error: Shell.join element %zu is not a string", i);
      }
      const char* es = elem->as.text.text;
      size_t eslen = strlen(es);
      size_t needed = rpos + eslen * 5 + 4;
      if (needed >= total_cap) {
        total_cap = needed * 2;
        result = (char*)realloc(result, total_cap);
      }
      result[rpos++] = '\'';
      for (size_t j = 0; j < eslen; j++) {
        size_t need2 = rpos + 6;
        if (need2 >= total_cap) { total_cap = need2 * 2; result = (char*)realloc(result, total_cap); }
        if (es[j] == '\'') {
          result[rpos++] = '\'';
          result[rpos++] = '"';
          result[rpos++] = '\'';
          result[rpos++] = '"';
          result[rpos++] = '\'';
        } else {
          result[rpos++] = es[j];
        }
      }
      result[rpos++] = '\'';
    }
    result[rpos] = '\0';
    TnObj *sj_result_obj = tn_new_obj(TN_OBJ_STRING);
    sj_result_obj->as.text.text = strdup(result);
    free(result);
    return tn_heap_store(sj_result_obj);
  }

  /* enum_random: returns a random element from a list */
  if (strcmp(key, "enum_random") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Enum.random expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal list_val = args[1];
    free(args);
    if (!tn_is_boxed(list_val)) {
      return tn_runtime_fail("host error: Enum.random expects a list argument");
    }
    TnObj *list_obj = tn_get_obj(list_val);
    if (list_obj == NULL || list_obj->kind != TN_OBJ_LIST) {
      return tn_runtime_fail("host error: Enum.random expects a list argument");
    }
    size_t count = list_obj->as.list.len;
    if (count == 0) {
      return tn_runtime_fail("host error: Enum.random called on empty list");
    }
    static int tn_erand_seeded = 0;
    if (!tn_erand_seeded) { srand((unsigned)time(NULL)); tn_erand_seeded = 1; }
    size_t idx = (size_t)rand() % count;
    return list_obj->as.list.items[idx];
  }

  /* env_set: sets an environment variable, returns :ok atom */
  if (strcmp(key, "env_set") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: Env.set expects exactly 2 arguments, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1]) || !tn_is_boxed(args[2])) {
      free(args);
      return tn_runtime_fail("host error: Env.set expects string arguments");
    }
    TnObj *key_obj = tn_get_obj(args[1]);
    TnObj *val_obj = tn_get_obj(args[2]);
    if (key_obj == NULL || key_obj->kind != TN_OBJ_STRING ||
        val_obj == NULL || val_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Env.set expects string arguments");
    }
    setenv(key_obj->as.text.text, val_obj->as.text.text, 1);
    free(args);
    return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
  }

  /* env_delete: unsets an environment variable, returns :ok atom */
  if (strcmp(key, "env_delete") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Env.delete expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Env.delete expects a string argument");
    }
    TnObj *key_obj = tn_get_obj(args[1]);
    if (key_obj == NULL || key_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Env.delete expects a string argument");
    }
    unsetenv(key_obj->as.text.text);
    free(args);
    return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
  }

  /* env_has_key: checks if an environment variable exists, returns boolean */
  if (strcmp(key, "env_has_key") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Env.has_key expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Env.has_key expects a string argument");
    }
    TnObj *key_obj = tn_get_obj(args[1]);
    if (key_obj == NULL || key_obj->kind != TN_OBJ_STRING) {
      free(args);
      return tn_runtime_fail("host error: Env.has_key expects a string argument");
    }
    char *val = getenv(key_obj->as.text.text);
    free(args);
    return tn_runtime_const_bool(val != NULL ? 1 : 0);
  }

  /* env_all: returns all environment variables as a map of string->string */
  if (strcmp(key, "env_all") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: Env.all expects exactly 0 arguments, found %zu", argc - 1);
    }
    free(args);
    extern char **environ;
    size_t count = 0;
    for (char **ep = environ; *ep != NULL; ep++) count++;
    TnObj *map_obj = tn_new_obj(TN_OBJ_MAP);
    map_obj->as.map_like.len = count;
    map_obj->as.map_like.items = count == 0 ? NULL : (TnPair *)calloc(count, sizeof(TnPair));
    if (count > 0 && map_obj->as.map_like.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    for (size_t i = 0; i < count; i++) {
      char *entry = environ[i];
      char *eq = strchr(entry, '=');
      size_t key_len = eq ? (size_t)(eq - entry) : strlen(entry);
      char *k = (char *)malloc(key_len + 1);
      if (k == NULL) { fprintf(stderr, "error: native runtime allocation failure\n"); exit(1); }
      memcpy(k, entry, key_len);
      k[key_len] = '\0';
      TnObj *k_obj = tn_new_obj(TN_OBJ_STRING);
      k_obj->as.text.text = k;
      char *v_src = eq ? eq + 1 : "";
      size_t v_len = strlen(v_src);
      char *v = (char *)malloc(v_len + 1);
      if (v == NULL) { fprintf(stderr, "error: native runtime allocation failure\n"); exit(1); }
      memcpy(v, v_src, v_len + 1);
      TnObj *v_obj = tn_new_obj(TN_OBJ_STRING);
      v_obj->as.text.text = v;
      map_obj->as.map_like.items[i].key = tn_heap_store(k_obj);
      map_obj->as.map_like.items[i].value = tn_heap_store(v_obj);
    }
    return tn_heap_store(map_obj);
  }

  /* enum_shuffle: Fisher-Yates shuffle on a list */
  if (strcmp(key, "enum_shuffle") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Enum.shuffle expects exactly 1 argument, found %zu", argc - 1);
    }
    TnVal list_val = args[1];
    free(args);
    if (!tn_is_boxed(list_val)) {
      return tn_runtime_fail("host error: Enum.shuffle expects a list argument");
    }
    TnObj *src = tn_get_obj(list_val);
    if (src == NULL || src->kind != TN_OBJ_LIST) {
      return tn_runtime_fail("host error: Enum.shuffle expects a list argument");
    }
    size_t n = src->as.list.len;
    TnObj *result = tn_new_obj(TN_OBJ_LIST);
    result->as.list.len = n;
    result->as.list.items = n == 0 ? NULL : (TnVal *)calloc(n, sizeof(TnVal));
    if (n > 0 && result->as.list.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    for (size_t i = 0; i < n; i++) {
      result->as.list.items[i] = src->as.list.items[i];
    }
    if (n > 1) {
      static int tn_shuf_seeded = 0;
      if (!tn_shuf_seeded) { srand((unsigned)time(NULL)); tn_shuf_seeded = 1; }
      for (size_t i = n - 1; i >= 1; i--) {
        size_t j = (size_t)rand() % (i + 1);
        TnVal tmp = result->as.list.items[i];
        result->as.list.items[i] = result->as.list.items[j];
        result->as.list.items[j] = tmp;
      }
    }
    return tn_heap_store(result);
  }

  /* datetime_utc_now: returns ISO 8601 UTC string like "2026-03-29T12:34:56Z" */
  if (strcmp(key, "datetime_utc_now") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: DateTime.utc_now expects exactly 0 arguments, found %zu", argc - 1);
    }
    free(args);
    time_t now = time(NULL);
    struct tm utc;
    gmtime_r(&now, &utc);
    char buf[32];
    strftime(buf, sizeof(buf), "%Y-%m-%dT%H:%M:%SZ", &utc);
    size_t slen = strlen(buf);
    char *result = (char *)malloc(slen + 1);
    if (result == NULL) { fprintf(stderr, "error: native runtime allocation failure\n"); exit(1); }
    memcpy(result, buf, slen + 1);
    TnObj *str_obj = tn_new_obj(TN_OBJ_STRING);
    str_obj->as.text.text = result;
    return tn_heap_store(str_obj);
  }

  /* Logger module — global level (debug=0, info=1 default, warn=2, error=3, none=4) */
  static int tn_logger_level = 1;

  if (strcmp(key, "logger_debug") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Logger.debug expects exactly 1 argument, found %zu", argc - 1);
    }
    const char *msg = tn_expect_host_string_arg("Logger.debug", args[1], 1);
    free(args);
    if (tn_logger_level <= 0) {
      fprintf(stderr, "[debug] %s\n", msg);
    }
    return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
  }

  if (strcmp(key, "logger_info") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Logger.info expects exactly 1 argument, found %zu", argc - 1);
    }
    const char *msg = tn_expect_host_string_arg("Logger.info", args[1], 1);
    free(args);
    if (tn_logger_level <= 1) {
      fprintf(stderr, "[info] %s\n", msg);
    }
    return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
  }

  if (strcmp(key, "logger_warn") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Logger.warn expects exactly 1 argument, found %zu", argc - 1);
    }
    const char *msg = tn_expect_host_string_arg("Logger.warn", args[1], 1);
    free(args);
    if (tn_logger_level <= 2) {
      fprintf(stderr, "[warn] %s\n", msg);
    }
    return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
  }

  if (strcmp(key, "logger_error") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Logger.error expects exactly 1 argument, found %zu", argc - 1);
    }
    const char *msg = tn_expect_host_string_arg("Logger.error", args[1], 1);
    free(args);
    if (tn_logger_level <= 3) {
      fprintf(stderr, "[error] %s\n", msg);
    }
    return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
  }

  if (strcmp(key, "logger_set_level") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Logger.set_level expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Logger.set_level expects an atom (:debug, :info, :warn, :error, :none)");
    }
    TnObj *atom_obj = tn_get_obj(args[1]);
    if (atom_obj == NULL || atom_obj->kind != TN_OBJ_ATOM) {
      free(args);
      return tn_runtime_fail("host error: Logger.set_level expects an atom (:debug, :info, :warn, :error, :none)");
    }
    const char *lvl = atom_obj->as.text.text;
    if (strcmp(lvl, "debug") == 0) { tn_logger_level = 0; }
    else if (strcmp(lvl, "info") == 0) { tn_logger_level = 1; }
    else if (strcmp(lvl, "warn") == 0) { tn_logger_level = 2; }
    else if (strcmp(lvl, "error") == 0) { tn_logger_level = 3; }
    else if (strcmp(lvl, "none") == 0) { tn_logger_level = 4; }
    else {
      free(args);
      return tn_runtime_failf("host error: Logger.set_level expects :debug, :info, :warn, :error, or :none, got :%s", lvl);
    }
    free(args);
    return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
  }

  if (strcmp(key, "logger_get_level") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: Logger.get_level expects exactly 0 arguments, found %zu", argc - 1);
    }
    free(args);
    const char *name;
    switch (tn_logger_level) {
      case 0: name = "debug"; break;
      case 1: name = "info"; break;
      case 2: name = "warn"; break;
      case 3: name = "error"; break;
      default: name = "none"; break;
    }
    return tn_runtime_const_atom((TnVal)(intptr_t)name);
  }

  /* ---- file_cp: copy a file ---- */
  if (strcmp(key, "file_cp") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: File.cp expects exactly 2 arguments, found %zu", argc - 1);
    }
    const char *src_path = tn_expect_host_string_arg("File.cp", args[1], 1);
    const char *dst_path = tn_expect_host_string_arg("File.cp", args[2], 2);
    FILE *fin = fopen(src_path, "rb");
    if (fin == NULL) {
      char err_buf[512];
      snprintf(err_buf, sizeof(err_buf), "%s", strerror(errno));
      free(args);
      TnVal err_str = tn_runtime_const_string((TnVal)(intptr_t)err_buf);
      TnVal atom = tn_runtime_const_atom((TnVal)(intptr_t)"error");
      return tn_runtime_make_tuple(atom, err_str);
    }
    FILE *fout = fopen(dst_path, "wb");
    if (fout == NULL) {
      char err_buf[512];
      snprintf(err_buf, sizeof(err_buf), "%s", strerror(errno));
      fclose(fin);
      free(args);
      TnVal err_str = tn_runtime_const_string((TnVal)(intptr_t)err_buf);
      TnVal atom = tn_runtime_const_atom((TnVal)(intptr_t)"error");
      return tn_runtime_make_tuple(atom, err_str);
    }
    char cp_buf[8192];
    size_t n;
    while ((n = fread(cp_buf, 1, sizeof(cp_buf), fin)) > 0) {
      if (fwrite(cp_buf, 1, n, fout) != n) {
        char err_buf[512];
        snprintf(err_buf, sizeof(err_buf), "%s", strerror(errno));
        fclose(fin);
        fclose(fout);
        free(args);
        TnVal err_str = tn_runtime_const_string((TnVal)(intptr_t)err_buf);
        TnVal atom = tn_runtime_const_atom((TnVal)(intptr_t)"error");
        return tn_runtime_make_tuple(atom, err_str);
      }
    }
    fclose(fin);
    fclose(fout);
    free(args);
    return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
  }

  /* ---- file_rename: rename/move a file ---- */
  if (strcmp(key, "file_rename") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: File.rename expects exactly 2 arguments, found %zu", argc - 1);
    }
    const char *src_path = tn_expect_host_string_arg("File.rename", args[1], 1);
    const char *dst_path = tn_expect_host_string_arg("File.rename", args[2], 2);
    if (rename(src_path, dst_path) == 0) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    } else {
      char err_buf[512];
      snprintf(err_buf, sizeof(err_buf), "%s", strerror(errno));
      free(args);
      TnVal err_str = tn_runtime_const_string((TnVal)(intptr_t)err_buf);
      TnVal atom = tn_runtime_const_atom((TnVal)(intptr_t)"error");
      return tn_runtime_make_tuple(atom, err_str);
    }
  }

  /* ---- file_stat: return {ok, %{size, is_dir, is_file}} or {error, msg} ---- */
  if (strcmp(key, "file_stat") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: File.stat expects exactly 1 argument, found %zu", argc - 1);
    }
    const char *path = tn_expect_host_string_arg("File.stat", args[1], 1);
    struct stat st;
    if (stat(path, &st) != 0) {
      char err_buf[512];
      snprintf(err_buf, sizeof(err_buf), "%s", strerror(errno));
      free(args);
      TnVal err_str = tn_runtime_const_string((TnVal)(intptr_t)err_buf);
      TnVal atom = tn_runtime_const_atom((TnVal)(intptr_t)"error");
      return tn_runtime_make_tuple(atom, err_str);
    }
    /* Build map with 3 entries: size, is_dir, is_file */
    TnObj *map_obj = tn_new_obj(TN_OBJ_MAP);
    map_obj->as.map_like.len = 3;
    map_obj->as.map_like.items = (TnPair *)calloc(3, sizeof(TnPair));
    if (map_obj->as.map_like.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    map_obj->as.map_like.items[0].key = tn_runtime_const_string((TnVal)(intptr_t)"size");
    map_obj->as.map_like.items[0].value = (TnVal)((int64_t)st.st_size);
    tn_runtime_retain(map_obj->as.map_like.items[0].key);
    map_obj->as.map_like.items[1].key = tn_runtime_const_string((TnVal)(intptr_t)"is_dir");
    map_obj->as.map_like.items[1].value = tn_runtime_const_bool(S_ISDIR(st.st_mode) ? 1 : 0);
    tn_runtime_retain(map_obj->as.map_like.items[1].key);
    tn_runtime_retain(map_obj->as.map_like.items[1].value);
    map_obj->as.map_like.items[2].key = tn_runtime_const_string((TnVal)(intptr_t)"is_file");
    map_obj->as.map_like.items[2].value = tn_runtime_const_bool(S_ISREG(st.st_mode) ? 1 : 0);
    tn_runtime_retain(map_obj->as.map_like.items[2].key);
    tn_runtime_retain(map_obj->as.map_like.items[2].value);
    TnVal map_val = tn_heap_store(map_obj);
    free(args);
    TnVal ok_atom = tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    return tn_runtime_make_tuple(ok_atom, map_val);
  }

  /* ---- url_encode_query: encode a map as a query string ---- */
  if (strcmp(key, "url_encode_query") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Url.encode_query expects exactly 1 argument, found %zu", argc - 1);
    }
    if (!tn_is_boxed(args[1])) {
      free(args);
      return tn_runtime_fail("host error: Url.encode_query expects a map or keyword list, found number");
    }
    TnObj *map_obj = tn_get_obj(args[1]);
    if (map_obj == NULL || (map_obj->kind != TN_OBJ_MAP && map_obj->kind != TN_OBJ_KEYWORD)) {
      free(args);
      return tn_runtime_failf("host error: Url.encode_query expects a map or keyword list, found %s", tn_runtime_value_kind(args[1]));
    }
    /* Build query string using open_memstream */
    char *qs_buf = NULL;
    size_t qs_len = 0;
    FILE *qs_stream = open_memstream(&qs_buf, &qs_len);
    if (qs_stream == NULL) {
      free(args);
      return tn_runtime_fail("host error: Url.encode_query: open_memstream failed");
    }
    static const char qhex[] = "0123456789ABCDEF";
    for (size_t i = 0; i < map_obj->as.map_like.len; i++) {
      if (i > 0) fputc('&', qs_stream);
      /* Helper: render key and value to temp strings, then query-encode them */
      TnVal parts[2] = { map_obj->as.map_like.items[i].key, map_obj->as.map_like.items[i].value };
      for (int p = 0; p < 2; p++) {
        if (p == 1) fputc('=', qs_stream);
        /* Get string representation of the value */
        char *repr = NULL;
        size_t repr_len = 0;
        FILE *repr_stream = open_memstream(&repr, &repr_len);
        if (repr_stream == NULL) { fclose(qs_stream); free(qs_buf); free(args); return tn_runtime_fail("host error: Url.encode_query: open_memstream failed"); }
        /* For strings, use raw text; for others, use render */
        TnVal pv = parts[p];
        int wrote_raw = 0;
        if (tn_is_boxed(pv)) {
          TnObj *pobj = tn_get_obj(pv);
          if (pobj != NULL && (pobj->kind == TN_OBJ_STRING || pobj->kind == TN_OBJ_ATOM)) {
            fputs(pobj->as.text.text, repr_stream);
            wrote_raw = 1;
          }
        }
        if (!wrote_raw) {
          if (!tn_is_boxed(pv)) {
            fprintf(repr_stream, "%" PRId64, (int64_t)pv);
          } else {
            TnObj *pobj = tn_get_obj(pv);
            if (pobj != NULL && pobj->kind == TN_OBJ_BOOL) {
              fputs(pobj->as.bool_value ? "true" : "false", repr_stream);
            } else if (pobj != NULL && pobj->kind == TN_OBJ_FLOAT) {
              fputs(pobj->as.text.text, repr_stream);
            } else if (pobj != NULL && pobj->kind == TN_OBJ_NIL) {
              /* empty string for nil */
            } else {
              tn_render_value(repr_stream, pv);
            }
          }
        }
        fclose(repr_stream);
        /* Query-encode the repr string */
        for (size_t ci = 0; ci < repr_len; ci++) {
          unsigned char c = (unsigned char)repr[ci];
          if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') ||
              c == '-' || c == '_' || c == '.') {
            fputc((char)c, qs_stream);
          } else if (c == ' ') {
            fputc('+', qs_stream);
          } else {
            fputc('%', qs_stream);
            fputc(qhex[(c >> 4) & 0x0f], qs_stream);
            fputc(qhex[c & 0x0f], qs_stream);
          }
        }
        free(repr);
      }
    }
    fclose(qs_stream);
    TnObj *result_obj = tn_new_obj(TN_OBJ_STRING);
    result_obj->as.text.text = qs_buf;
    free(args);
    return tn_heap_store(result_obj);
  }

  /* ---- url_decode_query: decode a query string to a map ---- */
  if (strcmp(key, "url_decode_query") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: Url.decode_query expects exactly 1 argument, found %zu", argc - 1);
    }
    const char *qs = tn_expect_host_string_arg("Url.decode_query", args[1], 1);
    size_t qs_slen = strlen(qs);
    if (qs_slen == 0) {
      /* Return empty map */
      TnObj *map_obj = tn_new_obj(TN_OBJ_MAP);
      map_obj->as.map_like.len = 0;
      map_obj->as.map_like.items = NULL;
      free(args);
      return tn_heap_store(map_obj);
    }
    /* Count pairs (number of & + 1) */
    size_t pair_count = 1;
    for (size_t i = 0; i < qs_slen; i++) { if (qs[i] == '&') pair_count++; }
    TnObj *map_obj = tn_new_obj(TN_OBJ_MAP);
    map_obj->as.map_like.items = (TnPair *)calloc(pair_count, sizeof(TnPair));
    if (map_obj->as.map_like.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    map_obj->as.map_like.len = 0;
    /* Parse pairs */
    const char *cursor = qs;
    while (*cursor) {
      /* Find end of this pair */
      const char *amp = strchr(cursor, '&');
      size_t pair_len = amp ? (size_t)(amp - cursor) : strlen(cursor);
      /* Find = within pair */
      const char *eq = NULL;
      for (size_t i = 0; i < pair_len; i++) {
        if (cursor[i] == '=') { eq = cursor + i; break; }
      }
      const char *key_start = cursor;
      size_t key_len = eq ? (size_t)(eq - cursor) : pair_len;
      const char *val_start = eq ? eq + 1 : cursor + pair_len;
      size_t val_len = eq ? pair_len - key_len - 1 : 0;
      /* Percent-decode key */
      char *dk = (char *)malloc(key_len + 1);
      size_t dki = 0;
      for (size_t i = 0; i < key_len; ) {
        if (key_start[i] == '%' && i + 2 < key_len) {
          unsigned char hi = (unsigned char)key_start[i+1], lo = (unsigned char)key_start[i+2];
          int h = -1, l = -1;
          if (hi >= '0' && hi <= '9') h = hi - '0'; else if (hi >= 'A' && hi <= 'F') h = hi - 'A' + 10; else if (hi >= 'a' && hi <= 'f') h = hi - 'a' + 10;
          if (lo >= '0' && lo <= '9') l = lo - '0'; else if (lo >= 'A' && lo <= 'F') l = lo - 'A' + 10; else if (lo >= 'a' && lo <= 'f') l = lo - 'a' + 10;
          if (h >= 0 && l >= 0) { dk[dki++] = (char)((h << 4) | l); i += 3; } else { dk[dki++] = key_start[i++]; }
        } else if (key_start[i] == '+') { dk[dki++] = ' '; i++; }
        else { dk[dki++] = key_start[i++]; }
      }
      dk[dki] = '\0';
      /* Percent-decode value */
      char *dv = (char *)malloc(val_len + 1);
      size_t dvi = 0;
      for (size_t i = 0; i < val_len; ) {
        if (val_start[i] == '%' && i + 2 < val_len) {
          unsigned char hi = (unsigned char)val_start[i+1], lo = (unsigned char)val_start[i+2];
          int h = -1, l = -1;
          if (hi >= '0' && hi <= '9') h = hi - '0'; else if (hi >= 'A' && hi <= 'F') h = hi - 'A' + 10; else if (hi >= 'a' && hi <= 'f') h = hi - 'a' + 10;
          if (lo >= '0' && lo <= '9') l = lo - '0'; else if (lo >= 'A' && lo <= 'F') l = lo - 'A' + 10; else if (lo >= 'a' && lo <= 'f') l = lo - 'a' + 10;
          if (h >= 0 && l >= 0) { dv[dvi++] = (char)((h << 4) | l); i += 3; } else { dv[dvi++] = val_start[i++]; }
        } else if (val_start[i] == '+') { dv[dvi++] = ' '; i++; }
        else { dv[dvi++] = val_start[i++]; }
      }
      dv[dvi] = '\0';
      size_t idx = map_obj->as.map_like.len;
      map_obj->as.map_like.items[idx].key = tn_runtime_const_string((TnVal)(intptr_t)dk);
      map_obj->as.map_like.items[idx].value = tn_runtime_const_string((TnVal)(intptr_t)dv);
      tn_runtime_retain(map_obj->as.map_like.items[idx].key);
      tn_runtime_retain(map_obj->as.map_like.items[idx].value);
      map_obj->as.map_like.len++;
      free(dk);
      free(dv);
      cursor += pair_len;
      if (*cursor == '&') cursor++;
    }
    free(args);
    return tn_heap_store(map_obj);
  }

  /* ---- sys_constant_time_eq: constant-time string comparison ---- */
  if (strcmp(key, "sys_constant_time_eq") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: sys_constant_time_eq expects exactly 2 arguments, found %zu", argc - 1);
    }
    const char *left = tn_expect_host_string_arg("sys_constant_time_eq", args[1], 1);
    const char *right = tn_expect_host_string_arg("sys_constant_time_eq", args[2], 2);
    size_t left_len = strlen(left);
    size_t right_len = strlen(right);
    /* Use volatile to prevent compiler from short-circuiting */
    volatile unsigned char result = (left_len != right_len) ? 1 : 0;
    size_t cmp_len = left_len < right_len ? left_len : right_len;
    for (size_t i = 0; i < cmp_len; i++) {
      result |= (unsigned char)((unsigned char)left[i] ^ (unsigned char)right[i]);
    }
    free(args);
    return tn_runtime_const_bool(result == 0 ? 1 : 0);
  }

  /* ---- assert: check truthiness ---- */
  if (strcmp(key, "assert") == 0) {
    if (argc < 2 || argc > 3) {
      return tn_runtime_failf("host error: Assert.assert expects 1-2 arguments, found %zu", argc - 1);
    }
    TnVal value = args[1];
    int is_falsy = 0;
    TnObj *val_obj = tn_get_obj(value);
    if (val_obj != NULL && val_obj->kind == TN_OBJ_NIL) is_falsy = 1;
    if (val_obj != NULL && val_obj->kind == TN_OBJ_BOOL && val_obj->as.bool_value == 0) is_falsy = 1;
    if (!is_falsy) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    }
    const char *msg = (argc == 3) ? tn_expect_host_string_arg("Assert.assert", args[2], 2) : "assertion failed: expected truthy value";
    TnVal assert_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assertion_failed");
    TnVal type_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assert");
    TnVal msg_val = tn_runtime_const_string((TnVal)(intptr_t)msg);
    TnVal inner = tn_runtime_make_tuple(type_atom, msg_val);
    TnVal detail = tn_runtime_make_tuple(assert_atom, inner);
    free(args);
    return tn_runtime_make_err(detail);
  }

  /* ---- refute: check falsiness ---- */
  if (strcmp(key, "refute") == 0) {
    if (argc < 2 || argc > 3) {
      return tn_runtime_failf("host error: Assert.refute expects 1-2 arguments, found %zu", argc - 1);
    }
    TnVal value = args[1];
    int is_falsy = 0;
    TnObj *val_obj = tn_get_obj(value);
    if (val_obj != NULL && val_obj->kind == TN_OBJ_NIL) is_falsy = 1;
    if (val_obj != NULL && val_obj->kind == TN_OBJ_BOOL && val_obj->as.bool_value == 0) is_falsy = 1;
    if (is_falsy) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    }
    const char *msg = (argc == 3) ? tn_expect_host_string_arg("Assert.refute", args[2], 2) : "refute failed: expected falsy value";
    TnVal assert_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assertion_failed");
    TnVal type_atom = tn_runtime_const_atom((TnVal)(intptr_t)"refute");
    TnVal msg_val = tn_runtime_const_string((TnVal)(intptr_t)msg);
    TnVal inner = tn_runtime_make_tuple(type_atom, msg_val);
    TnVal detail = tn_runtime_make_tuple(assert_atom, inner);
    free(args);
    return tn_runtime_make_err(detail);
  }

  /* ---- assert_equal: deep equality check ---- */
  if (strcmp(key, "assert_equal") == 0) {
    if (argc < 3 || argc > 4) {
      return tn_runtime_failf("host error: Assert.assert_equal expects 2-3 arguments, found %zu", argc - 1);
    }
    TnVal left = args[1];
    TnVal right = args[2];
    if (tn_runtime_value_equal(left, right)) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    }
    const char *msg = (argc == 4) ? tn_expect_host_string_arg("Assert.assert_equal", args[3], 3) : "values are not equal";
    TnVal af_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assertion_failed");
    TnVal detail = tn_runtime_make_list_varargs((TnVal)4,
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"type"), tn_runtime_const_atom((TnVal)(intptr_t)"assert_equal")),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"left"), left),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"right"), right),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"message"), tn_runtime_const_string((TnVal)(intptr_t)msg))
    );
    TnVal result = tn_runtime_make_tuple(af_atom, detail);
    free(args);
    return tn_runtime_make_err(result);
  }

  /* ---- assert_not_equal: deep inequality check ---- */
  if (strcmp(key, "assert_not_equal") == 0) {
    if (argc < 3 || argc > 4) {
      return tn_runtime_failf("host error: Assert.assert_not_equal expects 2-3 arguments, found %zu", argc - 1);
    }
    TnVal left = args[1];
    TnVal right = args[2];
    if (!tn_runtime_value_equal(left, right)) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    }
    const char *msg = (argc == 4) ? tn_expect_host_string_arg("Assert.assert_not_equal", args[3], 3) : "values should not be equal";
    TnVal af_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assertion_failed");
    TnVal detail = tn_runtime_make_list_varargs((TnVal)4,
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"type"), tn_runtime_const_atom((TnVal)(intptr_t)"assert_not_equal")),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"left"), left),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"right"), right),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"message"), tn_runtime_const_string((TnVal)(intptr_t)msg))
    );
    TnVal result = tn_runtime_make_tuple(af_atom, detail);
    free(args);
    return tn_runtime_make_err(result);
  }

  /* ---- assert_contains: string substring or list membership ---- */
  if (strcmp(key, "assert_contains") == 0) {
    if (argc < 3 || argc > 4) {
      return tn_runtime_failf("host error: Assert.assert_contains expects 2-3 arguments, found %zu", argc - 1);
    }
    TnVal container = args[1];
    TnVal element = args[2];
    TnObj *cont_obj = tn_get_obj(container);
    int found = 0;
    if (cont_obj != NULL && cont_obj->kind == TN_OBJ_STRING) {
      TnObj *elem_obj = tn_get_obj(element);
      if (elem_obj != NULL && elem_obj->kind == TN_OBJ_STRING) {
        found = (strstr(cont_obj->as.text.text, elem_obj->as.text.text) != NULL) ? 1 : 0;
      }
    } else if (cont_obj != NULL && cont_obj->kind == TN_OBJ_LIST) {
      for (size_t i = 0; i < cont_obj->as.list.len; i++) {
        if (tn_runtime_value_equal(cont_obj->as.list.items[i], element)) {
          found = 1;
          break;
        }
      }
    }
    if (found) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    }
    const char *msg = (argc == 4) ? tn_expect_host_string_arg("Assert.assert_contains", args[3], 3) : "element not found in container";
    TnVal af_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assertion_failed");
    TnVal detail = tn_runtime_make_list_varargs((TnVal)4,
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"type"), tn_runtime_const_atom((TnVal)(intptr_t)"assert_contains")),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"container"), container),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"element"), element),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"message"), tn_runtime_const_string((TnVal)(intptr_t)msg))
    );
    TnVal result = tn_runtime_make_tuple(af_atom, detail);
    free(args);
    return tn_runtime_make_err(result);
  }

  /* ---- assert_in_delta: numeric |left - right| <= delta ---- */
  if (strcmp(key, "assert_in_delta") == 0) {
    if (argc < 4 || argc > 5) {
      return tn_runtime_failf("host error: Assert.assert_in_delta expects 3-4 arguments, found %zu", argc - 1);
    }
    /* Extract numeric values as doubles */
    double left_f = 0.0, right_f = 0.0, delta_f = 0.0;
    for (int ai = 0; ai < 3; ai++) {
      TnObj *num_obj = tn_get_obj(args[1 + ai]);
      double *target = (ai == 0) ? &left_f : (ai == 1) ? &right_f : &delta_f;
      if (num_obj == NULL) {
        /* Non-boxed value = integer stored as raw TnVal */
        *target = (double)(intptr_t)args[1 + ai];
      } else if (num_obj->kind == TN_OBJ_FLOAT) {
        *target = strtod(num_obj->as.text.text, NULL);
      } else {
        return tn_runtime_failf("host error: Assert.assert_in_delta arg %d must be a number", ai + 1);
      }
    }
    double diff = left_f - right_f;
    if (diff < 0) diff = -diff;
    if (diff <= delta_f) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    }
    const char *msg = (argc == 5) ? tn_expect_host_string_arg("Assert.assert_in_delta", args[4], 4) : "values are not within delta";
    TnVal af_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assertion_failed");
    TnVal detail = tn_runtime_make_list_varargs((TnVal)5,
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"type"), tn_runtime_const_atom((TnVal)(intptr_t)"assert_in_delta")),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"left"), args[1]),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"right"), args[2]),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"delta"), args[3]),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"message"), tn_runtime_const_string((TnVal)(intptr_t)msg))
    );
    TnVal result = tn_runtime_make_tuple(af_atom, detail);
    free(args);
    return tn_runtime_make_err(result);
  }

  /* ---- skip: return {:err, {:test_skipped, reason}} ---- */
  if (strcmp(key, "skip") == 0) {
    if (argc > 2) {
      return tn_runtime_failf("host error: Assert.skip expects 0-1 arguments, found %zu", argc - 1);
    }
    const char *reason = (argc == 2) ? tn_expect_host_string_arg("Assert.skip", args[1], 1) : "";
    TnVal skip_atom = tn_runtime_const_atom((TnVal)(intptr_t)"test_skipped");
    TnVal reason_val = tn_runtime_const_string((TnVal)(intptr_t)reason);
    TnVal detail = tn_runtime_make_tuple(skip_atom, reason_val);
    free(args);
    return tn_runtime_make_err(detail);
  }

  /* ---- assert_match: map subset matching or equality fallback ---- */
  if (strcmp(key, "assert_match") == 0) {
    if (argc < 3 || argc > 4) {
      return tn_runtime_failf("host error: Assert.assert_match expects 2-3 arguments, found %zu", argc - 1);
    }
    TnVal expected = args[1];
    TnVal actual = args[2];
    TnObj *exp_obj = tn_get_obj(expected);
    TnObj *act_obj = tn_get_obj(actual);
    if (exp_obj != NULL && exp_obj->kind == TN_OBJ_MAP && act_obj != NULL && act_obj->kind == TN_OBJ_MAP) {
      /* Map subset matching: every key in expected must exist in actual with equal value */
      int all_match = 1;
      for (size_t ei = 0; ei < exp_obj->as.map_like.len; ei++) {
        TnVal ek = exp_obj->as.map_like.items[ei].key;
        TnVal ev = exp_obj->as.map_like.items[ei].value;
        int found = 0;
        for (size_t ai = 0; ai < act_obj->as.map_like.len; ai++) {
          if (tn_runtime_value_equal(act_obj->as.map_like.items[ai].key, ek)) {
            if (tn_runtime_value_equal(act_obj->as.map_like.items[ai].value, ev)) {
              found = 1;
            }
            break;
          }
        }
        if (!found) { all_match = 0; break; }
      }
      if (all_match) {
        free(args);
        return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
      }
    } else {
      /* Non-map fallback: equality */
      if (tn_runtime_value_equal(expected, actual)) {
        free(args);
        return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
      }
    }
    const char *msg = (argc == 4) ? tn_expect_host_string_arg("Assert.assert_match", args[3], 3) : "values do not match";
    TnVal af_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assertion_failed");
    TnVal detail = tn_runtime_make_list_varargs((TnVal)4,
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"type"), tn_runtime_const_atom((TnVal)(intptr_t)"assert_match")),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"expected"), expected),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"actual"), actual),
      tn_runtime_make_tuple(tn_runtime_const_atom((TnVal)(intptr_t)"message"), tn_runtime_const_string((TnVal)(intptr_t)msg))
    );
    TnVal result = tn_runtime_make_tuple(af_atom, detail);
    free(args);
    return tn_runtime_make_err(result);
  }

  /* ---- assert_raises_check: string contains check ---- */
  if (strcmp(key, "assert_raises_check") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: assert_raises_check expects 2 arguments, found %zu", argc - 1);
    }
    const char *raised = tn_expect_host_string_arg("assert_raises_check", args[1], 1);
    const char *expected = tn_expect_host_string_arg("assert_raises_check", args[2], 2);
    if (strstr(raised, expected) != NULL) {
      free(args);
      return tn_runtime_const_atom((TnVal)(intptr_t)"ok");
    }
    /* Build error message */
    size_t msg_len = strlen("expected raise matching \"") + strlen(expected) + strlen("\", got: ") + strlen(raised) + 1;
    char *msg_buf = (char *)malloc(msg_len);
    snprintf(msg_buf, msg_len, "expected raise matching \"%s\", got: %s", expected, raised);
    TnVal af_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assertion_failed");
    TnVal type_atom = tn_runtime_const_atom((TnVal)(intptr_t)"assert_raises");
    TnVal msg_val = tn_runtime_const_string((TnVal)(intptr_t)msg_buf);
    free(msg_buf);
    TnVal inner = tn_runtime_make_tuple(type_atom, msg_val);
    TnVal detail = tn_runtime_make_tuple(af_atom, inner);
    free(args);
    return tn_runtime_make_err(detail);
  }

"###,
    );
}
