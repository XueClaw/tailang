# Tailang（太语言）

**道法自然，码由心生**

Tailang 是一种创新的编程语言，让每个人都能用母语编程。

## 🎯 核心特性

- **零语法** - 会说话就会编程
- **全语言支持** - 支持 50+ 编程语言补充
- **结构化预编译** - `.meng` 预编译为稳定的 `.tai` JSON IR
- **Provider 可插拔** - 兼容阿里云百炼、Ollama，并预留更多扩展
- **确定性** - 预编译固定温度，可版本控制

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

### 编译运行

```bash
# 预编译为 .tai JSON
meng precompile src/main.meng

# 校验 .tai 结构
meng validate-tai src/main.tai

# 编译
meng build src/main.meng

meng run src/main.meng
```

## 📚 文档

- [CLI 指南](docs/CLI_GUIDE.md) - 命令行使用说明
- [`.tai` Schema](docs/spec/tai.schema.json) - 统一中间表示结构定义
- [`.tai` Schema 说明](docs/spec/TAI_SCHEMA.md) - Go/Rust 共用约束说明
- [示例](examples/) - 示例代码

## ⚠️ 当前状态

- `meng precompile` 已输出规范化 `.tai` JSON
- `meng validate-tai` 已可校验 `.tai` 是否符合共享 schema
- `meng build` 当前仍生成占位可执行文件，真实编译后端仍在完善中
- `meng test` 和 `meng doc` 已有命令入口，但实现仍未完全成熟

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
