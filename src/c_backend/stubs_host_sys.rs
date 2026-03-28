pub(super) fn emit_stubs_host_sys(out: &mut String) {
    out.push_str(
        r###"  if (strcmp(key, "sys_path_exists") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_path_exists expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_path_exists expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    int exists = access(path_obj->as.text.text, F_OK) == 0;
    free(args);
    return tn_runtime_const_bool((TnVal)(exists != 0));
  }

  if (strcmp(key, "sys_list_dir") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_list_dir expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_list_dir expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *dir_path = path_obj->as.text.text;
    if (dir_path[0] == '\0') {
      return tn_runtime_fail("host error: sys_list_dir path must not be empty");
    }
    DIR *dir = opendir(dir_path);
    if (dir == NULL) {
      return tn_runtime_failf("host error: sys_list_dir failed for '%s': %s", dir_path, strerror(errno));
    }
    size_t names_cap = 16;
    size_t names_len = 0;
    char **names = (char **)calloc(names_cap, sizeof(char *));
    if (names == NULL) {
      closedir(dir);
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
      if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
        continue;
      }
      if (names_len >= names_cap) {
        names_cap *= 2;
        char **next = (char **)realloc(names, names_cap * sizeof(char *));
        if (next == NULL) {
          for (size_t i = 0; i < names_len; i++) free(names[i]);
          free(names);
          closedir(dir);
          fprintf(stderr, "error: native runtime allocation failure\n");
          exit(1);
        }
        names = next;
      }
      names[names_len] = strdup(entry->d_name);
      if (names[names_len] == NULL) {
        for (size_t i = 0; i < names_len; i++) free(names[i]);
        free(names);
        closedir(dir);
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }
      names_len += 1;
    }
    closedir(dir);
    /* sort names lexicographically */
    for (size_t i = 1; i < names_len; i++) {
      for (size_t j = i; j > 0 && strcmp(names[j - 1], names[j]) > 0; j--) {
        char *tmp = names[j - 1]; names[j - 1] = names[j]; names[j] = tmp;
      }
    }
    TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
    list_obj->as.list.len = names_len;
    list_obj->as.list.items = names_len == 0 ? NULL : (TnVal *)calloc(names_len, sizeof(TnVal));
    if (names_len > 0 && list_obj->as.list.items == NULL) {
      for (size_t i = 0; i < names_len; i++) free(names[i]);
      free(names);
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    for (size_t i = 0; i < names_len; i++) {
      list_obj->as.list.items[i] = tn_runtime_const_string((TnVal)(intptr_t)names[i]);
      tn_runtime_retain(list_obj->as.list.items[i]);
      free(names[i]);
    }
    free(names);
    free(args);
    return tn_heap_store(list_obj);
  }

  if (strcmp(key, "sys_is_dir") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_is_dir expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_is_dir expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    struct stat st;
    int is_directory = (stat(path_obj->as.text.text, &st) == 0 && S_ISDIR(st.st_mode)) ? 1 : 0;
    free(args);
    return tn_runtime_const_bool((TnVal)(is_directory != 0));
  }

  if (strcmp(key, "sys_list_files_recursive") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_list_files_recursive expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_list_files_recursive expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *lfr_path = path_obj->as.text.text;
    if (lfr_path[0] == '\0') {
      return tn_runtime_fail("host error: sys_list_files_recursive path must not be empty");
    }
    if (strlen(lfr_path) >= PATH_MAX) {
      return tn_runtime_fail("host error: sys_list_files_recursive path is too long");
    }

    char error_message[512] = {0};
    TnPathStringList files = {0};
    if (!tn_collect_relative_files_recursive(lfr_path, "", &files, error_message, sizeof(error_message))) {
      tn_path_string_list_free(&files);
      if (error_message[0] == '\0') {
        return tn_runtime_failf("host error: sys_list_files_recursive failed for '%s'", lfr_path);
      }
      return tn_runtime_fail(error_message);
    }

    TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
    list_obj->as.list.len = files.len;
    list_obj->as.list.items = files.len == 0 ? NULL : (TnVal *)calloc(files.len, sizeof(TnVal));
    if (files.len > 0 && list_obj->as.list.items == NULL) {
      tn_path_string_list_free(&files);
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    for (size_t i = 0; i < files.len; i += 1) {
      list_obj->as.list.items[i] = tn_runtime_const_string((TnVal)(intptr_t)files.items[i]);
      tn_runtime_retain(list_obj->as.list.items[i]);
    }
    tn_path_string_list_free(&files);
    free(args);
    return tn_heap_store(list_obj);
  }

  if (strcmp(key, "sys_ensure_dir") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_ensure_dir expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_ensure_dir expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    if (path[0] == '\0') {
      return tn_runtime_fail("host error: sys_ensure_dir path must not be empty");
    }

    size_t path_len = strlen(path);
    if (path_len >= PATH_MAX) {
      return tn_runtime_fail("host error: sys_ensure_dir path is too long");
    }

    char mkdir_path[PATH_MAX];
    memcpy(mkdir_path, path, path_len + 1);

    for (size_t i = 1; i < path_len; i += 1) {
      if (mkdir_path[i] != '/') {
        continue;
      }

      mkdir_path[i] = '\0';
      if (mkdir_path[0] != '\0' && mkdir(mkdir_path, 0777) != 0 && errno != EEXIST) {
        return tn_runtime_failf("host error: sys_ensure_dir failed for '%s'", path);
      }
      mkdir_path[i] = '/';
    }

    if (mkdir(mkdir_path, 0777) != 0 && errno != EEXIST) {
      return tn_runtime_failf("host error: sys_ensure_dir failed for '%s'", path);
    }

    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  if (strcmp(key, "sys_remove_tree") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_remove_tree expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_remove_tree expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    if (path[0] == '\0') {
      return tn_runtime_fail("host error: sys_remove_tree path must not be empty");
    }
    if (strlen(path) >= PATH_MAX) {
      return tn_runtime_fail("host error: sys_remove_tree path is too long");
    }

    char error_message[512] = {0};
    int remove_result = tn_remove_path_recursive(path, error_message, sizeof(error_message));
    if (remove_result == 0) {
      if (error_message[0] == '\0') {
        return tn_runtime_failf("host error: sys_remove_tree failed for '%s'", path);
      }
      return tn_runtime_fail(error_message);
    }

    free(args);
    return tn_runtime_const_bool((TnVal)(remove_result == 1));
  }

  if (strcmp(key, "sys_write_text") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: sys_write_text expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    TnObj *content_obj = tn_get_obj(args[2]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_write_text expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (content_obj == NULL || content_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_write_text expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    FILE *handle = fopen(path_obj->as.text.text, "w");
    if (handle == NULL) {
      return tn_runtime_failf("host error: sys_write_text failed for '%s'", path_obj->as.text.text);
    }
    if (fputs(content_obj->as.text.text, handle) < 0 || fclose(handle) != 0) {
      return tn_runtime_failf("host error: sys_write_text failed for '%s'", path_obj->as.text.text);
    }
    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  if (strcmp(key, "sys_append_text") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: sys_append_text expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    TnObj *content_obj = tn_get_obj(args[2]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_append_text expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (content_obj == NULL || content_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_append_text expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    const char *path = path_obj->as.text.text;
    FILE *handle = fopen(path, "a");
    if (handle == NULL) {
      return tn_runtime_failf("host error: sys_append_text failed for '%s': %s", path, strerror(errno));
    }
    if (fputs(content_obj->as.text.text, handle) < 0) {
      int io_errno = errno != 0 ? errno : EIO;
      fclose(handle);
      return tn_runtime_failf("host error: sys_append_text failed for '%s': %s", path, strerror(io_errno));
    }
    if (fflush(handle) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      fclose(handle);
      return tn_runtime_failf("host error: sys_append_text failed for '%s': %s", path, strerror(io_errno));
    }
    if (fclose(handle) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      return tn_runtime_failf("host error: sys_append_text failed for '%s': %s", path, strerror(io_errno));
    }
    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  if (strcmp(key, "sys_write_text_atomic") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: sys_write_text_atomic expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    TnObj *content_obj = tn_get_obj(args[2]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_write_text_atomic expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (content_obj == NULL || content_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_write_text_atomic expects string argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    const char *path = path_obj->as.text.text;
    const char *content = content_obj->as.text.text;
    const char *last_slash = strrchr(path, '/');
    if (last_slash != NULL) {
      size_t parent_len = (size_t)(last_slash - path);
      if (parent_len > 0) {
        char *parent = (char *)malloc(parent_len + 1);
        if (parent == NULL) {
          fprintf(stderr, "error: native runtime allocation failure\n");
          exit(1);
        }
        memcpy(parent, path, parent_len);
        parent[parent_len] = '\0';
        for (size_t i = 1; i < parent_len; i += 1) {
          if (parent[i] != '/') {
            continue;
          }
          parent[i] = '\0';
          if (parent[0] != '\0' && mkdir(parent, 0777) != 0 && errno != EEXIST) {
            int mkdir_errno = errno;
            free(parent);
            return tn_runtime_failf("host error: sys_write_text_atomic failed for '%s': %s", path, strerror(mkdir_errno));
          }
          parent[i] = '/';
        }
        if (mkdir(parent, 0777) != 0 && errno != EEXIST) {
          int mkdir_errno = errno;
          free(parent);
          return tn_runtime_failf("host error: sys_write_text_atomic failed for '%s': %s", path, strerror(mkdir_errno));
        }
        free(parent);
      }
    }
    const char *base_name = last_slash == NULL ? path : last_slash + 1;
    const char *temp_base = base_name[0] == '\0' ? "tmp" : base_name;
    char *temp_path = NULL;
    if (last_slash == NULL) {
      size_t temp_len = strlen(temp_base) + strlen("..tmp.XXXXXX") + 1;
      temp_path = (char *)malloc(temp_len);
      if (temp_path == NULL) {
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }
      snprintf(temp_path, temp_len, ".%s.tmp.XXXXXX", temp_base);
    } else {
      size_t dir_len = (size_t)(last_slash - path + 1);
      size_t temp_len = dir_len + 1 + strlen(temp_base) + strlen(".tmp.XXXXXX") + 1;
      temp_path = (char *)malloc(temp_len);
      if (temp_path == NULL) {
        fprintf(stderr, "error: native runtime allocation failure\n");
        exit(1);
      }
      snprintf(temp_path, temp_len, "%.*s.%s.tmp.XXXXXX", (int)dir_len, path, temp_base);
    }
    int temp_fd = mkstemp(temp_path);
    if (temp_fd < 0) {
      int io_errno = errno != 0 ? errno : EIO;
      free(temp_path);
      return tn_runtime_failf("host error: sys_write_text_atomic failed for '%s': %s", path, strerror(io_errno));
    }
    const char *cursor = content;
    size_t remaining = strlen(content);
    while (remaining > 0) {
      ssize_t written = write(temp_fd, cursor, remaining);
      if (written < 0) {
        if (errno == EINTR) {
          continue;
        }
        int io_errno = errno != 0 ? errno : EIO;
        close(temp_fd);
        unlink(temp_path);
        free(temp_path);
        return tn_runtime_failf("host error: sys_write_text_atomic failed for '%s': %s", path, strerror(io_errno));
      }
      cursor += (size_t)written;
      remaining -= (size_t)written;
    }
    if (fsync(temp_fd) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      close(temp_fd);
      unlink(temp_path);
      free(temp_path);
      return tn_runtime_failf("host error: sys_write_text_atomic failed for '%s': %s", path, strerror(io_errno));
    }
    if (close(temp_fd) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      unlink(temp_path);
      free(temp_path);
      return tn_runtime_failf("host error: sys_write_text_atomic failed for '%s': %s", path, strerror(io_errno));
    }
    if (rename(temp_path, path) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      unlink(temp_path);
      free(temp_path);
      return tn_runtime_failf("host error: sys_write_text_atomic failed for '%s': %s", path, strerror(io_errno));
    }
    free(temp_path);
    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  if (strcmp(key, "sys_lock_acquire") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_lock_acquire expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_lock_acquire expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    const char *last_slash = strrchr(path, '/');
    if (last_slash != NULL) {
      size_t parent_len = (size_t)(last_slash - path);
      if (parent_len > 0) {
        char *parent = (char *)malloc(parent_len + 1);
        if (parent == NULL) {
          fprintf(stderr, "error: native runtime allocation failure\n");
          exit(1);
        }
        memcpy(parent, path, parent_len);
        parent[parent_len] = '\0';
        for (size_t i = 1; i < parent_len; i += 1) {
          if (parent[i] != '/') {
            continue;
          }
          parent[i] = '\0';
          if (parent[0] != '\0' && mkdir(parent, 0777) != 0 && errno != EEXIST) {
            int mkdir_errno = errno;
            free(parent);
            return tn_runtime_failf("host error: sys_lock_acquire failed for '%s': %s", path, strerror(mkdir_errno));
          }
          parent[i] = '/';
        }
        if (mkdir(parent, 0777) != 0 && errno != EEXIST) {
          int mkdir_errno = errno;
          free(parent);
          return tn_runtime_failf("host error: sys_lock_acquire failed for '%s': %s", path, strerror(mkdir_errno));
        }
        free(parent);
      }
    }
    FILE *handle = fopen(path, "wx");
    if (handle == NULL) {
      if (errno == EEXIST) {
        free(args);
        return tn_runtime_const_bool((TnVal)0);
      }
      return tn_runtime_failf("host error: sys_lock_acquire failed for '%s': %s", path, strerror(errno));
    }
    struct timeval now;
    if (gettimeofday(&now, NULL) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      fclose(handle);
      return tn_runtime_failf("host error: sys_lock_acquire failed for '%s': %s", path, strerror(io_errno));
    }
    long long timestamp_ms = ((long long)now.tv_sec * 1000LL) + ((long long)now.tv_usec / 1000LL);
    char marker[128];
    int marker_len = snprintf(marker, sizeof(marker), "pid=%d timestamp_ms=%lld\n", getpid(), timestamp_ms);
    if (marker_len < 0 || (size_t)marker_len >= sizeof(marker) || fputs(marker, handle) < 0) {
      int io_errno = errno != 0 ? errno : EIO;
      fclose(handle);
      return tn_runtime_failf("host error: sys_lock_acquire failed for '%s': %s", path, strerror(io_errno));
    }
    if (fflush(handle) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      fclose(handle);
      return tn_runtime_failf("host error: sys_lock_acquire failed for '%s': %s", path, strerror(io_errno));
    }
    if (fsync(fileno(handle)) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      fclose(handle);
      return tn_runtime_failf("host error: sys_lock_acquire failed for '%s': %s", path, strerror(io_errno));
    }
    if (fclose(handle) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      return tn_runtime_failf("host error: sys_lock_acquire failed for '%s': %s", path, strerror(io_errno));
    }
    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  if (strcmp(key, "sys_lock_release") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_lock_release expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_lock_release expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    if (unlink(path) == 0) {
      free(args);
      return tn_runtime_const_bool((TnVal)1);
    }
    if (errno == ENOENT) {
      free(args);
      return tn_runtime_const_bool((TnVal)0);
    }
    return tn_runtime_failf("host error: sys_lock_release failed for '%s': %s", path, strerror(errno));
  }

  if (strcmp(key, "sys_read_text") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_read_text expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *path_obj = tn_get_obj(args[1]);
    if (path_obj == NULL || path_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_read_text expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *path = path_obj->as.text.text;
    FILE *handle = fopen(path, "rb");
    if (handle == NULL) {
      return tn_runtime_failf("host error: sys_read_text failed for '%s': %s", path, strerror(errno));
    }
    if (fseek(handle, 0, SEEK_END) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      fclose(handle);
      return tn_runtime_failf("host error: sys_read_text failed for '%s': %s", path, strerror(io_errno));
    }
    long size = ftell(handle);
    if (size < 0) {
      int io_errno = errno != 0 ? errno : EIO;
      fclose(handle);
      return tn_runtime_failf("host error: sys_read_text failed for '%s': %s", path, strerror(io_errno));
    }
    if (fseek(handle, 0, SEEK_SET) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      fclose(handle);
      return tn_runtime_failf("host error: sys_read_text failed for '%s': %s", path, strerror(io_errno));
    }
    char *buffer = (char *)malloc((size_t)size + 1);
    if (buffer == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    size_t bytes_read = fread(buffer, 1, (size_t)size, handle);
    if (bytes_read != (size_t)size) {
      int io_errno = errno != 0 ? errno : EIO;
      free(buffer);
      fclose(handle);
      return tn_runtime_failf("host error: sys_read_text failed for '%s': %s", path, strerror(io_errno));
    }
    buffer[size] = '\0';
    if (fclose(handle) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      free(buffer);
      return tn_runtime_failf("host error: sys_read_text failed for '%s': %s", path, strerror(io_errno));
    }
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)buffer);
    free(buffer);
    free(args);
    return result;
  }

  if (strcmp(key, "sys_read_stdin") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: sys_read_stdin expects exactly 0 arguments, found %zu", argc - 1);
    }
    size_t buffer_cap = 4096;
    size_t buffer_len = 0;
    char *buffer = (char *)malloc(buffer_cap + 1);
    if (buffer == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    for (;;) {
      char chunk[4096];
      size_t bytes_read = fread(chunk, 1, sizeof(chunk), stdin);
      if (bytes_read > 0) {
        size_t required = buffer_len + bytes_read;
        if (required > buffer_cap) {
          size_t next_cap = buffer_cap;
          while (next_cap < required) {
            if (next_cap > SIZE_MAX / 2) {
              next_cap = required;
              break;
            }
            next_cap *= 2;
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
        memcpy(buffer + buffer_len, chunk, bytes_read);
        buffer_len += bytes_read;
      }
      if (bytes_read < sizeof(chunk)) {
        if (ferror(stdin)) {
          int io_errno = errno != 0 ? errno : EIO;
          free(buffer);
          return tn_runtime_failf("host error: sys_read_stdin failed: %s", strerror(io_errno));
        }
        break;
      }
    }
    buffer[buffer_len] = '\0';
    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)buffer);
    free(buffer);
    free(args);
    return result;
  }

  if (strcmp(key, "sys_env") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_env expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *name_obj = tn_get_obj(args[1]);
    if (name_obj == NULL || name_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_env expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *value = getenv(name_obj->as.text.text);
    free(args);
    if (value == NULL) {
      return tn_runtime_const_nil();
    }
    return tn_runtime_const_string((TnVal)(intptr_t)value);
  }

  if (strcmp(key, "sys_which") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_which expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *name_obj = tn_get_obj(args[1]);
    if (name_obj == NULL || name_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_which expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    const char *name = name_obj->as.text.text;
    if (strchr(name, ' ') != NULL || strchr(name, '\t') != NULL || strchr(name, '\n') != NULL || strchr(name, '\r') != NULL) {
      return tn_runtime_fail("host error: sys_which command name contains unsupported whitespace");
    }
    char command[4096];
    int written = snprintf(command, sizeof(command), "command -v %s 2>/dev/null", name);
    if (written <= 0 || (size_t)written >= sizeof(command)) {
      return tn_runtime_fail("host error: sys_which command too long");
    }
    FILE *pipe = popen(command, "r");
    if (pipe == NULL) {
      return tn_runtime_fail("host error: sys_which failed to spawn shell");
    }
    char found[4096];
    if (fgets(found, sizeof(found), pipe) == NULL) {
      pclose(pipe);
      free(args);
      return tn_runtime_const_nil();
    }
    pclose(pipe);
    size_t len = strlen(found);
    while (len > 0 && (found[len - 1] == '\n' || found[len - 1] == '\r')) {
      found[len - 1] = '\0';
      len -= 1;
    }
    free(args);
    return tn_runtime_const_string((TnVal)(intptr_t)found);
  }

  if (strcmp(key, "sys_cwd") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: sys_cwd expects exactly 0 arguments, found %zu", argc - 1);
    }
    char cwd_buffer[PATH_MAX];
    if (getcwd(cwd_buffer, sizeof(cwd_buffer)) == NULL) {
      return tn_runtime_fail("host error: sys_cwd failed to read current directory");
    }
    free(args);
    return tn_runtime_const_string((TnVal)(intptr_t)cwd_buffer);
  }

  if (strcmp(key, "sys_argv") == 0) {
    if (argc != 1) {
      return tn_runtime_failf("host error: sys_argv expects exactly 0 arguments, found %zu", argc - 1);
    }
    
    // allocate list of strings for sys_argv
    TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
    list_obj->as.list.len = tn_global_argc;
    list_obj->as.list.items = tn_global_argc == 0 ? NULL : (TnVal *)calloc(tn_global_argc, sizeof(TnVal));
    if (tn_global_argc > 0 && list_obj->as.list.items == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    for (int i = 0; i < tn_global_argc; i++) {
      list_obj->as.list.items[i] = tn_runtime_const_string((TnVal)(intptr_t)tn_global_argv[i]);
      tn_runtime_retain(list_obj->as.list.items[i]);
    }
    free(args);
    return tn_heap_store(list_obj);
  }

  if (strcmp(key, "sys_sleep_ms") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_sleep_ms expects exactly 1 argument, found %zu", argc - 1);
    }
    long long delay_ms = tn_expect_host_int_arg("sys_sleep_ms", args[1], 1);
    if (delay_ms < 0 || delay_ms > 300000) {
      return tn_runtime_failf("host error: sys_sleep_ms delay_ms out of range: %lld", delay_ms);
    }
    if (delay_ms > 0) {
      struct timespec request;
      request.tv_sec = (time_t)(delay_ms / 1000);
      request.tv_nsec = (long)((delay_ms % 1000) * 1000000L);
      while (nanosleep(&request, &request) != 0) {
        if (errno == EINTR) {
          continue;
        }
        return tn_runtime_failf("host error: sys_sleep_ms failed: %s", strerror(errno));
      }
    }
    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  if (strcmp(key, "sys_log") == 0) {
    if (argc != 4) {
      return tn_runtime_failf("host error: sys_log expects exactly 3 arguments, found %zu", argc - 1);
    }

    TnObj *level_obj = tn_get_obj(args[1]);
    if (level_obj == NULL || (level_obj->kind != TN_OBJ_STRING && level_obj->kind != TN_OBJ_ATOM)) {
      return tn_runtime_failf(
          "host error: sys_log expects string-or-atom argument 1; found %s",
          tn_runtime_value_kind(args[1]));
    }
    const char *level = tn_sys_log_level_label(level_obj->as.text.text);
    if (level == NULL) {
      return tn_runtime_failf(
          "host error: sys_log level must be one of debug|info|warn|error; found %s",
          level_obj->as.text.text);
    }

    TnObj *event_obj = tn_get_obj(args[2]);
    if (event_obj == NULL || (event_obj->kind != TN_OBJ_STRING && event_obj->kind != TN_OBJ_ATOM)) {
      return tn_runtime_failf(
          "host error: sys_log expects string-or-atom argument 2; found %s",
          tn_runtime_value_kind(args[2]));
    }
    const char *event = event_obj->as.text.text;
    if (tn_sys_string_is_blank(event)) {
      return tn_runtime_fail("host error: sys_log event must not be empty");
    }

    TnObj *fields_obj = tn_get_obj(args[3]);
    if (fields_obj == NULL || fields_obj->kind != TN_OBJ_MAP) {
      return tn_runtime_failf(
          "host error: sys_log expects map argument 3; found %s",
          tn_runtime_value_kind(args[3]));
    }

    const char *sink_path = getenv("TONIC_SYSTEM_LOG_PATH");
    FILE *sink = stderr;
    int close_sink = 0;
    if (sink_path != NULL) {
      while (*sink_path != '\0' && tn_sys_log_is_space(*sink_path)) {
        sink_path += 1;
      }
      if (*sink_path != '\0') {
        tn_sys_ensure_parent_dir_for_file("sys_log", sink_path);
        sink = fopen(sink_path, "a");
        if (sink == NULL) {
          return tn_runtime_failf(
              "host error: sys_log failed to open sink '%s': %s",
              sink_path,
              strerror(errno));
        }
        close_sink = 1;
      }
    }

    fprintf(sink, "{\"timestamp_ms\":%lld,\"level\":", tn_sys_unix_timestamp_ms());
    tn_sys_json_write_string(sink, level);
    fputs(",\"event\":", sink);
    tn_sys_json_write_string(sink, event);
    fputs(",\"fields\":", sink);
    tn_sys_log_write_json_map_like(sink, "fields", fields_obj);
    fputs("}\n", sink);

    if (fflush(sink) != 0 || ferror(sink)) {
      int io_errno = errno != 0 ? errno : EIO;
      if (close_sink) {
        fclose(sink);
        return tn_runtime_failf(
            "host error: sys_log failed to append sink '%s': %s",
            sink_path,
            strerror(io_errno));
      }
      return tn_runtime_failf(
          "host error: sys_log failed to append sink 'stderr': %s",
          strerror(io_errno));
    }
    if (close_sink && fclose(sink) != 0) {
      int io_errno = errno != 0 ? errno : EIO;
      return tn_runtime_failf(
          "host error: sys_log failed to append sink '%s': %s",
          sink_path,
          strerror(io_errno));
    }

    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  if (strcmp(key, "sys_random_token") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_random_token expects exactly 1 argument, found %zu", argc - 1);
    }
    long long bytes = tn_expect_host_int_arg("sys_random_token", args[1], 1);
    if (bytes < 16 || bytes > 256) {
      return tn_runtime_failf("host error: sys_random_token bytes out of range: %lld", bytes);
    }

    unsigned char *buffer = (unsigned char *)malloc((size_t)bytes);
    if (buffer == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    tn_sys_fill_random_bytes(buffer, (size_t)bytes);
    char *encoded = tn_sys_base64url_encode(buffer, (size_t)bytes);
    free(buffer);

    TnVal result = tn_runtime_const_string((TnVal)(intptr_t)encoded);
    free(encoded);
    free(args);
    return result;
  }

  if (strcmp(key, "sys_run") == 0) {
    if (argc != 2 && argc != 3) {
      return tn_runtime_failf("host error: sys_run expects 1 or 2 arguments, found %zu", argc - 1);
    }

    const char *command_text = tn_expect_host_string_arg("sys_run", args[1], 1);
    int stream_output = 0;
    if (argc == 3) {
      TnObj *opts_obj = tn_expect_host_map_arg("sys_run", args[2], 2);
      for (size_t i = 0; i < opts_obj->as.map_like.len; i += 1) {
        TnVal opt_key = opts_obj->as.map_like.items[i].key;
        TnObj *opt_key_obj = tn_get_obj(opt_key);
        if (opt_key_obj == NULL || opt_key_obj->kind != TN_OBJ_ATOM) {
          return tn_runtime_failf(
              "host error: sys_run opts expects atom keys; found %s",
              tn_runtime_value_kind(opt_key));
        }

        const char *opt_name = opt_key_obj->as.text.text;
        TnVal opt_value = opts_obj->as.map_like.items[i].value;
        if (strcmp(opt_name, "stream") == 0) {
          TnObj *stream_obj = tn_get_obj(opt_value);
          if (stream_obj == NULL || stream_obj->kind != TN_OBJ_BOOL) {
            return tn_runtime_failf(
                "host error: sys_run opts.stream expects bool; found %s",
                tn_runtime_value_kind(opt_value));
          }
          stream_output = stream_obj->as.bool_value ? 1 : 0;
        } else {
          return tn_runtime_failf("host error: sys_run unsupported opts key: %s", opt_name);
        }
      }
    }

    int stdout_pipe[2] = {-1, -1};
    int stderr_pipe[2] = {-1, -1};
    if (pipe(stdout_pipe) != 0) {
      return tn_runtime_fail("host error: sys_run failed to spawn shell");
    }
    if (pipe(stderr_pipe) != 0) {
      close(stdout_pipe[0]);
      close(stdout_pipe[1]);
      return tn_runtime_fail("host error: sys_run failed to spawn shell");
    }

    pid_t child_pid = fork();
    if (child_pid < 0) {
      close(stdout_pipe[0]);
      close(stdout_pipe[1]);
      close(stderr_pipe[0]);
      close(stderr_pipe[1]);
      return tn_runtime_fail("host error: sys_run failed to spawn shell");
    }

    if (child_pid == 0) {
      close(stdout_pipe[0]);
      close(stderr_pipe[0]);
      if (dup2(stdout_pipe[1], STDOUT_FILENO) < 0 || dup2(stderr_pipe[1], STDERR_FILENO) < 0) {
        _exit(127);
      }
      close(stdout_pipe[1]);
      close(stderr_pipe[1]);
      execl("/bin/sh", "sh", "-lc", command_text, (char *)NULL);
      _exit(127);
    }

    close(stdout_pipe[1]);
    close(stderr_pipe[1]);

    size_t cap = 256;
    size_t len = 0;
    char *buffer = (char *)malloc(cap);
    if (buffer == NULL) {
      close(stdout_pipe[0]);
      close(stderr_pipe[0]);
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    buffer[0] = '\0';

    int stdout_fd = stdout_pipe[0];
    int stderr_fd = stderr_pipe[0];
    while (stdout_fd >= 0 || stderr_fd >= 0) {
      fd_set read_fds;
      FD_ZERO(&read_fds);
      int max_fd = -1;
      if (stdout_fd >= 0) {
        FD_SET(stdout_fd, &read_fds);
        if (stdout_fd > max_fd) {
          max_fd = stdout_fd;
        }
      }
      if (stderr_fd >= 0) {
        FD_SET(stderr_fd, &read_fds);
        if (stderr_fd > max_fd) {
          max_fd = stderr_fd;
        }
      }

      int ready = select(max_fd + 1, &read_fds, NULL, NULL, NULL);
      if (ready < 0) {
        if (errno == EINTR) {
          continue;
        }
        if (stdout_fd >= 0) close(stdout_fd);
        if (stderr_fd >= 0) close(stderr_fd);
        waitpid(child_pid, NULL, 0);
        free(buffer);
        return tn_runtime_failf("host error: sys_run failed to read stdout: %s", strerror(errno));
      }

      int active_fds[2] = {stdout_fd, stderr_fd};
      for (int stream_index = 0; stream_index < 2; stream_index += 1) {
        int current_fd = active_fds[stream_index];
        if (current_fd < 0 || !FD_ISSET(current_fd, &read_fds)) {
          continue;
        }

        char chunk[4096];
        ssize_t bytes_read = read(current_fd, chunk, sizeof(chunk));
        if (bytes_read < 0) {
          if (errno == EINTR) {
            continue;
          }
          if (stdout_fd >= 0) close(stdout_fd);
          if (stderr_fd >= 0) close(stderr_fd);
          waitpid(child_pid, NULL, 0);
          free(buffer);
          return tn_runtime_failf(
              stream_index == 0 ? "host error: sys_run failed to read stdout: %s" : "host error: sys_run failed to read stderr: %s",
              strerror(errno));
        }

        if (bytes_read == 0) {
          close(current_fd);
          if (stream_index == 0) {
            stdout_fd = -1;
          } else {
            stderr_fd = -1;
          }
          continue;
        }

        size_t required = len + (size_t)bytes_read + 1;
        if (required > cap) {
          size_t next_cap = cap;
          while (next_cap < required) {
            next_cap *= 2;
          }
          char *next = (char *)realloc(buffer, next_cap);
          if (next == NULL) {
            if (stdout_fd >= 0) close(stdout_fd);
            if (stderr_fd >= 0) close(stderr_fd);
            waitpid(child_pid, NULL, 0);
            free(buffer);
            fprintf(stderr, "error: native runtime allocation failure\n");
            exit(1);
          }
          buffer = next;
          cap = next_cap;
        }

        memcpy(buffer + len, chunk, (size_t)bytes_read);
        len += (size_t)bytes_read;
        buffer[len] = '\0';

        if (stream_output) {
          FILE *target = stream_index == 0 ? stdout : stderr;
          if (stream_index == 0) {
            tn_runtime_observe_stdout();
          }
          fwrite(chunk, 1, (size_t)bytes_read, target);
          fflush(target);
        }
      }
    }

    int status = 0;
    if (waitpid(child_pid, &status, 0) < 0) {
      free(buffer);
      return tn_runtime_failf("host error: sys_run failed to wait for shell command: %s", strerror(errno));
    }

    int exit_code = -1;
#ifdef WIFEXITED
    if (WIFEXITED(status)) {
      exit_code = WEXITSTATUS(status);
    }
#else
    exit_code = status;
#endif

    TnVal result = tn_runtime_make_map(
      tn_runtime_const_atom((TnVal)(intptr_t)"exit_code"),
      (TnVal)exit_code
    );
    result = tn_runtime_map_put(
      result,
      tn_runtime_const_atom((TnVal)(intptr_t)"output"),
      tn_runtime_const_string((TnVal)(intptr_t)buffer)
    );

    free(buffer);
    free(args);
    return result;
  }

"###,
    );
}
