# @assert - Assertion Functions

Mark a function as an assertion on one of its parameters. When the function is
called **as a statement** and returns normally, the argument passed for that
parameter is narrowed for the rest of the flow, the same way `assert(x)` or
`if x then ... end` narrows it.

## Syntax

```lua
---@assert <param>          # the argument is truthy after the call
---@assert <param> -?       # the argument is not nil after the call
---@assert <param> <type>   # the argument has <type> after the call
```

`<param>` must name a parameter of the annotated function. `<type>` is any type
expression, including literals such as `true`, `false` and `nil`.

## Examples

```lua
---@param value any
---@param message? string
---@assert value
local function check(value, message)
    if not value then error(message or "check failed", 2) end
end

---@param value any
---@assert value -?
local function check_not_nil(value)
    if value == nil then error("value is nil", 2) end
end

---@param value any
---@assert value string
local function check_string(value)
    if type(value) ~= "string" then error("expected a string", 2) end
end

local x = maybe_integer()     -- integer|false|nil
check(x)
print(x)                      -- integer

local y = maybe_integer()     -- integer|false|nil
check_not_nil(y)
print(y)                      -- integer|false

local z ---@type any
check_string(z)
print(z)                      -- string
```

### Assertions on a status flag

When the argument is a local flag returned together with other values, the
other values narrow too, exactly like `if ok then`:

```lua
---@return_overload true, integer
---@return_overload false, string
local function parse(s) end

local ok, value = parse(input)
check(ok)
print(value)                  -- integer
```

### Statement-only

Narrowing only applies when the call is a statement. Using the call as an
expression does not narrow its argument:

```lua
check(x)                      -- narrows x
local _ = check(x)            -- does not narrow x
if check(x) then end          -- does not narrow x
```

Methods work too; `<param>` refers to the declared parameter name, so
`self` is skipped when the method is called with `:`.

## Notes

- `@assert` describes what holds when the call **returns**. Use
  [`@return`](./return.md) with `TypeGuard<T>` for functions that *return*
  whether a value has a type (`if is_string(x) then`).
- The built-in luassert definitions use `@assert` for the flat assertions
  (`assert.truthy`, `assert.is_true`, `assert.is_not_nil`, ...). Chained forms
  such as `assert.is_not.truthy(x)` do not narrow.
