# Tailang `.tai` Schema

统一 `.tai` schema 文档见 [tai.schema.json](/C:/Users/xueyihan/.openclaw/workspace/tailang/docs/spec/tai.schema.json)。

这份 schema 是当前项目内 `.tai` 中间表示的唯一结构来源，目标是约束以下几处实现保持一致：

- Go CLI 预编译输出与校验
- Rust compiler 中的 `.tai` 反序列化与规范化
- Rust standalone precompiler 的 provider 输出规范化

## 设计原则

1. `.tai` 是结构化中间表示，不是示例文本，也不是人手写模板。
2. `.tai` 必须是稳定 JSON，便于缓存、版本控制和跨语言消费。
3. `.tai` 只承载真实存在或可稳定推断的结构，不允许为了凑模板臆造逻辑。

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

后续若调整 `.tai` 结构，必须同时更新：

1. [tai.schema.json](/C:/Users/xueyihan/.openclaw/workspace/tailang/docs/spec/tai.schema.json)
2. [compiler/src/tai.rs](/C:/Users/xueyihan/.openclaw/workspace/tailang/compiler/src/tai.rs)
3. [compiler/src/precompiler.rs](/C:/Users/xueyihan/.openclaw/workspace/tailang/compiler/src/precompiler.rs)
4. [precompiler/src/lib.rs](/C:/Users/xueyihan/.openclaw/workspace/tailang/precompiler/src/lib.rs)
5. [cli/cmd/llm.go](/C:/Users/xueyihan/.openclaw/workspace/tailang/cli/cmd/llm.go)
