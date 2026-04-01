// Shared embedded stdlib catalog for runtime lazy-loading and generated docs.

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
    ("Tuple", OPTIONAL_STDLIB_TUPLE_SOURCE),
    ("Assert", OPTIONAL_STDLIB_ASSERT_SOURCE),
    ("Json", OPTIONAL_STDLIB_JSON_SOURCE),
];

pub(crate) fn stdlib_module_names() -> impl Iterator<Item = &'static str> {
    STDLIB_SOURCES.iter().map(|(module_name, _)| *module_name)
}

pub(super) const OPTIONAL_STDLIB_IO_SOURCE: &str = r#"defmodule IO do
  ## Prints a value followed by a newline to stdout.
  ##
  ## Parameters:
  ##   value: any — the value to print
  ##
  ## Returns: :ok
  def puts(value) do
    host_call(:io_puts, value)
  end

  ## Inspects a value and prints its internal representation to stderr.
  ##
  ## Parameters:
  ##   value: any — the value to inspect
  ##
  ## Returns: the original value (pass-through)
  def inspect(value) do
    host_call(:io_inspect, value)
  end

  ## Reads a line from stdin, showing prompt.
  ##
  ## Parameters:
  ##   prompt: string — the prompt to display
  ##
  ## Returns: string
  def gets(prompt) do
    host_call(:io_gets, prompt)
  end

  ## Renders a markdown string to formatted terminal output.
  ##
  ## Parameters:
  ##   markdown: string — the markdown text to render
  ##
  ## Returns: :ok
  def render_markdown(markdown) do
    host_call(:io_render_markdown, markdown)
  end

  ## Wraps value in ANSI red escape codes.
  ##
  ## Parameters:
  ##   value: any — the value to colorize
  ##
  ## Returns: string
  def ansi_red(value) do
    host_call(:io_ansi_red, value)
  end

  ## Wraps value in ANSI green escape codes.
  ##
  ## Parameters:
  ##   value: any — the value to colorize
  ##
  ## Returns: string
  def ansi_green(value) do
    host_call(:io_ansi_green, value)
  end

  ## Wraps value in ANSI yellow escape codes.
  ##
  ## Parameters:
  ##   value: any — the value to colorize
  ##
  ## Returns: string
  def ansi_yellow(value) do
    host_call(:io_ansi_yellow, value)
  end

  ## Wraps value in ANSI blue escape codes.
  ##
  ## Parameters:
  ##   value: any — the value to colorize
  ##
  ## Returns: string
  def ansi_blue(value) do
    host_call(:io_ansi_blue, value)
  end

  ## Returns the ANSI reset escape sequence.
  ##
  ## Returns: string
  def ansi_reset() do
    host_call(:io_ansi_reset)
  end
end
"#;
pub(super) const OPTIONAL_STDLIB_LIST_SOURCE: &str =
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

