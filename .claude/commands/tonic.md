Write Tonic code for the following request: $ARGUMENTS

You are a Tonic language expert. Tonic is an Elixir-inspired language compiled via Rust. Write idiomatic, working Tonic code.

---

# Tonic Language Reference

## Program Structure

Every Tonic program is one or more `defmodule` blocks. The entry point is `Module.run/0`.

```tonic
defmodule Demo do
  def run() do
    "hello world"
  end
end
```

Multiple modules in one file:
```tonic
defmodule Math do
  def add(a, b) do
    a + b
  end
end

defmodule Demo do
  def run() do
    Math.add(1, 2)
  end
end
```

## Types & Literals

| Type | Example | Notes |
|------|---------|-------|
| Integer | `42`, `1_000` | i64, underscore separators |
| Float | `3.14`, `1.0e-2` | IEEE 754 double |
| String | `"hello"`, `"""heredoc"""` | UTF-8, interpolation via `#{}` |
| Atom | `:ok`, `:error`, `:foo` | Singleton symbols |
| Boolean | `true`, `false` | |
| Nil | `nil` | |
| Tuple | `{1, :ok, "hi"}` | Fixed size |
| List | `[1, 2, 3]` | Dynamic array |
| Map | `%{key: val}` or `%{"k" => v}` | Atom keys use colon shorthand |
| Keyword | `[ok: 1, err: 2]` | Ordered atom-keyed pairs |
| Range | `1..10`, `1..100//5` | Inclusive, optional step |
| Bitstring | `<<1, 2, 3>>` | Binary data |

String interpolation: `"result: #{1 + 2}"`

## Functions

### Public and Private
```tonic
defmodule Demo do
  def public_fn(x) do    # callable from other modules
    helper(x)
  end

  defp helper(x) do      # private to this module
    x * 2
  end
end
```

### Multi-clause Pattern Dispatch
```tonic
def process({:ok, val}) do
  val
end

def process({:error, reason}) do
  raise reason
end
```

### Guards
```tonic
def fizzbuzz(n) when rem(n, 15) == 0 do "FizzBuzz" end
def fizzbuzz(n) when rem(n, 3) == 0 do "Fizz" end
def fizzbuzz(n) when rem(n, 5) == 0 do "Buzz" end
def fizzbuzz(n) do n end
```

Guard builtins (also work as regular expressions outside guards — in `if`, `case`, function bodies, etc.): `is_integer/1`, `is_binary/1`, `is_atom/1`, `is_boolean/1`, `is_nil/1`, `is_list/1`, `is_map/1`, `is_tuple/1`, `is_float/1`, `is_number/1`

### Default Arguments
```tonic
def greet(name \\ "World") do
  "Hello, #{name}"
end
```

### Anonymous Functions & Captures
```tonic
# Lambda syntax
double = fn x -> x * 2 end
double.(5)  # => 10

# Capture shorthand (&1, &2, ... are positional args)
double = &(&1 * 2)
double.(5)  # => 10

# Invoke with .()
fun.(arg1, arg2)
```

## Operators

### Arithmetic
`+`, `-`, `*`, `/` (work with Int, Float, or mixed — mixed promotes to Float), `div`, `rem` (Int only)

### Comparison
`==`, `!=`, `===`, `!==`, `<`, `<=`, `>`, `>=`

### Logical
`and`, `or`, `not` (strict bool), `&&`, `||`, `!` (truthy)

### String & List
`<>` (string concat), `++` (list concat), `--` (list subtract)

### Membership & Range
`in`, `not in`, `..` (range), `//` (step)

### Bitwise
`&&&`, `|||`, `^^^`, `~~~`, `<<<`, `>>>`

### Kernel Builtins (bare functions)
`abs/1`, `length/1`, `hd/1`, `tl/1`, `elem/2`, `tuple_size/1`, `to_string/1`, `inspect/1`, `max/2`, `min/2`, `round/1`, `trunc/1`, `map_size/1`, `put_elem/3`, `div/2`, `rem/2`

