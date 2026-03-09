# Changelog

All notable changes to Papyrus are documented here.

---

## [Unreleased] — v0.1.1

### Added

- **TextState machine** (`papyrus-core/src/parser/mod.rs`)
  - Tracks PDF text matrix via `Tm`, `Td`, `TD`, `T*`, `TL` operators
  - Detects rotated text when matrix has non-zero `b` or `c` components
  - Advances X-cursor after each `Tj`/`TJ` call using `char_count * font_size * 0.6` estimate
  - Resets cleanly on `BT` (begin text object)

- **Position fields on `RawTextSegment`**
  - `x: f32` — baseline X cursor position before the segment was printed
  - `y: f32` — baseline Y position from the text matrix
  - `is_rotated: bool` — true when text matrix has rotation

- **Spatial layout module** (`papyrus-core/src/layout/`)
  - `group_into_lines()` — groups segments by Y-proximity (tolerance: `font_size * 0.5`), sorted Y-descending then X-ascending
  - `reconstruct_line_text()` — joins segments within a line, inserting spaces where X-gap exceeds `space_width * 0.8`
  - `is_paragraph_break()` — returns true when Y-gap between lines exceeds `median_line_height * 1.5`
  - `compute_median_line_height()` — median inter-line gap, fallback to `body_font_size * 1.2`
  - `collect_rotated()` — extracts rotated segments for quarantine

- **Pipeline integration** (`papyrus-core/src/lib.rs`)
  - `apply_spatial_layout()` runs between raw segment extraction and AST build
  - Per-page processing: group → layout → reconstruct → paragraph detect
  - Rotated segments quarantined and appended after normal text

- **New warning variants** (`papyrus-core/src/ast/mod.rs`)
  - `Warning::RotatedTextDetected { page, segment_count }`
  - `Warning::ImageOnlyPage { page }`

- **New tests** (21 tests added, total: 131)
  - 8 `TextState` unit tests (Tm tracking, Td offsets, T*, rotation, reset)
  - 9 spatial layout unit tests (Y-grouping, sorting, X-gap spacing, paragraph breaks, median height, superscript tolerance, rotated exclusion)
  - 2 H1 integration tests (Tm coordinates plausible, distinct Y positions across headings)
  - 2 H6 integration tests (X-cursor advancement produces unique positions)

### Changed

- `TJ` word-spacing threshold changed from static `-100` to font-relative `font_size * 0.3`:
  - Old: `displacement < -100.0`
  - New: `displacement_pts > font_size * 0.3 * 0.8` (where `displacement_pts = -displacement / 1000.0 * font_size`)
- `papyrus-cli` warning renderer updated to display new warning variants

### Known Remaining Issues

- **Junction word spacing**: spaces missing where PDF line N ends and line N+1 begins (e.g., `"or\nconvolutional"` → `"orconvolutional"`). The PDF encodes each visual line as a separate text object with no cross-line spacing signal. Planned fix: v0.1.2 with font advance-width metrics.
- **Multi-column layouts**: reading order not detected. Text from adjacent columns can interleave. Planned: v0.2.0.

---

## [0.1.0] — 2026-03-07

Initial release.

### Added

- **Phase 1 — Scaffold and Oracle**
  - Cargo workspace: `papyrus-core` (library) + `papyrus-cli` (binary)
  - Oracle-based testing infrastructure using PyMuPDF as ground truth
  - Test fixtures: `simple.pdf`, `multi-page.pdf`, `bold-italic.pdf`, `multi-heading.pdf`, `corrupted.pdf`

- **Phase 2 — Low-Level Extraction**
  - PDF loading and validation with graceful error handling
  - Font resolution from `/Resources` dictionary and `/FontDescriptor`
  - Text encoding: UTF-16BE (with/without BOM), WinAnsiEncoding, ASCII
  - Content stream parsing: `Tf`, `Tj`, `TJ`, `BT`, `ET` operators
  - Word spacing in TJ arrays via positioning heuristics

- **Phase 3 — Smart Outline and API**
  - Heading detection using font-size ratios (4 levels: H1–H4)
  - Bold/italic detection from font names and descriptor metrics
  - AST-based document representation (`Document`, `Node`, `Span`)
  - Configurable detection via `PapyrusBuilder`
  - Public `convert()` function and `Papyrus::extract()` method

- **Phase 4 — CommonMark Renderer**
  - Markdown output with full CommonMark special-character escaping
  - Heading levels (`#` through `######`)
  - Bold (`**text**`), italic (`*text*`), bold-italic (`***text***`)
  - HTML structural character prevention (`<`, `>`, `&`)
  - Single trailing newline normalization

- **Phase 5 — CLI Interface**
  - `papyrus convert <input>` — single file or directory
  - Stdout mode (omit `--output`), stdin mode (`-` as input)
  - Batch directory conversion with per-file warnings
  - `--quiet` flag to suppress warnings
  - `--heading-ratio`, `--no-bold`, `--no-italic` flags
  - Exit codes: 0 success, 1 I/O error, 2 invalid args
  - 110 tests (14 CLI unit + 8 CLI integration + 88 core)