pub(super) const OPTIONAL_STDLIB_MAP_SOURCE: &str =
    "defmodule Map do\n  def keys(map) do\n    host_call(:map_keys, map)\n  end\n\n  def values(map) do\n    host_call(:map_values, map)\n  end\n\n  def merge(left, right, fun \\\\ nil) do\n    case fun do\n      nil -> host_call(:map_merge, left, right)\n      _ -> merge_with_resolver(left, right, fun)\n    end\n  end\n\n  def drop(map, keys) do\n    host_call(:map_drop, map, keys)\n  end\n\n  def take(map, keys) do\n    host_call(:map_take, map, keys)\n  end\n\n  def get(map, key, default \\\\ nil) do\n    host_call(:map_get, map, key, default)\n  end\n\n  def put(map, key, value) do\n    host_call(:map_put, map, key, value)\n  end\n\n  def delete(map, key) do\n    host_call(:map_delete, map, key)\n  end\n\n  def has_key?(map, key) do\n    host_call(:map_has_key, map, key)\n  end\n\n  def to_list(map) do\n    for item <- map do\n      item\n    end\n  end\n\n  def new() do\n    %{}\n  end\n\n  def from_list(list) do\n    new_from_list(list, %{})\n  end\n\n  def update(map, key, default, fun) do\n    case host_call(:map_has_key, map, key) do\n      true -> host_call(:map_put, map, key, fun.(host_call(:map_get, map, key, default)))\n      _ -> host_call(:map_put, map, key, default)\n    end\n  end\n\n  def put_new(map, key, value) do\n    case host_call(:map_has_key, map, key) do\n      true -> map\n      _ -> host_call(:map_put, map, key, value)\n    end\n  end\n\n  def filter(map, fun) do\n    map_filter_list(for item <- map do item end, fun, %{})\n  end\n\n  def reject(map, fun) do\n    map_reject_list(for item <- map do item end, fun, %{})\n  end\n\n  def pop(map, key, default \\\\ nil) do\n    case host_call(:map_has_key, map, key) do\n      true -> {host_call(:map_get, map, key, default), host_call(:map_delete, map, key)}\n      _ -> {default, map}\n    end\n  end\n\n  defp map_filter_list([], _fun, acc) do\n    acc\n  end\n\n  defp map_filter_list([head | tail], fun, acc) do\n    case fun.(head) do\n      true -> do\n        {k, v} = head\n        map_filter_list(tail, fun, host_call(:map_put, acc, k, v))\n      end\n      _ -> map_filter_list(tail, fun, acc)\n    end\n  end\n\n  defp map_reject_list([], _fun, acc) do\n    acc\n  end\n\n  defp map_reject_list([head | tail], fun, acc) do\n    case fun.(head) do\n      true -> map_reject_list(tail, fun, acc)\n      _ -> do\n        {k, v} = head\n        map_reject_list(tail, fun, host_call(:map_put, acc, k, v))\n      end\n    end\n  end\n\n  defp merge_with_resolver(left, right, fun) do\n    merge_entries(left, for item <- right do item end, fun)\n  end\n\n  defp merge_entries(acc, [], _fun) do\n    acc\n  end\n\n  defp merge_entries(acc, [{key, value} | tail], fun) do\n    case host_call(:map_has_key, acc, key) do\n      true -> do\n        resolved = fun.(key, host_call(:map_get, acc, key, nil), value)\n        merge_entries(host_call(:map_put, acc, key, resolved), tail, fun)\n      end\n      _ -> merge_entries(host_call(:map_put, acc, key, value), tail, fun)\n    end\n  end\n\n  defp new_from_list([], acc) do\n    acc\n  end\n\n  defp new_from_list([{key, value} | tail], acc) do\n    new_from_list(tail, host_call(:map_put, acc, key, value))\n  end\nend\n";

pub(super) const OPTIONAL_STDLIB_ENUM_SOURCE: &str = r#"defmodule Enum do
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

  defp last_list([only]) do
    only
  end

  defp last_list([_head | tail]) do
    last_list(tail)
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

