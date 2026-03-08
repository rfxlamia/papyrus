# Phase 2: Low-Level Extraction (lopdf Engine) - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a deterministic low-level PDF extraction pipeline in `papyrus-core` that converts PDF bytes into ordered raw text segments with reliable font metadata and structured warnings.

**Depends on:** Phase 1 (Scaffold and Oracle)  
**Blocks:** Phase 3 (Smart Outline and Public API)

---

## Why This Rewrite Exists (RED Baseline Findings)

Baseline review on the previous Phase 2 doc found high-risk ambiguity in:
- Failure contracts (`Option<Document>` fallback behavior vs design's best-effort contract)
- Page numbering base (`0` vs `1` based)
- Warning taxonomy mapping (which failure maps to which warning variant)
- Decode stage ownership (`extract_text_segments` vs orchestration layer)
- False-green tests (aggregate text checks that can pass while ordering/state is wrong)

This implementation plan resolves those ambiguities up-front.

---

## Canonical Contracts (Source of Truth for Phase 2)

### 1. Page Numbering Convention

- Public parser outputs use **1-based** page numbers (`page = 1` for first page).
- Internal loops may use 0-based indices, but must convert before storing to public structs or warnings.

### 2. Parser Failure Semantics

`parse_pdf(&[u8])` never panics and never returns `Result`.

On load failure (`empty bytes`, `invalid header`, `encrypted/password protected`, malformed body):
- Return `Vec<RawTextSegment>::new()`
- Return `DocumentMetadata { title: None, author: None, page_count: 0 }`
- Return at least one `Warning::MalformedPdfObject { detail }`

`RawText` fallback nodes are **not** created in Phase 2 parser; that belongs to higher extraction/detector layers.

### 3. Warning Mapping Rules

Use existing warning enum in `papyrus-core/src/ast/mod.rs`.

| Failure mode | Warning variant | Required fields |
|---|---|---|
| General parse/load failure, empty bytes, invalid header, encrypted file, malformed object | `Warning::MalformedPdfObject` | `detail` includes root cause text |
| Missing base font name or font dictionary for a used resource | `Warning::MissingFontMetrics` | `font_name` (or fallback `"<unknown>"`), `page` |
| Cannot decode page content stream bytes/operators | `Warning::UnreadableTextStream` | `page`, `detail` |
| Unknown/unsupported text encoding | `Warning::UnsupportedEncoding` | `encoding`, `page` |

### 4. Segment and Ordering Invariants

`RawTextSegment` records must satisfy:
- `text` is already decoded to UTF-8 (lossy allowed only on unsupported encoding path)
- `font_size` comes from current `Tf` state
- `font_resource_name` is the exact content-stream resource name (`F1`, `F42`, etc.)
- Global output order is stable: by page, then operator encounter order

### 5. Text State Defaults

If `Tj/TJ` appears before any `Tf` within current text object:
- Emit one `Warning::MalformedPdfObject` with detail containing `"text state not set before Tj/TJ"`
- Use defaults for that emitted segment:
  - `font_resource_name = b"<unknown>".to_vec()`
  - `font_size = 0.0`

### 6. Font Size Source of Truth

- `RawTextSegment.font_size` must always come from `Tf` runtime state.
- Optional descriptor size in `FontInfo` is diagnostic/fallback metadata only and must not override `Tf`.

---

## Target API Surface for Phase 2

Place all parser internals under `papyrus-core/src/parser/mod.rs` (or parser submodules if split).

```rust
use std::collections::HashMap;

use crate::ast::{DocumentMetadata, Warning};

#[derive(Debug, Clone, PartialEq)]
pub struct FontInfo {
    pub name: String,
    pub size: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawTextSegment {
    pub text: String,
    pub font_resource_name: Vec<u8>,
    pub font_size: f32,
    pub page_number: usize, // 1-based
}

pub fn load_pdf(bytes: &[u8]) -> (Option<lopdf::Document>, Vec<Warning>);

pub fn resolve_fonts_for_page(
    doc: &lopdf::Document,
    page_number: usize,
) -> (HashMap<Vec<u8>, FontInfo>, Vec<Warning>);

pub fn extract_text_segments_for_page(
    doc: &lopdf::Document,
    page_number: usize,
    fonts: &HashMap<Vec<u8>, FontInfo>,
) -> (Vec<RawTextSegment>, Vec<Warning>);

pub fn parse_pdf(bytes: &[u8]) -> (Vec<RawTextSegment>, DocumentMetadata, Vec<Warning>);
```

### Backward-compatibility transition

- Replace old stub `parse_pdf_bytes` with `parse_pdf`.
- Update tests consuming parser surface (`papyrus-core/tests/module_surface.rs`) in this phase.

---

## Execution Notes

- Use `superpowers:test-driven-development` in each task.
- Use `superpowers:verification-before-completion` before claiming each task done.
- Do not use `unwrap()` / `expect()` on PDF data paths.
- Keep warnings additive; never drop prior warnings during aggregation.
- Keep parser deterministic for stable oracle assertions.

---

## Task Breakdown (TDD Order)

### Task 2.1 - Introduce Parser Types and Surface

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs`
- Modify: `papyrus-core/tests/module_surface.rs`

**Step 1 - Write failing tests**
- Add tests for type construction and parser surface shape.
- Update module surface test to call `parser::parse_pdf`.

**Step 2 - Verify RED**
Run:
```bash
cargo test -p papyrus-core module_surfaces_are_linked -v
```
Expected: fail due to missing `parse_pdf` and new structs.

**Step 3 - Minimal GREEN implementation**
- Define `FontInfo` and `RawTextSegment`.
- Add `parse_pdf` skeleton returning empty segments + zeroed metadata + no warnings.

**Step 4 - Verify GREEN**
Run the same test until pass.

**Step 5 - Commit**
```bash
git add papyrus-core/src/parser/mod.rs papyrus-core/tests/module_surface.rs
git commit -m "feat(parser): define phase-2 parser surface"
```

### Task 2.2 - Implement `load_pdf` Failure Mapping

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs`

**Step 1 - Write failing unit tests**
Cover:
- Empty bytes
- Invalid header bytes
- Corrupted fixture (`tests/fixtures/corrupted.pdf`)

Expectations:
- `Option<Document>` is `None`
- `warnings` includes `Warning::MalformedPdfObject`
- `detail` is non-empty

**Step 2 - Verify RED**
```bash
cargo test -p papyrus-core load_pdf_ -v
```

**Step 3 - Minimal GREEN implementation**
- Parse via `lopdf::Document::load_mem`.
- Map every failure path to `Warning::MalformedPdfObject`.

**Step 4 - Verify GREEN**
Re-run tests until pass.

**Step 5 - Commit**
```bash
git add papyrus-core/src/parser/mod.rs
git commit -m "feat(parser): implement load_pdf warning-mapped failures"
```

### Task 2.3 - Implement Font Resolution per Page

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs`

**Step 1 - Write failing unit tests**
Cover:
- Subset prefix stripping (`ABCDEF+Helvetica-Bold` -> `Helvetica-Bold`)
- Missing `/BaseFont` -> `Warning::MissingFontMetrics`
- Resource names preserved as keys (`F1`, `F2`, ...)

**Step 2 - Verify RED**
```bash
cargo test -p papyrus-core resolve_fonts_for_page_ -v
```

**Step 3 - Minimal GREEN implementation**
- Read page resources and font dictionaries.
- Normalize font names.
- Emit warnings with 1-based `page`.

**Step 4 - Verify GREEN**
Re-run tests until pass.

**Step 5 - Commit**
```bash
git add papyrus-core/src/parser/mod.rs
git commit -m "feat(parser): resolve per-page font dictionaries"
```

### Task 2.4 - Implement Content Stream Text Extraction (`Tf`, `Tj`, `TJ`, `BT/ET`)

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs`

**Step 1 - Write failing unit tests**
Cover:
- `Tf` updates state
- `Tj` emits one segment with current state
- `TJ` concatenates text entries (ignore numeric kerning entries)
- `BT/ET` resets state
- `Tj/TJ` before `Tf` emits warning + default state segment

**Step 2 - Verify RED**
```bash
cargo test -p papyrus-core extract_text_segments_for_page_ -v
```

**Step 3 - Minimal GREEN implementation**
- Parse content stream operations.
- Track font resource and size state.
- Emit warnings and continue on malformed operations.

**Step 4 - Verify GREEN**
Re-run tests until pass.

**Step 5 - Commit**
```bash
git add papyrus-core/src/parser/mod.rs
git commit -m "feat(parser): extract text segments from content streams"
```

### Task 2.5 - Implement Text Decoding Rules

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs`

**Step 1 - Write failing unit tests**
Cover:
- WinAnsi path
- UTF-16BE path (including BOM and no-BOM variants)
- Unsupported encoding -> warning + lossy fallback
- Supported encoding should not emit replacement chars unless source bytes are invalid

**Step 2 - Verify RED**
```bash
cargo test -p papyrus-core decode_ -v
```

**Step 3 - Minimal GREEN implementation**
- Decode using declared/derived encoding strategy.
- Attach `Warning::UnsupportedEncoding` on unknown encoding.

**Step 4 - Verify GREEN**
Re-run tests until pass.

**Step 5 - Commit**
```bash
git add papyrus-core/src/parser/mod.rs
git commit -m "feat(parser): add text decoding with unsupported-encoding fallback"
```

### Task 2.6 - Implement `parse_pdf` Orchestration

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs`

**Step 1 - Write failing tests**
Cover:
- Successful parse returns metadata + ordered segments + aggregated warnings
- Failed load returns empty segments + metadata page_count=0 + warning(s)
- Page numbering in segments/warnings is 1-based

**Step 2 - Verify RED**
```bash
cargo test -p papyrus-core parse_pdf_ -v
```

**Step 3 - Minimal GREEN implementation**
- `load_pdf`
- metadata extraction (`title`, `author`, `page_count`)
- per-page `resolve_fonts_for_page` + `extract_text_segments_for_page`
- aggregate all warnings in stable order

**Step 4 - Verify GREEN**
Re-run tests until pass.

**Step 5 - Commit**
```bash
git add papyrus-core/src/parser/mod.rs
git commit -m "feat(parser): orchestrate end-to-end parse_pdf flow"
```

### Task 2.7 - Oracle Integration Tests (Strict, Not Fuzzy-Only)

**Files:**
- Create: `papyrus-core/tests/integration_extraction.rs`

**Step 1 - Write failing integration tests**
For each fixture with `.oracle.json`:
- Compare per-segment ordered `(page_number, normalized_text)` sequence
- Assert segment counts match
- Assert per-index font names match (after normalization)
- Assert per-index font sizes within `abs_diff <= 0.1`
- Assert metadata parity (`page_count`, `title`, `author` when available)

For `corrupted.pdf`:
- No panic
- At least one warning
- Warning variant and detail assertions (not merely `warnings.len() > 0`)

**Step 2 - Verify RED**
```bash
cargo test -p papyrus-core --test integration_extraction -v
```

**Step 3 - Minimal GREEN implementation**
Implement only what is required to satisfy strict assertions.

**Step 4 - Verify GREEN**
Re-run integration tests until pass.

**Step 5 - Commit**
```bash
git add papyrus-core/tests/integration_extraction.rs
git commit -m "test(parser): add strict oracle integration extraction coverage"
```

### Task 2.8 - Full Verification Gate

Run full verification:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace -v
python3 -m pytest tests/oracle -q
```

If any fail: fix, re-run all, then commit.

Suggested commit:
```bash
git add papyrus-core papyrus-cli docs/plans/phase-2-low-level-extraction.md
git commit -m "chore: complete phase-2 low-level extraction implementation"
```

---

## Definition of Done

- [ ] `parse_pdf(&[u8]) -> (Vec<RawTextSegment>, DocumentMetadata, Vec<Warning>)` implemented
- [ ] All public parser page numbers are 1-based
- [ ] `load_pdf` maps all load failures to `Warning::MalformedPdfObject` with non-empty detail
- [ ] Font resolution strips subset prefixes and keeps font resource names stable
- [ ] Content stream extraction supports `Tf`, `Tj`, `TJ`, `BT`, `ET`
- [ ] `Tj/TJ` before `Tf` emits warning and uses documented default state
- [ ] WinAnsi and UTF-16BE decoding implemented
- [ ] Unsupported encoding path emits `Warning::UnsupportedEncoding` with page context
- [ ] Integration tests compare ordered segment sequences, not only fuzzy aggregate text
- [ ] Integration tests enforce 1:1 segment alignment (count + per-index font metadata)
- [ ] Integration tests assert metadata parity (`page_count`, title/author where available)
- [ ] `corrupted.pdf` path produces warnings and never panics
- [ ] No `unwrap()` / `expect()` on PDF data paths
- [ ] `cargo test --workspace` passes
- [ ] `python3 -m pytest tests/oracle -q` passes

---

## Anti-Regression Checklist (False-Green Guards)

- [ ] Ordering assertion exists (`(page_number, text)` sequence equality)
- [ ] Segment count assertion exists
- [ ] Warning variant assertions exist (not only count)
- [ ] Warning page assertions exist
- [ ] Encoding tests include non-ASCII samples
- [ ] Supported decode paths assert no replacement-char leakage when input is valid
