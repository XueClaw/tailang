# Tailang `.tai` Support Matrix

This matrix tracks the implementation state of the formal textual `.tai` language.

Status labels:

- `parser-ready`: accepted by textual `.tai` parsing
- `HIR-ready`: lowered into semantic HIR
- `LLVM-ready`: executable through the LLVM backend
- `self-native-ready`: executable through the Windows x64 self-native backend
- `tested`: covered by parser/HIR/backend or CLI regression tests

| Capability | parser-ready | HIR-ready | LLVM-ready | self-native-ready | tested | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `.版本` / `.version` | yes | n/a | n/a | n/a | yes | Parsed as top-level program metadata. |
| `.程序集` / `.module` | yes | n/a | n/a | n/a | yes | Parsed as top-level module declarations. |
| `.子程序` / `.subprogram` with inline params | yes | yes | yes | yes | yes | Main formal function signature path. |
| Typed local declarations (`name: int = 0`) | yes | yes | yes | yes | yes | Works for Chinese and English type names already accepted by `TaiType`. |
| Chinese `如果 / 否则 / 如果结束` | yes | yes | yes | yes | yes | Formal control-flow path. |
| English `.if / .else / .end` | yes | yes | yes | yes | yes | End-to-end covered for boolean logic and returns. |
| Chinese `循环判断首 / 循环判断尾` | yes | yes | yes | yes | yes | Formal loop path. |
| English `.while / .end` | yes | yes | yes | yes | partial | Parser and execution path exist; dedicated project-level regression samples should remain in `tests/`. |
| Chinese `判断开始 / 判断 / 默认 / 判断结束` | yes | yes | yes | yes | yes | Formal match path. |
| English `.match / .case / .default / .end` | yes | yes | yes | yes | partial | Parser and execution path exist; project-level regression coverage should remain in `tests/`. |
| Logical operators `并且 / 或者 / 非` | yes | yes | yes | yes | yes | Chinese boolean ops fully supported. |
| Logical operators `&& / \|\| / !` | yes | yes | yes | yes | yes | English C-style boolean ops supported in formal exec syntax. |
| Integer / boolean / text literals | yes | yes | yes | yes | yes | Shared formal scalar path. |
| User function calls | yes | yes | yes | yes | yes | Direct named calls only. |
| Constant object literals | yes | yes | n/a | n/a | yes | Supported through constant-folded collection flow. |
| Constant member access / string-key object index | yes | yes | n/a | n/a | yes | Works only when the object is statically evaluable. |
| Runtime array literals and index access | yes | yes | yes | no | yes | `self-native` rejects runtime arrays and must point users to `--backend llvm`. |
| Runtime object literals and member access | yes | no | no | no | partial | Parsed today, but still lowered only through constant-folded collection semantics. |
| Direct array printing | yes | yes | no | no | yes | Not yet a formal runtime capability in either backend. |
| Direct object printing | yes | no | no | no | no | Not yet part of the formal runtime surface. |

## Current Policy

- Formal `.tai` work should prioritize closing `parser-ready` to executable gaps instead of adding new syntax-only surface area.
- Runtime collections are currently split:
  - Arrays are formal on the LLVM path.
  - Objects remain constant-collection-only until a runtime object model is added.
- Any new syntax or semantic feature should land with at least:
  - one parser or lexer regression test,
  - one HIR or MIR lowering test,
  - one backend or CLI integration test.