pub(super) const OPTIONAL_STDLIB_STRING_SOURCE: &str = r#"defmodule String do
  ## Splits a string by delimiter.
  ##
  ## Parameters:
  ##   str: string — the string to split
  ##   delimiter: string — the delimiter to split on (default: " ")
  ##
  ## Returns: list of strings
  def split(str, delimiter \\ " ") do
    host_call(:str_split, str, delimiter)
  end

  ## Returns the grapheme clusters of a string.
  ##
  ## Parameters:
  ##   str: string — the input string
  ##
  ## Returns: list of strings (one per grapheme cluster)
  def graphemes(str) do
    host_call(:str_graphemes, str)
  end

  ## Replaces all occurrences of pattern in str with replacement.
  ##
  ## Parameters:
  ##   str: string — the input string
  ##   pattern: string — the pattern to match
  ##   replacement: string — the replacement text
  ##
  ## Returns: string
  def replace(str, pattern, replacement) do
    host_call(:str_replace, str, pattern, replacement)
  end

  ## Trims leading and trailing whitespace.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: string
  def trim(str) do
    host_call(:str_trim, str)
  end

  ## Trims leading whitespace.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: string
  def trim_leading(str) do
    host_call(:str_trim_leading, str)
  end

  ## Trims trailing whitespace.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: string
  def trim_trailing(str) do
    host_call(:str_trim_trailing, str)
  end

  ## Checks if str starts with the given prefix.
  ##
  ## Parameters:
  ##   str: string
  ##   prefix: string
  ##
  ## Returns: boolean
  def starts_with(str, prefix) do
    host_call(:str_starts_with, str, prefix)
  end

  ## Checks if str ends with the given suffix.
  ##
  ## Parameters:
  ##   str: string
  ##   suffix: string
  ##
  ## Returns: boolean
  def ends_with(str, suffix) do
    host_call(:str_ends_with, str, suffix)
  end

  ## Checks if str contains the given substring.
  ##
  ## Parameters:
  ##   str: string
  ##   substr: string
  ##
  ## Returns: boolean
  def contains(str, substr) do
    host_call(:str_contains, str, substr)
  end

  ## Converts a string to uppercase.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: string
  def upcase(str) do
    host_call(:str_upcase, str)
  end

  ## Converts a string to lowercase.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: string
  def downcase(str) do
    host_call(:str_downcase, str)
  end

  ## Returns the number of grapheme clusters in the string.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: integer
  def length(str) do
    host_call(:str_length, str)
  end

  ## Converts a string to a list of codepoints.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: list of integers
  def to_charlist(str) do
    host_call(:str_to_charlist, str)
  end

  ## Returns the grapheme at the given index.
  ##
  ## Parameters:
  ##   str: string
  ##   index: integer — zero-based index
  ##
  ## Returns: string (single grapheme) or nil
  def at(str, index) do
    host_call(:str_at, str, index)
  end

  ## Returns a substring starting at start with the given length.
  ##
  ## Parameters:
  ##   str: string
  ##   start: integer — zero-based start index
  ##   len: integer — number of graphemes
  ##
  ## Returns: string
  def slice(str, start, len) do
    host_call(:str_slice, str, start, len)
  end

  ## Parses a string as an integer.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: integer
  def to_integer(str) do
    host_call(:str_to_integer, str)
  end

  ## Parses a string as a float.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: float
  def to_float(str) do
    host_call(:str_to_float, str)
  end

  ## Pads the string on the left to the given count with padding.
  ##
  ## Parameters:
  ##   str: string
  ##   count: integer — desired total length
  ##   padding: string — the padding character(s)
  ##
  ## Returns: string
  def pad_leading(str, count, padding) do
    host_call(:str_pad_leading, str, count, padding)
  end

  ## Pads the string on the right to the given count with padding.
  ##
  ## Parameters:
  ##   str: string
  ##   count: integer — desired total length
  ##   padding: string — the padding character(s)
  ##
  ## Returns: string
  def pad_trailing(str, count, padding) do
    host_call(:str_pad_trailing, str, count, padding)
  end

  ## Reverses the grapheme order of a string.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: string
  def reverse(str) do
    host_call(:str_reverse, str)
  end

  ## Duplicates a string the given number of times.
  ##
  ## Parameters:
  ##   str: string
  ##   count: integer — number of repetitions
  ##
  ## Returns: string
  def duplicate(_str, count) when count <= 0 do
    ""
  end

  def duplicate(str, count) do
    duplicate_acc(str, count, "")
  end

  ## Converts a string to an atom.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: atom
  def to_atom(str) do
    host_call(:str_to_atom, str)
  end

  ## Capitalizes the first grapheme and lowercases the rest.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: string
  def capitalize("") do
    ""
  end

  def capitalize(str) do
    first = host_call(:str_upcase, host_call(:str_at, str, 0))
    len = host_call(:str_length, str)
    rest = host_call(:str_downcase, host_call(:str_slice, str, 1, len))
    first <> rest
  end

  defp duplicate_acc(_str, 0, acc) do
    acc
  end

  defp duplicate_acc(str, count, acc) do
    duplicate_acc(str, count - 1, acc <> str)
  end
