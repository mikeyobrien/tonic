pub(crate) const STDLIB_SOURCES: &[(&str, &str)] = &[
    ("System", OPTIONAL_STDLIB_SYSTEM_SOURCE),
    ("String", OPTIONAL_STDLIB_STRING_SOURCE),
    ("Path", OPTIONAL_STDLIB_PATH_SOURCE),
    ("IO", OPTIONAL_STDLIB_IO_SOURCE),
    ("List", OPTIONAL_STDLIB_LIST_SOURCE),
    ("Map", OPTIONAL_STDLIB_MAP_SOURCE),
    ("Enum", OPTIONAL_STDLIB_ENUM_SOURCE),
    ("Integer", OPTIONAL_STDLIB_INTEGER_SOURCE),
    ("Float", OPTIONAL_STDLIB_FLOAT_SOURCE),
    ("Json", OPTIONAL_STDLIB_JSON_SOURCE),
    ("Toml", OPTIONAL_STDLIB_TOML_SOURCE),
    ("Shell", OPTIONAL_STDLIB_SHELL_SOURCE),
    ("DateTime", OPTIONAL_STDLIB_DATETIME_SOURCE),
    ("Base64", OPTIONAL_STDLIB_BASE64_SOURCE),
    ("Crypto", OPTIONAL_STDLIB_CRYPTO_SOURCE),
    ("Http", OPTIONAL_STDLIB_HTTP_SOURCE),
    ("Uuid", OPTIONAL_STDLIB_UUID_SOURCE),
    ("Yaml", OPTIONAL_STDLIB_YAML_SOURCE),
    ("Env", OPTIONAL_STDLIB_ENV_SOURCE),
    ("Url", OPTIONAL_STDLIB_URL_SOURCE),
];

const OPTIONAL_STDLIB_IO_SOURCE: &str =
    "defmodule IO do\n  def puts(value) do\n    host_call(:io_puts, value)\n  end\n\n  def inspect(value) do\n    host_call(:io_inspect, value)\n  end\n\n  def gets(prompt) do\n    host_call(:io_gets, prompt)\n  end\n\n  def ansi_red(value) do\n    host_call(:io_ansi_red, value)\n  end\n\n  def ansi_green(value) do\n    host_call(:io_ansi_green, value)\n  end\n\n  def ansi_yellow(value) do\n    host_call(:io_ansi_yellow, value)\n  end\n\n  def ansi_blue(value) do\n    host_call(:io_ansi_blue, value)\n  end\n\n  def ansi_reset() do\n    host_call(:io_ansi_reset)\n  end\nend\n";

const OPTIONAL_STDLIB_LIST_SOURCE: &str =
    "defmodule List do\n  def first(list, default \\\\ nil) do\n    first_impl(list, default)\n  end\n\n  def last(list, default \\\\ nil) do\n    last_impl(list, default)\n  end\n\n  defp first_impl([], default) do\n    default\n  end\n\n  defp first_impl([head | _tail], _default) do\n    head\n  end\n\n  defp last_impl([], default) do\n    default\n  end\n\n  defp last_impl([value], _default) do\n    value\n  end\n\n  defp last_impl([_head | tail], default) do\n    last_impl(tail, default)\n  end\n\n  def flatten([]) do\n    []\n  end\n\n  def flatten([head | tail]) do\n    flatten_value(head) ++ flatten(tail)\n  end\n\n  def zip([], _right) do\n    []\n  end\n\n  def zip(_left, []) do\n    []\n  end\n\n  def zip([left_head | left_tail], [right_head | right_tail]) do\n    [{left_head, right_head}] ++ zip(left_tail, right_tail)\n  end\n\n  def unzip([]) do\n    {[], []}\n  end\n\n  def unzip([{left, right} | tail]) do\n    unzip_with_pair(left, right, unzip(tail))\n  end\n\n  def wrap(nil) do\n    []\n  end\n\n  def wrap(value) when is_list(value) do\n    value\n  end\n\n  def wrap(value) do\n    [value]\n  end\n\n  def delete([], _value) do\n    []\n  end\n\n  def delete([head | tail], value) do\n    case head == value do\n      true -> tail\n      _ -> [head] ++ delete(tail, value)\n    end\n  end\n\n  def duplicate(_value, count) when count <= 0 do\n    []\n  end\n\n  def duplicate(value, count) do\n    [value] ++ duplicate(value, count - 1)\n  end\n\n  def insert_at(list, 0, value) do\n    [value] ++ list\n  end\n\n  def insert_at([], _index, value) do\n    [value]\n  end\n\n  def insert_at([head | tail], index, value) do\n    [head] ++ insert_at(tail, index - 1, value)\n  end\n\n  def starts_with([], _prefix) do
    starts_with_check([], _prefix)
  end

  def starts_with(list, prefix) do
    starts_with_check(list, prefix)
  end

  defp starts_with_check(_list, []) do
    true
  end

  defp starts_with_check([], _prefix) do
    false
  end

  defp starts_with_check([lh | lt], [ph | pt]) do
    case lh == ph do
      true -> starts_with_check(lt, pt)
      _ -> false
    end
  end

  defp flatten_value(value) when is_list(value) do\n    flatten(value)\n  end\n\n  defp flatten_value(value) do\n    [value]\n  end\n\n  defp unzip_with_pair(left, right, {lefts, rights}) do\n    {[left] ++ lefts, [right] ++ rights}\n  end\n\n  def delete_at(list, index) do\n    delete_at_impl(list, index, 0)\n  end\n\n  defp delete_at_impl([], _index, _current) do\n    []\n  end\n\n  defp delete_at_impl([_head | tail], index, current) when current == index do\n    tail\n  end\n\n  defp delete_at_impl([head | tail], index, current) do\n    [head] ++ delete_at_impl(tail, index, current + 1)\n  end\n\n  def update_at(list, index, fun) do\n    update_at_impl(list, index, fun, 0)\n  end\n\n  defp update_at_impl([], _index, _fun, _current) do\n    []\n  end\n\n  defp update_at_impl([head | tail], index, fun, current) when current == index do\n    [fun.(head)] ++ tail\n  end\n\n  defp update_at_impl([head | tail], index, fun, current) do\n    [head] ++ update_at_impl(tail, index, fun, current + 1)\n  end\n\n  def to_tuple(list) do\n    host_call(:list_to_tuple, list)\n  end\nend\n";

