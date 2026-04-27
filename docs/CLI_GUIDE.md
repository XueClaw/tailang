# Tailang CLI Guide

## Core Workflow

Tailang uses a `.tai-first` workflow.

```bash
meng precompile src/main.meng
meng validate-tai src/main.tai
meng build src/main.tai
meng run src/main.tai
meng test tests/ --backend llvm --opt-level 2
meng bench cli/bench_numeric.tai --report cli/bench_numeric.bench.json
meng doc src/
```

## Commands

### `meng precompile`

Convert `.meng` natural-language input into formal `.tai` source.

Current behavior:

- reads `.meng` as UTF-8 only
- calls configured provider
- writes normalized textual `.tai`

### `meng validate-tai`

Validate a `.tai` file.

Current behavior:

- accepts legacy JSON snapshot input for compatibility
- validates textual `.tai v0.3` source rules
- rejects non-UTF-8 text

### `meng build`

Compile `.meng` or `.tai` into a native artifact.

Current behavior:

- `.meng` is normalized to `.tai`
- `.tai` is the formal compiler input
- compiler backend is native
- currently supported native output target: Windows x64
- current native executable subset includes returns, conditionals, loops, `match`, text comparison, user function calls, and English `&&` / `||` / `!`
- runtime arrays are formal on both self-native and LLVM paths
- runtime object member and string-key reads are formal on both self-native and LLVM paths
- deeper object runtime parity still converges between the two backends

### `meng run`

Build and run `.meng` or `.tai`.

Current behavior:

- uses the same build pipeline as `meng build`
- executes the produced artifact
- nested or parity-sensitive object workloads may still prefer `--backend llvm`

### `meng bench`

Build and benchmark a `.tai` target against a Python baseline.

Current behavior:

- uses the same build pipeline as `meng build`
- builds the requested benchmark target
- runs the produced native artifact for N iterations
- runs a matching Python baseline for N iterations
- prints timing summaries and speedup
- can write a JSON report via `--report`

Example:

```bash
meng bench cli/bench_numeric.tai --backend llvm --opt-level 2 --iterations 5 --report cli/bench_numeric.bench.json
```

### `meng doc`

Generate documentation from `.tai-first` project inputs.

Current behavior:

- prefers `.tai` when both `.meng` and `.tai` exist
- only supports markdown output in current implementation
- extracts modules, functions, code blocks, unresolved items

### `meng test`

Run Tailang project tests.

Current behavior:

- command exists
- discovers `*_test.meng` / `.test.meng` test specs
- resolves matching `.tai` first, then `.meng` source
- builds the target program and executes the produced artifact
- forwards `--backend` and `--opt-level` to the test build
- supports `期望 输出 "..."` stdout line assertions
- supports `期望 退出码 N` exit-code assertions
- project-level syntax/runtime regression samples can live under `tests/syntax/`, `tests/runtime/`, and `tests/compat/`

Example:

```bash
meng test tests/ --backend llvm --opt-level 2
```

## UTF-8 Policy

All Tailang source inputs must be UTF-8.

- `.meng`: UTF-8 only
- `.tai`: UTF-8 only
- UTF-16: rejected
- GBK / ANSI: rejected

## Formal `.tai` Notes

- Chinese and English mirror keywords are both formal textual input
- dot-prefixed style
- E-language-like natural syntax
- semantic closing keywords instead of braces

Examples:

- `.程序集`
- `.子程序`
- `.参数`
- `.局部变量`
- `.如果`
- `.如果结束`
- `.判断开始`
- `.判断结束`
- `.循环判断首`
- `.循环判断尾`
- `.代码`
- `.代码结束`
- `.显示`
- `.if`
- `.while`
- `.match`
- `&&`
- `||`
- `!`
