# Tailang CLI 使用指南

**版本**: 0.1.0  
**最后更新**: 2026-04-16

---

## 📦 安装

### 从源码安装

```bash
cd cli
go install
```

### 从 Release 安装

```bash
# Windows
curl -LO https://github.com/XueClaw/tailang/releases/latest/download/meng-windows.exe
move meng-windows.exe meng.exe

# macOS
curl -LO https://github.com/XueClaw/tailang/releases/latest/download/meng-macos
chmod +x meng-macos
mv meng-macos meng

# Linux
curl -LO https://github.com/XueClaw/tailang/releases/latest/download/meng-linux
chmod +x meng-linux
mv meng-linux meng
```

---

## 🚀 快速开始

### 1. 创建项目

```bash
meng init my-blog
cd my-blog
```

**输出**:
```
🚀 Creating Tailang project: my-blog

  ✓ Created my-blog/src/
  ✓ Created my-blog/tests/
  ✓ Created my-blog/docs/
  ✓ Created my-blog/assets/
  ✓ Created .gitignore
  ✓ Created src/main.meng
  ✓ Created tests/main_test.meng
  ✓ Created README.md
  ✓ Created tailang.yaml

✅ Project 'my-blog' initialized successfully!

📚 Next steps:
  cd my-blog
  meng build src/main.meng    # Build the project
  meng run src/main.meng      # Build and run
  edit src/main.meng          # Start coding!
```

### 2. 编写代码

编辑 `src/main.meng`:

```meng
# 打印 Hello World
打印 "Hello, Tailang!" qwq

```python
# Python 代码补充
print("Hello from Python!")
```
```

### 3. 推荐的 `.tai-first` 编译运行流程

```bash
# 预编译：把 .meng 转成可审查的 .tai
meng precompile src/main.meng

# 校验/审查 .tai
meng validate-tai src/main.tai

# 编译 .tai
meng build src/main.tai

# 运行 .tai
meng run src/main.tai
```

### 4. 直接从 `.meng` 构建

```bash
meng build src/main.meng
meng run src/main.meng
```

说明：
- 官方推荐把 `.tai` 作为可审查、可缓存、可版本控制的正式太语言源码。
- Go CLI 负责预编译、校验、编排构建流程。
- 真实代码生成后端应由 `compiler/` 中的 Rust 编译器实现。
- `.tai` 正式文本语法必须使用易语言风格点号中文关键字，例如 `.元信息 / .目标平台 / .程序集 / .子程序 / .校验 / .代码 / .待定`。
- `.tai` 正式块结构不使用 `{}`，而使用语义化结束词，例如 `.如果结束 / .判断结束 / .循环判断尾 / .代码结束`。
- `.子程序` 体内直接书写原生执行语法，不再推荐旧式 `.实现 .开始 ... .结束`。
- 子程序内的执行语句同样使用点号关键字，例如 `.令 / .如果 / .否则如果 / .判断开始 / .循环判断首 / .返回`。
- 不推荐跳过 `.tai` 直接长期以 `.meng` 作为唯一构建输入，否则难以审查预编译结果。

---

## 📖 命令参考

### meng init

初始化新项目。

```bash
meng init <project-name>
```

**选项**:
- 无

**示例**:
```bash
meng init my-blog
meng init todo-api
meng init data-analysis
```

---

### meng build

编译 `.meng` 或 `.tai` 文件为构建产物。

```bash
meng build <file> [选项]
```

**选项**:
- `-o, --output <name>` - 输出文件名
- `--target <platform>` - 目标平台 (`windows`, `macos`, `linux`)

**示例**:
```bash
# 推荐：从 .tai 构建
meng build src/main.tai

# 也支持直接从 .meng 构建
meng build src/main.meng

# 指定目标平台
meng build src/main.tai --target windows
```

**输出**:
```
🔨 Building src/main.tai...
   Output: main.exe
   Target: windows

Step 1/5: Reading source file...
  ✓ File read successfully
Step 2/5: Normalizing source to .tai...
  ✓ .tai normalized
Step 3/5: Extracting code supplements from .tai...
  ✓ Found 1 code block(s)
Step 4/5: Generating intermediate representation...
  ✓ IR generated
Step 5/5: Compiling to executable...
  ... delegated to compiler backend

✅ Build complete!

📦 Output: main.exe
📊 Size: depends on backend

🚀 Run with:
   ./main.exe

Or use:
   meng run src/main.tai
```