const OPTIONAL_STDLIB_MAP_SOURCE: &str =
    "defmodule Map do\n  def keys(map) do\n    host_call(:map_keys, map)\n  end\n\n  def values(map) do\n    host_call(:map_values, map)\n  end\n\n  def merge(left, right, fun \\\\ nil) do\n    case fun do\n      nil -> host_call(:map_merge, left, right)\n      _ -> merge_with_resolver(left, right, fun)\n    end\n  end\n\n  def drop(map, keys) do\n    host_call(:map_drop, map, keys)\n  end\n\n  def take(map, keys) do\n    host_call(:map_take, map, keys)\n  end\n\n  def get(map, key, default \\\\ nil) do\n    host_call(:map_get, map, key, default)\n  end\n\n  def put(map, key, value) do\n    host_call(:map_put, map, key, value)\n  end\n\n  def delete(map, key) do\n    host_call(:map_delete, map, key)\n  end\n\n  def has_key(map, key) do\n    host_call(:map_has_key, map, key)\n  end\n\n  def to_list(map) do\n    for item <- map do\n      item\n    end\n  end\n\n  def new() do\n    %{}\n  end\n\n  def from_list(list) do\n    new_from_list(list, %{})\n  end\n\n  def filter(map, fun) do\n    map_filter_list(for item <- map do item end, fun, %{})\n  end\n\n  def reject(map, fun) do\n    map_reject_list(for item <- map do item end, fun, %{})\n  end\n\n  def pop(map, key, default \\\\ nil) do\n    case host_call(:map_has_key, map, key) do\n      true -> {host_call(:map_get, map, key, default), host_call(:map_delete, map, key)}\n      _ -> {default, map}\n    end\n  end\n\n  defp map_filter_list([], _fun, acc) do\n    acc\n  end\n\n  defp map_filter_list([head | tail], fun, acc) do\n    case fun.(head) do\n      true -> do\n        {k, v} = head\n        map_filter_list(tail, fun, host_call(:map_put, acc, k, v))\n      end\n      _ -> map_filter_list(tail, fun, acc)\n    end\n  end\n\n  defp map_reject_list([], _fun, acc) do\n    acc\n  end\n\n  defp map_reject_list([head | tail], fun, acc) do\n    case fun.(head) do\n      true -> map_reject_list(tail, fun, acc)\n      _ -> do\n        {k, v} = head\n        map_reject_list(tail, fun, host_call(:map_put, acc, k, v))\n      end\n    end\n  end\n\n  defp merge_with_resolver(left, right, fun) do\n    merge_entries(left, for item <- right do item end, fun)\n  end\n\n  defp merge_entries(acc, [], _fun) do\n    acc\n  end\n\n  defp merge_entries(acc, [{key, value} | tail], fun) do\n    case host_call(:map_has_key, acc, key) do\n      true -> do\n        resolved = fun.(key, host_call(:map_get, acc, key, nil), value)\n        merge_entries(host_call(:map_put, acc, key, resolved), tail, fun)\n      end\n      _ -> merge_entries(host_call(:map_put, acc, key, value), tail, fun)\n    end\n  end\n\n  defp new_from_list([], acc) do\n    acc\n  end\n\n  defp new_from_list([{key, value} | tail], acc) do\n    new_from_list(tail, host_call(:map_put, acc, key, value))\n  end\nend\n";

