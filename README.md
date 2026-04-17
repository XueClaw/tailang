# Tailang（太语言）

**道法自然，码由心生**

Tailang 是一种创新的编程语言，让每个人都能用母语编程。

## 🎯 核心特性

- **零语法** - 会说话就会编程
- **全语言支持** - 支持 50+ 编程语言补充
- **结构化预编译** - `.meng` 预编译为可审查的 `.tai` 太语言源码
- **`.tai-first` 工作流** - `build/run/doc` 可直接消费 `.tai`
- **Provider 可插拔** - 兼容阿里云百炼、Ollama，并预留更多扩展
- **确定性** - 预编译固定温度，可版本控制
- **Rust 编译后端** - 真实代码生成应由 `compiler/` 中的 Rust 编译器负责

## 📁 项目结构

```
tailang/
├── compiler/          # Rust 编译器
│   ├── src/
│   │   ├── lib.rs
│   │   ├── lexer.rs
│   │   ├── parser.rs
│   │   ├── translator.rs
│   │   └── emitter.rs
│   └── Cargo.toml
├── cli/               # Go CLI 工具
│   ├── cmd/
│   │   ├── init.go
│   │   ├── build.go
│   │   └── run.go
│   └── go.mod
├── gui/               # Flutter GUI
│   ├── lib/
│   │   ├── main.dart
│   │   ├── screens/
│   │   └── widgets/
│   └── pubspec.yaml
├── docs/              # 文档
│   ├── CLI_GUIDE.md
│   └── spec/
├── examples/          # 示例
│   └── hello/
└── tests/             # 测试
```

## 🚀 快速开始

### 安装 CLI

```bash
# Windows
go install github.com/XueClaw/tailang/cli@latest

# 或使用 releases
```

### 创建项目

```bash
meng init my-project
cd my-project
```

### 编写代码

创建 `src/main.meng`:

```meng
邮箱密码登录 qwq

```python
import bcrypt

def verify_password(stored, provided):
    return bcrypt.checkpw(provided.encode(), stored.encode())
```
```

### 推荐工作流

```bash
# 1. 预编译为 .tai 源码
meng precompile src/main.meng

# 2. 人工审查 + 语法校验 .tai
meng validate-tai src/main.tai

# 3. 交给 Rust 编译器后端构建
meng build src/main.tai

# 4. 运行
meng run src/main.tai
```

`.tai-first` 的核心原则：

- `.meng` 负责表达需求与意图
- `.tai` 是进入版本控制、审查、构建、运行、文档生成的正式源码
- `build/run/doc` 优先直接读取 `.tai`
- `compiler/` 中的 Rust 编译器才是真实后端，Go CLI 只负责编排与入口
- `.tai` 文本语法使用易语言风格点号中文关键字，例如 `.程序集`、`.子程序`
- `.tai` 正式块语法不使用 `{}`，而是使用语义化结束词，例如 `.如果结束 / .判断结束 / .循环判断尾 / .代码结束`
- `.tai` 不只是结构说明；`.子程序` 体内直接书写原生执行语法与声明，不再依赖旧式 `.实现 .开始 ... .结束`
- 子程序执行关键字保持点号中文风格，例如 `.令 / .如果 / .否则 / .判断开始 / .循环判断首 / .返回`

### 也可直接从 `.meng` 一步到运行

```bash
meng build src/main.meng
meng run src/main.meng
```

## 📚 文档

- [CLI 指南](docs/CLI_GUIDE.md) - 命令行使用说明
- [`.tai v0.3 正式方案`](docs/spec/TAI_LANGUAGE_V3.md) - `.tai` 作为正式源码语言的设计方案（易语言风格点号中文关键字）
- [旧版 `.tai` JSON Schema](docs/spec/tai.schema.json) - 当前兼容快照格式
- [示例](examples/) - 示例代码

## ⚠️ 当前状态

- `meng precompile` 当前仍可能产出旧版 JSON 兼容快照，但产品方向已切换为文本 `.tai` 源码语言
- `meng validate-tai` 已区分旧版 JSON 快照与新版文本 `.tai`
- `meng build` / `meng run` / `meng doc` 均支持直接读取 `.tai`
- `compiler/` 正在从“消费 JSON IR”重构为“消费正式 `.tai` 源码语言”
- `compiler/` 当前主线已切到 `.tai v0.3` 行式解析与原生执行语法降级到 Rust
- Rust compiler 是后续实现原生目标产物的主战场
- `meng test` 和 `meng doc` 已可用，但整体能力仍在继续完善

## 🛠️ 技术栈

| 组件 | 语言 | 框架 |
|------|------|------|
| 编译器 | Rust | tree-sitter |
| CLI | Go | cobra |
| GUI | Dart | Flutter |
| 预编译 | Go / Rust | DashScope / Ollama / Custom Provider |

## 📋 开发计划

| 里程碑 | 日期 | 状态 |
|--------|------|------|
| MVP | 2026-05-26 | ⏳ |
| Alpha | 2026-06-30 | ⏳ |
| Beta | 2026-08-11 | ⏳ |
| v1.0 | 2026-09-01 | ⏳ |

## 🤝 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md)

## 📄 许可证

MIT License

## 🌟 Star History

[![Star History Chart](https://api.star-history.com/svg?repos=XueClaw/tailang&type=Date)](https://star-history.com/#XueClaw/tailang&Date)

---

**GitHub**: https://github.com/XueClaw/tailang  
**文档**: https://docs.tailang.org  
**社区**: 敬请期待
# 编码要求

Tailang 整个项目强制使用 `UTF-8` 编码。

- 所有源码、文档、`.tai`、`.meng`、配置文件一律使用 `UTF-8`
- 文本文件统一使用 `LF`
- 禁止提交 `GBK`、`ANSI`、`UTF-16` 等其他编码
- `.tai` 是全中文语法，编码不统一会直接破坏词法和语法解析