end
"#;

pub(super) const OPTIONAL_STDLIB_INTEGER_SOURCE: &str = r#"defmodule Integer do
  ## Converts an integer to its string representation.
  ##
  ## Parameters:
  ##   n: integer
  ##
  ## Returns: string
  def to_string(n) do
    host_call(:integer_to_string, n)
  end

  ## Parses a string as an integer.
  ##
  ## Parameters:
  ##   str: string
  ##
  ## Returns: integer
  def parse(str) do
    host_call(:integer_parse, str)
  end
end
"#;

pub(super) const OPTIONAL_STDLIB_FLOAT_SOURCE: &str = r#"defmodule Float do
  ## Converts a float to its string representation.
  ##
  ## Parameters:
  ##   n: float
  ##
  ## Returns: string
  def to_string(n) do
    host_call(:float_to_string, n)
  end

  ## Rounds a float to the given precision.
  ##
  ## Parameters:
  ##   n: float
  ##   precision: integer — number of decimal places
  ##
  ## Returns: float
  def round(n, precision) do
    host_call(:float_round, n, precision)
  end

  ## Returns the smallest integer greater than or equal to n.
  ##
  ## Parameters:
  ##   n: float
  ##
  ## Returns: integer
  def ceil(n) do
    host_call(:float_ceil, n)
  end

  ## Returns the largest integer less than or equal to n.
  ##
  ## Parameters:
  ##   n: float
  ##
  ## Returns: integer
  def floor(n) do
    host_call(:float_floor, n)
  end
end
"#;

pub(super) const OPTIONAL_STDLIB_PATH_SOURCE: &str = r#"defmodule Path do
  ## Joins two path segments.
  ##
  ## Parameters:
  ##   a: string — the base path
  ##   b: string — the path to append
  ##
  ## Returns: string
  def join(a, b) do
    host_call(:path_join, a, b)
  end

  ## Returns the directory portion of a path.
  ##
  ## Parameters:
  ##   path: string
  ##
  ## Returns: string
  def dirname(path) do
    host_call(:path_dirname, path)
  end

  ## Returns the filename portion of a path.
  ##
  ## Parameters:
  ##   path: string
  ##
  ## Returns: string
  def basename(path) do
    host_call(:path_basename, path)
  end

  ## Returns the file extension of a path.
  ##
  ## Parameters:
  ##   path: string
  ##
  ## Returns: string
  def extname(path) do
    host_call(:path_extname, path)
  end

  ## Expands a path to its absolute form.
  ##
  ## Parameters:
  ##   path: string
  ##
  ## Returns: string
  def expand(path) do
    host_call(:path_expand, path)
  end

  ## Returns the relative path from base to path.
  ##
  ## Parameters:
  ##   path: string — the target path
  ##   base: string — the base path
  ##
  ## Returns: string
  def relative_to(path, base) do
    host_call(:path_relative_to, path, base)
  end
end
"#;

