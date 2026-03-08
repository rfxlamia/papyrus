# Papyrus

> PDF-to-Markdown conversion that understands document structure.

[![Crates.io](https://img.shields.io/crates/v/papyrus-cli)](https://crates.io/crates/papyrus-cli)
[![Crates.io (core)](https://img.shields.io/crates/v/papyrus-core?label=core)](https://crates.io/crates/papyrus-core)
[![docs.rs](https://docs.rs/papyrus-core/badge.svg)](https://docs.rs/papyrus-core)
[![GitHub Sponsors](https://img.shields.io/github/sponsors/rfxlamia?color=brightgreen)](https://github.com/sponsors/rfxlamia)

Papyrus extracts text from PDFs and converts it to clean, structured Markdown. It detects headings, bold, and italic formatting—producing CommonMark output that works with LLMs, knowledge bases, and Markdown-based tools.

## Features

- **Structure-aware extraction** — Detects heading hierarchy (H1–H4) from font sizes
- **Formatting preservation** — Identifies bold and italic text
- **Best-effort parsing** — Handles corrupted PDFs gracefully with warning reports
- **Pipe-friendly CLI** — Works with stdin/stdout for shell pipelines
- **Batch conversion** — Convert entire directories with progress bars
- **Rust-native** — Single static binary, no runtime dependencies

## Installation

### From Source

Requires Rust 1.70+:

```bash
git clone https://github.com/rfxlamia/papyrus.git
cd papyrus
cargo build --release
```

The binary will be at `./target/release/papyrus`.

### Cargo Install

```bash
cargo install papyrus-cli
```

## Quick Start

### CLI Usage

Convert a single PDF:

```bash
papyrus convert document.pdf -o output.md
```

Convert from stdin:

```bash
cat document.pdf | papyrus convert - > output.md
```

Batch convert a directory:

```bash
papyrus convert ./pdfs/ -o ./markdown/
```

### Library Usage

```rust
use papyrus_core;

// Zero-config extraction
let result = papyrus_core::convert(&pdf_bytes);
let markdown = result.to_markdown();

// Or with custom configuration
let papyrus = papyrus_core::Papyrus::builder()
    .heading_size_ratio(1.2)
    .detect_bold(true)
    .detect_italic(true)
    .build();

let result = papyrus.extract(&pdf_bytes);
```

## CLI Options

```
papyrus convert [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input PDF file or directory. Use "-" for stdin

Options:
  -o, --output <OUTPUT>      Output file or directory
      --heading-ratio <RATIO>  Heading size ratio threshold [default: 1.2]
      --no-bold                Disable bold detection
      --no-italic              Disable italic detection
      --quiet                  Suppress warning output
  -h, --help                 Print help
```

## What Gets Extracted

| Element | Status |
|---------|--------|
| Text content | ✅ Full support |
| Headings (H1–H4) | ✅ Detected via font size |
| Bold formatting | ✅ From font names/descriptors |
| Italic formatting | ✅ From font names/descriptors |
| Tables | ❌ Plain text only |
| Images | ❌ Not extracted |
| Lists | ❌ Plain text only |
| Links | ❌ Not preserved |

Papyrus focuses on **semantic structure** (what the text means) rather than **visual layout** (how it looked on the page).

## Exit Codes

- `0` — Success (warnings may still be present)
- `1` — I/O error (file not found, permission denied)
- `2` — Invalid arguments

## Testing

```bash
# Run all Rust tests
cargo test --workspace

# Run with oracle validation (requires PyMuPDF)
python3 -m pytest tests/fixtures tests/oracle -q
```

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development setup, architecture overview, and contribution guidelines.

## License

MIT OR Apache-2.0 — See [LICENSE-MIT](./LICENSE-MIT) or [LICENSE-APACHE](./LICENSE-APACHE) for details.

## Support

If you find Papyrus useful, consider [sponsoring the project](https://github.com/sponsors/rfxlamia) ❤️