### Pipe
```tonic
[1, 2, 3, 4, 5]
|> filter_even()
|> double_all()
|> sum_all()
```
Passes left result as first argument to right function.

## Pattern Matching

### Match Operator
```tonic
{a, b} = {1, 2}          # a=1, b=2
[head | tail] = [1, 2, 3] # head=1, tail=[2,3]
%{name: name} = user       # extract name from map
```

### Pin Operator
```tonic
x = 1
case {1, 2} do
  {^x, y} -> y   # match x's current value (1), bind y
end
```

### Wildcard
```tonic
{_, important} = {1, 2}  # ignore first element
```

## Control Flow

### if / unless
```tonic
if x > 0 do
  "positive"
else
  "non-positive"
end

unless empty do
  process()
end
```

### case (pattern matching)
```tonic
case value do
  {:ok, val} when val > 0 -> "positive: #{val}"
  {:ok, _} -> "zero or negative"
  {:error, reason} -> "error: #{reason}"
  _ -> "other"
end
```

### cond (multi-way conditional)
```tonic
cond do
  x > 100 -> "large"
  x > 10 -> "medium"
  true -> "small"
end
```

### with (happy-path chaining)
```tonic
with {:ok, a} <- fetch_a(),
     {:ok, b} <- fetch_b(a) do
  a + b
else
  {:error, reason} -> reason
  _ -> :unknown
end
```

### for (comprehension)
```tonic
# Basic
for x <- [1, 2, 3] do
  x * 2
end

# With guard filter
for x when rem(x, 2) == 0 <- list do
  x
end

# Multiple generators (cartesian product)
for x <- [1, 2], y <- [:a, :b] do
  {x, y}
end

# Reduce (accumulation)
for x <- list, reduce: 0 do
  acc -> acc + x
end

# Into map
for x <- [1, 2], into: map(:seed, 0) do
  {x, x * 10}
end
```

### try / rescue / catch / after
```tonic
try do
  risky_operation()
rescue
  _ -> "caught error"
catch
  kind, value -> "caught #{kind}"
after
  cleanup()
end
```

## Error Handling

### Result Types
```tonic
ok(value)      # wraps in success
err(reason)    # wraps in error
```

### Question Operator (? postfix)
```tonic
def fetch_data() do
  val = some_operation()?   # if err, returns err immediately
  val + 1                   # only reached if ok
end
```

### with + ? pattern (idiomatic error chaining)
```tonic
def execute() do
  with _a <- step_one()?,
       _b <- step_two()?,
       _c <- step_three()? do
    ok(:ok)
  end
end
```

## Modules & Imports

### alias
```tonic
alias Math, as: M
alias Data.{List, Map}   # multi-alias
M.add(1, 2)
```

### import
```tonic
import Math               # bring all functions into scope
import Math, only: [add: 2]
import Math, except: [add: 2]
```

### require / use
```tonic
require Logger
use SomeModule
```

### Module Attributes
```tonic
@moduledoc "Module documentation"
@doc "Function documentation"
@custom_attr 42
```

## Structs
```tonic
defmodule User do
  defstruct name: "", age: 0
end

# Create
user = %User{name: "Alice", age: 30}

# Access
user.name

# Update (creates new struct)
%User{user | age: 31}

# Pattern match
case user do
  %User{name: name, age: age} -> {name, age}
end
```

## Protocols
```tonic
defprotocol Size do
  def size(value)
end

defimpl Size, for: Map do
  def size(_value) do
    1
  end
end

defimpl Size, for: User do
  def size(user) do
    user.age
  end
end
```

## Standard Library

### IO
- `IO.puts(value)` — print to stdout with newline (accepts any type, auto-converts to string)
- `IO.inspect(value)` — print to stderr, returns value
- `IO.gets(prompt)` — read line from stdin

