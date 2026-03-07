# Phase 3: Smart Outline & Public API (The Brain)

**Goal**: Implement the heading detection and bold/italic heuristics on top of raw text segments, construct the final AST, and expose the public `Papyrus` builder API.

**Depends on**: Phase 2 (Low-Level Extraction)
**Blocks**: Phase 4 (CommonMark Renderer)

---

## Tasks

### 3.1 Implement Body Size Computation

In the `detector` module, implement the algorithm that determines the "body font size" for a page (or entire document):

```rust
pub fn compute_body_size(segments: &[RawTextSegment]) -> f32
```

Algorithm:
1. Collect all font sizes from segments
2. Compute the **mode** (most frequently occurring size), not the mean
3. If there's a tie, use the smaller size (conservative: fewer false headings)
4. If no segments exist, return a sensible default (e.g., 12.0)

### 3.2 Implement Heading Detection

```rust
pub fn detect_headings(
    segments: &[RawTextSegment],
    body_size: f32,
    heading_size_ratio: f32,  // default: 1.2
) -> Vec<ClassifiedSegment>
```

Where `ClassifiedSegment` carries the original segment plus a classification:

```rust
pub struct ClassifiedSegment {
    pub segment: RawTextSegment,
    pub classification: SegmentClass,
}

pub enum SegmentClass {
    Heading(u8),   // level 1-6
    Body,
}
```

Ratio-to-level mapping:
- `ratio >= 2.0` -> H1
- `ratio >= 1.7` -> H2
- `ratio >= 1.4` -> H3
- `ratio >= heading_size_ratio` -> H4
- Below threshold -> `Body`

### 3.3 Implement Bold/Italic Detection

```rust
pub fn detect_formatting(font_name: &str, font_info: &FontInfo) -> (bool, bool)
// Returns (is_bold, is_italic)
```

Algorithm:
1. Normalize font name: lowercase, strip subset prefix
2. Check font name patterns:
   - Contains `"bold"` -> bold
   - Contains `"italic"` or `"oblique"` -> italic
   - Contains `"bolditalic"` or `"boldoblique"` -> both
3. Fallback to FontDescriptor (if available from Phase 2's `FontInfo`):
   - `FontWeight > 600` -> bold
   - `ItalicAngle != 0` -> italic

### 3.4 Implement AST Construction

Combine heading detection and formatting detection to build the final `Document` AST:

```rust
pub fn build_document(
    segments: Vec<RawTextSegment>,
    fonts: &HashMap<Vec<u8>, FontInfo>,
    config: &DetectorConfig,
) -> (Document, Vec<Warning>)
```

This function:
1. Computes body size (3.1)
2. Classifies each segment as heading or body (3.2)
3. Detects bold/italic per segment (3.3)
4. Groups consecutive same-classification segments into `Node`s:
   - Heading segments -> `Node::Heading { level, spans }`
   - Body segments -> `Node::Paragraph { spans }`
   - Unclassifiable (no font info) -> `Node::RawText`
5. Each segment becomes a `Span` with text, bold, italic, font_size, font_name
6. Populates `DocumentMetadata` from the extraction phase

### 3.5 Implement `PapyrusBuilder` and Public API

Implement the builder pattern and the two-phase public API:

```rust
impl Papyrus {
    pub fn builder() -> PapyrusBuilder { ... }
    pub fn extract(&self, pdf_bytes: &[u8]) -> ConversionResult { ... }
}

impl PapyrusBuilder {
    pub fn heading_size_ratio(mut self, ratio: f32) -> Self { ... }
    pub fn detect_bold(mut self, enabled: bool) -> Self { ... }
    pub fn detect_italic(mut self, enabled: bool) -> Self { ... }
    pub fn build(self) -> Papyrus { ... }
}

/// Convenience function with default config
pub fn convert(pdf_bytes: &[u8]) -> ConversionResult { ... }
```

The `extract()` method orchestrates the full pipeline:
1. Parse PDF (Phase 2's `parse_pdf`)
2. Build document AST (this phase's `build_document`)
3. Aggregate all warnings from both phases
4. Return `ConversionResult { document, warnings }`

### 3.6 Unit Tests for Detector

- Body size computation: uniform sizes, mixed sizes, empty input
- Heading classification: exact boundary ratios, edge cases (ratio = 1.199 vs 1.2)
- Bold/italic detection: various font name patterns (`Arial-Bold`, `TimesNewRoman-BoldItalic`, `ABCDEF+Helvetica-Oblique`, garbled names)
- FontDescriptor fallback when name detection fails
- AST construction: correct grouping of consecutive segments into nodes

### 3.7 Integration Tests for Full Pipeline

Test the complete `Papyrus::extract()` pipeline:

1. Load test fixtures
2. Run `extract()` with default config
3. Assert AST structure:
   - `simple.pdf` -> 1 Heading node + Paragraph nodes
   - `multi-heading.pdf` -> multiple Heading nodes at different levels
   - `bold-italic.pdf` -> Paragraph nodes with bold/italic spans
   - `corrupted.pdf` -> warnings populated, some RawText nodes, no panic
4. Test builder overrides:
   - Setting `heading_size_ratio(2.0)` should reduce the number of detected headings
   - Setting `detect_bold(false)` should produce spans with `bold: false` everywhere

---

## Definition of Done

- [ ] `compute_body_size()` correctly returns the mode font size
- [ ] Heading detection classifies segments into H1-H4 based on ratio thresholds
- [ ] Bold/italic detection works for font name patterns and FontDescriptor fallback
- [ ] `build_document()` produces a correctly structured `Document` AST
- [ ] `PapyrusBuilder` allows configuring all three knobs (ratio, bold, italic)
- [ ] `Papyrus::extract()` orchestrates the full parse -> detect -> AST pipeline
- [ ] `convert()` convenience function works with defaults
- [ ] Unit tests cover all heuristic edge cases
- [ ] Integration tests verify AST structure against all test fixtures
- [ ] `cargo test` passes with all new tests green
