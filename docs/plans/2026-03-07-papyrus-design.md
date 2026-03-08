# Papyrus Design Document

> "Unlocking the Digital Fossil" -- A high-performance PDF-to-Markdown conversion engine in pure Rust.

## 1. Problem Statement

PDF is a prison for information. When text is copied from a PDF, structural context (headings, emphasis, logical flow) is lost. Modern tools -- LLMs, knowledge bases (Obsidian), Markdown-based writing apps -- need clean, structured Markdown. PDF is rigid and not responsive. Papyrus bridges the gap by reading PDFs like a human reader: understanding hierarchy, emphasis, and document flow.

## 2. Strategy

Papyrus is **not** a literal port of PyMuPDF. It is inspired by lessons learned from PyMuPDF prototyping, but built from scratch in idiomatic Rust. PyMuPDF serves as a **validation oracle** for testing -- a ground truth to compare output against during development.

### Why Rust?

- **Performance**: Process thousands of pages in milliseconds. Single static binary, no runtime.
- **Reliability**: Rust's error handling prevents crashes on malformed PDFs.
- **Portability**: Easy integration into desktop apps (Tauri), CLI tools, server backends, and future WASM targets.

### Why `lopdf`?

- **Pure Rust**: No C dependencies. Clean cross-compilation, small binaries, memory-safe.
- **Full control**: Direct access to the PDF object tree (Dictionaries, Streams, Arrays). No abstractions hiding the font metrics and glyph positioning data that the Smart Outline Detector needs.
- **Text Matrix access**: PDF stores text as graphic operators (`Tf`, `Tm`, `Tj`). `lopdf` exposes these raw instructions without opinionated processing.

## 3. Architecture Overview

### Workspace Layout

```
papyrus/
├── Cargo.toml              # Workspace root
├── papyrus-core/           # The "Brain" — library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs           # Public API surface
│       ├── parser/          # PDF stream parsing (text operators, font dictionaries)
│       ├── detector/        # Smart Outline Detector (heading, bold, italic heuristics)
│       ├── ast/             # Document AST types (Node, Span, Warning, etc.)
│       └── renderer/        # CommonMark output renderer
└── papyrus-cli/            # The "Hands" — binary crate
    ├── Cargo.toml
    └── src/
        └── main.rs          # CLI interface (clap, file I/O, progress, colored warnings)
```

### Key Dependencies

| Crate | Purpose | Lives in |
|-------|---------|----------|
| `lopdf` | Low-level PDF object tree access | `papyrus-core` |
| `clap` | CLI argument parsing | `papyrus-cli` |
| `indicatif` | Progress bars for batch processing | `papyrus-cli` |
| `owo-colors` | Colored terminal output for warnings | `papyrus-cli` |

### Data Flow

```
PDF bytes
  │
  ▼
┌──────────────────────────────────────────────┐
│  papyrus-core                                │
│                                              │
│  1. Parser (lopdf)                           │
│     └─ Read PDF object tree                  │
│     └─ Extract text operators (Tf, Tm, Tj)   │
│     └─ Resolve font dictionaries             │
│                                              │
│  2. Detector (Smart Outline)                 │
│     └─ Compute relative font-size hierarchy  │
│     └─ Inspect font names for Bold/Italic    │
│     └─ Assign heading levels (H1-H6)        │
│     └─ Tag inline formatting (bold, italic)  │
│                                              │
│  3. AST (Document)                           │
│     └─ Vec<Node> with structured metadata    │
│                                              │
│  4. Renderer                                 │
│     └─ AST → CommonMark string               │
└──────────────────────────────────────────────┘
  │
  ▼
ConversionResult { document, warnings }
```

## 4. Core Types (AST)

```rust
/// The top-level result returned by every conversion.
pub struct ConversionResult {
    pub document: Document,
    pub warnings: Vec<Warning>,
}

/// A structured representation of the PDF content.
pub struct Document {
    pub metadata: DocumentMetadata,
    pub nodes: Vec<Node>,
}

pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: usize,
}

/// A block-level element in the document.
pub enum Node {
    Heading {
        level: u8,        // 1-6
        spans: Vec<Span>,
    },
    Paragraph {
        spans: Vec<Span>,
    },
    /// Raw text that couldn't be classified
    RawText(String),
}

/// An inline segment of text with formatting metadata.
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub font_size: f32,
    pub font_name: Option<String>,
}

/// A non-fatal issue encountered during conversion.
pub enum Warning {
    MissingFontMetrics { font_name: String, page: usize },
    UnreadableTextStream { page: usize, detail: String },
    UnsupportedEncoding { encoding: String, page: usize },
    MalformedPdfObject { detail: String },
}
```

### Design Rationale

- `Span` carries raw font metadata alongside boolean flags. Callers can inspect *why* something was marked bold for debugging or overriding.
- `Node::RawText` is the fallback. If the detector can't classify a text block, it still appears in output rather than being silently dropped. Aligns with the "best effort" philosophy.
- `Warning` is an enum, not a string. Callers can pattern-match on specific warning types for programmatic handling.

### Parser Conventions (Phase 2 Clarifications)

- Parser-layer warning page fields are **1-based** (`page = 1` is the first page).
- Parser load/open failures are normalized into `Warning::MalformedPdfObject` with descriptive `detail` text.
- `RawText` fallback nodes are created by higher extraction/detection layers, not by low-level parser internals.

## 5. Public API Surface

