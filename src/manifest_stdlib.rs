// Stdlib source constants for lazy-loaded optional modules.

pub(crate) const STDLIB_SOURCES: &[(&str, &str)] = &[
    ("System", OPTIONAL_STDLIB_SYSTEM_SOURCE),
    ("String", OPTIONAL_STDLIB_STRING_SOURCE),
    ("Path", OPTIONAL_STDLIB_PATH_SOURCE),
    ("IO", OPTIONAL_STDLIB_IO_SOURCE),
    ("List", OPTIONAL_STDLIB_LIST_SOURCE),
    ("Map", OPTIONAL_STDLIB_MAP_SOURCE),
    ("Enum", OPTIONAL_STDLIB_ENUM_SOURCE),
];

pub(super) const OPTIONAL_STDLIB_IO_SOURCE: &str =
    "defmodule IO do\n  def puts(value) do\n    host_call(:io_puts, value)\n  end\n\n  def inspect(value) do\n    host_call(:io_inspect, value)\n  end\n\n  def gets(prompt) do\n    host_call(:io_gets, prompt)\n  end\n\n  def ansi_red(value) do\n    host_call(:io_ansi_red, value)\n  end\n\n  def ansi_green(value) do\n    host_call(:io_ansi_green, value)\n  end\n\n  def ansi_yellow(value) do\n    host_call(:io_ansi_yellow, value)\n  end\n\n  def ansi_blue(value) do\n    host_call(:io_ansi_blue, value)\n  end\n\n  def ansi_reset() do\n    host_call(:io_ansi_reset)\n  end\nend\n";

pub(super) const OPTIONAL_STDLIB_LIST_SOURCE: &str =
    "defmodule List do\n  def first([]) do\n    nil\n  end\n\n  def first([head | _tail]) do\n    head\n  end\n\n  def last([]) do\n    nil\n  end\n\n  def last([value]) do\n    value\n  end\n\n  def last([_head | tail]) do\n    last(tail)\n  end\n\n  def flatten([]) do\n    []\n  end\n\n  def flatten([head | tail]) do\n    flatten_value(head) ++ flatten(tail)\n  end\n\n  def zip([], _right) do\n    []\n  end\n\n  def zip(_left, []) do\n    []\n  end\n\n  def zip([left_head | left_tail], [right_head | right_tail]) do\n    [{left_head, right_head}] ++ zip(left_tail, right_tail)\n  end\n\n  def unzip([]) do\n    {[], []}\n  end\n\n  def unzip([{left, right} | tail]) do\n    unzip_with_pair(left, right, unzip(tail))\n  end\n\n  def wrap(nil) do\n    []\n  end\n\n  def wrap(value) when is_list(value) do\n    value\n  end\n\n  def wrap(value) do\n    [value]\n  end\n\n  defp flatten_value(value) when is_list(value) do\n    flatten(value)\n  end\n\n  defp flatten_value(value) do\n    [value]\n  end\n\n  defp unzip_with_pair(left, right, {lefts, rights}) do\n    {[left] ++ lefts, [right] ++ rights}\n  end\nend\n";

pub(super) const OPTIONAL_STDLIB_MAP_SOURCE: &str =
    "defmodule Map do\n  def keys(map) do\n    host_call(:map_keys, map)\n  end\n\n  def values(map) do\n    host_call(:map_values, map)\n  end\n\n  def merge(left, right) do\n    host_call(:map_merge, left, right)\n  end\n\n  def drop(map, keys) do\n    host_call(:map_drop, map, keys)\n  end\n\n  def take(map, keys) do\n    host_call(:map_take, map, keys)\n  end\n\n  def get(map, key, default) do\n    host_call(:map_get, map, key, default)\n  end\n\n  def put(map, key, value) do\n    host_call(:map_put, map, key, value)\n  end\n\n  def delete(map, key) do\n    host_call(:map_delete, map, key)\n  end\nend\n";

