# Tailang Native Backend Architecture

## Current Scope

Tailang 当前把文本 `.tai` 视为正式源语言，原生后端围绕它构建。

当前已实现并实际验收的原生目标：

- Windows x64
- 输出格式：PE32+
- CLI 主线路径支持 `build` / `run` / `bench`

当前仍处于设计阶段的目标：

- Linux x64
- macOS

## Implemented Pipeline

当前主线编译分层：

1. `tai_parser.rs`
   解析正式 `.tai` 文本结构
2. `tai_exec.rs`
   解析原生执行语法
3. `hir.rs`
   负责语义绑定、基本类型约束、控制流表达
4. `native_ir.rs`
   降到目标无关 MIR
5. 后端发射器
   - `codegen.rs`：self-native Windows PE 路径
   - `llvm_backend.rs`：LLVM 路径

## Backend Responsibilities

### Frontend / Parser Layer

- 解析正式 `.tai`
- 保留中文关键字主线
- 为 HIR 提供稳定结构输入

### HIR Layer

- 名称绑定
- 基础类型约束
- `如果 / 循环 / 判断 / 显示 / 返回 / 调用` 等语义建模

### MIR Layer

- 常量
- 控制流
- 返回语义
- 本地槽位
- 函数调用
- 目标无关 lowering 边界

### Target Emitters

- `codegen.rs`
  self-native Windows x64 PE 发射
- `llvm_backend.rs`
  通过 LLVM IR + clang 生成 Windows 可执行文件

## Current Capability Boundary

当前已经打通或验证过的能力包括：

- 整数、布尔、文本、空返回
- 条件分支
- 循环
- `match`
- 用户函数调用
- 文本输出
- 文本相等 / 不等判断

当前仍未完成或未打通的能力包括：

- 数组 / 对象运行时
- 成员访问
- 下标访问
- 更完整的文本运行时
- 多平台原生产物

## Windows Native Notes

当前 Windows x64 self-native 路径仍然是最核心的交付后端。

使用的 Win32 API 仍然集中在：

- `KERNEL32.dll!GetStdHandle`
- `KERNEL32.dll!WriteFile`
- `KERNEL32.dll!ExitProcess`

## Near-Term Work

下一阶段更值得投入的后端工作：

1. 补全 `.tai` 语言能力缺口，而不是继续保留前端可解析但后端不能执行的语义
2. 为数组 / 对象 / 成员 / 下标访问建立可执行语义链
3. 继续收敛 self-native 与 LLVM 路径的行为一致性
4. 在保持 Windows 主线稳定的前提下，再评估 Linux / macOS 原生目标
