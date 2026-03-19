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

  if (strcmp(key, "sys_run") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_run expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *command_obj = tn_get_obj(args[1]);
    if (command_obj == NULL || command_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_run expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }

    const char *command_text = command_obj->as.text.text;
    size_t command_len = strlen(command_text);
    char *shell_command = (char *)malloc(command_len + 6);
    if (shell_command == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    snprintf(shell_command, command_len + 6, "%s 2>&1", command_text);

    FILE *pipe = popen(shell_command, "r");
    free(shell_command);
    if (pipe == NULL) {
      return tn_runtime_fail("host error: sys_run failed to spawn shell");
    }

    size_t cap = 256;
    size_t len = 0;
    char *buffer = (char *)malloc(cap);
    if (buffer == NULL) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    buffer[0] = '\0';

    int next_ch = 0;
    while ((next_ch = fgetc(pipe)) != EOF) {
      if (len + 2 > cap) {
        cap *= 2;
        char *next = (char *)realloc(buffer, cap);
        if (next == NULL) {
          free(buffer);
          fprintf(stderr, "error: native runtime allocation failure\n");
          exit(1);
        }
        buffer = next;
      }

      buffer[len] = (char)next_ch;
      len += 1;
      buffer[len] = '\0';
    }

    int status = pclose(pipe);
    int exit_code = -1;
    if (status >= 0) {
#ifdef WIFEXITED
      if (WIFEXITED(status)) {
        exit_code = WEXITSTATUS(status);
      }
#else
      exit_code = status;
#endif
    }

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