pub(super) const OPTIONAL_STDLIB_SYSTEM_SOURCE: &str =
    "defmodule System do\n  def run(command, opts \\\\ %{}) do\n    host_call(:sys_run, command, opts)\n  end\n\n  def sleep_ms(delay_ms) do\n    host_call(:sys_sleep_ms, delay_ms)\n  end\n\n  def retry_plan(status_code, attempt, max_attempts, base_delay_ms, max_delay_ms, jitter_ms, retry_after) do\n    host_call(:sys_retry_plan, status_code, attempt, max_attempts, base_delay_ms, max_delay_ms, jitter_ms, retry_after)\n  end\n\n  def log(level, event, fields) do\n    host_call(:sys_log, level, event, fields)\n  end\n\n  def path_exists(path) do\n    host_call(:sys_path_exists, path)\n  end\n\n  def list_dir(path) do\n    host_call(:sys_list_dir, path)\n  end\n\n  def is_dir(path) do\n    host_call(:sys_is_dir, path)\n  end\n\n  def list_files_recursive(path) do\n    host_call(:sys_list_files_recursive, path)\n  end\n\n  def ensure_dir(path) do\n    host_call(:sys_ensure_dir, path)\n  end\n\n  def remove_tree(path) do\n    host_call(:sys_remove_tree, path)\n  end\n\n  def write_text(path, content) do\n    host_call(:sys_write_text, path, content)\n  end\n\n  def append_text(path, content) do\n    host_call(:sys_append_text, path, content)\n  end\n\n  def write_text_atomic(path, content) do\n    host_call(:sys_write_text_atomic, path, content)\n  end\n\n  def lock_acquire(path) do\n    host_call(:sys_lock_acquire, path)\n  end\n\n  def lock_release(path) do\n    host_call(:sys_lock_release, path)\n  end\n\n  def read_text(path) do\n    host_call(:sys_read_text, path)\n  end\n\n  def read_stdin() do\n    host_call(:sys_read_stdin)\n  end\n\n  def http_request(method, url, headers, body, opts) do\n    host_call(:sys_http_request, method, url, headers, body, opts)\n  end\n\n  def env(name) do\n    host_call(:sys_env, name)\n  end\n\n  def which(name) do\n    host_call(:sys_which, name)\n  end\n\n  def cwd() do\n    host_call(:sys_cwd)\n  end\n\n  def argv() do\n    host_call(:sys_argv)\n  end\n\n  def random_token(bytes) do\n    host_call(:sys_random_token, bytes)\n  end\n\n  def hmac_sha256_hex(secret, message) do\n    host_call(:sys_hmac_sha256_hex, secret, message)\n  end\n\n  def constant_time_eq(left, right) do\n    host_call(:sys_constant_time_eq, left, right)\n  end\n\n  def discord_ed25519_verify(public_key_hex, signature_hex, timestamp, body) do\n    host_call(:sys_discord_ed25519_verify, public_key_hex, signature_hex, timestamp, body)\n  end\n\n  def http_listen(host, port) do\n    host_call(:sys_http_listen, host, port)\n  end\n\n  def http_accept(listener_id, timeout_ms) do\n    host_call(:sys_http_accept, listener_id, timeout_ms)\n  end\n\n  def http_read_request(connection_id) do\n    host_call(:sys_http_read_request, connection_id)\n  end\n\n  def http_write_response(connection_id, status, headers, body) do\n    host_call(:sys_http_write_response, connection_id, status, headers, body)\n  end\nend\n";

pub(super) const OPTIONAL_STDLIB_TUPLE_SOURCE: &str = r#"defmodule Tuple do
  ## Converts a tuple to a list.
  ##
  ## Parameters:
  ##   tuple: tuple
  ##
  ## Returns: list
  def to_list(tuple) do
    host_call(:tuple_to_list, tuple)
  end
end
"#;

pub(super) const OPTIONAL_STDLIB_ASSERT_SOURCE: &str =
    "defmodule Assert do\n  def assert(value, message \\\\ nil) do\n    host_call(:assert, value, message)\n  end\n\n  def refute(value, message \\\\ nil) do\n    host_call(:refute, value, message)\n  end\n\n  def assert_equal(left, right, message \\\\ nil) do\n    host_call(:assert_equal, left, right, message)\n  end\n\n  def assert_not_equal(left, right, message \\\\ nil) do\n    host_call(:assert_not_equal, left, right, message)\n  end\n\n  def assert_contains(container, element, message \\\\ nil) do\n    host_call(:assert_contains, container, element, message)\n  end\n\n  def assert_in_delta(left, right, delta, message \\\\ nil) do\n    host_call(:assert_in_delta, left, right, delta, message)\n  end\n\n  def skip(reason \\\\ nil) do\n    host_call(:skip, reason)\n  end\n\n  def assert_match(expected, actual, message \\\\ nil) do\n    host_call(:assert_match, expected, actual, message)\n  end\n\n  def assert_raises(fun, expected \\\\ nil) do\n    check_raises(do_try_raises(fun), expected)\n  end\n\n  defp do_try_raises(fun) do\n    try do\n      fun.()\n      {:no_raise, :ok}\n    rescue\n      e -> {:raised, to_string(e)}\n    end\n  end\n\n  defp check_raises({:raised, _msg}, nil) do\n    :ok\n  end\n\n  defp check_raises({:raised, msg}, expected) do\n    host_call(:assert_raises_check, msg, expected)\n  end\n\n  defp check_raises(_, _expected) do\n    err({:assertion_failed, {:assert_raises, \"expected function to raise, but it returned normally\"}})\n  end\nend\n";

