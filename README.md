# Tailang（太语言）

Tailang 是一个以 `.tai` 为正式源码、以 `.meng` 为自然语言工程输入的中文编程项目。

## Current State

- `.meng` 不是编程语言，只是自然语言工程文件
- `.tai` 才是正式太语言源码
- 当前正式工作流是 `.meng -> precompile -> .tai -> native compile`
- 当前唯一已实现并验收的原生目标是 Windows x64
- 编译器已经不再经过 `rustc` 生成宿主源码，而是直接生成最小原生 PE 可执行文件

## `.tai-first` Workflow

```bash
meng precompile src/main.meng
meng validate-tai src/main.tai
meng build src/main.tai
meng run src/main.tai
meng bench cli/bench_numeric.tai --report cli/bench_numeric.bench.json
```

说明：

- `.meng` 用来表达需求
- `.tai` 用来审查、版本控制、构建、运行、生成文档
- CLI 负责编排
- `compiler/` 负责原生编译

## Native Backend

当前原生后端状态：

- implemented: Windows x64 PE32+
- design only: Linux ELF64 / macOS Mach-O

当前已实现能力：

- `.tai` 顶层解析
- 原生执行语法解析
- HIR lowering
- MIR lowering
- runtime ABI 骨架
- 入口函数检测
- `.显示 "文本"` 语法接入
- 原生 Windows 可执行文件输出

当前未完成能力：

- 面向性能的 x64 后端
- 真正使用 runtime 的输出路径
- 数组/对象运行时
- 用户函数调用
- 多平台原生产物实现

当前已新增的 benchmark 能力：

- `meng bench` 可构建基准目标
- 可运行生成的原生程序并采集耗时
- 可运行对应 Python 基线并输出对比结果
- 可写出 JSON benchmark 报告

详见：

- [CLI Guide](docs/CLI_GUIDE.md)
- [`.tai v0.3`](docs/spec/TAI_LANGUAGE_V3.md)
- [Native Backend Architecture](docs/spec/NATIVE_BACKEND_ARCHITECTURE.md)

## Repository Layout

```text
tailang/
├── compiler/   # native compiler backend
├── cli/        # Go orchestration CLI
├── gui/        # Flutter workbench skeleton
├── docs/       # project and language docs
├── examples/   # sample inputs
└── tests/      # project-level tests
```

## Encoding Policy

Tailang 强制整个项目使用 UTF-8。

- 所有源码、文档、`.tai`、`.meng`、配置文件一律使用 UTF-8
- 文本文件统一使用 LF
- 禁止提交 GBK、ANSI、UTF-16 等其他编码
- `.tai` 是全中文语法，编码错误会直接破坏词法和语法解析

## Status Notes

- 旧 JSON `.tai` 快照仍可兼容读取，但不再是默认主线
- GUI 当前是骨架状态，不是完整产品闭环
- 文档只描述已实现能力，不再把过渡架构写成正式设计
