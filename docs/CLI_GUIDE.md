# Tailang CLI 使用指南

**版本**: 0.1.0  
**最后更新**: 2026-04-14

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

### 3. 编译运行

```bash
# 编译
meng build src/main.meng

# 运行
meng run src/main.meng
```

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

编译 .meng 文件为可执行文件。

```bash
meng build <file.meng> [选项]
```

**选项**:
- `-o, --output <name>` - 输出文件名
- `--target <platform>` - 目标平台 (windows, macos, linux)

**示例**:
```bash
# 基本用法
meng build src/main.meng

# 指定输出文件名
meng build src/main.meng -o myapp

# 指定目标平台
meng build src/main.meng --target windows
meng build src/main.meng --target macos
```

**输出**:
```
🔨 Building src/main.meng...
   Output: main.exe
   Target: windows

Step 1/5: Reading .meng file...
  ✓ File read successfully
Step 2/5: Precompiling natural language...
  ✓ Natural language expanded
Step 3/5: Extracting code supplements...
  ✓ Found 1 code block(s)
Step 4/5: Generating intermediate representation...
  ✓ IR generated
Step 5/5: Compiling to executable...
  ✓ Compilation successful

✅ Build complete!

📦 Output: main.exe
📊 Size: 1.2 MB

🚀 Run with:
   ./main.exe

Or use:
   meng run src/main.meng
```

---

### meng run

编译并立即运行 .meng 文件。

```bash
meng run <file.meng> [选项]
```

**选项**:
- `--args <args>` - 传递给程序的参数

**示例**:
```bash
# 基本用法
meng run src/main.meng

# 带参数
meng run src/main.meng --args "arg1 arg2"
```

**输出**:
```
🚀 Running src/main.meng...

Step 1/2: Building...
  ✓ Build complete

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

**A**: 检查 .meng 文件语法:

```bash
# 验证文件存在
ls src/main.meng

# 检查语法错误
meng build src/main.meng --verbose
```

### Q: 如何调试？

**A**: 使用 `--verbose` 模式:

```bash
meng build src/main.meng --verbose
meng run src/main.meng --verbose
```

---

## 📚 更多资源

- [GitHub](https://github.com/XueClaw/tailang)
- [示例代码](https://github.com/XueClaw/tailang/examples)
- [语言规范](https://github.com/XueClaw/tailang/docs/spec)
- [问题反馈](https://github.com/XueClaw/tailang/issues)

---

**Happy Coding! 🎉**