const OPTIONAL_STDLIB_ENUM_SOURCE: &str = r#"defmodule Enum do
  def count(enumerable, fun \\ nil) do
    case fun do
      nil -> count_list(to_list(enumerable))
      _ -> count_filtered(to_list(enumerable), fun, 0)
    end
  end

  def sum(enumerable) do
    sum_list(to_list(enumerable))
  end

  def join(enumerable, separator \\ "") do
    host_call(:enum_join, enumerable, separator)
  end

  def sort(enumerable, fun \\ nil) do
    case fun do
      nil -> host_call(:enum_sort, enumerable)
      _ -> sort_with_compare(to_list(enumerable), fun)
    end
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

  def map(enumerable, fun) do
    for item <- to_list(enumerable) do
      fun.(item)
    end
  end

  def filter(enumerable, fun) do
    filter_list(to_list(enumerable), fun, [])
  end

  def reduce(enumerable, acc, fun) do
    reduce_list(to_list(enumerable), acc, fun)
  end

  def find(enumerable, fun) do
    find_list(to_list(enumerable), fun)
  end

  def any(enumerable, fun) do
    any_list(to_list(enumerable), fun)
  end

  def all(enumerable, fun) do
    all_list(to_list(enumerable), fun)
  end

  def min([]) do
    raise "Enum.min called on empty list"
  end

  def min(enumerable) do
    items = to_list(enumerable)
    min_list(items)
  end

  def max([]) do
    raise "Enum.max called on empty list"
  end

  def max(enumerable) do
    items = to_list(enumerable)
    max_list(items)
  end

  def flat_map(enumerable, fun) do
    flat_map_list(to_list(enumerable), fun)
  end

  def zip(left, right) do
    zip_lists(to_list(left), to_list(right))
  end

  def with_index(enumerable) do
    with_index_list(to_list(enumerable), 0)
  end

  def each(enumerable, fun) do
    each_list(to_list(enumerable), fun)
  end

  def at(enumerable, index) do
    at_list(to_list(enumerable), index)
  end

  def fetch(enumerable, index) do
    list = to_list(enumerable)
    case index < length(list) do
      true -> {:ok, at_list(list, index)}
      _ -> :error
    end
  end

  def member(enumerable, value) do
    member_list(to_list(enumerable), value)
  end

  def reject(enumerable, fun) do
    reject_list(to_list(enumerable), fun, [])
  end

  def sort_by(enumerable, fun) do
    sort_by_insert(to_list(enumerable), fun)
  end

  def group_by(enumerable, fun) do
    group_by_list(to_list(enumerable), fun, %{})
  end

  def min_by(enumerable, fun) do
    sorted = sort_by(enumerable, fun)
    at_list(sorted, 0)
  end

  def max_by(enumerable, fun) do
    sorted = sort_by(enumerable, fun)
    last_list(sorted)
  end

  def frequencies(enumerable) do
    freq_list(to_list(enumerable), %{})
  end

  def uniq_by(enumerable, fun) do
    uniq_by_list(to_list(enumerable), fun, %{}, [])
  end

  def map_join(enumerable, separator, fun) do
    mapped = for item <- to_list(enumerable) do
      fun.(item)
    end
    host_call(:enum_join, mapped, separator)
  end

  def dedup(enumerable) do
    dedup_list(to_list(enumerable))
  end

  def intersperse(enumerable, separator) do
    intersperse_list(to_list(enumerable), separator)
  end

  def zip_with(left, right, fun) do
    zip_with_lists(to_list(left), to_list(right), fun)
  end

  def take_while(enumerable, fun) do
    take_while_list(to_list(enumerable), fun)
  end

  def drop_while(enumerable, fun) do
    drop_while_list(to_list(enumerable), fun)
  end

  def chunk_by(enumerable, fun) do
    chunk_by_list(to_list(enumerable), fun)
  end

  def scan(enumerable, acc, fun) do
    scan_list(to_list(enumerable), acc, fun)
  end

  def split(enumerable, count) do
    items = to_list(enumerable)
    {take_list(items, count), drop_list(items, count)}
  end

  def count_by(enumerable, fun) do
    count_by_list(to_list(enumerable), fun, 0)
  end

  def uniq(enumerable) do
    unique(enumerable)
  end

  def map_reduce(enumerable, acc, fun) do
    map_reduce_list(to_list(enumerable), acc, fun, [])
  end

  def concat(left, right) do
    to_list(left) ++ to_list(right)
  end

  def product(enumerable) do
    product_list(to_list(enumerable))
  end

  def slice(enumerable, start, count) do
    host_call(:enum_slice, enumerable, start, count)
  end

  def random(enumerable) do
    host_call(:enum_random, enumerable)
  end

  def find_index(enumerable, fun) do
    find_index_list(to_list(enumerable), fun, 0)
  end

  def reduce_while(enumerable, acc, fun) do
    reduce_while_list(to_list(enumerable), acc, fun)
  end

  def shuffle(enumerable) do
    host_call(:enum_shuffle, enumerable)
  end

  defp find_index_list([], _fun, _index) do
    nil
  end

  defp find_index_list([head | tail], fun, index) do
    case fun.(head) do
      true -> index
      _ -> find_index_list(tail, fun, index + 1)
    end
  end

  defp reduce_while_list([], acc, _fun) do
    acc
  end

  defp reduce_while_list([head | tail], acc, fun) do
    result = fun.(head, acc)
    case elem(result, 0) do
      :cont -> reduce_while_list(tail, elem(result, 1), fun)
      _ -> elem(result, 1)
    end
  end

  defp sort_with_compare([], _fun) do
    []
  end

  defp sort_with_compare([head | tail], fun) do
    sorted_tail = sort_with_compare(tail, fun)
    insert_with_compare(head, sorted_tail, fun)
  end

  defp insert_with_compare(elem, [], _fun) do
    [elem]
  end

  defp insert_with_compare(elem, [head | tail], fun) do
    case fun.(elem, head) do
      true -> [elem] ++ [head] ++ tail
      _ -> [head] ++ insert_with_compare(elem, tail, fun)
    end
  end

  defp product_list([]) do
    1
  end

  defp product_list([head | tail]) do
    head * product_list(tail)
  end

  defp map_reduce_list([], acc, _fun, mapped) do
    {reverse_list(mapped, []), acc}
  end

  defp map_reduce_list([head | tail], acc, fun, mapped) do
    {new_elem, new_acc} = fun.(head, acc)
    map_reduce_list(tail, new_acc, fun, [new_elem] ++ mapped)
  end

  defp find_list([], _fun) do
    nil
  end

  defp find_list([head | tail], fun) do
    case fun.(head) do
      true -> head
      _ -> find_list(tail, fun)
    end
  end

  defp any_list([], _fun) do
    false
  end

  defp any_list([head | tail], fun) do
    case fun.(head) do
      true -> true
      _ -> any_list(tail, fun)
    end
  end

  defp all_list([], _fun) do
    true
  end

  defp all_list([head | tail], fun) do
    case fun.(head) do
      true -> all_list(tail, fun)
      _ -> false
    end
  end

  defp min_list([only]) do
    only
  end

  defp min_list([head | tail]) do
    tail_min = min_list(tail)
    case head < tail_min do
      true -> head
      _ -> tail_min
    end
  end

  defp max_list([only]) do
    only
  end

  defp max_list([head | tail]) do
    tail_max = max_list(tail)
    case head > tail_max do
      true -> head
      _ -> tail_max
    end
  end

  defp with_index_list([], _index) do
    []
  end

  defp with_index_list([head | tail], index) do
    [{head, index}] ++ with_index_list(tail, index + 1)
  end

  defp each_list([], _fun) do
    :ok
  end

  defp each_list([head | tail], fun) do
    fun.(head)
    each_list(tail, fun)
  end

  defp at_list([], _index) do
    nil
  end

  defp at_list([head | _tail], 0) do
    head
  end

  defp at_list([_head | tail], index) do
    at_list(tail, index - 1)
  end

  defp member_list([], _value) do
    false
  end

  defp member_list([head | tail], value) do
    case head == value do
      true -> true
      _ -> member_list(tail, value)
    end
  end

  defp flat_map_list([], _fun) do
    []
  end

  defp flat_map_list([head | tail], fun) do
    fun.(head) ++ flat_map_list(tail, fun)
  end

  defp zip_lists([], _right) do
    []
  end

  defp zip_lists(_left, []) do
    []
  end

  defp zip_lists([lh | lt], [rh | rt]) do
    [{lh, rh}] ++ zip_lists(lt, rt)
  end

  defp filter_list([], _fun, acc) do
    reverse_list(acc, [])
  end

  defp filter_list([head | tail], fun, acc) do
    case fun.(head) do
      true -> filter_list(tail, fun, [head] ++ acc)
      _ -> filter_list(tail, fun, acc)
    end
  end

  defp reduce_list([], acc, _fun) do
    acc
  end

  defp reduce_list([head | tail], acc, fun) do
    reduce_list(tail, fun.(head, acc), fun)
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

  defp last_list([only]) do
    only
  end

  defp last_list([_head | tail]) do
    last_list(tail)
  end

  defp sort_by_insert([], _fun) do
    []
  end

  defp sort_by_insert([head | tail], fun) do
    sorted_tail = sort_by_insert(tail, fun)
    insert_by(head, fun.(head), sorted_tail, fun)
  end

  defp insert_by(elem, _key, [], _fun) do
    [elem]
  end

  defp insert_by(elem, key, [head | tail], fun) do
    head_key = fun.(head)
    sorted = host_call(:enum_sort, [key, head_key])
    first = at_list(sorted, 0)
    case first == key do
      true -> [elem] ++ [head] ++ tail
      _ -> [head] ++ insert_by(elem, key, tail, fun)
    end
  end

  defp group_by_list([], _fun, acc) do
    acc
  end

  defp group_by_list([head | tail], fun, acc) do
    key = fun.(head)
    existing = host_call(:map_get, acc, key, [])
    new_acc = host_call(:map_put, acc, key, existing ++ [head])
    group_by_list(tail, fun, new_acc)
  end

  defp freq_list([], acc) do
    acc
  end

  defp freq_list([head | tail], acc) do
    current = host_call(:map_get, acc, head, 0)
    freq_list(tail, host_call(:map_put, acc, head, current + 1))
  end

  defp uniq_by_list([], _fun, _seen, acc) do
    reverse_list(acc, [])
  end

  defp uniq_by_list([head | tail], fun, seen, acc) do
    key = fun.(head)
    already = host_call(:map_has_key, seen, key)
    case already do
      true -> uniq_by_list(tail, fun, seen, acc)
      _ -> do
        new_seen = host_call(:map_put, seen, key, true)
        uniq_by_list(tail, fun, new_seen, [head] ++ acc)
      end
    end
  end

  defp dedup_list([]) do
    []
  end

  defp dedup_list([only]) do
    [only]
  end

  defp dedup_list([head | tail]) do
    dedup_rest(tail, head, [head])
  end

  defp dedup_rest([], _prev, acc) do
    reverse_list(acc, [])
  end

  defp dedup_rest([head | tail], prev, acc) do
    case head == prev do
      true -> dedup_rest(tail, prev, acc)
      _ -> dedup_rest(tail, head, [head] ++ acc)
    end
  end

  defp reject_list([], _fun, acc) do
    reverse_list(acc, [])
  end

  defp reject_list([head | tail], fun, acc) do
    case fun.(head) do
      true -> reject_list(tail, fun, acc)
      _ -> reject_list(tail, fun, [head] ++ acc)
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

  defp intersperse_list([], _sep) do
    []
  end

  defp intersperse_list([only], _sep) do
    [only]
  end

  defp intersperse_list([head | tail], sep) do
    [head, sep] ++ intersperse_list(tail, sep)
  end

  defp zip_with_lists([], _right, _fun) do
    []
  end

  defp zip_with_lists(_left, [], _fun) do
    []
  end

  defp zip_with_lists([lh | lt], [rh | rt], fun) do
    [fun.(lh, rh)] ++ zip_with_lists(lt, rt, fun)
  end

  defp take_while_list([], _fun) do
    []
  end

  defp take_while_list([head | tail], fun) do
    case fun.(head) do
      true -> [head] ++ take_while_list(tail, fun)
      _ -> []
    end
  end

  defp drop_while_list([], _fun) do
    []
  end

  defp drop_while_list([head | tail], fun) do
    case fun.(head) do
      true -> drop_while_list(tail, fun)
      _ -> [head] ++ tail
    end
  end

  defp chunk_by_list([], _fun) do
    []
  end

  defp chunk_by_list([head | tail], fun) do
    key = fun.(head)
    chunk_by_acc(tail, fun, key, [head], [])
  end

  defp chunk_by_acc([], _fun, _key, current, chunks) do
    reverse_list([reverse_list(current, [])] ++ chunks, [])
  end

  defp chunk_by_acc([head | tail], fun, key, current, chunks) do
    new_key = fun.(head)
    case new_key == key do
      true -> chunk_by_acc(tail, fun, key, [head] ++ current, chunks)
      _ -> chunk_by_acc(tail, fun, new_key, [head], [reverse_list(current, [])] ++ chunks)
    end
  end

  defp scan_list([], _acc, _fun) do
    []
  end

  defp scan_list([head | tail], acc, fun) do
    new_acc = fun.(head, acc)
    [new_acc] ++ scan_list(tail, new_acc, fun)
  end

  defp count_filtered([], _fun, acc) do
    acc
  end

  defp count_filtered([head | tail], fun, acc) do
    case fun.(head) do
      true -> count_filtered(tail, fun, acc + 1)
      _ -> count_filtered(tail, fun, acc)
    end
  end

  defp count_by_list([], _fun, acc) do
    acc
  end

  defp count_by_list([head | tail], fun, acc) do
    case fun.(head) do
      true -> count_by_list(tail, fun, acc + 1)
      _ -> count_by_list(tail, fun, acc)
    end
  end

  def to_list(enumerable) do
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