pub(super) const OPTIONAL_STDLIB_JSON_SOURCE: &str = r#"defmodule Json do
  ## Decodes a JSON string into a Tonic value.
  ##
  ## Parameters:
  ##   text: string — the JSON text to decode
  ##
  ## Returns: the decoded value (map, list, string, integer, float, boolean, or nil)
  def decode(text) do
    host_call(:json_decode, text)
  end

  ## Encodes a Tonic value as a JSON string.
  ##
  ## Parameters:
  ##   value: any — the value to encode
  ##
  ## Returns: string
  def encode(value) do
    host_call(:json_encode, value)
  end

  ## Encodes a Tonic value as a pretty-printed JSON string.
  ##
  ## Parameters:
  ##   value: any — the value to encode
  ##
  ## Returns: string
  def encode_pretty(value) do
    host_call(:json_encode_pretty, value)
  end

  ## Extracts a top-level field from a JSON object string.
  ##
  ## Parameters:
  ##   json_text: string — the JSON text
  ##   key: string — the field name to extract
  ##
  ## Returns: the extracted value, or nil if not found
  def extract_field(json_text, key) do
    host_call(:json_extract_field, json_text, key)
  end

  ## Extracts a nested field using dot notation path.
  ##
  ## Parameters:
  ##   json_text: string — the JSON text
  ##   path: string — dot-separated path (e.g., "assistantMessageEvent.delta")
  ##
  ## Returns: the extracted value, or nil if any segment is missing
  def extract_path(json_text, path) do
    host_call(:json_extract_path, json_text, path)
  end

  ## Parses a JSON object string into a map.
  ##
  ## Parameters:
  ##   text: string — the JSON text (must be an object)
  ##
  ## Returns: {:ok, map} | {:error, reason}
  def parse_object(text) do
    parse_typed(text, :map)
  end

  ## Parses a JSON array string into a list.
  ##
  ## Parameters:
  ##   text: string — the JSON text (must be an array)
  ##
  ## Returns: {:ok, list} | {:error, reason}
  def parse_array(text) do
    parse_typed(text, :list)
  end

  ## Processes a list of JSON lines with a stateful handler.
  ##
  ## Parameters:
  ##   lines: list of strings — each line is a JSON value
  ##   initial_state: any — the starting accumulator
  ##   handler: fn(parsed_value, state) -> state — called for each successfully decoded line
  ##
  ## Returns: the final state
  def stream_parse(lines, initial_state, handler) do
    stream_fold(lines, initial_state, handler)
  end

  defp parse_typed(text, expected_type) do
    try do
      decoded = host_call(:json_decode, text)
      case expected_type do
        :map ->
          case is_map(decoded) do
            true -> {:ok, decoded}
            _ -> {:error, "expected JSON object, got different type"}
          end
        _ ->
          case is_list(decoded) do
            true -> {:ok, decoded}
            _ -> {:error, "expected JSON array, got different type"}
          end
      end
    rescue
      e -> {:error, to_string(e)}
    end
  end

  defp stream_fold([], state, _handler) do
    state
  end

  defp stream_fold([line | rest], state, handler) do
    new_state = try do
      decoded = host_call(:json_decode, line)
      handler.(decoded, state)
    rescue
      _e -> state
    end
    stream_fold(rest, new_state, handler)
  end
end
"#;