pub(super) const OPTIONAL_STDLIB_ENUM_SOURCE: &str = r#"defmodule Enum do
  def count(enumerable) do
    count_list(to_list(enumerable))
  end

  def sum(enumerable) do
    sum_list(to_list(enumerable))
  end

  def join(enumerable, separator) do
    host_call(:enum_join, enumerable, separator)
  end

  def sort(enumerable) do
    host_call(:enum_sort, enumerable)
  end

  def reverse(enumerable) do
    reverse_list(to_list(enumerable), [])
  end

  def take(_enumerable, count) when count <= 0 do
    []
  end

  def take(enumerable, count) do
    take_list(to_list(enumerable), count)
  end

  def drop(enumerable, count) when count <= 0 do
    to_list(enumerable)
  end

  def drop(enumerable, count) do
    drop_list(to_list(enumerable), count)
  end

  def chunk_every(_enumerable, count) when count <= 0 do
    raise "Enum.chunk_every chunk size must be positive"
  end

  def chunk_every(enumerable, count) do
    chunk_every_list(to_list(enumerable), count)
  end

  def unique(enumerable) do
    reverse_list(unique_list(to_list(enumerable), []), [])
  end

  def into([], collectable) when is_list(collectable) do
    collectable
  end

  def into([head | tail], collectable) when is_list(collectable) do
    collectable ++ [head] ++ tail
  end

  def into(enumerable, collectable) when is_list(collectable) do
    collectable ++ to_list(enumerable)
  end

  def into([], collectable) when is_map(collectable) do
    collectable
  end

  def into([head | tail], collectable) when is_map(collectable) do
    into_map([head] ++ tail, collectable, 1)
  end

  def into(enumerable, collectable) when is_map(collectable) do
    into_map(to_list(enumerable), collectable, 1)
  end

  def into(_enumerable, collectable) do
    raise "Enum.into collectable must be list or map; found #{value_kind(collectable)}"
  end

  defp count_list([]) do
    0
  end

  defp count_list([_head | tail]) do
    1 + count_list(tail)
  end

  defp sum_list([]) do
    0
  end

  defp sum_list([head | tail]) do
    head + sum_list(tail)
  end

  defp reverse_list([], acc) do
    acc
  end

  defp reverse_list([head | tail], acc) do
    reverse_list(tail, [head] ++ acc)
  end

  defp take_list([], _count) do
    []
  end

  defp take_list(_items, count) when count <= 0 do
    []
  end

  defp take_list([head | tail], count) do
    [head] ++ take_list(tail, count - 1)
  end

  defp drop_list([], _count) do
    []
  end

  defp drop_list(items, count) when count <= 0 do
    items
  end

  defp drop_list([_head | tail], count) do
    drop_list(tail, count - 1)
  end

  defp chunk_every_list([], _count) do
    []
  end

  defp chunk_every_list(items, count) do
    [take_list(items, count)] ++ chunk_every_list(drop_list(items, count), count)
  end

  defp unique_list([], acc) do
    acc
  end

  defp unique_list([head | tail], acc) do
    case head in acc do
      true -> unique_list(tail, acc)
      false -> unique_list(tail, [head] ++ acc)
      _ -> unique_list(tail, acc)
    end
  end

  defp into_map([], acc, _index) do
    acc
  end

  defp into_map([{key, value} | tail], acc, index) do
    into_map(tail, host_call(:map_put, acc, key, value), index + 1)
  end

  defp into_map([value | _tail], _acc, index) do
    raise "Enum.into entry #{index} must be a tuple when collecting into map; found #{value_kind(value)}"
  end

  defp to_list(enumerable) do
    for item <- enumerable do
      item
    end
  end

  defp value_kind(nil) do
    "nil"
  end

  defp value_kind(true) do
    "bool"
  end

  defp value_kind(false) do
    "bool"
  end

  defp value_kind(value) when is_integer(value) do
    "int"
  end

  defp value_kind(value) when is_float(value) do
    "float"
  end

  defp value_kind(value) when is_binary(value) do
    "string"
  end

  defp value_kind(value) when is_atom(value) do
    "atom"
  end

  defp value_kind(value) when is_tuple(value) do
    "tuple"
  end

  defp value_kind(value) when is_list(value) do
    "list"
  end

  defp value_kind(value) when is_map(value) do
    "map"
  end

  defp value_kind(_value) do
    "value"
  end
end
"#;

pub(super) const OPTIONAL_STDLIB_STRING_SOURCE: &str =
    "defmodule String do\n  def split(str, delimiter) do\n    host_call(:str_split, str, delimiter)\n  end\n\n  def replace(str, pattern, replacement) do\n    host_call(:str_replace, str, pattern, replacement)\n  end\n\n  def trim(str) do\n    host_call(:str_trim, str)\n  end\n\n  def trim_leading(str) do\n    host_call(:str_trim_leading, str)\n  end\n\n  def trim_trailing(str) do\n    host_call(:str_trim_trailing, str)\n  end\n\n  def starts_with(str, prefix) do\n    host_call(:str_starts_with, str, prefix)\n  end\n\n  def ends_with(str, suffix) do\n    host_call(:str_ends_with, str, suffix)\n  end\n\n  def contains(str, substr) do\n    host_call(:str_contains, str, substr)\n  end\n\n  def upcase(str) do\n    host_call(:str_upcase, str)\n  end\n\n  def downcase(str) do\n    host_call(:str_downcase, str)\n  end\n\n  def length(str) do\n    host_call(:str_length, str)\n  end\n\n  def to_charlist(str) do\n    host_call(:str_to_charlist, str)\n  end\n\n  def at(str, index) do\n    host_call(:str_at, str, index)\n  end\n\n  def slice(str, start, len) do\n    host_call(:str_slice, str, start, len)\n  end\n\n  def to_integer(str) do\n    host_call(:str_to_integer, str)\n  end\n\n  def to_float(str) do\n    host_call(:str_to_float, str)\n  end\n\n  def pad_leading(str, count, padding) do\n    host_call(:str_pad_leading, str, count, padding)\n  end\n\n  def pad_trailing(str, count, padding) do\n    host_call(:str_pad_trailing, str, count, padding)\n  end\n\n  def reverse(str) do\n    host_call(:str_reverse, str)\n  end\nend\n";

