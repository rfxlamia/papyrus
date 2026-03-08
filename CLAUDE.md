# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Papyrus is a high-performance PDF-to-Markdown conversion engine written in pure Rust. It reads PDFs like a human would: understanding hierarchy (headings), emphasis (bold, italic), and document flow.

## Workspace Structure

This is a Cargo workspace with two crates:

- **papyrus-core/**: The library crate containing the extraction engine
  - `parser/`: Low-level PDF parsing (text operators, font dictionaries) using `lopdf`
  - `detector/`: Smart Outline Detector (heading levels, bold/italic heuristics)
  - `ast/`: Document AST types (`Document`, `Node`, `Span`, `Warning`)
  - `renderer/`: CommonMark output renderer
- **papyrus-cli/**: The CLI binary (currently stubbed)

## Common Commands

### Build
```bash
cargo build --workspace          # Build all crates
cargo build -p papyrus-core      # Build core library only
cargo build -p papyrus-cli       # Build CLI only
```

### Test
```bash
cargo test --workspace           # Run all Rust tests
cargo test -p papyrus-core       # Run core library tests only
cargo test <test_name> -v        # Run a specific test with verbose output
```

### Python Tests (Oracle)
```bash
python3 -m pytest tests/fixtures tests/oracle -q
python3 tests/oracle/extract_oracle.py tests/fixtures/simple.pdf --out /tmp/out.json
```

### Generate Fixtures
```bash
python3 tests/fixtures/generate_fixtures.py
```

## Architecture Patterns

### Four-Stage Pipeline
The extraction follows a strict pipeline:

1. **Parser** (`parser/mod.rs`): Uses `lopdf` to read PDF object trees, extract text operators (Tf, Tm, Tj, TJ), resolve font dictionaries. Returns `RawTextSegment` vectors.

2. **Detector** (`detector/mod.rs`): Computes body font size (mode), classifies headings by font-size ratio, detects bold/italic from font names/descriptors. Returns classified `Node` elements.

3. **AST** (`ast/mod.rs`): Strongly-typed document representation with `Document`, `Node` (Heading/Paragraph/RawText), `Span`, and `Warning` enums.

4. **Renderer** (`renderer/mod.rs`): Converts AST to CommonMark Markdown.

### Public API

The library exposes a builder-pattern API:

```rust
// Zero-config extraction
let result = papyrus_core::convert(&pdf_bytes);

// Configured extraction
let papyrus = Papyrus::builder()
    .heading_size_ratio(1.2)
    .detect_bold(true)
    .detect_italic(true)
    .build();
let result = papyrus.extract(&pdf_bytes);
```

Key contracts:
- `extract()` never returns `Err` — it always produces *something*, even if the PDF is corrupted
- Warnings are accumulated in `ConversionResult.warnings` rather than causing failures
- Page numbers are **1-based** in the API (PDF spec convention)

### Error Handling Philosophy

Best-effort with warning accumulation. The pipeline continues even when:
- Font metrics are missing → emits `Warning::MissingFontMetrics`, creates `Node::RawText`
- Text streams are unreadable → emits `Warning::UnreadableTextStream`
- PDF is corrupted → emits `Warning::MalformedPdfObject`, returns empty `Document`

### Testing Strategy

**Oracle-based validation**: PyMuPDF serves as the ground-truth oracle:
- `tests/fixtures/`: PDF test fixtures (`simple.pdf`, `bold-italic.pdf`, `multi-page.pdf`, `corrupted.pdf`)
- `tests/fixtures/*.oracle.json`: Baseline JSON outputs from PyMuPDF
- `tests/oracle/extract_oracle.py`: PyMuPDF extraction script
- Regenerate fixtures: `python3 tests/fixtures/generate_fixtures.py`

**Rust unit tests**: Each module has inline `#[cfg(test)]` tests covering:
- PDF loading (empty, corrupted, valid)
- Font resolution (subset prefix stripping, weight/angle extraction)
- Text extraction (operator order, font size from Tf state)
- Heading detection (ratio boundaries H1-H4)
- Bold/italic detection (font name patterns + descriptor fallback)

### Font Handling Conventions

- Font resource names are `Vec<u8>` (PDF names can be any bytes)
- Subset prefixes are stripped: `ABCDEF+Helvetica-Bold` → `Helvetica-Bold`
- Font size comes from `Tf` operator state, not the font descriptor
- Bold/italic detection: first check font name patterns, then fall back to `FontDescriptor` metrics (`FontWeight > 600`, `ItalicAngle != 0`)

### Encoding

PDF strings are decoded using:
1. UTF-16BE if BOM present (`FE FF`)
2. UTF-16BE heuristic if starts with `0x00` and even length
3. WinAnsiEncoding (PDF spec §D.1) as fallback

### String Decoding

PDF text strings use `decode_pdf_string()` which handles:
- UTF-16BE with/without BOM
- WinAnsiEncoding (0x80-0x9F range mapped per PDF spec)
- Invalid sequences produce Unicode replacement character (`U+FFFD`)
