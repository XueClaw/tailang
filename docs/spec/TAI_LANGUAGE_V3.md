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
6. 旧写法只保留兼容，不再作为推荐写法
7. E 风格子程序头中的空槽位保留

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

## 兼容策略

当前仍允许旧写法输入，但不再作为主规范：

- `.参数`
- `.局部变量`
- `.常量`
- 无点号顶层中文声明
- 旧版文本 `.tai` 兼容快照路径

## 实施优先级

1. AST 支持新的子程序头结构
2. parser 支持新的中文/英文子程序头
3. validator 支持新的主语法
4. HIR/lowering 改为从函数头与标准变量声明读取结构
5. 旧 `.参数/.局部变量` 逐步降级为兼容