const OPTIONAL_STDLIB_STRING_SOURCE: &str =
    "defmodule String do\n  def split(str, delimiter \\\\ \" \") do\n    host_call(:str_split, str, delimiter)\n  end\n\n  def graphemes(str) do\n    host_call(:str_graphemes, str)\n  end\n\n  def replace(str, pattern, replacement) do\n    host_call(:str_replace, str, pattern, replacement)\n  end\n\n  def trim(str) do\n    host_call(:str_trim, str)\n  end\n\n  def trim_leading(str) do\n    host_call(:str_trim_leading, str)\n  end\n\n  def trim_trailing(str) do\n    host_call(:str_trim_trailing, str)\n  end\n\n  def starts_with(str, prefix) do\n    host_call(:str_starts_with, str, prefix)\n  end\n\n  def ends_with(str, suffix) do\n    host_call(:str_ends_with, str, suffix)\n  end\n\n  def contains(str, substr) do\n    host_call(:str_contains, str, substr)\n  end\n\n  def upcase(str) do\n    host_call(:str_upcase, str)\n  end\n\n  def downcase(str) do\n    host_call(:str_downcase, str)\n  end\n\n  def length(str) do\n    host_call(:str_length, str)\n  end\n\n  def to_charlist(str) do\n    host_call(:str_to_charlist, str)\n  end\n\n  def at(str, index) do\n    host_call(:str_at, str, index)\n  end\n\n  def slice(str, start, len) do\n    host_call(:str_slice, str, start, len)\n  end\n\n  def to_integer(str) do\n    host_call(:str_to_integer, str)\n  end\n\n  def to_float(str) do\n    host_call(:str_to_float, str)\n  end\n\n  def pad_leading(str, count, padding) do\n    host_call(:str_pad_leading, str, count, padding)\n  end\n\n  def pad_trailing(str, count, padding) do\n    host_call(:str_pad_trailing, str, count, padding)\n  end\n\n  def reverse(str) do\n    host_call(:str_reverse, str)\n  end\n\n  def duplicate(_str, count) when count <= 0 do\n    \"\"\n  end\n\n  def duplicate(str, count) do\n    duplicate_acc(str, count, \"\")\n  end\n\n  def to_atom(str) do\n    host_call(:str_to_atom, str)\n  end\n\n  def capitalize(\"\") do\n    \"\"\n  end\n\n  def capitalize(str) do\n    first = host_call(:str_upcase, host_call(:str_at, str, 0))\n    len = host_call(:str_length, str)\n    rest = host_call(:str_downcase, host_call(:str_slice, str, 1, len))\n    first <> rest\n  end\n\n  defp duplicate_acc(_str, 0, acc) do\n    acc\n  end\n\n  defp duplicate_acc(str, count, acc) do\n    duplicate_acc(str, count - 1, acc <> str)\n  end\nend\n";