**说明**:
- 输入可以是 `.meng` 或 `.tai`，但推荐将 `.tai` 作为正式构建输入
- Go CLI 负责把输入整理为稳定的 `.tai` 源码或兼容快照
- 真实代码生成应由 Rust compiler 接手，CLI 当前不应内置语言后端

---

### meng precompile

将 `.meng` 预编译为 `.tai`。

```bash
meng precompile <file.meng> [选项]
```

**选项**:
- `-o, --output <path>` - 输出 `.tai` 文件路径

**环境变量**:
- `TAILANG_LLM_PROVIDER` - Provider 名称 (`dashscope`, `ollama`, `custom`)
- `TAILANG_LLM_MODEL` - 模型名覆盖
- `TAILANG_LLM_BASE_URL` - 自定义/OpenAI 兼容接口地址
- `TAILANG_LLM_API_KEY` - 通用 API Key
- `DASHSCOPE_API_KEY` - 百炼 API Key
- `DASHSCOPE_BASE_URL` - 百炼 Base URL
- `OLLAMA_BASE_URL` - Ollama Base URL

**示例**:
```bash
meng precompile src/main.meng
meng precompile src/main.meng -o src/main.tai
```

**输出**:
```
🔄 Precompiling src/main.meng...
   Output: src/main.tai

Step 1/3: Reading .meng file...
  ✓ File read successfully
Step 2/3: Calling configured provider...
  ✓ Provider returned normalized .tai output
Step 3/3: Writing .tai file...
  ✓ .tai file written

✅ Precompilation complete!

🚀 Next step:
   meng build src/main.tai
```

---

### meng run

编译并立即运行 `.meng` 或 `.tai` 文件。

```bash
meng run <file> [选项]
```

**选项**:
- `--args <args>` - 传递给程序的参数

**示例**:
```bash
# 从 .tai 运行
meng run src/main.tai

# 从 .meng 运行
meng run src/main.meng

# 带参数
meng run src/main.meng --args "arg1 arg2"
```

**输出**:
```
🚀 Running src/main.tai...

Step 1/2: Building...
  ✓ Compilation successful

Step 2/2: Executing...
────────────────────────────────────────
Hello, Tailang!
Hello from Python!
────────────────────────────────────────

✅ Execution complete!
```

---

### meng test

运行测试。

```bash
meng test [path]
```

**选项**:
- 无

**示例**:
```bash
# 运行所有测试
meng test

# 运行指定目录的测试
meng test tests/

# 运行单个测试文件
meng test tests/auth_test.meng
```

**输出**:
```
🧪 Running tests...

Found 3 test file(s):

  Running tests/auth_test.meng... ✓ PASSED
  Running tests/api_test.meng... ✓ PASSED
  Running tests/utils_test.meng... ✓ PASSED

✅ Tests complete: 3 passed, 0 failed
```

**说明**:
- 当前 `meng test` 命令已存在，但底层测试执行仍是简化实现

---

### meng doc

生成文档。

```bash
meng doc [path] [选项]
```

**选项**:
- `-o, --output <dir>` - 输出目录 (默认：docs)
- `--format <format>` - 输出格式 (markdown, html, pdf)

**示例**:
```bash
# 生成当前项目的文档
meng doc

# 生成指定目录的文档
meng doc src/

# 生成 HTML 格式
meng doc --format html -o docs/html
```

**输出**:
```
📚 Generating documentation...
   Source: .
   Format: markdown
   Output: docs

✅ Documentation generated: docs/README.md
```

**说明**:
- `meng doc` 会优先读取同名 `.tai`，避免重复触发 `.meng` 预编译
- 若目录内只有 `.meng`，则会即时预编译后再生成文档

---

### meng version

显示版本信息。

```bash
meng version
```

**输出**:
```
meng version 0.1.0
tailang-compiler 0.1.0
go 1.21
```

---

### meng validate-tai

校验 `.tai` 文件。

```bash
meng validate-tai <file.tai>
```

**示例**:
```bash
meng validate-tai src/main.tai
```

**输出**:
```
✓ Valid .tai: src/main.tai
```

