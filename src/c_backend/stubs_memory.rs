pub(super) fn emit_stubs_memory(out: &mut String) {
    out.push_str(
        r###"
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

static void tn_runtime_gc_free_payload(TnObj *obj) {
  switch (obj->kind) {
    case TN_OBJ_ATOM:
    case TN_OBJ_STRING:
    case TN_OBJ_FLOAT:
      free(obj->as.text.text);
      return;
    case TN_OBJ_LIST:
    case TN_OBJ_BINARY:
      free(obj->as.list.items);
      return;
    case TN_OBJ_MAP:
    case TN_OBJ_KEYWORD:
      free(obj->as.map_like.items);
      return;
    case TN_OBJ_BOOL:
    case TN_OBJ_NIL:
    case TN_OBJ_TUPLE:
    case TN_OBJ_RANGE:
    case TN_OBJ_RESULT:
    case TN_OBJ_CLOSURE:
      return;
  }
}

static void tn_runtime_gc_mark_value(TnVal value) {
  TnObj *obj = tn_get_obj(value);
  if (obj == NULL || (obj->gc_flags & TN_GC_FLAG_MARK) != 0) {
    return;
  }

  obj->gc_flags |= TN_GC_FLAG_MARK;

  switch (obj->kind) {
    case TN_OBJ_TUPLE:
      tn_runtime_gc_mark_value(obj->as.tuple.left);
      tn_runtime_gc_mark_value(obj->as.tuple.right);
      return;
    case TN_OBJ_LIST:
    case TN_OBJ_BINARY:
      for (size_t i = 0; i < obj->as.list.len; i += 1) {
        tn_runtime_gc_mark_value(obj->as.list.items[i]);
      }
      return;
    case TN_OBJ_MAP:
    case TN_OBJ_KEYWORD:
      for (size_t i = 0; i < obj->as.map_like.len; i += 1) {
        tn_runtime_gc_mark_value(obj->as.map_like.items[i].key);
        tn_runtime_gc_mark_value(obj->as.map_like.items[i].value);
      }
      return;
    case TN_OBJ_RANGE:
      tn_runtime_gc_mark_value(obj->as.range.start);
      tn_runtime_gc_mark_value(obj->as.range.end);
      return;
    case TN_OBJ_RESULT:
      tn_runtime_gc_mark_value(obj->as.result.value);
      return;
    case TN_OBJ_BOOL:
    case TN_OBJ_NIL:
    case TN_OBJ_ATOM:
    case TN_OBJ_STRING:
    case TN_OBJ_FLOAT:
    case TN_OBJ_CLOSURE:
      return;
  }
}

static void tn_runtime_gc_collect(void) {
  if (!tn_runtime_memory_trace_enabled()) {
    return;
  }

  if (tn_true_value != 0) {
    tn_runtime_gc_mark_value(tn_true_value);
  }
  if (tn_false_value != 0) {
    tn_runtime_gc_mark_value(tn_false_value);
  }
  if (tn_nil_value != 0) {
    tn_runtime_gc_mark_value(tn_nil_value);
  }
  for (size_t i = 0; i < tn_root_stack_len; i += 1) {
    tn_runtime_gc_mark_value(tn_root_stack[i]);
  }

  for (size_t i = 0; i < tn_heap_len; i += 1) {
    TnObj *obj = tn_heap[i];
    if (obj == NULL) {
      continue;
    }

    if ((obj->gc_flags & TN_GC_FLAG_MARK) != 0) {
      obj->gc_flags &= ~TN_GC_FLAG_MARK;
      continue;
    }

    tn_heap[i] = NULL;
    tn_heap_push_free_id(i + 1);
    if (tn_memory_heap_live_slots > 0) {
      tn_memory_heap_live_slots -= 1;
    }
    tn_memory_reclaims_total += 1;

    tn_runtime_gc_free_payload(obj);
    free(obj);
  }

  tn_memory_gc_collections_total += 1;
}

static TnObj *tn_get_obj_for_rc(TnVal value, const char *action, size_t *id_out) {
  if (!tn_is_boxed(value)) {
    return NULL;
  }

  size_t id = tn_box_id(value);
  if (id == 0 || id > tn_heap_len) {
    fprintf(stderr,
            "error: native runtime ownership misuse during %s: unknown object id %zu\n",
            action,
            id);
    exit(1);
  }

  TnObj *obj = tn_heap[id - 1];
  if (obj == NULL) {
    fprintf(stderr,
            "error: native runtime ownership misuse during %s: object id %zu already reclaimed\n",
            action,
            id);
    exit(1);
  }

  if (id_out != NULL) {
    *id_out = id;
  }
  return obj;
}

static void tn_runtime_retain(TnVal value) {
  if (!tn_runtime_memory_rc_enabled() || !tn_is_boxed(value)) {
    return;
  }

  TnObj *obj = tn_get_obj_for_rc(value, "retain", NULL);
  if (obj->refcount < UINT32_MAX) {
    obj->refcount += 1;
  }
}

static void tn_runtime_release(TnVal value) {
  if (!tn_runtime_memory_rc_enabled() || !tn_is_boxed(value)) {
    return;
  }

  size_t id = 0;
  TnObj *obj = tn_get_obj_for_rc(value, "release", &id);
  if (obj->refcount == 0) {
    fprintf(stderr,
            "error: native runtime ownership misuse during release: object id %zu already released\n",
            id);
    exit(1);
  }

  obj->refcount -= 1;
  if (obj->refcount > 0) {
    return;
  }

  tn_heap[id - 1] = NULL;
  tn_heap_push_free_id(id);
  if (tn_memory_heap_live_slots > 0) {
    tn_memory_heap_live_slots -= 1;
  }
  tn_memory_reclaims_total += 1;

  TnVal self_value = tn_make_box(id);

  switch (obj->kind) {
    case TN_OBJ_ATOM:
    case TN_OBJ_STRING:
    case TN_OBJ_FLOAT:
      free(obj->as.text.text);
      break;
    case TN_OBJ_TUPLE:
      if (obj->as.tuple.left != self_value) {
        tn_runtime_release(obj->as.tuple.left);
      }
      if (obj->as.tuple.right != self_value) {
        tn_runtime_release(obj->as.tuple.right);
      }
      break;
    case TN_OBJ_LIST:
    case TN_OBJ_BINARY:
      for (size_t i = 0; i < obj->as.list.len; i += 1) {
        if (obj->as.list.items[i] != self_value) {
          tn_runtime_release(obj->as.list.items[i]);
        }
      }
      free(obj->as.list.items);
      break;
    case TN_OBJ_MAP:
    case TN_OBJ_KEYWORD:
      for (size_t i = 0; i < obj->as.map_like.len; i += 1) {
        if (obj->as.map_like.items[i].key != self_value) {
          tn_runtime_release(obj->as.map_like.items[i].key);
        }
        if (obj->as.map_like.items[i].value != self_value) {
          tn_runtime_release(obj->as.map_like.items[i].value);
        }
      }
      free(obj->as.map_like.items);
      break;
    case TN_OBJ_RANGE:
      if (obj->as.range.start != self_value) {
        tn_runtime_release(obj->as.range.start);
      }
      if (obj->as.range.end != self_value) {
        tn_runtime_release(obj->as.range.end);
      }
      break;
    case TN_OBJ_RESULT:
      if (obj->as.result.value != self_value) {
        tn_runtime_release(obj->as.result.value);
      }
      break;
    case TN_OBJ_BOOL:
    case TN_OBJ_NIL:
    case TN_OBJ_CLOSURE:
      break;
  }

  free(obj);
}

"###,
    );
}