const OPTIONAL_STDLIB_INTEGER_SOURCE: &str =
    "defmodule Integer do\n  def to_string(n) do\n    host_call(:integer_to_string, n)\n  end\n\n  def parse(str) do\n    host_call(:integer_parse, str)\n  end\nend\n";

const OPTIONAL_STDLIB_FLOAT_SOURCE: &str =
    "defmodule Float do\n  def to_string(n) do\n    host_call(:float_to_string, n)\n  end\n\n  def round(n, precision) do\n    host_call(:float_round, n, precision)\n  end\n\n  def ceil(n) do\n    host_call(:float_ceil, n)\n  end\n\n  def floor(n) do\n    host_call(:float_floor, n)\n  end\nend\n";

const OPTIONAL_STDLIB_PATH_SOURCE: &str =
    "defmodule Path do\n  def join(a, b) do\n    host_call(:path_join, a, b)\n  end\n\n  def dirname(path) do\n    host_call(:path_dirname, path)\n  end\n\n  def basename(path) do\n    host_call(:path_basename, path)\n  end\n\n  def extname(path) do\n    host_call(:path_extname, path)\n  end\n\n  def rootname(path) do\n    host_call(:path_rootname, path)\n  end\n\n  def expand(path) do\n    host_call(:path_expand, path)\n  end\n\n  def relative_to(path, base) do\n    host_call(:path_relative_to, path, base)\n  end\n\n  def split(path) do\n    host_call(:path_split, path)\n  end\nend\n";

