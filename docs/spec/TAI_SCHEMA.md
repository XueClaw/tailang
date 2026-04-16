# Tailang `.tai` JSON Snapshot Schema

统一 `.tai` schema 文档见 [tai.schema.json](/C:/Users/xueyihan/.openclaw/workspace/tailang/docs/spec/tai.schema.json)。

注意：根据 PRD v2.2，`.tai` 的产品定位已经调整为“太语言正式源码”。
本文件描述的 **不是 `.tai` 语言本身**，而是当前仓库仍在使用的 **JSON 兼容快照格式**。

新的语言方向见 [TAI_LANGUAGE_V3.md](/C:/Users/xueyihan/.openclaw/workspace/tailang/docs/spec/TAI_LANGUAGE_V3.md)。

这份 schema 当前只用于约束以下兼容实现保持一致：

- Go CLI 预编译输出与校验
- Rust compiler 中的旧版 `.tai` JSON 反序列化与规范化
- Rust standalone precompiler 的 provider 输出规范化

## 设计原则

1. 该格式是结构化快照，不是最终 `.tai` 文本语言规范。
2. 该格式必须是稳定 JSON，便于缓存、版本控制和跨语言消费。
3. 该格式只承载真实存在或可稳定推断的结构，不允许为了凑模板臆造逻辑。

## 顶层字段

- `version`
- `source`
- `modules`
- `code_blocks`
- `unresolved_items`

## 当前实现约定

- `source.temperature` 当前以字符串保存，保证 Go/Rust 序列化一致。
- `linked_to` 为可选字段。
- `unresolved_items` 用于承载信息不足但不应臆造补全的语义空缺。

## 维护规则

后续若调整这个 JSON 快照格式，必须同时更新：

1. [tai.schema.json](/C:/Users/xueyihan/.openclaw/workspace/tailang/docs/spec/tai.schema.json)
2. [compiler/src/tai.rs](/C:/Users/xueyihan/.openclaw/workspace/tailang/compiler/src/tai.rs)
3. [compiler/src/precompiler.rs](/C:/Users/xueyihan/.openclaw/workspace/tailang/compiler/src/precompiler.rs)
4. [precompiler/src/lib.rs](/C:/Users/xueyihan/.openclaw/workspace/tailang/precompiler/src/lib.rs)
5. [cli/cmd/llm.go](/C:/Users/xueyihan/.openclaw/workspace/tailang/cli/cmd/llm.go)