### String
`split/1`, `split/2`, `replace/3`, `trim/1`, `trim_leading/1`, `trim_trailing/1`, `starts_with/2`, `ends_with/2`, `contains/2`, `upcase/1`, `downcase/1`, `length/1`, `at/2`, `slice/3`, `to_integer/1`, `to_float/1`, `pad_leading/3`, `pad_trailing/3`, `reverse/1`, `to_charlist/1`, `duplicate/2`, `capitalize/1`, `to_atom/1`, `graphemes/1`

**Note**: String function names do NOT use `?` suffix (e.g. `String.contains`, not `String.contains?`).

### Enum
`count/1`, `count/2`, `sum/1`, `product/1`, `join/1`, `join/2`, `sort/1`, `sort/2`, `reverse/1`, `take/2`, `drop/2`, `chunk_every/2`, `unique/1`, `uniq/1`, `into/2`, `map/2`, `filter/2`, `reduce/3`, `find/2`, `find_index/2`, `any/2`, `all/2`, `min/1`, `max/1`, `flat_map/2`, `zip/2`, `with_index/1`, `each/2`, `at/2`, `fetch/2`, `to_list/1`, `member/2`, `reject/2`, `sort_by/2`, `group_by/2`, `min_by/2`, `max_by/2`, `frequencies/1`, `uniq_by/2`, `map_join/3`, `dedup/1`, `intersperse/2`, `zip_with/3`, `take_while/2`, `drop_while/2`, `chunk_by/2`, `scan/3`, `split/2`, `count_by/2`, `map_reduce/3`, `concat/2`, `product/1`, `slice/3`, `random/1`, `reduce_while/3`, `shuffle/1`

Higher-order Enum functions take closures:
```tonic
Enum.map([1, 2, 3], &(&1 * 2))           # [2, 4, 6]
Enum.filter([1, 2, 3, 4], &(&1 > 2))     # [3, 4]
Enum.reduce([1, 2, 3], 0, &(&1 + &2))    # 6
Enum.find([1, 2, 3, 4], &(&1 > 2))       # 3
Enum.any([1, 2, 3], &(&1 > 2))           # true
Enum.each([1, 2, 3], &(IO.puts("#{&1}"))) # prints each, returns :ok
```

### Map
`keys/1`, `values/1`, `merge/2`, `merge/3`, `drop/2`, `take/2`, `get/2`, `get/3`, `put/3`, `delete/2`, `has_key/2`, `update/4`, `put_new/3`, `to_list/1`, `new/0`, `from_list/1`, `filter/2`, `reject/2`, `pop/2`, `pop/3`

**Note**: `Map.has_key`, NOT `Map.has_key?` (no `?` in function names).

### Integer
`to_string/1`, `parse/1`

### Float
`to_string/1`, `round/2`, `ceil/1`, `floor/1`

### List
`first/1,2`, `last/1,2`, `flatten/1`, `zip/2`, `unzip/1`, `wrap/1`, `delete/2`, `delete_at/2`, `update_at/3`, `insert_at/3`, `duplicate/2`, `starts_with/2`, `to_tuple/1`

### Tuple
`to_list/1`

### System
- `System.run(cmd)` — execute shell command, returns `%{exit_code: int, output: string}`
- `System.cwd()` — current directory
- `System.argv()` — command-line args
- `System.env(name)` — environment variable
- `System.which(cmd)` — find command on PATH
- `System.path_exists(path)` — file/directory exists?
- `System.read_text(path)` — read file contents
- `System.write_text(path, content)` — write file
- `System.append_text(path, content)` — append to file
- `System.ensure_dir(path)` — create directory
- `System.remove_tree(path)` — delete file/directory
- `System.list_files_recursive(path)` — list all files
- `System.http_request(method, url, headers, body, opts)` — HTTP request
- `System.sleep_ms(ms)` — sleep
- `System.random_token(bytes)` — random base64 token
- `System.hmac_sha256_hex(key, data)` — HMAC-SHA256
- `System.log(level, event, fields)` — structured JSON logging

### Path
`join/2`, `dirname/1`, `basename/1`, `extname/1`, `expand/1`, `relative_to/2`

