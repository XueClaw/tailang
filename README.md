# Tailang（太语言）

Tailang 是一个以 `.meng -> .tai -> native compile` 为主线的编程系统：`.meng` 用来表达自然语言需求，`.tai` 是正式源码格式，`meng` CLI 负责把工程输入编排到原生编译流程里。

当前主线工作流是：

```text
.meng -> precompile -> .tai -> native compile -> executable
```

项目现在以 `.tai-first` 为正式路线：`.meng` 更像工程输入和需求描述，`.tai` 才是审查、版本控制、构建、测试和文档生成的稳定载体。当前规范中，中文语法和英文镜像语法都属于正式输入语法。

## 当前状态

- `.tai` 是正式太语言源码格式
- `.meng` 不是独立编程语言，而是进入 `.tai` 流程前的自然语言工程输入
- 编译链路已经不再经过 `rustc` 生成宿主语言源码，而是直接生成原生可执行文件
- 当前唯一已实现并验收的原生目标是 Windows x64
- 仓库同时包含 CLI、原生编译器、示例、测试，以及一个仍处于骨架期的 GUI

## 快速上手

### 1. 构建 CLI

```bash
cd cli
go build -o meng.exe .
```

### 2. 编译一个示例

```bash
cd cli
.\meng.exe build ..\examples\hello\main.meng
```

### 3. 直接运行

```bash
cd cli
.\meng.exe run ..\examples\hello\main.meng
```

如果你已经先生成了 `.tai`，也可以直接以 `.tai` 作为正式输入：

```bash
cd cli
.\meng.exe build sample.tai
.\meng.exe run sample.tai
```

## 核心工作流

推荐的 `.tai-first` 工作流如下：

```bash
meng precompile src/main.meng
meng validate-tai src/main.tai
meng build src/main.tai
meng run src/main.tai
meng test tests/ --backend llvm --opt-level 2
meng bench cli/bench_numeric.tai --report cli/bench_numeric.bench.json
meng doc src/
```

各步骤含义：

- `meng precompile`：把 `.meng` 归一化为正式 `.tai`
- `meng validate-tai`：校验 `.tai` 文本源码或兼容读取旧 JSON 快照
- `meng build`：把 `.meng` 或 `.tai` 编译为原生产物
- `meng run`：构建并执行目标程序
- `meng bench`：运行原生目标并与 Python 基线进行对比
- `meng doc`：从 `.tai-first` 项目输入生成 Markdown 文档
- `meng test`：发现测试规格，构建目标并校验 stdout / 退出码，可通过 `--backend llvm` 覆盖 LLVM 特性

## 已实现能力

当前原生后端已经具备以下基础能力：

- `.tai` 顶层解析
- 原生执行语法解析
- HIR lowering
- MIR lowering
- runtime ABI 骨架
- 入口函数检测
- `.显示 "文本"` 语法接入
- 英文 `&&` / `||` / `!` 逻辑运算
- Windows x64 原生可执行文件输出
- `meng bench` benchmark 流程
- `meng test` 目标构建、执行与断言校验
- self-native 与 LLVM 路径的标量元素运行时数组
- self-native 与 LLVM 路径的标量成员运行时对象读取

当前 `meng build` 主线支持：

- 输入 `.meng` 或 `.tai`
- `.meng` 自动归一化为 `.tai`
- 后端参数 `self-native` / `llvm`
- 优化等级 `0` / `1` / `2`
- 当前默认正式目标：Windows x64
- 标量元素运行时数组当前在 `self-native` 与 `llvm` 都可用
- 标量成员运行时对象读取当前在 `self-native` 与 `llvm` 都可用

## 尚未完成的部分

当前仍在推进中的能力包括：

- 面向性能的完整 x64 后端优化
- 真正使用 runtime 的输出路径
- 更深层的 self-native 运行时对象能力收敛
- Linux ELF64 / macOS Mach-O 原生产物实现
- GUI 从骨架演进到完整产品闭环

## 仓库结构

```text
tailang/
├── cli/          # Go CLI，负责 .meng -> .tai -> native build 编排
├── compiler/     # Rust 原生编译器与后端
├── precompiler/  # 预编译相关实验或实现
├── gui/          # Flutter workbench skeleton
├── examples/     # 示例工程输入
├── tests/        # 项目级测试
├── docs/         # CLI、语言规范、原生后端设计文档
└── out/          # 构建输出与产物目录
```

可以直接从这些示例开始：

- `examples/hello/main.meng`
- `examples/api/main.meng`
- `examples/auth/main.meng`
- `cli/sample.tai`
- `cli/sample.meng`

## 开发与验证

常用开发验证命令：

```bash
cd compiler
cargo test
```

```bash
cd cli
go test ./...
```

文档需要和实现状态保持一致，尤其不要把尚未落地的过渡架构写成已经完成的产品能力。

## 编码与格式策略

Tailang 强制整个项目使用 UTF-8。

- 所有源码、文档、`.tai`、`.meng`、配置文件统一使用 UTF-8
- 文本文件统一使用 LF
- 禁止提交 GBK、ANSI、UTF-16 等其他编码
- `.tai` 依赖 UTF-8 文本输入，编码错误会直接破坏词法与语法解析

## `.tai` 语言风格说明

当前 `.tai` 采用点前缀、语义闭合的文本形式，中文语法和英文镜像语法并存。

中文示例关键字：

- `.程序集`
- `.子程序`
- `.如果`
- `.如果结束`
- `.判断开始`
- `.判断结束`
- `.循环判断首`
- `.循环判断尾`
- `.代码`
- `.代码结束`
- `.显示`

英文镜像示例关键字：

- `.module`
- `.subprogram`
- `.if`
- `.else`
- `.end`
- `.match`
- `.case`
- `.default`
- `.while`
- `.code`
- `.endcode`

## 更多文档

- [CLI Guide](docs/CLI_GUIDE.md)
- [`.tai v0.3` 语言规范](docs/spec/TAI_LANGUAGE_V3.md)
- [`.tai` 支持矩阵](docs/spec/TAI_SUPPORT_MATRIX.md)
- [Native Backend Architecture](docs/spec/NATIVE_BACKEND_ARCHITECTURE.md)
- [Contributing](CONTRIBUTING.md)

## 状态备注

- 旧 JSON `.tai` 快照仍可兼容读取，但不再是默认主线
- 文档应只描述已实现能力，不把中间态方案包装成正式设计
- GUI 当前是演示和骨架阶段，不代表整体产品已经闭环