const OPTIONAL_STDLIB_SYSTEM_SOURCE: &str =
    "defmodule System do\n  def run(command, opts \\\\ %{}) do\n    host_call(:sys_run, command, opts)\n  end\n\n  def sleep_ms(delay_ms) do\n    host_call(:sys_sleep_ms, delay_ms)\n  end\n\n  def retry_plan(status_code, attempt, max_attempts, base_delay_ms, max_delay_ms, jitter_ms, retry_after) do\n    host_call(:sys_retry_plan, status_code, attempt, max_attempts, base_delay_ms, max_delay_ms, jitter_ms, retry_after)\n  end\n\n  def log(level, event, fields) do\n    host_call(:sys_log, level, event, fields)\n  end\n\n  def path_exists(path) do\n    host_call(:sys_path_exists, path)\n  end\n\n  def list_dir(path) do\n    host_call(:sys_list_dir, path)\n  end\n\n  def is_dir(path) do\n    host_call(:sys_is_dir, path)\n  end\n\n  def list_files_recursive(path) do\n    host_call(:sys_list_files_recursive, path)\n  end\n\n  def ensure_dir(path) do\n    host_call(:sys_ensure_dir, path)\n  end\n\n  def remove_tree(path) do\n    host_call(:sys_remove_tree, path)\n  end\n\n  def write_text(path, content) do\n    host_call(:sys_write_text, path, content)\n  end\n\n  def append_text(path, content) do\n    host_call(:sys_append_text, path, content)\n  end\n\n  def write_text_atomic(path, content) do\n    host_call(:sys_write_text_atomic, path, content)\n  end\n\n  def lock_acquire(path) do\n    host_call(:sys_lock_acquire, path)\n  end\n\n  def lock_release(path) do\n    host_call(:sys_lock_release, path)\n  end\n\n  def read_text(path) do\n    host_call(:sys_read_text, path)\n  end\n\n  def read_stdin() do\n    host_call(:sys_read_stdin)\n  end\n\n  def http_request(method, url, headers, body, opts) do\n    host_call(:sys_http_request, method, url, headers, body, opts)\n  end\n\n  def env(name) do\n    host_call(:sys_env, name)\n  end\n\n  def which(name) do\n    host_call(:sys_which, name)\n  end\n\n  def cwd() do\n    host_call(:sys_cwd)\n  end\n\n  def argv() do\n    host_call(:sys_argv)\n  end\n\n  def random_token(bytes) do\n    host_call(:sys_random_token, bytes)\n  end\n\n  def hmac_sha256_hex(secret, message) do\n    host_call(:sys_hmac_sha256_hex, secret, message)\n  end\n\n  def constant_time_eq(left, right) do\n    host_call(:sys_constant_time_eq, left, right)\n  end\n\n  def discord_ed25519_verify(public_key_hex, signature_hex, timestamp, body) do\n    host_call(:sys_discord_ed25519_verify, public_key_hex, signature_hex, timestamp, body)\n  end\n\n  def http_listen(host, port) do\n    host_call(:sys_http_listen, host, port)\n  end\n\n  def http_accept(listener_id, timeout_ms) do\n    host_call(:sys_http_accept, listener_id, timeout_ms)\n  end\n\n  def http_read_request(connection_id) do\n    host_call(:sys_http_read_request, connection_id)\n  end\n\n  def http_write_response(connection_id, status, headers, body) do\n    host_call(:sys_http_write_response, connection_id, status, headers, body)\n  end\nend\n";

