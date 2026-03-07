# Phase 1: Scaffold & The Oracle

**Goal**: Set up the Cargo workspace, define the core AST types, and build a test harness that uses PyMuPDF as a ground truth oracle.

**Depends on**: Design document (`2026-03-07-papyrus-design.md`)
**Blocks**: Phase 2 (Low-Level Extraction)

---

## Tasks

### 1.1 Initialize Cargo Workspace

Create the workspace root `Cargo.toml` with two member crates:

- `papyrus-core` (library crate)
- `papyrus-cli` (binary crate, depends on `papyrus-core`)

Add `lopdf` as a dependency to `papyrus-core`. Add `clap` as a placeholder dependency to `papyrus-cli` (version only, no implementation yet).

**Files created:**
- `Cargo.toml` (workspace root)
- `papyrus-core/Cargo.toml`
- `papyrus-core/src/lib.rs`
- `papyrus-cli/Cargo.toml`
- `papyrus-cli/src/main.rs` (stub: prints "papyrus-cli: not yet implemented")

### 1.2 Define Core AST Types

In `papyrus-core`, create the following modules and types as defined in the design doc:

- `src/ast/mod.rs` -- `Document`, `DocumentMetadata`, `Node`, `Span`, `Warning`, `ConversionResult`
- All types derive `Debug`, `Clone`, `PartialEq` for testability
- `Warning` enum with all four variants: `MissingFontMetrics`, `UnreadableTextStream`, `UnsupportedEncoding`, `MalformedPdfObject`
- `Node::RawText` as the fallback variant

**Files created:**
- `papyrus-core/src/ast/mod.rs`

**Files modified:**
- `papyrus-core/src/lib.rs` (re-export `ast` module)

### 1.3 Create Module Stubs

Create empty module directories with placeholder files for the parser, detector, and renderer. Each module exposes a public function stub that returns a `todo!()` or a dummy value.

**Files created:**
- `papyrus-core/src/parser/mod.rs` (stub)
- `papyrus-core/src/detector/mod.rs` (stub)
- `papyrus-core/src/renderer/mod.rs` (stub)

### 1.4 Collect Test PDF Fixtures

Create a `tests/fixtures/` directory at the workspace root. Add 3-5 test PDF files covering key scenarios:

- `simple.pdf` -- A basic document with one heading and body text
- `multi-heading.pdf` -- Multiple heading levels (different font sizes)
- `bold-italic.pdf` -- Text with bold and italic formatting
- `corrupted.pdf` -- A deliberately malformed PDF (truncated or missing font dict)

These can be generated programmatically using a Python script or sourced manually. The important thing is they exist and are committed.

**Files created:**
- `tests/fixtures/simple.pdf`
- `tests/fixtures/multi-heading.pdf`
- `tests/fixtures/bold-italic.pdf`
- `tests/fixtures/corrupted.pdf`

### 1.5 Build PyMuPDF Oracle Script

Create a Python script `tests/oracle/extract_oracle.py` that:

1. Takes a PDF file path as input
2. Uses PyMuPDF (`fitz`) to extract per-page text blocks with font metadata (font name, font size, flags for bold/italic)
3. Outputs a JSON file with the structure:

```json
{
  "pages": [
    {
      "page_number": 0,
      "blocks": [
        {
          "text": "Chapter 1",
          "font_name": "Helvetica-Bold",
          "font_size": 24.0,
          "is_bold": true,
          "is_italic": false
        }
      ]
    }
  ]
}
```

4. Also create a `tests/oracle/requirements.txt` with `PyMuPDF` pinned.

**Files created:**
- `tests/oracle/extract_oracle.py`
- `tests/oracle/requirements.txt`

### 1.6 Generate Oracle Baseline Files

Run `extract_oracle.py` against each test PDF fixture and save the JSON output as baseline files:

- `tests/fixtures/simple.oracle.json`
- `tests/fixtures/multi-heading.oracle.json`
- `tests/fixtures/bold-italic.oracle.json`

These baselines are committed to git and serve as the ground truth for integration tests in Phase 2.

### 1.7 Write Initial Unit Tests

Create basic tests to verify:

- All AST types can be constructed and compared (`#[derive(PartialEq)]`)
- `ConversionResult` can hold an empty document with warnings
- `Node::RawText` fallback works as expected
- The workspace compiles with `cargo build` and `cargo test` passes

**Files created/modified:**
- `papyrus-core/src/ast/mod.rs` (inline `#[cfg(test)]` module)

---

## Definition of Done

- [ ] `cargo build` succeeds with zero errors for both crates
- [ ] `cargo test` passes with all AST unit tests green
- [ ] Workspace structure matches the design doc layout
- [ ] All 4 test PDF fixtures exist in `tests/fixtures/`
- [ ] PyMuPDF oracle script runs and produces valid JSON for each fixture
- [ ] Oracle baseline `.json` files are committed alongside fixtures
- [ ] Module stubs exist for parser, detector, renderer (compiles but unimplemented)
- [ ] `papyrus-cli` binary builds and prints a stub message
