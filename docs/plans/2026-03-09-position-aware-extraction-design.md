# Position-Aware Extraction Design

**Date:** 2026-03-09  
**Status:** Approved  
**Scope:** v0.1.1 – v0.1.3 (foundation for v0.2.0)

## Motivation

Papyrus v0.1.0 has two critical quality issues visible to users:

1. **No line/paragraph breaks** — entire pages render as one text block.
2. **Word spacing misses** — the static TJ threshold (`-100`) fails across PDF generators.

Both stem from the same root cause: papyrus doesn't know *where* text sits on the page. PDF is a 2D canvas, not a text stream. Without X/Y coordinates, we're guessing in the dark.

## Approach: Position-Aware Extraction

Re-implement text extraction to track absolute coordinates for every segment, then use spatial analysis (not operator heuristics) to reconstruct lines, words, and paragraphs.

This is the technique used by PyMuPDF and pdfplumber. We pay the complexity cost once and get line breaks, word spacing, and multi-column foundation in return.

### Rejected Alternatives

**Operator-Level Line Signals (rejected):** Treating `Td`/`Tm`/`T*` as line break indicators fails because PDF generators (LaTeX, Word) use these operators for kerning, not line breaks. Would produce vertical gibberish.

**Post-Processing Heuristics (rejected):** Applying NLP/statistical rules on already-broken text is building on sand. If spacing is wrong at parse time, no post-processor can fix it reliably.

## Architecture

### RawTextSegment (extended)

```rust
pub struct RawTextSegment {
    pub text: String,
    pub font_resource_name: Vec<u8>,
    pub font_size: f32,
    pub page_number: usize,
    pub x: f32,           // cursor X before printing
    pub y: f32,           // baseline Y from matrix
    pub is_rotated: bool, // true when Tm has non-zero b or c
}
```

### TextState (new)

```rust
pub struct TextState {
    pub matrix: [f32; 6],         // [a, b, c, d, e, f]
    pub current_x: f32,           // advances after each Tj/TJ
    pub current_y: f32,           // from matrix[5], updates on Tm/Td/TD/T*
    pub font_size: f32,
    pub font_resource_name: Vec<u8>,
}
```

### Operator Update Rules

| Operator | Action |
|---|---|
| `Tm a b c d e f` | `current_x = e`, `current_y = f`, update full matrix |
| `Td tx ty` | `current_x += tx`, `current_y += ty` |
| `TD tx ty` | same as `Td`, plus update TL |
| `T*` | `current_x = 0`, `current_y -= TL` |
| `Tj (str)` | record `segment.x = current_x`, then `current_x += string_width(str, font, size)` |
| `TJ [...]` | per element: string → same as Tj; number → `current_x -= (n / 1000.0) * size` |

### X-Cursor Advancement (Critical)

Many PDF generators split single words across multiple `Tj` operators without intervening `Tm`/`Td`:

```
10 100 Td         % cursor at X=10
(Hel) Tj          % print "Hel", cursor advances by width("Hel")
(lo) Tj           % print "lo", cursor advances again
```

If we only read `Tm[e]` for X positions, all segments would have `x = 10`. The text state machine must advance `current_x` after every `Tj`/`TJ` string element using:

```
string_width = sum(glyph_widths) / 1000.0 * font_size
```

Fallback when font metrics unavailable: `char_count * font_size * 0.6`.

## Spatial Layout Pipeline

```
[RawTextSegment list per page]
        │
        ├── filter is_rotated == true ──→ quarantine list
        │
        └── normal segments
                │
                ▼
        Y-grouping: |y1 - y2| < font_size * 0.5
                │
                ▼
        Sort lines: Y descending, X ascending
                │
                ▼
        X-gap word spacing: gap > space_width * 0.8
        (space_width from font metrics, fallback font_size * 0.3)
                │
                ▼
        Y-gap paragraph: gap > median_line_height * 1.5
                │
                ▼
        Append quarantined rotated text + emit Warning
```

### Y-Grouping Tolerance

Threshold is `font_size * 0.5`, not a fixed point value. For 12pt body text, tolerance is ±6pt — enough to catch superscript baselines (~4pt shift) without merging adjacent lines.

### Paragraph Detection

`line_height` is the **median** Y-gap across all consecutive lines on the page. Median resists outliers (large section gaps, footnotes). Paragraph break fires when `y_gap > line_height * 1.5`.

### Word Spacing

X-gap between adjacent segments on the same line. Word boundary when `x_gap > space_width * 0.8`, where `space_width` comes from font glyph metrics. Fallback: `font_size * 0.3`.

This replaces the static TJ threshold (`-100`) with font-aware measurement.

## Rotated Text: Extract & Quarantine

Detection: `Tm` parameters `b != 0.0 || c != 0.0` (beyond epsilon ≈ 0.001).

Behavior:
- Segments marked `is_rotated = true` are excluded from Y-grouping pipeline.
- Collected per page and appended after layout-reconstructed text.
- Warning emitted: `Warning::RotatedTextDetected { page, segment_count, preview }`.

Rationale: Rotated segments would corrupt Y-grouping (vertical text has rapidly changing Y per character). Skipping them violates best-effort philosophy. Quarantine preserves the text without poisoning layout analysis.

## Image-Only Page Detection

After parsing a page's content stream, if zero text segments found, emit `Warning::ImageOnlyPage { page }`. Output for that page is empty. Foundation for future papyrus-ocr crate.

## Hypotheses

These must be verified during implementation planning with test PDFs against PyMuPDF ground truth.

| # | Hypothesis | Verification |
|---|---|---|
| H1 | Tm `e,f` gives correct absolute coordinates | Compare segment positions vs PyMuPDF `.get_text("dict")` across LaTeX, Word, Adobe PDFs |
| H2 | Y-tolerance `font_size * 0.5` handles superscript without merging lines | Academic paper with citation superscripts stays single line |
| H3 | X-gap from font metrics is more accurate than static threshold | Test 3 PDFs: tight kerning (design font), wide tracking (ALL CAPS), monospace code |
| H4 | Paragraph break at `y_gap > 1.5x median_line_height` is universal | Multi-paragraph docs, measure actual gap distributions in PyMuPDF output |
| H5 | Y-then-X sort correct for single-column, semantically interleaved for multi-column | Single-column → correct order; dual-column → interleaved (not crash), test asserts interleaved pattern |
| H6 | X-cursor advancement after Tj/TJ produces correct positions for consecutive segments | PDF splitting one word across multiple Tj → X positions differ, concatenation correct |

## Version Mapping

| Release | Deliverable |
|---|---|
| **0.1.1** | H1 + H6: Tm tracking, X-cursor advancement, Y-sort, line breaks |
| **0.1.2** | H2 + H3: font-metrics word spacing, superscript Y-tolerance |
| **0.1.3** | H4 + H5: paragraph detection, image-only page warning |
| **0.2.0** | Multi-column layout analysis (requires H1–H6 as foundation) |

## Testing Strategy

- Unit tests per hypothesis with crafted minimal PDFs.
- Oracle validation: compare output against PyMuPDF `.get_text("dict")` for position data and `.get_text("text")` for final text.
- Integration tests: "Attention Is All You Need" paper (existing fixture) re-validated after each release.
- Regression: all 110 existing tests must stay green.
