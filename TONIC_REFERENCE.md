# Tonic Language Quick Reference

Tonic is an Elixir-inspired language. This reference covers the syntax and stdlib
that actually work. If something is not listed here, assume it is not supported.

## Module and Function Basics

All code lives in modules. The entry point is `Demo.run/0`.

```elixir
defmodule Demo do
  def run() do
    IO.puts("hello")
  end
end
```

Multiple modules are supported. Define helper modules before the `Demo` module.

```elixir
defmodule Math do
  def add(a, b) do
    a + b
  end
end

defmodule Demo do
  def run() do
    IO.puts("#{Math.add(1, 2)}")
  end
end
```

Private functions use `defp`:

```elixir
defmodule Demo do
  def run() do
    IO.puts("#{helper()}")
  end

  defp helper() do
    42
  end
end
```

## Data Types

- **Integers**: `42`, `0`, `-1`, `1_000`
- **Floats**: `3.14`, `-0.5`
- **Strings**: `"hello"` (double-quoted only)
- **Atoms**: `:ok`, `:error`, `true`, `false`, `nil`
- **Lists**: `[1, 2, 3]`
- **Tuples**: `{:ok, 42}`, `{1, 2, 3}`
- **Maps**: `%{name: "Alice", age: 30}` or `%{"key" => "value"}`
- **Ranges**: `1..10`, `1..100//2` (stepped)

## String Interpolation

Use `#{}` inside strings. Works with integers, floats, atoms, strings, booleans.

```elixir
name = "world"
IO.puts("hello #{name}")
IO.puts("count: #{42}")
```

String interpolation works with all types, including lists, maps, and tuples:

```elixir
IO.puts("list: #{[1, 2, 3]}")       # => list: [1, 2, 3]
IO.puts("map: #{%{a: 1}}")          # => map: %{a => 1}
IO.puts("tuple: #{{:ok, 42}}")      # => tuple: {:ok, 42}
```

## Multiline Strings

Use raw heredocs when you want exact content preservation:

```elixir
raw = """line one
  line two
"""
```

Use `~t"""..."""` for source-indented text blocks. Tonic trims one optional newline after the opener, trims one optional newline before the closer, and removes the minimum common indentation across non-blank output lines.

```elixir
help = ~t"""
  Usage:
    tonic run <path>

  Prints the dedented text exactly as shown here.
"""
```

The example above produces `"Usage:\n  tonic run <path>\n\nPrints the dedented text exactly as shown here."`.

Text blocks also support the normal `#{}` interpolation pipeline:

```elixir
name = "world"
message = ~t"""
  hello #{name}
  #{String.upcase("ok")}
"""
```

That example produces `"hello world\nOK"`.

When an interpolation spans multiple source lines, Tonic computes text-block dedent from the rendered text layout: blank lines stay blank, a line containing only `#{...}` still participates in dedent, and the indentation inside the expression source itself does not affect the surrounding text indentation.

## String Concatenation

Use `<>` to join strings. Both sides must be strings.

```elixir
greeting = "hello" <> " " <> "world"
```

**Important**: When using `<>` with stdlib function results, bind the result to a
variable first. Direct inline calls may trigger type checker errors.

```elixir
# Good
trimmed = String.trim("  hi  ")
IO.puts("[" <> trimmed <> "]")

# May fail with type error
IO.puts("[" <> String.trim("  hi  ") <> "]")
```

## Variables and Assignment

Variables are bound with `=`. Rebinding is allowed.

```elixir
x = 1
x = x + 1
IO.puts("#{x}")
```

## Pattern Matching

Pattern matching works in function heads, `case`, and `=`.

```elixir
# Function head patterns
defp process([]) do
  :done
end

defp process([head | tail]) do
  IO.puts("#{head}")
  process(tail)
end
```

### Case Expressions

Every `case` must include a wildcard (`_`) branch.

```elixir
case value do
  :ok -> "success"
  :error -> "failure"
  _ -> "unknown"
end
```

### Tuple Patterns

```elixir
case result do
  {:ok, value} -> IO.puts("got #{value}")
  {:error, msg} -> IO.puts("error: #{msg}")
  _ -> IO.puts("unexpected")
end
```

### Map Patterns

```elixir
case data do
  %{type: :leaf} -> "leaf"
  %{type: :node} -> "node"
  _ -> "other"
end
```

### Pin Operator

Use `^` to match against an existing variable's value:

```elixir
target = "admin"
case user.role do
  ^target -> "is admin"
  _ -> "not admin"
end
```