pub(super) const OPTIONAL_STDLIB_PATH_SOURCE: &str =
    "defmodule Path do\n  def join(a, b) do\n    host_call(:path_join, a, b)\n  end\n\n  def dirname(path) do\n    host_call(:path_dirname, path)\n  end\n\n  def basename(path) do\n    host_call(:path_basename, path)\n  end\n\n  def extname(path) do\n    host_call(:path_extname, path)\n  end\n\n  def expand(path) do\n    host_call(:path_expand, path)\n  end\n\n  def relative_to(path, base) do\n    host_call(:path_relative_to, path, base)\n  end\nend\n";

pub(super) const OPTIONAL_STDLIB_SYSTEM_SOURCE: &str =
    "defmodule System do\n  def run(command) do\n    host_call(:sys_run, command)\n  end\n\n  def sleep_ms(delay_ms) do\n    host_call(:sys_sleep_ms, delay_ms)\n  end\n\n  def retry_plan(status_code, attempt, max_attempts, base_delay_ms, max_delay_ms, jitter_ms, retry_after) do\n    host_call(:sys_retry_plan, status_code, attempt, max_attempts, base_delay_ms, max_delay_ms, jitter_ms, retry_after)\n  end\n\n  def log(level, event, fields) do\n    host_call(:sys_log, level, event, fields)\n  end\n\n  def path_exists(path) do\n    host_call(:sys_path_exists, path)\n  end\n\n  def list_files_recursive(path) do\n    host_call(:sys_list_files_recursive, path)\n  end\n\n  def ensure_dir(path) do\n    host_call(:sys_ensure_dir, path)\n  end\n\n  def remove_tree(path) do\n    host_call(:sys_remove_tree, path)\n  end\n\n  def write_text(path, content) do\n    host_call(:sys_write_text, path, content)\n  end\n\n  def append_text(path, content) do\n    host_call(:sys_append_text, path, content)\n  end\n\n  def write_text_atomic(path, content) do\n    host_call(:sys_write_text_atomic, path, content)\n  end\n\n  def lock_acquire(path) do\n    host_call(:sys_lock_acquire, path)\n  end\n\n  def lock_release(path) do\n    host_call(:sys_lock_release, path)\n  end\n\n  def read_text(path) do\n    host_call(:sys_read_text, path)\n  end\n\n  def read_stdin() do\n    host_call(:sys_read_stdin)\n  end\n\n  def http_request(method, url, headers, body, opts) do\n    host_call(:sys_http_request, method, url, headers, body, opts)\n  end\n\n  def env(name) do\n    host_call(:sys_env, name)\n  end\n\n  def which(name) do\n    host_call(:sys_which, name)\n  end\n\n  def cwd() do\n    host_call(:sys_cwd)\n  end\n\n  def argv() do\n    host_call(:sys_argv)\n  end\n\n  def random_token(bytes) do\n    host_call(:sys_random_token, bytes)\n  end\n\n  def hmac_sha256_hex(secret, message) do\n    host_call(:sys_hmac_sha256_hex, secret, message)\n  end\n\n  def constant_time_eq(left, right) do\n    host_call(:sys_constant_time_eq, left, right)\n  end\n\n  def discord_ed25519_verify(public_key_hex, signature_hex, timestamp, body) do\n    host_call(:sys_discord_ed25519_verify, public_key_hex, signature_hex, timestamp, body)\n  end\n\n  def http_listen(host, port) do\n    host_call(:sys_http_listen, host, port)\n  end\n\n  def http_accept(listener_id, timeout_ms) do\n    host_call(:sys_http_accept, listener_id, timeout_ms)\n  end\n\n  def http_read_request(connection_id) do\n    host_call(:sys_http_read_request, connection_id)\n  end\n\n  def http_write_response(connection_id, status, headers, body) do\n    host_call(:sys_http_write_response, connection_id, status, headers, body)\n  end\nend\n";
