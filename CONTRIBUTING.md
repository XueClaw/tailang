# Contributing

## Ground Rules

- Tailang formal source code is `.tai`
- `.tai` uses Chinese-only, dot-prefixed keywords
- All project text files must use UTF-8 and LF
- Do not introduce temporary host-language compiler paths as product-facing architecture

## Development Focus

- `compiler/` is the native backend core
- `cli/` orchestrates `.meng -> .tai -> native build`
- `gui/` is the editor/workbench surface

## Validation

- Rust changes: run `cargo test` in `compiler/`
- Go CLI changes: run `go test ./...` in `cli/`
- Keep docs aligned with actual implementation state
