# Tailang CLI Guide

## Core Workflow

Tailang uses a `.tai-first` workflow.

```bash
meng precompile src/main.meng
meng validate-tai src/main.tai
meng build src/main.tai
meng run src/main.tai
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
- current native executable subset supports `.返回` and `.显示 "文本"`

### `meng run`

Build and run `.meng` or `.tai`.

Current behavior:

- uses the same build pipeline as `meng build`
- executes the produced artifact

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
- current execution model is still simplified
- not yet a full semantic test runtime for `.tai`

## UTF-8 Policy

All Tailang source inputs must be UTF-8.

- `.meng`: UTF-8 only
- `.tai`: UTF-8 only
- UTF-16: rejected
- GBK / ANSI: rejected

## Formal `.tai` Notes

- Chinese-only keywords
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
