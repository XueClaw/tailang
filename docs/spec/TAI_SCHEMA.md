# Tailang `.tai` JSON 兼容快照说明

统一 JSON schema 定义见 [tai.schema.json](/C:/Users/xueyihan/.openclaw/workspace/tailang/docs/spec/tai.schema.json)。

## 定位

`.tai` 当前正式主线是文本源码语言，规范见 [TAI_LANGUAGE_V3.md](/C:/Users/xueyihan/.openclaw/workspace/tailang/docs/spec/TAI_LANGUAGE_V3.md)。

本文件描述的不是 `.tai` 语言本身，而是仓库仍保留的 **JSON 兼容快照格式**。这一兼容格式仅用于旧路径兼容、快照归一化和部分预编译输出约束。

## 仍在使用兼容快照的场景

- Go CLI 的部分 LLM / 预编译输出兼容路径
- Rust compiler 中的旧版 `.tai` JSON 反序列化与规范化
- 兼容旧快照输入的校验与迁移路径

## 不再把它当作什么

- 不是 `.tai` 语言规范
- 不是默认主线输入格式
- 不是未来新增语言能力的设计中心

## 当前维护原则

1. 文本 `.tai` 是正式主线；JSON 快照只是兼容层。
2. 兼容格式允许继续存在，但不应主导新设计。
3. 若兼容格式变化，必须同步更新 schema、CLI 兼容代码和 Rust 兼容代码。

## 当前顶层字段

- `version`
- `source`
- `modules`
- `code_blocks`
- `unresolved_items`

## 同步修改点

如需修改 JSON 兼容快照格式，至少同步检查以下位置：

1. [tai.schema.json](/C:/Users/xueyihan/.openclaw/workspace/tailang/docs/spec/tai.schema.json)
2. [compiler/src/tai.rs](/C:/Users/xueyihan/.openclaw/workspace/tailang/compiler/src/tai.rs)
3. [compiler/src/precompiler.rs](/C:/Users/xueyihan/.openclaw/workspace/tailang/compiler/src/precompiler.rs)
4. [cli/cmd/llm.go](/C:/Users/xueyihan/.openclaw/workspace/tailang/cli/cmd/llm.go)
5. [cli/cmd/validate_tai.go](/C:/Users/xueyihan/.openclaw/workspace/tailang/cli/cmd/validate_tai.go)

## 建议终态

- 文本 `.tai` 继续扩展语言能力
- JSON 兼容快照逐步收缩为导入/迁移/缓存专用格式
- 新功能优先写入文本 `.tai` 规范和实现，而不是继续扩张快照语义
