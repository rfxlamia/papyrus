# Phase 2: Low-Level Extraction (lopdf Engine)

**Goal**: Parse PDF byte streams into raw text segments with font metadata (name, size) using `lopdf`. Implement the warning system for malformed PDFs.

**Depends on**: Phase 1 (Scaffold & The Oracle)
**Blocks**: Phase 3 (Smart Outline & Public API)

---

## Tasks

### 2.1 Implement PDF Document Loading

In the `parser` module, implement a function that takes `&[u8]` and returns a parsed `lopdf::Document`, wrapping any `lopdf` errors into `Warning::MalformedPdfObject` rather than propagating errors.

```rust
pub fn load_pdf(bytes: &[u8]) -> (Option<lopdf::Document>, Vec<Warning>)
```

Handle edge cases:
- Empty byte slice -> warning, return `None`
- Invalid PDF header -> warning, return `None`
- Password-protected PDF -> warning, return `None`

### 2.2 Implement Font Dictionary Resolution

Create a font resolver that, given a `lopdf::Document`, extracts the font dictionaries for each page:

```rust
pub struct FontInfo {
    pub name: String,       // Normalized base font name (subset prefix stripped)
    pub size: Option<f32>,  // From font descriptor if available
}

pub fn resolve_fonts(doc: &lopdf::Document, page_num: usize) -> (HashMap<Vec<u8>, FontInfo>, Vec<Warning>)
```

The `HashMap` key is the font resource name as it appears in the content stream (e.g., `F1`, `F2`). This maps resource names to resolved font metadata.

Handle:
- Missing `/BaseFont` key -> emit `Warning::MissingFontMetrics`
- Subset prefix stripping (`ABCDEF+Helvetica-Bold` -> `Helvetica-Bold`)
- Font descriptor fallback for weight and italic angle

### 2.3 Implement Content Stream Text Extraction

Parse the PDF content stream operators to extract text segments with their associated font context:

```rust
pub struct RawTextSegment {
    pub text: String,
    pub font_resource_name: Vec<u8>,  // Reference into font dictionary
    pub font_size: f32,               // From Tf operator
    pub page_number: usize,
}

pub fn extract_text_segments(doc: &lopdf::Document) -> (Vec<RawTextSegment>, Vec<Warning>)
```

Operators to handle:
- `Tf` (Set Text Font and Size) -> track current font resource name and size
- `Tj` (Show Text String) -> emit text segment with current font state
- `TJ` (Show Text with positioning) -> emit text segments, handle kerning arrays
- `Tm` (Set Text Matrix) -> track for future spatial analysis, store but don't use in v0.1
- `BT` / `ET` (Begin/End Text Object) -> reset text state

Handle:
- Unreadable content stream -> emit `Warning::UnreadableTextStream`, skip page
- Unknown encoding -> emit `Warning::UnsupportedEncoding`, attempt raw extraction
- Text state not set before `Tj` -> emit warning, use defaults

### 2.4 Implement Text Encoding/Decoding

PDF text strings can use various encodings (WinAnsi, MacRoman, Identity-H for CIDFonts, etc.). Implement a basic decoder:

- WinAnsiEncoding (most common in Western PDFs)
- Standard encoding fallback
- For CIDFont / Identity-H: attempt UTF-16BE decoding
- If encoding is unrecognized: emit `Warning::UnsupportedEncoding`, pass through raw bytes as lossy UTF-8

### 2.5 Wire Parser Into Integration Entry Point

Create a top-level parser function that orchestrates 2.1-2.4:

```rust
pub fn parse_pdf(bytes: &[u8]) -> (Vec<RawTextSegment>, DocumentMetadata, Vec<Warning>)
```

This function:
1. Loads the PDF (2.1)
2. Extracts document metadata (title, author from PDF info dict, page count)
3. For each page: resolves fonts (2.2), extracts text segments (2.3), decodes text (2.4)
4. Aggregates all warnings

### 2.6 Integration Tests Against Oracle

Write integration tests that:

1. Load each test PDF fixture
2. Run `parse_pdf()` to extract raw text segments
3. Load the corresponding `.oracle.json` baseline
4. Assert that:
   - The same text content is extracted (fuzzy match, ignoring whitespace differences)
   - Font names match (after normalization)
   - Font sizes are within a tolerance (e.g., +/- 0.1pt)
5. For `corrupted.pdf`: assert that warnings are emitted and no panic occurs

**Files created:**
- `papyrus-core/tests/integration_extraction.rs`

### 2.7 Unit Tests for Parser Internals

Write unit tests for:
- Font name normalization (subset prefix stripping)
- Text encoding/decoding (WinAnsi, UTF-16BE, fallback)
- Content stream operator parsing (Tf, Tj, TJ state tracking)
- Warning generation for each malformed-PDF scenario

---

## Definition of Done

- [ ] `parse_pdf(&[u8])` returns `(Vec<RawTextSegment>, DocumentMetadata, Vec<Warning>)`
- [ ] Font dictionary resolution correctly strips subset prefixes and extracts font names
- [ ] Content stream operators `Tf`, `Tj`, `TJ` are parsed and text segments emitted
- [ ] At least WinAnsiEncoding and UTF-16BE decoding are implemented
- [ ] `corrupted.pdf` produces warnings without panicking
- [ ] Integration tests pass against all oracle baselines (text content + font metadata)
- [ ] Unit tests cover font normalization, encoding, and operator parsing
- [ ] `cargo test` passes with all new tests green
- [ ] No `unwrap()` or `expect()` on PDF data -- all failures become warnings