```rust
// ─── Builder ───
pub struct Papyrus { /* internal config */ }

pub struct PapyrusBuilder {
    heading_size_ratio: f32,    // default: 1.2 (20% larger than body = heading)
    detect_bold: bool,          // default: true
    detect_italic: bool,        // default: true
}

impl PapyrusBuilder {
    pub fn heading_size_ratio(mut self, ratio: f32) -> Self;
    pub fn detect_bold(mut self, enabled: bool) -> Self;
    pub fn detect_italic(mut self, enabled: bool) -> Self;
    pub fn build(self) -> Papyrus;
}

impl Papyrus {
    pub fn builder() -> PapyrusBuilder;

    /// Phase 1: Extract structured AST from PDF bytes
    pub fn extract(&self, pdf_bytes: &[u8]) -> ConversionResult;
}

// ─── Document rendering (Phase 2) ───
impl Document {
    /// Render to CommonMark string
    pub fn to_markdown(&self) -> String;
}

// ─── Convenience function for simple use cases ───
pub fn convert(pdf_bytes: &[u8]) -> ConversionResult;
```

### Key Contracts

- `extract()` never returns `Err`. It always produces *something*. Even if the PDF is heavily corrupted, you get a `Document` with `RawText` nodes and a full `warnings` vec.
- `convert()` is a one-liner convenience using default config.
- `to_markdown()` lives on `Document`, not on `Papyrus`. Callers can manipulate the AST before rendering. The AST is the currency, not the engine.

## 6. Smart Outline Detector (Heuristics)

### Heading Detection Algorithm

1. **First pass**: Collect all text segments with their font sizes.
2. **Compute body size**: The most frequently occurring font size on the page (mode, not mean -- avoids skew from headings).
3. **Classify each segment**:
   - `ratio = segment.font_size / body_size`
   - If `ratio >= heading_size_ratio` (default 1.2), classify as Heading.
   - Map ratio to level:
     - `ratio >= 2.0` -> H1
     - `ratio >= 1.7` -> H2
     - `ratio >= 1.4` -> H3
     - `ratio >= 1.2` -> H4
   - H5/H6 reserved for future spatial analysis (indentation, etc.)

### Bold/Italic Detection Algorithm

1. Extract font name from PDF Font Dictionary (`BaseFont` key).
2. Normalize: lowercase, strip subset prefix (e.g., `ABCDEF+` -> `""`).
3. Pattern match on font name:
   - Contains `"bold"` -> `is_bold = true`
   - Contains `"italic"` -> `is_italic = true`
   - Contains `"oblique"` -> `is_italic = true` (oblique ~ italic in PDF world)
   - Contains `"bolditalic"` -> both = true
4. Fallback: If `FontDescriptor` exists, check:
   - `FontWeight > 600` -> `is_bold = true`
   - `ItalicAngle != 0` -> `is_italic = true`

### Edge Cases & Graceful Degradation

| Scenario | Behavior |
|----------|----------|
| All text same font size | No headings detected, everything becomes `Paragraph` |
| Font dictionary missing | Emit `Warning::MissingFontMetrics`, text becomes `RawText` |
| Embedded/subset font with garbled name | Skip bold/italic detection, emit warning, still extract text |
| Empty page | Skip, no node emitted, no warning |
| Encrypted/password-protected PDF | Emit warning, return empty `Document` |

## 7. CLI Interface (`papyrus-cli`)

### Usage

```bash
# Single file conversion
papyrus convert input.pdf -o output.md

# Batch conversion (directory)
papyrus convert ./pdfs/ -o ./markdown/

# Pipe-friendly (stdin/stdout)
cat input.pdf | papyrus convert > output.md

# With custom heuristics
papyrus convert input.pdf -o output.md --heading-ratio 1.5 --no-bold
```

### Output Behavior

- **Single file**: Writes `.md` to specified path. Warnings to stderr.
- **Batch mode**: Converts all `.pdf` files in directory, preserving filenames (`report.pdf` -> `report.md`). Progress bar via `indicatif`.
- **Pipe mode**: PDF from stdin, Markdown to stdout, warnings to stderr. Unix-friendly.

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success (warnings may still exist) |
| `1` | Fatal error (file not found, unreadable, permission denied) |
| `2` | Invalid arguments |

Warnings never cause a non-zero exit. The CLI always tries to produce output, matching the core library's "best effort" philosophy.

## 8. Output Format

**CommonMark** (strict specification). Reasons:

- Perfect fit for v0.1 scope (headings, bold, italic are all first-class CommonMark).
- Unambiguous parsing -- critical for downstream consumption by Ronin and other tools.
- Progressive enhancement to GFM (tables, strikethrough) is trivial once spatial analysis lands in future versions.
- No vendor lock-in to specific tools like Obsidian.

## 9. Decisions Summary

| Decision | Choice |
|----------|--------|
| Primary target | Rust crate (library-first) |
| Secondary target | CLI binary |
| PDF parsing foundation | `lopdf` (pure Rust, full control) |
| v0.1 detection scope | Font-size hierarchy + bold/italic via font names |
| Output format | CommonMark |
| API shape | Two-phase (Builder -> AST -> Render) with `ConversionResult` |
| Error handling | Best effort with `Vec<Warning>`, never crashes |
| Workspace structure | Cargo workspace: `papyrus-core` + `papyrus-cli` |
| Validation strategy | PyMuPDF as ground truth oracle for integration tests |