const OPTIONAL_STDLIB_TUPLE_SOURCE: &str =
    "defmodule Tuple do\n  def to_list(tuple) do\n    host_call(:tuple_to_list, tuple)\n  end\nend\n";

const OPTIONAL_STDLIB_JSON_SOURCE: &str =
    "defmodule Json do\n  def encode(value) do\n    host_call(:json_encode, value)\n  end\n\n  def decode(string) do\n    host_call(:json_decode, string)\n  end\n\n  def encode_pretty(value) do\n    host_call(:json_encode_pretty, value)\n  end\nend\n";

const OPTIONAL_STDLIB_TOML_SOURCE: &str =
    "defmodule Toml do\n  def encode(value) do\n    host_call(:toml_encode, value)\n  end\n\n  def decode(string) do\n    host_call(:toml_decode, string)\n  end\nend\n";

const OPTIONAL_STDLIB_SHELL_SOURCE: &str =
    "defmodule Shell do\n  def quote(string) do\n    host_call(:shell_quote, string)\n  end\n\n  def join(args) do\n    host_call(:shell_join, args)\n  end\nend\n";

const OPTIONAL_STDLIB_DATETIME_SOURCE: &str =
    "defmodule DateTime do\n  def utc_now() do\n    host_call(:datetime_utc_now)\n  end\n\n  def unix_now() do\n    host_call(:datetime_unix_now)\n  end\n\n  def unix_now_ms() do\n    host_call(:datetime_unix_now_ms)\n  end\nend\n";

