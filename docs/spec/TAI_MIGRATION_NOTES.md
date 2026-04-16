# `.tai` 迁移说明

## 变更原因

PRD v2.2 重新定义了 `.tai`：

- 旧假设：`.tai` 是 JSON IR 文件
- 新定位：`.tai` 是太语言正式源码

此外，新约束已经明确：

- `.tai` 必须采用文本语法
- `.tai` 关键字必须全中文
- 不能再把英文关键字方案视为正式方向

本说明用于跟踪从旧模型迁移到新模型的过程。

## 当前状态

### 仍带有旧模型负担

- `cli/cmd/llm.go`
- `cli/cmd/precompile.go`
- `docs/spec/tai.schema.json`
- `compiler/src/tai.rs`
- `compiler/src/lib.rs` 中的兼容快照路径
- `compiler/src/codegen.rs` 中的兼容快照路径

### 已开始切换到新方向

- `docs/PRD.local.v2.2.md`
- `docs/spec/TAI_LANGUAGE_V3.md`
- `README.md`
- `docs/CLI_GUIDE.md`
- `docs/spec/TAI_SCHEMA.md`
- `compiler/src/tai_lexer.rs`
- `compiler/src/tai_parser.rs`
- `cli/cmd/validate_tai.go`

## 目标终态

1. `.meng`
   自然语言工程输入。
2. `.tai`
   使用全中文关键字的正式太语言源码。
3. `.tai.json` 或其他改名后的快照格式
   仅作为缓存、调试、传输产物。

## 已确定的中文关键字集合

- `元信息`
- `目标`
- `模块`
- `说明`
- `函数`
- `校验`
- `代码`
- `待定`

## 下一阶段工程任务

1. 让 Rust 编译器稳定解析中文 `.tai`
2. 将 `.tai` 语义模型与旧 JSON struct 继续解耦
3. 继续把旧 JSON 路径降级为 snapshot 语义
4. 让 `meng validate-tai` 以正式 `.tai` 语法为主，而不是以 JSON schema 为主
5. 把 provider 元数据收敛到兼容层与 `元信息` 声明中
