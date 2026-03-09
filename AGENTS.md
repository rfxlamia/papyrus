# Repository Guidelines

## Project Structure & Module Organization
Papyrus is a Cargo workspace with two crates. `papyrus-core/` contains the extraction pipeline and public library API; its main modules live under `src/parser`, `src/detector`, `src/ast`, and `src/renderer`. `papyrus-cli/` wraps the core crate with the `papyrus` binary and keeps CLI concerns in `src/cli.rs`, `src/convert.rs`, and `src/warning.rs`. Cross-repo fixtures and oracle data live in `tests/fixtures/` and `tests/oracle/`. Design notes and implementation plans belong in `docs/plans/`.

## Build, Test, and Development Commands
Use Cargo from the workspace root:

```bash
cargo build --workspace
cargo test --workspace
cargo build -p papyrus-cli --release
python3 -m pytest tests/fixtures tests/oracle -q
python3 tests/fixtures/generate_fixtures.py
```

`cargo build --workspace` compiles both crates. `cargo test --workspace` runs Rust unit and integration tests. The pytest command validates parser output against the PyMuPDF oracle. Regenerate fixture baselines only when expected extraction output changes. Always use `python3`, never `python`.

## Coding Style & Naming Conventions
This repo uses Rust 2021 and standard Rust formatting conventions. Run `cargo fmt` before submitting changes, and keep code Clippy-clean when practical. Use `snake_case` for modules, functions, and test names, and `PascalCase` for types and structs. Keep public APIs documented with rustdoc examples where useful. Follow the project’s best-effort parsing philosophy: prefer warning accumulation and graceful fallback over `panic!`, `unwrap()`, or `expect()` on PDF data paths.

## Testing Guidelines
Place unit tests inline with the module they cover. Keep integration coverage in crate-level `tests/` files such as `papyrus-core/tests/integration_extraction.rs` and `papyrus-cli/tests/cli_integration.rs`. Name tests after observable behavior, for example `invalid_arguments_exit_with_code_2`. When parser behavior changes, update or add matching fixture pairs like `tests/fixtures/simple.pdf` and `tests/fixtures/simple.oracle.json`.

## Commit & Pull Request Guidelines
Recent history follows short, imperative commit subjects with optional scopes, such as `fix(parser): ...`, `refactor(cli): ...`, and `docs: ...`. Keep that pattern for new commits. For pull requests, include the motivation, a concise summary of behavior changes, and the exact commands you ran to verify the work. If CLI output or fixture baselines changed, call that out explicitly.