## Naming Conventions
- Modules: `PascalCase`
- Functions/variables: `snake_case`
- Atoms: `:lowercase_atom`
- Predicates: Tonic does NOT use `?` suffix (e.g., `Map.has_key`, not `Map.has_key?`)
- Private helpers: use `defp`

## Key Differences from Elixir
1. `tuple()` constructor for creating tuples in some contexts (e.g., `tuple(1, 2)`)
2. `ok(val)` / `err(val)` built-in result constructors
3. `?` postfix operator for result propagation (like Rust's `?`)
4. `map(:seed, 0)` constructor for seeded maps in `into:`
5. For-comprehension filters use `when` guards: `for x when x > 3 <- list do x end` (NOT Elixir's comma syntax `for x <- list, x > 3`)
6. `div` and `rem` are keywords (not `/` for integer division)
7. `@doc` association is positional, not structural
8. Bitstrings are byte-only (no multi-byte size specifiers)
9. **No `?` in function names**: Use `Map.has_key` not `Map.has_key?`, `String.contains` not `String.contains?`
10. **`==` and `!=` work for all types**: String, atom, bool, list, map, tuple, and cross-type comparisons all work with `==` and `!=`. Example: `name == "admin"`, `:ok != :err`, `[1,2] == [1,2]`.
11. **`div()` may fail after variable bindings**: `div(scaled, n)` after complex expressions can cause parser errors. Wrap in a helper function:
    ```tonic
    defp int_div(a, b) do
      div(a, b)
    end
    ```
12. **Float arithmetic**: `+`, `-`, `*`, `/` all work with both integers and floats. `Int op Int` returns `Int`, any float operand returns `Float`. Use `div()` and `rem()` for explicit integer operations:
    ```tonic
    1.5 + 2.5     # => 4.0 (float addition)
    3.0 * 2.5     # => 7.5 (float multiplication)
    10 - 3.5      # => 6.5 (mixed → float)
    10.0 / 3.0    # => 3.3333333333333335 (float division)
    ```
13. **Multi-expression branch bodies**: For multi-expression case/cond/fn/rescue branches, wrap the body in `do`/`end`: `pattern -> do expr1; expr2; expr3 end`. Single-expression branches work as before without `do`/`end`.

## Idiomatic Patterns

### Pipeline with error handling
```tonic
def run() do
  safe_div(10, 2) |> format_result()
end

defp safe_div(_, 0) do {:error, :division_by_zero} end
defp safe_div(a, b) do {:ok, div(a, b)} end

defp format_result({:ok, val}) do "result: #{val}" end
defp format_result({:error, reason}) do "error: #{reason}" end
```

### Command dispatcher
```tonic
def dispatch(command) do
  case command do
    "help" -> ok(usage())
    "run" -> execute_run()
    _ -> err("unknown command: #{command}")
  end
end
```

### Sequential operations with with + ?
```tonic
def deploy() do
  with _build <- run_command("cargo build")?,
       _test <- run_command("cargo test")?,
       _ship <- run_command("./deploy.sh")? do
    ok(:deployed)
  end
end
```

### Apply function to list
```tonic
defp apply_to_list(fun, list) do
  for x <- list do
    fun.(x)
  end
end
```

---

## Instructions

When writing Tonic code:
1. **Always wrap code in `defmodule ... do ... end`** — bare expressions are not valid
2. **Entry point is `def run() do ... end`** in the main module
3. **Use `defp` for helpers** — only expose what's needed
4. **Prefer pattern matching over conditionals** — multi-clause functions are idiomatic
5. **Use `|>` pipes for data transformation chains**
6. **Use `with` + `?` for sequential fallible operations**
7. **Use `for` comprehensions instead of manual recursion for iteration**
8. **Return structured data** — tuples, maps, result types (`ok/err`)
9. **Follow snake_case for functions, PascalCase for modules**
10. **Keep modules focused** — one responsibility per module

Save `.tn` files in the appropriate location. If no path is specified, use `examples/` for standalone programs or `src/` for project code.

For the complete language reference including edge cases and workaround patterns, see `TONIC_REFERENCE.md`.
