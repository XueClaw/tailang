# Tailang Native Backend Architecture

## Current State

Tailang now treats `.tai` as the only formal source language.

Current implemented native target:

- Windows x64
- Output format: PE32+
- Entry behavior: direct PE entry with imported Win32 calls

Current implemented lowering scope:

- `.tai` top-level parsing
- HIR lowering
- MIR lowering
- function entry detection
- runtime ABI skeleton
- native PE image emission

Current non-goals of the implemented backend:

- full expression lowering
- string runtime
- array/object runtime
- user-defined function calls
- cross-platform binary emission

## Layering

### Frontend

- `tai_parser.rs`
- `tai_exec.rs`

Responsibilities:

- parse formal `.tai`
- parse native execution syntax
- preserve Chinese-only E-language style keywords

### HIR

- structured semantic layer for `.令 / .如果 / .循环 / .判断 / .显示 / .返回`
- name binding boundary
- no target-machine details

### MIR

The project now converges on a dedicated MIR layer between HIR and emitter.

Required MIR responsibilities:

- constants
- control flow
- return semantics
- local slots
- target-independent lowering boundary

### Runtime ABI

- `tailang_rt_print_utf8`
- `tailang_rt_print_i64`
- `tailang_rt_exit`

### Target Emitters

- Windows x64: PE32+ emitter
- Linux x64: ELF64 emitter design target
- macOS: Mach-O emitter design target

The emitter must consume MIR, not `.tai` text directly.

## Cross-Platform Design Direction

### Windows x64

- status: implemented minimally
- executable kind: PE32+
- imported API:
  - `KERNEL32.dll!GetStdHandle`
  - `KERNEL32.dll!WriteFile`
  - `KERNEL32.dll!ExitProcess`

### Linux x64

- status: design only
- target output: ELF64 executable
- expected first milestone: minimal process entry + syscall-based exit

### macOS

- status: design only
- target output: Mach-O
- expected first milestone: minimal process entry + exit syscall path

## Immediate Next Backend Work

1. Replace placeholder x64 PE emission with MIR-driven backend logic
2. Route `.显示` and `.返回` through the runtime ABI
3. Add benchmark harness for Python comparisons
4. Keep Windows x64 as the only fully supported native target until backend stabilizes
