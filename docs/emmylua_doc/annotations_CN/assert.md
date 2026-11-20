# @assert - 断言函数

将函数标记为对某个参数的断言。当函数**作为语句**调用并正常返回后，传给该参数的实参会在之后的流程中被收窄，效果与 `assert(x)` 或 `if x then ... end` 相同。

## 语法

```lua
---@assert <param>          # 调用后实参为真值
---@assert <param> -?       # 调用后实参不为 nil
---@assert <param> <type>   # 调用后实参的类型为 <type>
```

`<param>` 必须是被注解函数的参数名。`<type>` 可以是任意类型表达式，包括 `true`、`false`、`nil` 等字面量。

## 示例

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

local x = maybe_integer()     -- integer|false|nil
check(x)
print(x)                      -- integer

local y = maybe_integer()     -- integer|false|nil
check_not_nil(y)
print(y)                      -- integer|false
```

### 对状态标志的断言

当实参是与其他返回值一同返回的局部标志时，其他值也会被收窄，与 `if ok then` 一致：

```lua
---@return_overload true, integer
---@return_overload false, string
local function parse(s) end

local ok, value = parse(input)
check(ok)
print(value)                  -- integer
```

### 仅限语句

只有当调用是语句时才会收窄；作为表达式使用（`local _ = check(x)`、`if check(x) then`）不会收窄。

## 说明

- `@assert` 描述的是调用**返回后**成立的条件。若函数*返回*某值是否属于某类型，请使用 [`@return`](./return.md) 配合 `TypeGuard<T>`。
- 内置的 luassert 定义对扁平断言（`assert.truthy`、`assert.is_true`、`assert.is_not_nil` 等）使用了 `@assert`；链式写法（如 `assert.is_not.truthy(x)`）不会收窄。
