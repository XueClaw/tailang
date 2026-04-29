# Tailang `.tai` vNext 语法方案

## 状态

- 本文档描述当前准备推进的 `.tai` 主语法方向
- 目标是保留 E 风格的声明气质，同时减少无必要的散列声明语法
- 中文与英文都属于正式输入语法，风格保持一致

## 设计结论

`.tai` 采用以下原则：

1. 顶层与结构声明保留点号前缀
2. 中文 E 风格语法与英文镜像语法并存
3. `.参数 / .局部变量 / .常量` 不再作为主语法
4. 参数进入子程序头
5. 局部变量使用标准单行声明
6. 已废弃写法立即报错，不再保留兼容输入
7. E 风格子程序头中的空槽位保留
8. 所有可调用能力统一收敛为函数调用

## 顶层结构

### 中文

```tai
.版本 2
.程序集 窗口示例
```

### 英文

```tai
.version 2
.module window_demo
```

## 子程序头

### 中文标准写法

```tai
.子程序 启动程序() -> 整数型, , , 启动
```

```tai
.子程序 按钮被点击(按钮文本: 文本型) -> 整数型, , , 按钮1被点击
```

### 英文标准写法

```tai
.subprogram startup() -> int, , , startup
```

```tai
.subprogram on_button_click(button_text: text) -> int, , , button1_click
```

## 子程序头字段结构

统一结构为：

```text
.子程序 名称(参数列表) -> 返回类型, 槽位3, 槽位4, 绑定名
```

或英文：

```text
.subprogram name(params) -> return_type, slot3, slot4, binding
```

说明：

- `名称`
  子程序名
- `参数列表`
  零个或多个参数，使用标准编程写法
- `返回类型`
  当前必须显式给出
- `槽位3`
  保留位，可为空
- `槽位4`
  保留位，可为空
- `绑定名`
  入口/事件/外部绑定等语义名，可为空

## 参数

参数不再使用独立 `.参数` 声明作为主语法。

### 中文

```tai
.子程序 登录(邮箱: 文本型, 密码: 文本型) -> 文本型
```

### 英文

```tai
.subprogram login(email: text, password: text) -> text
```

## 局部变量

局部变量不再使用独立 `.局部变量` 声明作为主语法。

推荐使用标准单行声明：

### 中文

```tai
结果: 整数型 = 0
名称: 文本型 = "结衣"
已通过: 逻辑型 = 真
```

### 英文

```tai
result: int = 0
name: text = "Yui"
passed: bool = true
```

也允许使用推断式局部绑定：

```tai
结果 = "结衣"
数据 = [3, 5, 8]
```

```tai
result = "Yui"
data = [3, 5, 8]
```

## Prelude 与内置函数

`.tai` 只把结构与流程保留为关键字。所有可调用能力，包括标准输出、文本与集合工具，都统一表现为函数调用。

### 基本规则

- 编译器会自动加载内置 prelude 模块
- prelude 中的函数名属于保留名，用户代码不能重定义
- 相同函数名允许存在多个重载，但必须按参数类型精确匹配
- 当前不支持隐式类型转换、`any` 类型或泛型推断

### 第一批标准 prelude

输出：

```tai
显示("hello")
显示(1)
显示(真)
```

```tai
print("hello")
print(1)
print(true)
```

文本：

```tai
结果: 整数型 = 文本长度("结衣")
```

```tai
result: int = text_len("Yui")
```

集合：

```tai
数据: 整数型[] = [3, 5, 8]
返回 数组长度(数据)
```

```tai
data: int[] = [3, 5, 8]
return array_len(data)
```

## 表达式与值

### 中文值

- `真`
- `假`
- `空`

### 英文值

- `true`
- `false`
- `null`

### 中文比较/逻辑

- `等于`
- `不等于`
- `大于`
- `小于`
- `大于或等于`
- `小于或等于`
- `并且`
- `或者`
- `非`

### 英文比较/逻辑

- `==`
- `!=`
- `>`
- `<`
- `>=`
- `<=`
- `&&`
- `||`
- `!`

英文逻辑运算最小示例：

```tai
.if !flag || ready && valid
    return true
.else
    return false
.end
```

## 流程控制

### 中文

```tai
.如果 名称 等于 "结衣"
    返回 真
.否则
    返回 假
.如果结束
```

```tai
.判断开始 状态
.判断 "成功"
    返回 "ok"
.默认
    返回 "unknown"
.判断结束
```

```tai
.循环判断首 计数 小于 10
    计数 = 计数 + 1
.循环判断尾
```

### 英文

```tai
.if name == "Yui"
    return true
.else
    return false
.end
```

```tai
.match state
.case "ok"
    return "ok"
.default
    return "unknown"
.end
```

```tai
.while count < 10
    count = count + 1
.end
```

## 代码补充块

### 中文

```tai
.代码 Rust
println!("hello");
.代码结束
```

### 英文

```tai
.code Rust
println!("hello");
.endcode
```

## 最小示例

### 中文

```tai
.版本 2
.程序集 窗口示例

.子程序 _启动程序() -> 整数型, , , 启动
    结果: 整数型 = 0
    窗口_创建(0, 0, 0, 300, 200, "示例窗口", 空, 窗口_普通风格)
    按钮_创建(0, 10, 10, 100, 40, "点击我", 窗口_普通风格)
    窗口_显示()
返回 结果

.子程序 按钮被点击() -> 整数型, , , 按钮1被点击
    信息框("按钮被点击了！", 0, "提示")
返回 0
```

### 英文

```tai
.version 2
.module window_demo

.subprogram startup() -> int, , , startup
    result: int = 0
    window_create(0, 0, 0, 300, 200, "Demo Window", null, window_normal_style)
    button_create(0, 10, 10, 100, 40, "Click Me", window_normal_style)
    window_show()
return result

.subprogram on_button_click() -> int, , , button1_click
    info_box("Button clicked!", 0, "Hint")
return 0
```

## 硬切策略

以下写法现在属于无条件错误，而不是兼容输入：

- `.参数`
- `.局部变量`
- `.常量`
- 旧式子程序头
- 旧式 callable 关键字写法，例如 `.显示 "文本"`
- 依赖旧版文本 `.tai` 兼容快照的语法路径

## 实施优先级

1. 编译器自动加载 prelude，并把 prelude 名称纳入保留名集合
2. 调用解析支持精确重载匹配
3. HIR/MIR 建立 builtin-aware lowering
4. 第一批标准 prelude 在 LLVM 与 self-native 上都可执行
5. 规范与示例只保留新语法，不再描述兼容层
