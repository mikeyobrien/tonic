pub(super) fn emit_stubs_host_http(out: &mut String) {
    out.push_str(
        r###"  if (strcmp(key, "sys_http_listen") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: sys_http_listen expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *host_obj = tn_get_obj(args[1]);
    if (host_obj == NULL || host_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_http_listen expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (tn_is_boxed(args[2])) {
      return tn_runtime_failf("host error: sys_http_listen expects int argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    int64_t port = (int64_t)args[2];
    if (port < 0 || port > 65535) {
      return tn_runtime_failf("host error: sys_http_listen port out of range: %lld", (long long)port);
    }

    int server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd < 0) {
      return tn_runtime_failf("host error: sys_http_listen failed to bind %s:%lld: %s", host_obj->as.text.text, (long long)port, strerror(errno));
    }
    int opt = 1;
    setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in address;
    memset(&address, 0, sizeof(address));
    address.sin_family = AF_INET;
    address.sin_port = htons((uint16_t)port);
    if (inet_pton(AF_INET, host_obj->as.text.text, &address.sin_addr) <= 0) {
      address.sin_addr.s_addr = INADDR_ANY;
    }

    if (bind(server_fd, (struct sockaddr *)&address, sizeof(address)) < 0) {
      close(server_fd);
      if (errno == EACCES) {
        return tn_runtime_failf("host error: sys_http_listen failed to bind %s:%lld: permission denied", host_obj->as.text.text, (long long)port);
      } else if (errno == EADDRINUSE) {
        return tn_runtime_failf("host error: sys_http_listen failed to bind %s:%lld: address already in use", host_obj->as.text.text, (long long)port);
      }
      return tn_runtime_failf("host error: sys_http_listen failed to bind %s:%lld: %s", host_obj->as.text.text, (long long)port, strerror(errno));
    }

    if (listen(server_fd, 128) < 0) {
      close(server_fd);
      return tn_runtime_failf("host error: sys_http_listen failed to bind %s:%lld: %s", host_obj->as.text.text, (long long)port, strerror(errno));
    }

    int listener_idx = tn_http_listeners_count++;
    tn_http_listeners[listener_idx] = server_fd;

    char id_buf[64];
    snprintf(id_buf, sizeof(id_buf), "listener:%d", listener_idx);

    TnVal result = tn_runtime_make_map(
      tn_runtime_const_atom((TnVal)(intptr_t)"status"),
      tn_runtime_const_atom((TnVal)(intptr_t)"ok")
    );
    result = tn_runtime_map_put(
      result,
      tn_runtime_const_atom((TnVal)(intptr_t)"listener_id"),
      tn_runtime_const_string((TnVal)(intptr_t)id_buf)
    );
    free(args);
    return result;
  }

  if (strcmp(key, "sys_http_accept") == 0) {
    if (argc != 3) {
      return tn_runtime_failf("host error: sys_http_accept expects exactly 2 arguments, found %zu", argc - 1);
    }
    TnObj *listener_obj = tn_get_obj(args[1]);
    if (listener_obj == NULL || listener_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_http_accept expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (tn_is_boxed(args[2])) {
      return tn_runtime_failf("host error: sys_http_accept expects int argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    int64_t timeout_ms = (int64_t)args[2];
    if (timeout_ms < 0) {
      return tn_runtime_failf("host error: sys_http_accept timeout_ms must be >= 0, found %lld", (long long)timeout_ms);
    }
    if (timeout_ms > 3600000) {
      return tn_runtime_failf("host error: sys_http_accept timeout_ms out of range: %lld", (long long)timeout_ms);
    }

    int listener_idx = -1;
    if (sscanf(listener_obj->as.text.text, "listener:%d", &listener_idx) != 1 || listener_idx < 0 || listener_idx >= tn_http_listeners_count) {
      return tn_runtime_failf("host error: sys_http_accept unknown listener_id: %s", listener_obj->as.text.text);
    }

    int server_fd = tn_http_listeners[listener_idx];

    if (timeout_ms > 0) {
      fd_set readfds;
      FD_ZERO(&readfds);
      FD_SET(server_fd, &readfds);
      struct timeval tv;
      tv.tv_sec = timeout_ms / 1000;
      tv.tv_usec = (timeout_ms % 1000) * 1000;
      int ret = select(server_fd + 1, &readfds, NULL, NULL, &tv);
      if (ret == 0) {
        return tn_runtime_fail("host error: sys_http_accept accept timeout elapsed");
      } else if (ret < 0) {
        return tn_runtime_failf("host error: sys_http_accept failed: %s", strerror(errno));
      }
    }

    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);
    int client_fd = accept(server_fd, (struct sockaddr *)&client_addr, &client_len);
    if (client_fd < 0) {
      return tn_runtime_failf("host error: sys_http_accept failed: %s", strerror(errno));
    }

    int conn_idx = tn_http_connections_count++;
    tn_http_connections[conn_idx] = client_fd;

    char id_buf[64];
    snprintf(id_buf, sizeof(id_buf), "conn:%d", conn_idx);

    char ip_buf[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &client_addr.sin_addr, ip_buf, INET_ADDRSTRLEN);
    int client_port = ntohs(client_addr.sin_port);

    TnVal result = tn_runtime_make_map(
      tn_runtime_const_atom((TnVal)(intptr_t)"status"),
      tn_runtime_const_atom((TnVal)(intptr_t)"ok")
    );
    result = tn_runtime_map_put(result, tn_runtime_const_atom((TnVal)(intptr_t)"connection_id"), tn_runtime_const_string((TnVal)(intptr_t)id_buf));
    result = tn_runtime_map_put(result, tn_runtime_const_atom((TnVal)(intptr_t)"client_ip"), tn_runtime_const_string((TnVal)(intptr_t)ip_buf));
    result = tn_runtime_map_put(result, tn_runtime_const_atom((TnVal)(intptr_t)"client_port"), (TnVal)client_port);
    free(args);
    return result;
  }

  if (strcmp(key, "sys_http_read_request") == 0) {
    if (argc != 2) {
      return tn_runtime_failf("host error: sys_http_read_request expects exactly 1 argument, found %zu", argc - 1);
    }
    TnObj *conn_obj = tn_get_obj(args[1]);
    if (conn_obj == NULL || conn_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_http_read_request expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }

    int conn_idx = -1;
    if (sscanf(conn_obj->as.text.text, "conn:%d", &conn_idx) != 1 || conn_idx < 0 || conn_idx >= tn_http_connections_count) {
      return tn_runtime_failf("host error: sys_http_read_request unknown connection_id: %s", conn_obj->as.text.text);
    }

    int client_fd = tn_http_connections[conn_idx];
    if (client_fd < 0) {
      return tn_runtime_failf("host error: sys_http_read_request unknown connection_id: %s", conn_obj->as.text.text);
    }

    fd_set readfds;
    struct timeval tv;
    FD_ZERO(&readfds);
    FD_SET(client_fd, &readfds);
    tv.tv_sec = 30;
    tv.tv_usec = 0;
    if (select(client_fd + 1, &readfds, NULL, NULL, &tv) <= 0) {
      return tn_runtime_fail("host error: sys_http_read_request timeout reading headers");
    }

    char *buf = (char *)malloc(65536);
    if (!buf) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    int total_read = 0;
    int headers_end = -1;
    while (total_read < 65535) {
      int n = recv(client_fd, buf + total_read, 1, 0);
      if (n <= 0) break;
      total_read += n;
      if (total_read >= 4 && memcmp(buf + total_read - 4, "\r\n\r\n", 4) == 0) {
        headers_end = total_read;
        break;
      }
    }

    if (headers_end == -1) {
      free(buf);
      return tn_runtime_fail("host error: sys_http_read_request malformed request line: <EOF>");
    }

    buf[headers_end] = '\0';
    char *method = strtok(buf, " ");
    char *path_full = strtok(NULL, " ");
    char *version = strtok(NULL, "\r\n");

    if (!method || !path_full || !version) {
      free(buf);
      return tn_runtime_fail("host error: sys_http_read_request malformed request line: <missing fields>");
    }

    if (strcmp(method, "GET") != 0 && strcmp(method, "POST") != 0 && strcmp(method, "PUT") != 0 && 
        strcmp(method, "PATCH") != 0 && strcmp(method, "DELETE") != 0 && strcmp(method, "HEAD") != 0) {
      free(buf);
      return tn_runtime_failf("host error: sys_http_read_request unsupported method: %s", method);
    }

    char *query = strchr(path_full, '?');
    char *path = path_full;
    char *query_str = (char *)"";
    if (query) {
      *query = '\0';
      query_str = query + 1;
    }

    char *line = version + strlen(version) + 2;
    TnVal header_tuples[128];
    int header_count = 0;

    int content_length = 0;

    while (line && *line != '\r' && *line != '\0') {
      char *end = strstr(line, "\r\n");
      if (!end) break;
      *end = '\0';

      char *colon = strchr(line, ':');
      if (!colon) {
        free(buf);
        return tn_runtime_failf("host error: sys_http_read_request malformed header: %s", line);
      }
      *colon = '\0';
      char *header_name = line;
      char *header_value = colon + 1;
      while (*header_value == ' ') header_value++;

      for (char *p = header_name; *p; p++) {
        if (*p >= 'A' && *p <= 'Z') *p += 32;
      }

      if (strcmp(header_name, "content-length") == 0) {
        content_length = atoi(header_value);
      }

      header_tuples[header_count++] = tn_runtime_make_tuple(
        tn_runtime_const_string((TnVal)(intptr_t)header_name),
        tn_runtime_const_string((TnVal)(intptr_t)header_value)
      );

      line = end + 2;
    }

    if (content_length > 8388608) {
      free(buf);
      return tn_runtime_fail("host error: sys_http_read_request request body exceeded max size");
    }

    char *body = (char *)"";
    if (content_length > 0) {
      body = (char *)malloc(content_length + 1);
      int body_read = 0;
      while (body_read < content_length) {
        int n = recv(client_fd, body + body_read, content_length - body_read, 0);
        if (n <= 0) {
          free(buf);
          free(body);
          return tn_runtime_fail("host error: sys_http_read_request failed to read: EOF");
        }
        body_read += n;
      }
      body[content_length] = '\0';
    }

    TnObj *list_obj = tn_new_obj(TN_OBJ_LIST);
    list_obj->as.list.len = header_count;
    list_obj->as.list.items = header_count == 0 ? NULL : (TnVal *)calloc(header_count, sizeof(TnVal));
    for (int i = 0; i < header_count; i++) {
      list_obj->as.list.items[i] = header_tuples[i];
      tn_runtime_retain(header_tuples[i]);
    }
    TnVal headers_val = tn_heap_store(list_obj);

    TnVal result = tn_runtime_make_map(
      tn_runtime_const_atom((TnVal)(intptr_t)"status"),
      tn_runtime_const_atom((TnVal)(intptr_t)"ok")
    );
    result = tn_runtime_map_put(result, tn_runtime_const_atom((TnVal)(intptr_t)"method"), tn_runtime_const_string((TnVal)(intptr_t)method));
    result = tn_runtime_map_put(result, tn_runtime_const_atom((TnVal)(intptr_t)"path"), tn_runtime_const_string((TnVal)(intptr_t)path));
    result = tn_runtime_map_put(result, tn_runtime_const_atom((TnVal)(intptr_t)"query_string"), tn_runtime_const_string((TnVal)(intptr_t)query_str));
    result = tn_runtime_map_put(result, tn_runtime_const_atom((TnVal)(intptr_t)"headers"), headers_val);
    result = tn_runtime_map_put(result, tn_runtime_const_atom((TnVal)(intptr_t)"body"), tn_runtime_const_string((TnVal)(intptr_t)body));

    if (content_length > 0) free(body);
    free(buf);
    free(args);
    return result;
  }

  if (strcmp(key, "sys_http_write_response") == 0) {
    if (argc != 5) {
      return tn_runtime_failf("host error: sys_http_write_response expects exactly 4 arguments, found %zu", argc - 1);
    }
    TnObj *conn_obj = tn_get_obj(args[1]);
    if (conn_obj == NULL || conn_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_http_write_response expects string argument 1; found %s", tn_runtime_value_kind(args[1]));
    }
    if (tn_is_boxed(args[2])) {
      return tn_runtime_failf("host error: sys_http_write_response expects int argument 2; found %s", tn_runtime_value_kind(args[2]));
    }
    int64_t status = (int64_t)args[2];
    if (status < 100 || status > 599) {
      return tn_runtime_failf("host error: sys_http_write_response status code out of range: %lld", (long long)status);
    }
    TnObj *headers_obj = tn_get_obj(args[3]);
    if (headers_obj == NULL || headers_obj->kind != TN_OBJ_LIST) {
      return tn_runtime_failf("host error: sys_http_write_response expects list argument 3; found %s", tn_runtime_value_kind(args[3]));
    }
    TnObj *body_obj = tn_get_obj(args[4]);
    if (body_obj == NULL || body_obj->kind != TN_OBJ_STRING) {
      return tn_runtime_failf("host error: sys_http_write_response expects string argument 4; found %s", tn_runtime_value_kind(args[4]));
    }

    int conn_idx = -1;
    if (sscanf(conn_obj->as.text.text, "conn:%d", &conn_idx) != 1 || conn_idx < 0 || conn_idx >= tn_http_connections_count) {
      return tn_runtime_failf("host error: sys_http_write_response unknown connection_id: %s", conn_obj->as.text.text);
    }

    int client_fd = tn_http_connections[conn_idx];
    if (client_fd < 0) {
      return tn_runtime_failf("host error: sys_http_write_response unknown connection_id: %s", conn_obj->as.text.text);
    }

    char *response_head = (char *)malloc(65536);
    if (!response_head) {
      fprintf(stderr, "error: native runtime allocation failure\n");
      exit(1);
    }
    int len = snprintf(response_head, 65536, "HTTP/1.1 %lld OK\r\n", (long long)status);
    
    int has_content_length = 0;
    for (size_t i = 0; i < headers_obj->as.list.len; i++) {
      TnObj *tuple = tn_get_obj(headers_obj->as.list.items[i]);
      if (tuple == NULL || tuple->kind != TN_OBJ_TUPLE) {
        return tn_runtime_failf("host error: sys_http_write_response headers argument 3 entry %zu must be {string, string}; found %s", i + 1, tn_runtime_value_kind(headers_obj->as.list.items[i]));
      }
      TnObj *k = tn_get_obj(tuple->as.tuple.left);
      if (k == NULL || k->kind != TN_OBJ_STRING) {
        return tn_runtime_failf("host error: sys_http_write_response headers argument 3 entry %zu expects string header name; found %s", i + 1, tn_runtime_value_kind(tuple->as.tuple.left));
      }
      TnObj *v = tn_get_obj(tuple->as.tuple.right);
      if (v == NULL || v->kind != TN_OBJ_STRING) {
        return tn_runtime_failf("host error: sys_http_write_response headers argument 3 entry %zu expects string header value; found %s", i + 1, tn_runtime_value_kind(tuple->as.tuple.right));
      }
      
      char lower_k[256];
      strncpy(lower_k, k->as.text.text, sizeof(lower_k) - 1);
      lower_k[sizeof(lower_k)-1] = '\0';
      for (char *p = lower_k; *p; p++) {
        if (*p >= 'A' && *p <= 'Z') *p += 32;
      }
      if (strcmp(lower_k, "content-length") == 0) has_content_length = 1;

      len += snprintf(response_head + len, 65536 - len, "%s: %s\r\n", k->as.text.text, v->as.text.text);
    }

    if (!has_content_length) {
      len += snprintf(response_head + len, 65536 - len, "Content-Length: %zu\r\n", strlen(body_obj->as.text.text));
    }
    len += snprintf(response_head + len, 65536 - len, "\r\n");

    send(client_fd, response_head, len, 0);
    send(client_fd, body_obj->as.text.text, strlen(body_obj->as.text.text), 0);
    
    close(client_fd);
    tn_http_connections[conn_idx] = -1;

    free(response_head);
    free(args);
    return tn_runtime_const_bool((TnVal)1);
  }

  return tn_runtime_failf("host error: unknown host function: %s", key);
"###,
    );
    out.push_str("}\n\n");
}
