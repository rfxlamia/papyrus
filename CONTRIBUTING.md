# Contributing to Papyrus

Thank you for your interest in contributing to Papyrus! This document provides technical details about the architecture, development workflow, and contribution opportunities.

## Table of Contents

- [Development Setup](#development-setup)
- [Architecture Overview](#architecture-overview)
- [Project Philosophy](#project-philosophy)
- [Contribution Areas](#contribution-areas)
- [Code Standards](#code-standards)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)

## Development Setup

### Prerequisites

- Rust 1.70+ with Cargo
- Python 3.8+ (for oracle testing)
- PyMuPDF and ReportLab: `pip install pymupdf reportlab`

### Build

```bash
# Build all crates
cargo build --workspace

# Build core library only
cargo build -p papyrus-core

# Build CLI only
cargo build -p papyrus-cli

# Release build
cargo build --release
```

### Run Tests

```bash
# Rust tests
cargo test --workspace

# Oracle tests (Python)
python3 -m pytest tests/fixtures tests/oracle -q

# Regenerate test fixtures
python3 tests/fixtures/generate_fixtures.py
```

## Architecture Overview

Papyrus is organized as a Cargo workspace with two crates:

```
papyrus/
├── papyrus-core/       # Library crate ("the brain")
│   ├── parser/         # PDF parsing with lopdf
│   ├── detector/       # Smart outline detection
│   ├── ast/            # Document type definitions
│   └── renderer/       # CommonMark output
└── papyrus-cli/        # Binary crate ("the hands")
    ├── cli.rs          # Argument parsing
    ├── convert.rs      # Conversion logic
    └── warning.rs      # Warning formatting
```

### Four-Stage Pipeline

The extraction follows a strict pipeline:

1. **Parser** (`parser/mod.rs`)
   - Loads PDF using `lopdf`
   - Extracts text operators (Tf, Tm, Tj, TJ)
   - Resolves font dictionaries
   - Decodes text (UTF-16BE, WinAnsiEncoding)
   - Returns `RawTextSegment` vectors

2. **Detector** (`detector/mod.rs`)
   - Computes body font size (mode-based)
   - Classifies headings by font-size ratio:
     - H1: ratio >= 2.0
     - H2: ratio >= 1.7
     - H3: ratio >= 1.4
     - H4: ratio >= heading_size_ratio (default 1.2)
   - Detects bold/italic from font names and descriptors
   - Builds classified `Node` elements

3. **AST** (`ast/mod.rs`)
   - `Document`: Root with metadata and nodes
   - `Node`: Heading, Paragraph, or RawText
   - `Span`: Text with formatting flags
   - `Warning`: Non-fatal issues encountered

4. **Renderer** (`renderer/mod.rs`)
   - Converts AST to CommonMark Markdown
   - Proper escaping of special characters
   - Bold: `**text**`, Italic: `*text*`

### Public API

The library exposes a builder-pattern API:

```rust
// Zero-config
let result = papyrus_core::convert(&pdf_bytes);

// Configured
let papyrus = papyrus_core::Papyrus::builder()
    .heading_size_ratio(1.2)
    .detect_bold(true)
    .detect_italic(true)
    .build();
let result = papyrus.extract(&pdf_bytes);
```

Key contracts:
- `extract()` never returns `Err` — always produces *something*
- Warnings accumulate in `ConversionResult.warnings`
- Page numbers are **1-based** in the API

## Project Philosophy

### Best-Effort Extraction

The pipeline never fails completely. Even corrupted PDFs produce some output with appropriate warnings. This philosophy is implemented via:

- Warning accumulation instead of early returns
- Graceful degradation (e.g., RawText node when structure detection fails)
- Unicode replacement character (`U+FFFD`) for undecodable sequences

### Semantic Over Visual

Papyrus prioritizes **document structure** over **visual fidelity**:

- Headings detected by font-size ratio, not position
- Bold/italic detected by font metrics, not glyph shapes
- Output is CommonMark, not HTML with CSS

### Oracle-Based Validation

PyMuPDF serves as the ground-truth oracle for testing:

- Fixtures in `tests/fixtures/*.pdf`
- Expected outputs in `tests/fixtures/*.oracle.json`
- Regenerate with `python3 tests/fixtures/generate_fixtures.py`

## Contribution Areas

### High Priority

#### 1. Line Break Detection

**Problem**: Text within paragraphs is concatenated without line breaks.

**Approach**: Track vertical positioning (Tm operator y-coordinates) to detect:
- Line breaks within paragraphs
- Paragraph boundaries via spacing analysis

**Files**: `papyrus-core/src/parser/mod.rs`

#### 2. Improved Word Spacing

**Problem**: Current heuristic (-100 threshold in TJ arrays) doesn't work for all PDF generators.

**Approach**: Use actual font metrics (character widths from FontDescriptor) instead of heuristic thresholds.

**Files**: `papyrus-core/src/parser/mod.rs`

#### 3. Table Detection

**Problem**: Tables are extracted as plain text without structure.

**Approach**: Spatial analysis of text positions to identify grid structures, then output GFM-style Markdown tables.

**Files**: New module `papyrus-core/src/detector/tables.rs`

### Medium Priority

#### 4. Multi-Column Layout

**Problem**: Multi-column documents may have interleaved text.

**Approach**: Spatial analysis to determine reading order based on x-coordinates.

**Files**: `papyrus-core/src/detector/mod.rs`

#### 5. Image Extraction

**Problem**: Images are completely ignored.

**Approach**: Extract embedded images using `lopdf`, generate references in Markdown.

**Files**: `papyrus-core/src/parser/images.rs`

#### 6. Hyperlink Preservation

**Problem**: URL annotations are not extracted.

**Approach**: Parse annotation dictionaries, convert to Markdown links.

**Files**: `papyrus-core/src/parser/mod.rs`

### Low Priority

#### 7. Extended Metadata

**Problem**: Only title, author, and page_count are extracted.

**Approach**: Extract additional fields (subject, keywords, creator, producer, dates) and optionally output as YAML frontmatter.

**Files**: `papyrus-core/src/parser/mod.rs`, `papyrus-core/src/ast/mod.rs`

#### 8. Performance Optimization

**Problem**: Entire PDF loaded into memory.

**Approach**: Stream large documents page-by-page; parallelize batch conversion.

**Files**: `papyrus-core/src/lib.rs`, `papyrus-cli/src/convert.rs`

### Testing Contributions

- **Real-world PDF corpus**: We currently have only 5 test fixtures
- **Fuzzing tests**: No property-based testing exists yet
- **Benchmarks**: No performance benchmarks exist

## Code Standards

### Error Handling

```rust
// ✅ DO: Accumulate warnings
let mut warnings: Vec<Warning> = Vec::new();
// ... push warnings as they occur ...

// ❌ DON'T: panic on unexpected PDF data
// No unwrap()/expect() on PDF data paths
```

### Font Names

```rust
// Font resource names are Vec<u8>, not String
// PDF names can be any bytes, not just valid UTF-8
let resource_name: Vec<u8> = ...;
```

### Page Numbers

```rust
// Public API uses 1-based page numbers (PDF spec convention)
// Internal implementation may use 0-based indexing
```

### Testing

```rust
// Every module has inline tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

### Documentation

- Add doc comments to public APIs
- Include example code in rustdoc
- Document edge cases and limitations

## Testing

### Unit Tests

Unit tests live inline in each module:

```bash
cargo test -p papyrus-core
cargo test -p papyrus-cli
```

### Integration Tests

Integration tests are in `tests/` directories:

```bash
# Core integration tests
cargo test --test integration_test -p papyrus-core

# CLI integration tests
cargo test --test cli_integration -p papyrus-cli
```

### Oracle Tests

Oracle tests compare Papyrus output against PyMuPDF:

```bash
# Generate oracle files
python3 tests/fixtures/generate_fixtures.py

# Run pytest
python3 -m pytest tests/fixtures tests/oracle -q
```

### CommonMark Compliance

The renderer output is tested for CommonMark compliance using `pulldown-cmark`:

```rust
// Round-trip test: render -> parse -> verify no panics
let markdown = result.to_markdown();
let parser = pulldown_cmark::Parser::new(&markdown);
for _ in parser {} // Just verify it parses without error
```

## Submitting Changes

1. **Fork and branch**: Create a feature branch from `main`
2. **Write tests**: Add unit tests for new logic, integration tests for features
3. **Update documentation**: Reflect changes in doc comments and this file if needed
4. **Run full test suite**:
   ```bash
   cargo test --workspace
   python3 -m pytest tests/fixtures tests/oracle -q
   ```
5. **Submit PR**: Include description of changes and motivation

## Questions?

Open an issue for:
- Bug reports
- Feature requests
- Questions about architecture or implementation

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