const OPTIONAL_STDLIB_BASE64_SOURCE: &str =
    "defmodule Base64 do\n  def encode(string) do\n    host_call(:base64_encode, string)\n  end\n\n  def decode(string) do\n    host_call(:base64_decode, string)\n  end\n\n  def url_encode(string) do\n    host_call(:base64_url_encode, string)\n  end\n\n  def url_decode(string) do\n    host_call(:base64_url_decode, string)\n  end\nend\n";

const OPTIONAL_STDLIB_CRYPTO_SOURCE: &str =
    "defmodule Crypto do\n  def sha256(string) do\n    host_call(:crypto_sha256, string)\n  end\n\n  def hmac_sha256(key, message) do\n    host_call(:crypto_hmac_sha256, key, message)\n  end\n\n  def random_bytes(size) do\n    host_call(:crypto_random_bytes, size)\n  end\nend\n";

const OPTIONAL_STDLIB_HTTP_SOURCE: &str =
    "defmodule Http do\n  def get(url) do\n    get(url, [])\n  end\n\n  def get(url, headers) do\n    request(\"GET\", url, headers, \"\")\n  end\n\n  def post(url, body) do\n    post(url, body, [])\n  end\n\n  def post(url, body, headers) do\n    request(\"POST\", url, headers, body)\n  end\n\n  def put(url, body) do\n    put(url, body, [])\n  end\n\n  def put(url, body, headers) do\n    request(\"PUT\", url, headers, body)\n  end\n\n  def patch(url, body) do\n    patch(url, body, [])\n  end\n\n  def patch(url, body, headers) do\n    request(\"PATCH\", url, headers, body)\n  end\n\n  def delete(url) do\n    delete(url, [])\n  end\n\n  def delete(url, headers) do\n    request(\"DELETE\", url, headers, \"\")\n  end\n\n  def request(method, url, headers, body) do\n    request(method, url, headers, body, %{})\n  end\n\n  def request(method, url, headers, body, opts) do\n    host_call(:sys_http_request, method, url, headers, body, opts)\n  end\nend\n";

const OPTIONAL_STDLIB_UUID_SOURCE: &str =
    "defmodule Uuid do\n  def v4() do\n    host_call(:uuid_v4)\n  end\nend\n";

const OPTIONAL_STDLIB_YAML_SOURCE: &str =
    "defmodule Yaml do\n  def encode(value) do\n    host_call(:yaml_encode, value)\n  end\n\n  def decode(string) do\n    host_call(:yaml_decode, string)\n  end\nend\n";

const OPTIONAL_STDLIB_URL_SOURCE: &str =
    "defmodule Url do\n  def encode(string) do\n    host_call(:url_encode, string)\n  end\n\n  def decode(string) do\n    host_call(:url_decode, string)\n  end\n\n  def encode_query(params) do\n    host_call(:url_encode_query, params)\n  end\n\n  def decode_query(string) do\n    host_call(:url_decode_query, string)\n  end\nend\n";

const OPTIONAL_STDLIB_ENV_SOURCE: &str =
    "defmodule Env do\n  def get(key) do\n    System.env(key)\n  end\n\n  def get(key, default) do\n    case System.env(key) do\n      nil -> default\n      value -> value\n    end\n  end\n\n  def fetch!(key) do\n    case System.env(key) do\n      nil -> raise \"environment variable #{key} is not set\"\n      value -> value\n    end\n  end\n\n  def set(key, value) do\n    host_call(:env_set, key, value)\n  end\n\n  def delete(key) do\n    host_call(:env_delete, key)\n  end\n\n  def all() do\n    host_call(:env_all)\n  end\n\n  def has_key(key) do\n    host_call(:env_has_key, key)\n  end\nend\n";
