pub(super) fn emit_stubs_map(out: &mut String) {
    out.push_str(
        r###"static long tn_map_like_find_index(const TnObj *map_like, TnVal key) {
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

static TnObj *tn_expect_host_map_arg(const char *function, TnVal value, size_t index) {
  TnObj *obj = tn_get_obj(value);
  if (obj == NULL || obj->kind != TN_OBJ_MAP) {
    tn_runtime_failf(
        "host error: %s expects map argument %zu; found %s",
        function,
        index,
        tn_runtime_value_kind(value));
  }

  return obj;
}

static TnObj *tn_expect_host_list_arg(const char *function, TnVal value, size_t index) {
  TnObj *obj = tn_get_obj(value);
  if (obj == NULL || obj->kind != TN_OBJ_LIST) {
    tn_runtime_failf(
        "host error: %s expects list argument %zu; found %s",
        function,
        index,
        tn_runtime_value_kind(value));
  }

  return obj;
}

static const char *tn_expect_host_string_arg(const char *function, TnVal value, size_t index) {
  TnObj *obj = tn_get_obj(value);
  if (obj == NULL || obj->kind != TN_OBJ_STRING) {
    tn_runtime_failf(
        "host error: %s expects string argument %zu; found %s",
        function,
        index,
        tn_runtime_value_kind(value));
  }

  return obj->as.text.text;
}

static int tn_host_list_contains(const TnObj *list, TnVal value) {
  for (size_t i = 0; i < list->as.list.len; i += 1) {
    if (tn_runtime_value_equal(list->as.list.items[i], value)) {
      return 1;
    }
  }

  return 0;
}

static TnVal tn_host_map_keys(TnVal map_value) {
  TnObj *map = tn_expect_host_map_arg("Map.keys", map_value, 1);
  TnObj *list = tn_new_obj(TN_OBJ_LIST);
  list->as.list.len = map->as.map_like.len;
  list->as.list.items =
      map->as.map_like.len == 0 ? NULL : (TnVal *)calloc(map->as.map_like.len, sizeof(TnVal));
  if (map->as.map_like.len > 0 && list->as.list.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  for (size_t i = 0; i < map->as.map_like.len; i += 1) {
    list->as.list.items[i] = map->as.map_like.items[i].key;
    tn_runtime_retain(list->as.list.items[i]);
  }

  return tn_heap_store(list);
}

static TnVal tn_host_map_values(TnVal map_value) {
  TnObj *map = tn_expect_host_map_arg("Map.values", map_value, 1);
  TnObj *list = tn_new_obj(TN_OBJ_LIST);
  list->as.list.len = map->as.map_like.len;
  list->as.list.items =
      map->as.map_like.len == 0 ? NULL : (TnVal *)calloc(map->as.map_like.len, sizeof(TnVal));
  if (map->as.map_like.len > 0 && list->as.list.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  for (size_t i = 0; i < map->as.map_like.len; i += 1) {
    list->as.list.items[i] = map->as.map_like.items[i].value;
    tn_runtime_retain(list->as.list.items[i]);
  }

  return tn_heap_store(list);
}

static TnVal tn_host_map_merge(TnVal left_value, TnVal right_value) {
  TnObj *left = tn_expect_host_map_arg("Map.merge", left_value, 1);
  TnObj *right = tn_expect_host_map_arg("Map.merge", right_value, 2);
  TnVal cloned = tn_clone_map_like_with_capacity(left, TN_OBJ_MAP, right->as.map_like.len);
  TnObj *result = tn_get_obj(cloned);

  for (size_t i = 0; i < right->as.map_like.len; i += 1) {
    TnVal key = right->as.map_like.items[i].key;
    TnVal value = right->as.map_like.items[i].value;
    long existing_index = tn_map_like_find_index(result, key);

    if (existing_index >= 0) {
      tn_runtime_retain(value);
      tn_runtime_release(result->as.map_like.items[existing_index].value);
      result->as.map_like.items[existing_index].value = value;
    } else {
      size_t write_index = result->as.map_like.len;
      result->as.map_like.items[write_index].key = key;
      result->as.map_like.items[write_index].value = value;
      tn_runtime_retain(key);
      tn_runtime_retain(value);
      result->as.map_like.len += 1;
    }
  }

  return cloned;
}

static TnVal tn_host_map_filter_keys(TnVal map_value, TnVal keys_value, int keep_matches) {
  TnObj *map = tn_expect_host_map_arg(
      keep_matches ? "Map.take" : "Map.drop", map_value, 1);
  TnObj *keys = tn_expect_host_list_arg(
      keep_matches ? "Map.take" : "Map.drop", keys_value, 2);
  TnObj *result = tn_new_obj(TN_OBJ_MAP);
  result->as.map_like.len = 0;
  result->as.map_like.items =
      map->as.map_like.len == 0 ? NULL : (TnPair *)calloc(map->as.map_like.len, sizeof(TnPair));
  if (map->as.map_like.len > 0 && result->as.map_like.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  for (size_t i = 0; i < map->as.map_like.len; i += 1) {
    TnVal key = map->as.map_like.items[i].key;
    int matches = tn_host_list_contains(keys, key);
    if ((keep_matches && !matches) || (!keep_matches && matches)) {
      continue;
    }

    result->as.map_like.items[result->as.map_like.len].key = key;
    result->as.map_like.items[result->as.map_like.len].value =
        map->as.map_like.items[i].value;
    tn_runtime_retain(result->as.map_like.items[result->as.map_like.len].key);
    tn_runtime_retain(result->as.map_like.items[result->as.map_like.len].value);
    result->as.map_like.len += 1;
  }

  return tn_heap_store(result);
}

static TnVal tn_host_map_get(TnVal map_value, TnVal key, TnVal default_value) {
  TnObj *map = tn_expect_host_map_arg("Map.get", map_value, 1);
  long existing_index = tn_map_like_find_index(map, key);
  if (existing_index >= 0) {
    TnVal value = map->as.map_like.items[existing_index].value;
    tn_runtime_retain(value);
    return value;
  }

  tn_runtime_retain(default_value);
  return default_value;
}

static TnVal tn_host_map_has_key(TnVal map_value, TnVal key) {
  TnObj *map = tn_expect_host_map_arg("Map.has_key?", map_value, 1);
  long existing_index = tn_map_like_find_index(map, key);
  return tn_runtime_const_bool((TnVal)(existing_index >= 0));
}

static TnVal tn_host_map_delete(TnVal map_value, TnVal key) {
  TnObj *map = tn_expect_host_map_arg("Map.delete", map_value, 1);
  TnObj *result = tn_new_obj(TN_OBJ_MAP);
  result->as.map_like.len = 0;
  result->as.map_like.items =
      map->as.map_like.len == 0 ? NULL : (TnPair *)calloc(map->as.map_like.len, sizeof(TnPair));
  if (map->as.map_like.len > 0 && result->as.map_like.items == NULL) {
    fprintf(stderr, "error: native runtime allocation failure\n");
    exit(1);
  }

  for (size_t i = 0; i < map->as.map_like.len; i += 1) {
    if (tn_runtime_value_equal(map->as.map_like.items[i].key, key)) {
      continue;
    }

    result->as.map_like.items[result->as.map_like.len] = map->as.map_like.items[i];
    tn_runtime_retain(result->as.map_like.items[result->as.map_like.len].key);
    tn_runtime_retain(result->as.map_like.items[result->as.map_like.len].value);
    result->as.map_like.len += 1;
  }

  return tn_heap_store(result);
}

"###,
    );
}