## Guards

Guards refine pattern matching in function heads and case branches.

```elixir
def classify(n) when n > 0 do
  "positive"
end

def classify(n) when n < 0 do
  "negative"
end

def classify(_n) do
  "zero"
end
```

Available guard functions: `is_integer/1`, `is_float/1`, `is_number/1`, `is_boolean/1`, `is_atom/1`,
`is_binary/1`, `is_list/1`, `is_tuple/1`, `is_map/1`, `is_nil/1`.

Guard operators: `==`, `!=`, `<`, `<=`, `>`, `>=`, `and`, `or`, `not`.

### Type-Checking Functions

The `is_*` guard functions also work as **regular expressions** outside guards — in `if` conditions, case expressions, function bodies, and pipe chains. They return `true` or `false`:

```elixir
if is_integer(x) do
  "it's an integer"
else
  "something else"
end

# Use in boolean context
result = is_list(value) and length(value) > 0
```

## Operators

- **Arithmetic**: `+`, `-`, `*`, `/` (work with Int, Float, or mixed — mixed promotes to Float), `div`, `rem` (Int only)
- **Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`, `===`, `!==` (comparisons work across Int and Float)
- **Boolean**: `and`, `or`, `not`, `&&`, `||`, `!`
- **List**: `++` (concat), `--` (subtract), `in` (membership)
- **String**: `<>` (concat)
- **Range**: `..`, `..//` (stepped)

## Kernel Builtins

These are bare (unqualified) functions available everywhere, like in Elixir's `Kernel`:

- `abs(number)` — absolute value (works for Int and Float)
- `length(list)` — number of elements in a list
- `hd(list)` — head (first element) of a list; raises on empty list
- `tl(list)` — tail (all elements after first) of a list; raises on empty list
- `elem(tuple, index)` — element at `index` in a tuple (0-based)
- `tuple_size(tuple)` — number of elements in a tuple (always 2 in Tonic)
- `to_string(value)` — convert any value to a string (Int, Float, Bool, Atom, nil→"", List/Map/Tuple use render format)
- `max(a, b)` — returns the larger of two numbers (Int or Float; mixed promotes to Float)
- `min(a, b)` — returns the smaller of two numbers (Int or Float; mixed promotes to Float)
- `round(number)` — rounds a Float to the nearest Int (Int passes through unchanged)
- `trunc(number)` — truncates a Float toward zero to Int (Int passes through unchanged)
- `map_size(map)` — number of keys in a map
- `put_elem(tuple, index, value)` — returns new tuple with element at `index` replaced
- `inspect(value)` — converts any value to its string representation (like Elixir's `inspect/1`)
- `div(a, b)` — integer division
- `rem(a, b)` — integer remainder

```elixir
length([1, 2, 3])       # => 3
hd([10, 20, 30])        # => 10
tl([10, 20, 30])        # => [20, 30]
abs(-42)                 # => 42
elem({:ok, "done"}, 1)  # => "done"
tuple_size({:a, :b})    # => 2
to_string(42)            # => "42"
to_string(:hello)        # => "hello"
inspect([1, 2, 3])      # => "[1, 2, 3]"
inspect({:ok, 42})      # => "{:ok, 42}"
max(3, 7)               # => 7
min(3, 7)               # => 3
round(3.7)              # => 4
trunc(3.7)              # => 3
map_size(%{a: 1, b: 2}) # => 2
put_elem({1, 2}, 0, 99) # => {99, 2}
div(10, 3)              # => 3
rem(10, 3)              # => 1
```

## Sequential IO (Important!)

Tonic does NOT have implicit statement sequencing in all contexts. To execute
multiple IO operations in order, chain them with `case _ ->`:

```elixir
def run() do
  case IO.puts("line 1") do
    _ -> case IO.puts("line 2") do
      _ -> IO.puts("line 3")
    end
  end
end
```

Alternatively, break sequential operations into separate helper functions:

```elixir
def run() do
  case show_header() do
    _ -> show_body()
  end
end
```

Within function bodies (top-level `do...end` blocks), sequential variable bindings
work fine:

```elixir
defp compute() do
  x = 1
  y = x + 2
  z = y * 3
  z
end
```

## Pipe Operator

The pipe operator `|>` passes the result as the first argument:

```elixir
[5, 3, 1, 4, 2]
|> Enum.sort()
|> Enum.reverse()
|> Enum.take(3)
```

## Anonymous Functions (Closures)

Use the capture syntax `&(...)` with `&1`, `&2` for parameters:

```elixir
double = &(&1 * 2)
double.(5)  # => 10

add = &(&1 + &2)
add.(3, 4)  # => 7
```

The `fn -> end` syntax is also supported:

```elixir
double = fn x -> x * 2 end
double.(5)
```

Invoke closures with `fun.(args)` (dot-call syntax).

## For Comprehensions

Map over collections:

```elixir
doubled = for x <- [1, 2, 3] do
  x * 2
end
```

Multi-generator:

```elixir
pairs = for x <- [1, 2], y <- [:a, :b] do
  {x, y}
end
```

Filter with `when` guards:

```elixir
evens = for x when rem(x, 2) == 0 <- [1, 2, 3, 4, 5, 6] do
  x
end
# => [2, 4, 6]

big_pairs = for {k, v} when v > 10 <- pairs do
  {k, v}
end
```

**Note**: Use `when` guard syntax (not Elixir's comma-filter syntax `for x <- list, x > 3`).

## Control Flow

### if/unless

```elixir
if x > 0 do
  "positive"
else
  "non-positive"
end
```

### cond

```elixir
cond do
  x > 10 -> "big"
  x > 0 -> "small"
  true -> "zero or negative"
end
```

### with

```elixir
with {:ok, a} <- get_a(),
     {:ok, b} <- get_b(a) do
  {:ok, a + b}
else
  {:error, reason} -> {:error, reason}
  _ -> {:error, "unknown"}
end
```

### try/rescue

```elixir
try do
  risky_operation()
rescue
  error -> IO.puts("caught error")
end
```

## Map Access

Access map fields with dot syntax or `Map.get`:

```elixir
user = %{name: "Alice", age: 30}
user.name                        # => "Alice"
Map.get(user, :name)             # => "Alice" (defaults to nil)
Map.get(user, :name, "default")  # => "Alice" (explicit default)
```

Update maps with `Map.put/3` or map update syntax:

```elixir
updated = Map.put(user, :age, 31)
updated = %{user | age: 31}
```

## Recursion

Tonic is designed for recursive patterns. Use multi-clause functions:

```elixir
defp sum([]) do
  0
end

defp sum([head | tail]) do
  head + sum(tail)
end
```

Build lists with `[new_item] ++ existing_list`:

```elixir
defp build_list(0, acc) do
  acc
end

defp build_list(n, acc) do
  build_list(n - 1, [n] ++ acc)
end
```

## Standard Library

### IO

```
IO.puts(value)           # Print with newline (accepts any type)
IO.inspect(value)        # Debug print (returns value)
IO.gets(prompt)          # Read line from stdin
```

`IO.puts` accepts any type — integers, floats, booleans, atoms, lists, maps, tuples, and nil are auto-converted to string. `IO.puts(nil)` prints an empty line.

### String

```
String.split(str)                     # Split on whitespace (default)
String.split(str, delim)              # Split into list
String.replace(str, pattern, replacement)  # Replace all occurrences
String.trim(str)                      # Trim whitespace
String.trim_leading(str)              # Trim leading whitespace
String.trim_trailing(str)             # Trim trailing whitespace
String.upcase(str)                    # Uppercase
String.downcase(str)                  # Lowercase
String.reverse(str)                   # Reverse
String.length(str)                    # Character count
String.at(str, index)                 # Character at position
String.slice(str, start, length)      # Substring
String.contains(str, substr)          # Check containment
String.starts_with(str, prefix)       # Check prefix
String.ends_with(str, suffix)         # Check suffix
String.to_integer(str)                # Parse integer
String.to_float(str)                  # Parse float
String.pad_leading(str, count, pad)   # Left-pad
String.pad_trailing(str, count, pad)  # Right-pad
String.duplicate(str, count)          # Repeat string N times
String.capitalize(str)               # Capitalize first letter, lowercase rest
String.to_atom(str)                  # Convert string to atom
String.graphemes(str)                # Split into list of single characters
```

**Note**: String function names do NOT use `?` suffix (e.g. `String.contains`, not
`String.contains?`).

### Integer

```
Integer.to_string(42)     # "42" — convert integer to string
Integer.parse("123abc")   # {123, "abc"} — parse leading integer
Integer.parse("abc")      # :error — no leading digits
```

### Float

```
Float.to_string(3.14)     # "3.14" — convert float to string
Float.round(3.14159, 2)   # 3.14 — round to N decimal places
Float.ceil(3.2)           # 4.0 — round up
Float.floor(3.8)          # 3.0 — round down
```

### List

```
List.first(list)          # First element (nil if empty)
List.first(list, default) # First element (default if empty)
List.last(list)           # Last element (nil if empty)
List.last(list, default)  # Last element (default if empty)
List.flatten(list)        # Flatten nested lists
List.zip(left, right)     # Zip two lists into tuples
List.unzip(pairs)         # Unzip tuples into two lists
List.wrap(value)          # Wrap non-list in list, pass list through
List.delete(list, value)  # Delete first occurrence of value
List.insert_at(list, index, value) # Insert element at index
List.delete_at(list, index)   # Remove element at index
List.update_at(list, index, fun) # Apply function to element at index
List.duplicate(value, count)  # Create list with N copies of value
List.starts_with(list, prefix) # Check if list starts with prefix
List.to_tuple(list)       # Convert 2-element list to tuple
```

### Tuple

```
Tuple.to_list(tuple)      # Convert tuple to list: {a, b} => [a, b]
```

### Map

```
Map.get(map, key)            # Get value (nil if missing)
Map.get(map, key, default)   # Get value with default
Map.put(map, key, value)     # Set key-value
Map.delete(map, key)         # Remove key
Map.merge(left, right)       # Merge maps (right wins)
Map.merge(left, right, fun)  # Merge with conflict resolver fn(key, v1, v2)
Map.keys(map)                # List of keys
Map.values(map)              # List of values
Map.has_key(map, key)        # Check if key exists
Map.drop(map, keys_list)     # Remove multiple keys
Map.take(map, keys_list)     # Keep only listed keys
Map.update(map, key, default, fun) # Update value with function (default if key missing)
Map.put_new(map, key, value) # Set key only if not already present
Map.to_list(map)             # Convert to [{key, value}, ...] tuples
Map.new()                    # Create empty map (%{})
Map.from_list(list)           # Create map from [{key, value}, ...] tuples
Map.filter(map, fun)         # Keep entries where fun.({key, value}) is truthy
Map.reject(map, fun)         # Remove entries where fun.({key, value}) is truthy
Map.pop(map, key)            # Remove key, return {value, rest_map} (nil if missing)
Map.pop(map, key, default)   # Remove key, return {value, rest_map} (default if missing)
```

**Note**: `Map.has_key`, NOT `Map.has_key?` (no `?` in function names).

### Enum

```
Enum.count(enum)           # Count elements
Enum.count(enum, fun)      # Count elements where fun returns true
Enum.sum(enum)             # Sum numeric elements
Enum.sort(enum)            # Sort ascending
Enum.sort(enum, fun)       # Sort with custom comparator (fun.(a, b) returns true if a before b)
Enum.reverse(enum)         # Reverse
Enum.take(enum, n)         # First n elements
Enum.drop(enum, n)         # Skip first n elements
Enum.unique(enum)          # Remove duplicates
Enum.chunk_every(enum, n)  # Split into chunks of size n
Enum.join(enum)            # Join into string (no separator)
Enum.join(enum, sep)       # Join into string with separator
Enum.into(enum, target)    # Collect into list or map

Enum.map(enum, fun)        # Transform each element
Enum.filter(enum, fun)     # Keep elements where fun returns true
Enum.reduce(enum, acc, fun) # Fold with accumulator
Enum.find(enum, fun)       # First element where fun returns true (nil if none)
Enum.any(enum, fun)        # True if any element matches predicate
Enum.all(enum, fun)        # True if all elements match predicate
Enum.min(enum)             # Minimum element
Enum.max(enum)             # Maximum element
Enum.flat_map(enum, fun)   # Map then flatten one level
Enum.zip(left, right)      # Zip two lists into tuples
Enum.with_index(enum)      # Add index: [{elem, 0}, {elem, 1}, ...]
Enum.each(enum, fun)       # Side-effect iteration (returns :ok)
Enum.at(enum, index)       # Element at 0-based index (nil if out of bounds)
Enum.fetch(enum, index)    # Safe access: {:ok, value} or :error
Enum.to_list(enum)         # Materialize enumerable (e.g., range) to list
Enum.member(enum, value)   # Check if value is in list
Enum.reject(enum, fun)     # Keep elements where fun returns false (opposite of filter)
Enum.sort_by(enum, fun)    # Sort by key function
Enum.group_by(enum, fun)   # Group into map by key function
Enum.min_by(enum, fun)     # Element with minimum key
Enum.max_by(enum, fun)     # Element with maximum key
Enum.frequencies(enum)     # Count occurrences: %{value => count}
Enum.uniq_by(enum, fun)    # Remove duplicates by key function
Enum.map_join(enum, sep, fun)  # Map then join into string
Enum.dedup(enum)           # Remove consecutive duplicates
Enum.intersperse(enum, sep) # Insert separator between elements
Enum.zip_with(left, right, fun) # Zip two lists and apply function to pairs
Enum.take_while(enum, fun)  # Take elements while predicate is true
Enum.drop_while(enum, fun)  # Drop elements while predicate is true
Enum.chunk_by(enum, fun)    # Group consecutive elements by key function
Enum.scan(enum, acc, fun)   # Running accumulator, returns list of intermediate values
Enum.split(enum, count)     # Split list at index, returns {left, right} tuple
Enum.count_by(enum, fun)    # Count elements where predicate returns true
Enum.uniq(enum)             # Remove duplicates (alias for unique)
Enum.map_reduce(enum, acc, fun) # Single-pass map+reduce, returns {mapped_list, final_acc}
Enum.concat(left, right)    # Concatenate two enumerables into one list
Enum.product(enum)          # Product of all numeric elements
Enum.slice(enum, start, count) # Extract sublist starting at index for count elements
Enum.random(enum)           # Random element from list
Enum.find_index(enum, fun)  # Index of first element where fun.(elem) is truthy, or nil
Enum.reduce_while(enum, acc, fun) # Reduce with early termination ({:cont, acc} or {:halt, acc})
Enum.shuffle(enum)             # Randomly shuffle elements
```

All `Enum` functions work on lists and ranges.

Higher-order Enum functions take closures:

```elixir
Enum.map([1, 2, 3], &(&1 * 2))           # [2, 4, 6]
Enum.filter([1, 2, 3, 4], &(&1 > 2))     # [3, 4]
Enum.reduce([1, 2, 3], 0, &(&1 + &2))    # 6
```

More examples:

```elixir
Enum.find([1, 2, 3, 4], &(&1 > 2))      # 3
Enum.any([1, 2, 3], &(&1 > 2))           # true
Enum.all([1, 2, 3], &(&1 > 0))           # true
Enum.min([5, 3, 8, 1])                    # 1
Enum.max([5, 3, 8, 1])                    # 8
Enum.flat_map([1, 2], &([&1, &1 * 10]))  # [1, 10, 2, 20]
Enum.at([10, 20, 30], 1)                  # 20
Enum.member([1, 2, 3], 2)                 # true
```

Side-effect iteration with `Enum.each`:

```elixir
Enum.each([1, 2, 3], &(IO.puts("item: #{&1}")))
# Prints each item, returns :ok
```

Zip and with_index:

```elixir
Enum.zip([1, 2, 3], [:a, :b, :c])   # [{1, :a}, {2, :b}, {3, :c}]
Enum.with_index([:a, :b, :c])       # [{:a, 0}, {:b, 1}, {:c, 2}]
```

Chaining with pipe:

```elixir
[1, 2, 3, 4, 5]
|> Enum.filter(&(&1 > 2))
|> Enum.map(&(&1 * 10))
|> Enum.sort()
```

### System

```
System.read_text(path)     # Read file contents
System.write_text(path, content)  # Write file
System.env(name)           # Get env variable
System.argv()              # Command-line arguments
System.cwd()               # Current working directory
System.run(command)         # Run shell command
System.path_exists(path)   # Check if path exists
System.list_dir(path)      # List directory contents
```

### Path

```
Path.join(a, b)            # Join path segments
Path.dirname(path)         # Directory part
Path.basename(path)        # Filename part
Path.extname(path)         # Extension
```

## Formatting Lists for Output

String interpolation works for all types including lists, maps, and tuples:

```elixir
IO.puts("List: #{[1, 2, 3]}")       # List: [1, 2, 3]
IO.puts("Map: #{%{a: 1}}")          # Map: %{a: 1}
IO.puts("Tuple: #{{1, 2}}")         # Tuple: {1, 2}
```

For custom formatting, use `Enum.join/1` (no separator) or `Enum.join/2`:

```elixir
Enum.join(["a", "b", "c"])   # "abc" (no separator)
Enum.join([1, 2, 3], ", ")   # "1, 2, 3"
Enum.join(names, " | ")      # custom separator
```

## Key Differences from Elixir

1. **No `?` in function names**: Use `Map.has_key` not `Map.has_key?`, `String.contains` not `String.contains?`
2. **Sequential IO requires chaining**: Use `case _ ->` to sequence side effects
3. **String interpolation works for all types**: `"#{[1,2,3]}"` produces `[1, 2, 3]`. Lists, maps, tuples, atoms, bools, nil, and nested structures all interpolate correctly.
4. **For-comprehension filters use `when` guards**: `for x when x > 3 <- list do x end` (NOT Elixir's comma syntax `for x <- list, x > 3`)
5. **Case requires wildcard**: Every `case` must have a `_` catch-all branch
6. **Capture syntax preferred**: `&(&1 * 2)` is the most reliable closure syntax
7. **No `do:` single-line syntax**: Always use full `do...end` blocks
8. **Type checker quirks**: Bind stdlib results to variables before using with `<>` operator
9. **List construction**: Use `[item] ++ list` instead of list literals when building recursively
10. **Backslash escapes work in strings**: `\n` (newline), `\t` (tab), `\\` (backslash), `\"` (double quote), `\r` (carriage return) all work in regular and interpolated strings
11. **`==` and `!=` work for all types**: String, atom, bool, list, map, tuple, and cross-type comparisons all work with `==` and `!=`. Example: `name == "admin"`, `:ok != :err`, `[1,2] == [1,2]`.
12. **`div()` may fail after variable bindings**: `div(scaled, n)` after complex expressions can cause parser errors. Wrap in a helper function:
    ```elixir
    defp int_div(a, b) do
      div(a, b)
    end
    ```
13. **Float arithmetic**: `+`, `-`, `*`, `/` all work with both integers and floats. `Int op Int` returns `Int`, any float operand returns `Float`. Use `div()` and `rem()` for explicit integer operations:
    ```elixir
    1.5 + 2.5     # => 4.0 (float addition)
    3.0 * 2.5     # => 7.5 (float multiplication)
    10 - 3.5      # => 6.5 (mixed → float)
    10.0 / 3.0    # => 3.3333333333333335 (float division)
    div(10, 3)    # => 3 (integer division)
    ```
14. **Multi-expression branch bodies**: By default, case/cond/fn/rescue branches contain a single expression. For multi-expression bodies, wrap in `do`/`end`:
    ```elixir
    case value do
      :ok -> do
        step1 = compute()
        step2 = transform(step1)
        step2
      end
      :error -> handle_error()
    end
    ```
    This works in `case`, `cond`, anonymous `fn`, and `rescue` branches. Single-expression branches work as before without `do`/`end`.

## Complete Example

```elixir
defmodule WordCounter do
  def count(text) do
    words = String.split(text, " ")
    build_freq(words, %{})
  end

  defp build_freq([], freq) do
    freq
  end

  defp build_freq([word | rest], freq) do
    current = Map.get(freq, word, 0)
    build_freq(rest, Map.put(freq, word, current + 1))
  end

  def top_words(freq, n) do
    keys = Map.keys(freq)
    sorted_keys = sort_by_freq(keys, freq)
    Enum.take(sorted_keys, n)
  end

  defp sort_by_freq(keys, freq) do
    sort_helper(keys, freq, [])
  end

  defp sort_helper([], _freq, acc) do
    acc
  end

  defp sort_helper(keys, freq, acc) do
    best = find_max(keys, freq)
    remaining = remove_key(keys, best, [])
    sort_helper(remaining, freq, acc ++ [best])
  end

  defp find_max([only], _freq) do
    only
  end

  defp find_max([first | rest], freq) do
    find_max_helper(rest, freq, first, Map.get(freq, first, 0))
  end

  defp find_max_helper([], _freq, best, _best_count) do
    best
  end

  defp find_max_helper([key | rest], freq, best, best_count) do
    count = Map.get(freq, key, 0)
    case count > best_count do
      true -> find_max_helper(rest, freq, key, count)
      _ -> find_max_helper(rest, freq, best, best_count)
    end
  end

  defp remove_key([], _target, acc) do
    acc
  end

  defp remove_key([key | rest], target, acc) do
    case key do
      ^target -> acc ++ rest
      _ -> remove_key(rest, target, [key] ++ acc)
    end
  end
end

defmodule Demo do
  def run() do
    freq = WordCounter.count("the cat sat on the mat the cat")
    top = WordCounter.top_words(freq, 3)
    case IO.puts("Top words:") do
      _ -> print_words(top, freq)
    end
  end

  defp print_words([], _freq) do
    :ok
  end

  defp print_words([word | rest], freq) do
    count = Map.get(freq, word, 0)
    case IO.puts("  #{word}: #{count}") do
      _ -> print_words(rest, freq)
    end
  end
end
```
