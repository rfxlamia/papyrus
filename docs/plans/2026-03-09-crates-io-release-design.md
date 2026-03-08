# Papyrus crates.io Release Design

## Overview

Release Papyrus to crates.io as two separate crates:
- `papyrus-core` — Library crate for PDF-to-Markdown conversion
- `papyrus-cli` — Command-line tool

## License

Dual license: **MIT OR Apache-2.0**

## Release Strategy

### Phase 1: papyrus-core

Release the library crate first since papyrus-cli depends on it.

**Metadata (papyrus-core/Cargo.toml):**
```toml
[package]
name = "papyrus-core"
version = "0.1.0"
edition = "2021"
authors = ["rfxlamia"]
description = "PDF-to-Markdown conversion engine with smart heading detection, bold/italic text extraction, and CommonMark output. Pure Rust, best-effort parsing for corrupted PDFs."
repository = "https://github.com/rfxlamia/papyrus"
license = "MIT OR Apache-2.0"
keywords = ["pdf", "markdown", "extract", "convert", "text-extraction"]
categories = ["text-processing", "parser-implementations", "encoding"]
```

### Phase 2: papyrus-cli

After papyrus-core is published, update and release the CLI.

**Metadata (papyrus-cli/Cargo.toml):**
```toml
[package]
name = "papyrus-cli"
version = "0.1.0"
edition = "2021"
authors = ["rfxlamia"]
description = "Command-line tool for PDF-to-Markdown conversion with smart heading detection, bold/italic extraction, and CommonMark output. Pure Rust, pipe-friendly."
repository = "https://github.com/rfxlamia/papyrus"
license = "MIT OR Apache-2.0"
keywords = ["pdf", "markdown", "cli", "convert", "extract"]
categories = ["command-line-utilities", "text-processing"]

[dependencies]
papyrus-core = "0.1.0"  # Changed from path dependency
```

## GitHub Release

Create GitHub release tag `v0.1.0` with release notes after successful crates.io publish.

```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

## Files to Create/Update

| File | Action |
|------|--------|
| `LICENSE-MIT` | Create with MIT license text |
| `LICENSE-APACHE` | Create with Apache 2.0 license text |
| `papyrus-core/Cargo.toml` | Add metadata fields |
| `papyrus-cli/Cargo.toml` | Add metadata, switch to version dependency |

## Pre-Release Verification

```bash
# Clean build
cargo clean
cargo build --release --workspace

# Tests
cargo test --workspace

# Dry-run publish
cargo publish -p papyrus-core --dry-run
cargo publish -p papyrus-cli --dry-run

# Documentation
cargo doc --workspace --no-deps
```

## Install Instructions (Post-Release)

```bash
cargo install papyrus-cli
```