**当前行为**:
- 若输入是旧版 JSON 快照，则按 `docs/spec/tai.schema.json` 校验。
- 若输入是文本 `.tai` 源码，则按当前易语言风格点号关键字最小语法规则校验。
- 文本 `.tai` 中的结构关键字必须采用 `.程序集 / .子程序 / .参数 / .局部变量 / .校验 / .代码` 这类点号中文写法。
- 文本 `.tai` 当前主线采用 `.如果结束 / .判断结束 / .循环判断尾 / .代码结束` 这类语义化结束词。
- 子程序主体当前支持的最小执行关键字包括 `.令 / .如果 / .否则 / .否则如果 / .判断开始 / .判断 / .默认 / .循环判断首 / .返回 / .真 / .假 / .空 / .非`。
- Rust compiler 内部已开始实现正式文本 `.tai` parser，但 CLI 与 compiler 的校验逻辑还未完全统一。

**文本 `.tai` 示例**:

```tai
.版本 3
.目标平台 视窗
.程序集 认证
.说明 "认证流程"

.子程序 登录, 文本型
.参数 邮箱, 文本型
.参数 密码, 文本型
.局部变量 结果, 文本型
.校验 "邮箱不能为空"

.如果 邮箱 等于 ""
    .返回 "邮箱不能为空"
.否则如果 密码 等于 ""
    .返回 "密码不能为空"
.如果结束

.令 结果 = 邮箱
.返回 结果

.代码 Rust
println!("hello");
.代码结束

.待定 规则, "缺少密码复杂度规则"
```

---

## 📁 项目结构

标准的 Tailang 项目结构：

```
my-project/
├── src/                    # 源代码
│   ├── main.meng          # 主入口
│   ├── auth.meng          # 认证模块
│   └── api.meng           # API 模块
├── tests/                  # 测试文件
│   ├── main_test.meng
│   └── auth_test.meng
├── docs/                   # 文档
│   └── README.md
├── assets/                 # 资源文件
│   ├── images/
│   └── styles/
├── .gitignore              # Git 忽略文件
├── tailang.yaml            # 项目配置
└── README.md               # 项目说明
```

---

## 🔧 配置文件

### tailang.yaml

```yaml
name: my-blog
version: 0.1.0
tailang: 0.1.0
entry: src/main.meng

# 依赖管理
dependencies:
  - user-auth@1.0
  - database@2.0

# 构建配置
build:
  target: windows
  output: dist/
  optimize: true

# 运行配置
run:
  args: "--debug --verbose"
  env:
    - DEBUG=true
```

---

## 💡 最佳实践

### 1. 命名规范

- 项目名称：小写字母 + 连字符 (`my-blog`)
- 文件名：小写 + 下划线 (`main.meng`, `user_auth.meng`)
- 测试文件：`*_test.meng` 或 `*.test.meng`

### 2. 代码组织

```meng
# 清晰的模块划分
src/
  ├── main.meng          # 程序入口
  ├── auth.meng          # 认证逻辑
  ├── api.meng           # API 接口
  └── utils.meng         # 工具函数
```

### 3. 代码补充

```meng
# 自然语言描述意图
用户登录验证 qwq

# 复杂逻辑用代码补充
```python
import bcrypt
def verify_password(stored, provided):
    return bcrypt.checkpw(provided.encode(), stored.encode())
```
```

### 4. 测试

```meng
# 每个功能都应有测试
tests/
  ├── auth_test.meng     # 认证测试
  ├── api_test.meng      # API 测试
  └── utils_test.meng    # 工具测试
```

---

## 🐛 常见问题

### Q: `meng: command not found`

**A**: 确保 CLI 已安装并添加到 PATH:

```bash
# 添加到 PATH (Linux/macOS)
export PATH=$PATH:$(go env GOPATH)/bin

# 或使用完整路径
/path/to/meng init my-project
```

### Q: 编译失败

**A**: 先检查预编译输出和 `.tai` 结构:

```bash
meng precompile src/main.meng
meng validate-tai src/main.tai
meng build src/main.tai
```

### Q: 如何调试？

**A**: 当前建议直接检查 `.tai` 和编译器输入。

```bash
meng precompile src/main.meng
meng validate-tai src/main.tai
meng build src/main.tai
```

---

## 📚 更多资源

- [GitHub](https://github.com/XueClaw/tailang)
- [示例代码](https://github.com/XueClaw/tailang/examples)
- [语言规范](https://github.com/XueClaw/tailang/docs/spec)
- [问题反馈](https://github.com/XueClaw/tailang/issues)

---

**Happy Coding! 🎉**
