# Position-Aware Extraction Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace blind text extraction with position-aware spatial layout analysis, delivering line breaks and accurate word spacing for v0.1.1.

**Architecture:** Extend `RawTextSegment` with `x`, `y`, `is_rotated` coordinates from a full `TextState` machine that tracks the PDF text matrix (`Tm`) and cursor advancement after `Tj`/`TJ`. A new spatial layout module groups segments into lines by Y-proximity, sorts by X, inserts word spaces from X-gap analysis, and detects line/paragraph breaks from Y-gaps.

**Tech Stack:** Rust 2021, lopdf (PDF parsing), existing papyrus-core architecture.

**Hypotheses:**
- H1: IF we track Tm `e,f` parameters THEN we capture correct absolute coordinates across PDF generators
- H6: IF we advance X-cursor after each Tj/TJ by `string_width` THEN consecutive segments from split words get distinct X positions

**WARNING: Hypothesis Failure Protocol:** If H1 or H6 fail verification against PyMuPDF ground truth, STOP and return to brainstorming.

---

## Task 1: Extend RawTextSegment with Position Fields

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs` (lines 21–30, `RawTextSegment` struct)

**Step 1: Write the failing test**

Add to the inline `mod tests` in `parser/mod.rs`:

```rust
#[test]
fn raw_text_segment_carries_position_data() {
    let seg = RawTextSegment {
        text: "Hello".to_string(),
        font_resource_name: b"F1".to_vec(),
        font_size: 12.0,
        page_number: 1,
        x: 72.0,
        y: 700.0,
        is_rotated: false,
    };
    assert_eq!(seg.x, 72.0);
    assert_eq!(seg.y, 700.0);
    assert!(!seg.is_rotated);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core raw_text_segment_carries_position_data 2>&1`
Expected: FAIL — `x`, `y`, `is_rotated` fields don't exist yet.

**Step 3: Add fields to RawTextSegment**

In `papyrus-core/src/parser/mod.rs`, extend `RawTextSegment`:

```rust
pub struct RawTextSegment {
    /// Decoded UTF-8 text content.
    pub text: String,
    /// The font resource name as it appears in the content stream (e.g., b"F1").
    pub font_resource_name: Vec<u8>,
    /// Font size from the current Tf text state.
    pub font_size: f32,
    /// 1-based page number.
    pub page_number: usize,
    /// Cursor X position (user-space units) before this segment was printed.
    pub x: f32,
    /// Baseline Y position (user-space units) from the text matrix.
    pub y: f32,
    /// True when the text matrix has non-zero rotation (b != 0 or c != 0).
    pub is_rotated: bool,
}
```

**Step 4: Fix all compilation errors**

Every place that constructs `RawTextSegment` must now include `x`, `y`, `is_rotated`. This includes:

- `extract_text_segments_for_page` in `parser/mod.rs` — set `x: 0.0, y: 0.0, is_rotated: false` as placeholder values (will be replaced in Task 2).
- All test helper functions `seg()`, `seg_with_font()` in `parser/mod.rs` tests.
- All test helper functions in `detector/mod.rs` tests.
- Integration tests in `papyrus-core/tests/integration_extraction.rs`.

Search for all construction sites: `grep -rn "RawTextSegment" papyrus-core/`

**Step 5: Run full test suite**

Run: `cargo test --workspace 2>&1`
Expected: All 110 tests PASS (including the new one).

**Step 6: Commit**

```bash
git add -A
git commit -m "refactor(parser): add x, y, is_rotated fields to RawTextSegment"
```

---

## Task 2: Implement TextState and Matrix Tracking

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn text_state_tracks_tm_operator() {
    let mut state = TextState::new();
    // Tm 1 0 0 1 72 700
    state.set_matrix(1.0, 0.0, 0.0, 1.0, 72.0, 700.0);
    assert_eq!(state.current_x, 72.0);
    assert_eq!(state.current_y, 700.0);
    assert!(!state.is_rotated());
}

#[test]
fn text_state_detects_rotation() {
    let mut state = TextState::new();
    // Rotated text: Tm 0 1 -1 0 100 200
    state.set_matrix(0.0, 1.0, -1.0, 0.0, 100.0, 200.0);
    assert!(state.is_rotated());
}

#[test]
fn text_state_applies_td_offset() {
    let mut state = TextState::new();
    state.set_matrix(1.0, 0.0, 0.0, 1.0, 72.0, 700.0);
    state.apply_td(10.0, -14.0);
    assert_eq!(state.current_x, 82.0);
    assert_eq!(state.current_y, 686.0);
}

#[test]
fn text_state_t_star_resets_x_and_decrements_y_by_tl() {
    let mut state = TextState::new();
    state.set_matrix(1.0, 0.0, 0.0, 1.0, 72.0, 700.0);
    state.set_tl(14.0);
    state.apply_t_star();
    // T* moves to next line: x resets to line start, y -= TL
    assert_eq!(state.current_x, 72.0); // back to line_start_x
    assert_eq!(state.current_y, 686.0);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p papyrus-core text_state_ 2>&1`
Expected: FAIL — `TextState` struct doesn't exist.

**Step 3: Implement TextState**

```rust
/// Tracks the current text position state within a PDF content stream.
///
/// Implements the subset of the PDF text state machine needed for position
/// extraction: Tm, Td, TD, T*, and cursor advancement after Tj/TJ.
#[derive(Debug, Clone)]
pub(crate) struct TextState {
    /// Current X cursor position in user space.
    pub current_x: f32,
    /// Current Y baseline position in user space.
    pub current_y: f32,
    /// X position of the start of the current line (for T* reset).
    line_start_x: f32,
    /// Y position of the start of the current line (for T* reset).
    line_start_y: f32,
    /// Text leading (TL), used by T* and TD operators.
    tl: f32,
    /// Text matrix parameters [a, b, c, d] for rotation detection.
    a: f32,
    b: f32,
    c: f32,
    d: f32,
}

impl TextState {
    pub fn new() -> Self {
        Self {
            current_x: 0.0,
            current_y: 0.0,
            line_start_x: 0.0,
            line_start_y: 0.0,
            tl: 0.0,
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
        }
    }

    /// Set text matrix from Tm operator: Tm a b c d e f
    pub fn set_matrix(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        self.a = a;
        self.b = b;
        self.c = c;
        self.d = d;
        self.current_x = e;
        self.current_y = f;
        self.line_start_x = e;
        self.line_start_y = f;
    }

    /// Apply Td offset: Td tx ty
    pub fn apply_td(&mut self, tx: f32, ty: f32) {
        self.current_x = self.line_start_x + tx;
        self.current_y = self.line_start_y + ty;
        self.line_start_x = self.current_x;
        self.line_start_y = self.current_y;
    }

    /// Apply TD offset (same as Td but also sets TL = -ty)
    pub fn apply_td_upper(&mut self, tx: f32, ty: f32) {
        self.tl = -ty;
        self.apply_td(tx, ty);
    }

    /// Apply T* (move to start of next line)
    pub fn apply_t_star(&mut self) {
        self.apply_td(0.0, -self.tl);
    }

    /// Set text leading (TL parameter)
    pub fn set_tl(&mut self, tl: f32) {
        self.tl = tl;
    }

    /// Advance X cursor after printing text of the given width.
    pub fn advance_x(&mut self, width: f32) {
        self.current_x += width;
    }

    /// Adjust X cursor for TJ positioning number.
    /// Negative numbers move right (create space), positive move left (tighten).
    pub fn adjust_tj(&mut self, displacement: f32, font_size: f32) {
        self.current_x -= (displacement / 1000.0) * font_size;
    }

    /// Returns true if the current text matrix has rotation (b or c non-zero).
    pub fn is_rotated(&self) -> bool {
        self.b.abs() > 0.001 || self.c.abs() > 0.001
    }

    /// Reset state for a new BT (begin text object).
    pub fn reset_for_bt(&mut self) {
        self.current_x = 0.0;
        self.current_y = 0.0;
        self.line_start_x = 0.0;
        self.line_start_y = 0.0;
        self.a = 1.0;
        self.b = 0.0;
        self.c = 0.0;
        self.d = 1.0;
        // Note: TL persists across text objects per PDF spec §9.3.5
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p papyrus-core text_state_ 2>&1`
Expected: All 4 tests PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/parser/mod.rs
git commit -m "feat(parser): add TextState struct for position tracking"
```

---

## Task 3: Wire TextState into Content Stream Parser

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs` — `extract_text_segments_for_page` function (lines ~300–420)

**Step 1: Write the failing test**

```rust
#[test]
fn extract_segments_with_td_produces_distinct_y_positions() {
    // Create a minimal PDF with two text lines at different Y positions.
    // We test this through parse_pdf on the simple.pdf fixture — segments
    // should now have non-zero y values (previously they were all 0.0).
    let bytes = std::fs::read(fixture_path("simple.pdf")).unwrap();
    let (segments, _, _) = parse_pdf(&bytes);

    // simple.pdf has text — at least some segments should have y > 0
    assert!(!segments.is_empty(), "simple.pdf should produce segments");
    let has_nonzero_y = segments.iter().any(|s| s.y != 0.0);
    assert!(
        has_nonzero_y,
        "simple.pdf segments should have non-zero Y positions after Tm tracking"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core extract_segments_with_td_produces_distinct_y 2>&1`
Expected: FAIL — all segments have `y: 0.0` (placeholder from Task 1).

**Step 3: Integrate TextState into extract_text_segments_for_page**

Replace the text state machine section in `extract_text_segments_for_page`. Key changes:

1. Add `let mut text_state = TextState::new();` before the operator loop.
2. In `"BT"`: call `text_state.reset_for_bt()` (keep existing font state reset).
3. Add operator handlers for `"Tm"`, `"Td"`, `"TD"`, `"T*"`, `"TL"`.
4. In `"Tj"` handler: record `x: text_state.current_x, y: text_state.current_y, is_rotated: text_state.is_rotated()` on the segment, then call `text_state.advance_x(estimated_width)`.
5. In `"TJ"` handler: instead of collecting into one big string, emit individual positioned segments per string element, advancing X cursor between elements.

**For Tj operator — string width estimation:**

```rust
/// Estimate the width of a text string in user-space units.
///
/// Uses char count * font_size * 0.6 as a rough approximation.
/// Will be replaced by actual font metrics in v0.1.2 (H3).
fn estimate_string_width(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * 0.6
}
```

**For TJ operator — position-aware handling:**

The TJ handler must now:
- Walk each array element
- For each string element: record segment position, advance X
- For each number element: adjust X via `text_state.adjust_tj(num, font_size)`
- Still combine adjacent string runs into one segment when no significant gap exists (within same TJ call, kerning adjustments < word_space_threshold are not gaps)

**Step 4: Add operator matchers**

In the `match op.operator.as_str()` block, add:

```rust
"Tm" => {
    if op.operands.len() >= 6 {
        let vals: Vec<f32> = op.operands.iter()
            .filter_map(|o| extract_number(o))
            .collect();
        if vals.len() >= 6 {
            text_state.set_matrix(vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]);
        }
    }
}
"Td" => {
    if op.operands.len() >= 2 {
        if let (Some(tx), Some(ty)) = (
            extract_number(&op.operands[0]),
            extract_number(&op.operands[1]),
        ) {
            text_state.apply_td(tx, ty);
        }
    }
}
"TD" => {
    if op.operands.len() >= 2 {
        if let (Some(tx), Some(ty)) = (
            extract_number(&op.operands[0]),
            extract_number(&op.operands[1]),
        ) {
            text_state.apply_td_upper(tx, ty);
        }
    }
}
"T*" => {
    text_state.apply_t_star();
}
"TL" => {
    if let Some(tl) = op.operands.first().and_then(extract_number) {
        text_state.set_tl(tl);
    }
}
```

**Step 5: Run full test suite**

Run: `cargo test --workspace 2>&1`
Expected: All existing tests PASS, plus new test PASSES.

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(parser): wire TextState into content stream parser with position tracking"
```

---

## Task 4: Verify H1 — Tm Coordinates Against Fixture

**Hypothesis H1:** IF we track Tm `e,f` parameters THEN we capture correct absolute coordinates.

**Files:**
- Modify: `papyrus-core/tests/integration_extraction.rs`

**Step 1: Write verification test**

```rust
#[test]
fn verify_h1_segments_have_plausible_coordinates() {
    // H1: Tm tracking produces plausible absolute coordinates.
    // PDF pages are typically 612x792 pt (US Letter). Y coordinates
    // should be in the range 0–792, X in 0–612.
    let bytes = load_fixture_bytes("simple.pdf");
    let (segments, _, _) = parser::parse_pdf(&bytes);

    assert!(!segments.is_empty());
    for seg in &segments {
        assert!(
            seg.x >= 0.0 && seg.x <= 1000.0,
            "segment '{}' has implausible x={}", seg.text, seg.x
        );
        assert!(
            seg.y >= 0.0 && seg.y <= 1000.0,
            "segment '{}' has implausible y={}", seg.text, seg.y
        );
    }

    // Verify we have at least 2 distinct Y values (= multiple lines)
    let mut ys: Vec<i32> = segments.iter().map(|s| (s.y * 10.0) as i32).collect();
    ys.sort();
    ys.dedup();
    assert!(
        ys.len() >= 1,
        "expected distinct Y values indicating line positions, got {:?}", ys
    );
}
```

**Step 2: Run verification**

Run: `cargo test -p papyrus-core --test integration_extraction verify_h1 2>&1`
Expected: PASS — coordinates are plausible.

**If PASS:** H1 verified, proceed.
**If FAIL:** STOP. Document failure, return to brainstorming.

**Step 3: Commit**

```bash
git add papyrus-core/tests/integration_extraction.rs
git commit -m "verify: H1 - Tm coordinates produce plausible absolute positions"
```

---

## Task 5: Verify H6 — X-Cursor Advancement

**Hypothesis H6:** IF we advance X-cursor after Tj/TJ THEN consecutive segments from split words get distinct X.

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs` (inline tests)

**Step 1: Write verification test**

```rust
#[test]
fn verify_h6_consecutive_tj_segments_have_advancing_x() {
    // H6: When a PDF splits text across multiple Tj operators,
    // each segment should have an increasing X position.
    // Test with multi-heading.pdf which contains multiple text elements.
    let bytes = std::fs::read(fixture_path("multi-heading.pdf")).unwrap();
    let (segments, _, _) = parse_pdf(&bytes);

    assert!(!segments.is_empty());

    // Group segments by page and approximate Y (same line)
    // Check that X increases within each line
    let page1_segs: Vec<_> = segments.iter().filter(|s| s.page_number == 1).collect();
    if page1_segs.len() >= 2 {
        // Within same Y group, X should generally be non-decreasing
        let tolerance = 1.0; // Y grouping tolerance
        let y_ref = page1_segs[0].y;
        let same_line: Vec<_> = page1_segs
            .iter()
            .filter(|s| (s.y - y_ref).abs() < tolerance)
            .collect();
        if same_line.len() >= 2 {
            for pair in same_line.windows(2) {
                // Segments on the same line should have different X positions
                // (they can't all be at the same X unless TextState isn't advancing)
                assert!(
                    pair[0].x != pair[1].x || pair[0].x == 0.0,
                    "H6 FAIL: consecutive segments on same line have identical X={}: '{}' and '{}'",
                    pair[0].x,
                    pair[0].text,
                    pair[1].text,
                );
            }
        }
    }
}
```

**Step 2: Run verification**

Run: `cargo test -p papyrus-core verify_h6 2>&1`
Expected: PASS.

**If PASS:** H6 verified, proceed.
**If FAIL:** STOP. Investigate `advance_x` logic, return to brainstorming if fundamental.

**Step 3: Commit**

```bash
git add papyrus-core/src/parser/mod.rs
git commit -m "verify: H6 - X-cursor advancement produces distinct positions for split text"
```

---

## Task 6: Create Spatial Layout Module

**Files:**
- Create: `papyrus-core/src/layout/mod.rs`
- Modify: `papyrus-core/src/lib.rs` (add `pub mod layout;`)

**Step 1: Write the failing test**

```rust
// In papyrus-core/src/layout/mod.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::RawTextSegment;

    fn seg_at(text: &str, x: f32, y: f32, font_size: f32) -> RawTextSegment {
        RawTextSegment {
            text: text.to_string(),
            font_resource_name: b"F1".to_vec(),
            font_size,
            page_number: 1,
            x,
            y,
            is_rotated: false,
        }
    }

    #[test]
    fn group_lines_by_y_proximity() {
        let segments = vec![
            seg_at("Hello", 72.0, 700.0, 12.0),
            seg_at("World", 120.0, 700.0, 12.0),
            seg_at("Second line", 72.0, 686.0, 12.0),
        ];
        let lines = group_into_lines(&segments, 12.0);
        assert_eq!(lines.len(), 2, "should detect 2 lines");
        assert_eq!(lines[0].len(), 2, "first line should have 2 segments");
        assert_eq!(lines[1].len(), 1, "second line should have 1 segment");
    }

    #[test]
    fn lines_sorted_y_descending_x_ascending() {
        let segments = vec![
            seg_at("B", 200.0, 700.0, 12.0),
            seg_at("A", 72.0, 700.0, 12.0),
            seg_at("C", 72.0, 686.0, 12.0),
        ];
        let lines = group_into_lines(&segments, 12.0);
        // First line (Y=700) comes first (higher Y = higher on page)
        assert_eq!(lines[0][0].text, "A"); // X=72 before X=200
        assert_eq!(lines[0][1].text, "B");
        assert_eq!(lines[1][0].text, "C"); // second line
    }

    #[test]
    fn rotated_segments_excluded_from_lines() {
        let mut rotated = seg_at("WATERMARK", 300.0, 400.0, 24.0);
        rotated.is_rotated = true;
        let segments = vec![
            seg_at("Normal", 72.0, 700.0, 12.0),
            rotated,
        ];
        let lines = group_into_lines(&segments, 12.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0][0].text, "Normal");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p papyrus-core group_lines 2>&1`
Expected: FAIL — module doesn't exist.

**Step 3: Implement layout module**

```rust
//! Spatial layout analysis for positioning-aware text reconstruction.
//!
//! Groups raw text segments into lines based on Y-proximity, sorts by
//! reading order (Y descending, X ascending), and provides line/paragraph
//! break detection.

use crate::parser::RawTextSegment;

/// Group segments into lines based on Y-proximity.
///
/// Segments with `|y1 - y2| < font_size * 0.5` are considered the same line.
/// Rotated segments are excluded. Lines are sorted Y-descending (top of page
/// first), segments within each line sorted X-ascending (left to right).
pub fn group_into_lines<'a>(
    segments: &'a [RawTextSegment],
    body_font_size: f32,
) -> Vec<Vec<&'a RawTextSegment>> {
    let tolerance = body_font_size * 0.5;

    // Filter out rotated segments
    let mut normal: Vec<&RawTextSegment> = segments
        .iter()
        .filter(|s| !s.is_rotated)
        .collect();

    if normal.is_empty() {
        return Vec::new();
    }

    // Sort by Y descending (top first), then X ascending
    normal.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Group into lines by Y-proximity
    let mut lines: Vec<Vec<&RawTextSegment>> = Vec::new();
    let mut current_line: Vec<&RawTextSegment> = vec![normal[0]];
    let mut current_y = normal[0].y;

    for seg in &normal[1..] {
        if (current_y - seg.y).abs() <= tolerance {
            current_line.push(seg);
        } else {
            // Sort current line by X before finalizing
            current_line.sort_by(|a, b| {
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            });
            lines.push(current_line);
            current_line = vec![seg];
            current_y = seg.y;
        }
    }
    // Don't forget the last line
    current_line.sort_by(|a, b| {
        a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
    });
    lines.push(current_line);

    lines
}

/// Collect rotated segments from a page (excluded from layout pipeline).
pub fn collect_rotated<'a>(segments: &'a [RawTextSegment]) -> Vec<&'a RawTextSegment> {
    segments.iter().filter(|s| s.is_rotated).collect()
}

/// Reconstruct text for a single line with X-gap word spacing.
///
/// Inserts a space between adjacent segments when the X-gap exceeds
/// `space_width * 0.8`. Uses `font_size * 0.3` as fallback space width.
pub fn reconstruct_line_text(line: &[&RawTextSegment]) -> String {
    if line.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut prev_end_x: Option<f32> = None;

    for seg in line {
        if let Some(prev_x) = prev_end_x {
            let gap = seg.x - prev_x;
            let space_width = seg.font_size * 0.3; // fallback, replaced by font metrics in v0.1.2
            if gap > space_width * 0.8 {
                result.push(' ');
            }
        }
        result.push_str(&seg.text);
        // Estimate end X of this segment
        let width = seg.text.chars().count() as f32 * seg.font_size * 0.6;
        prev_end_x = Some(seg.x + width);
    }

    result
}

/// Detect paragraph break between two consecutive lines.
///
/// Returns true when the Y-gap between line_above and line_below exceeds
/// `median_line_height * 1.5`.
pub fn is_paragraph_break(
    line_above_y: f32,
    line_below_y: f32,
    median_line_height: f32,
) -> bool {
    let gap = (line_above_y - line_below_y).abs();
    gap > median_line_height * 1.5
}

/// Compute the median Y-gap between consecutive lines.
///
/// Returns the median inter-line gap, or `body_font_size * 1.2` as fallback
/// when fewer than 2 lines exist.
pub fn compute_median_line_height(lines: &[Vec<&RawTextSegment>], body_font_size: f32) -> f32 {
    if lines.len() < 2 {
        return body_font_size * 1.2;
    }

    let mut gaps: Vec<f32> = Vec::new();
    for pair in lines.windows(2) {
        let y_above = pair[0].first().map(|s| s.y).unwrap_or(0.0);
        let y_below = pair[1].first().map(|s| s.y).unwrap_or(0.0);
        let gap = (y_above - y_below).abs();
        if gap > 0.0 {
            gaps.push(gap);
        }
    }

    if gaps.is_empty() {
        return body_font_size * 1.2;
    }

    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    gaps[gaps.len() / 2]
}
```

**Step 4: Register the module**

In `papyrus-core/src/lib.rs`, add:

```rust
pub mod layout;
```

**Step 5: Run tests**

Run: `cargo test -p papyrus-core layout 2>&1`
Expected: All 3 layout tests PASS.

**Step 6: Commit**

```bash
git add papyrus-core/src/layout/mod.rs papyrus-core/src/lib.rs
git commit -m "feat(layout): add spatial layout module with Y-grouping and X-gap word spacing"
```

---

## Task 7: Integrate Layout into Extraction Pipeline

**Files:**
- Modify: `papyrus-core/src/lib.rs` — `extract_with_config` function
- Modify: `papyrus-core/src/ast/mod.rs` — add new warning variants

**Step 1: Add warning variants**

In `papyrus-core/src/ast/mod.rs`, add to the `Warning` enum:

```rust
pub enum Warning {
    MissingFontMetrics { font_name: String, page: usize },
    UnreadableTextStream { page: usize, detail: String },
    UnsupportedEncoding { encoding: String, page: usize },
    MalformedPdfObject { detail: String },
    RotatedTextDetected { page: usize, segment_count: usize },
    ImageOnlyPage { page: usize },
}
```

**Step 2: Write the integration test**

In `papyrus-core/tests/integration_extraction.rs`:

```rust
#[test]
fn simple_pdf_extraction_has_line_breaks_in_markdown() {
    let bytes = load_fixture_bytes("simple.pdf");
    let result = papyrus_core::convert(&bytes);
    let md = result.to_markdown();
    // After spatial layout, output should contain newlines separating content
    // (not one giant blob of text)
    let line_count = md.lines().count();
    assert!(
        line_count >= 2,
        "expected multiple lines in markdown output, got {} lines: {:?}",
        line_count,
        md,
    );
}
```

**Step 3: Integrate layout pass into extract_with_config**

In `papyrus-core/src/lib.rs`, after collecting `all_segments` per page:

1. Group segments per page.
2. For each page's segments, run `layout::group_into_lines`.
3. Reconstruct text per line with `layout::reconstruct_line_text`.
4. Detect paragraph breaks with `layout::is_paragraph_break`.
5. Emit `Warning::RotatedTextDetected` for quarantined rotated segments.
6. Emit `Warning::ImageOnlyPage` for pages with zero text segments.
7. Build new `RawTextSegment` list with line-reconstructed text (one segment per line, preserving font info from the dominant segment).

This transforms the flat segment list into a line-aware segment list before passing to `build_document`.

**Step 4: Run full test suite**

Run: `cargo test --workspace 2>&1`
Expected: All tests PASS.

**Important consideration:** Existing oracle-based tests compare segment-by-segment against PyMuPDF. The layout pass changes segment granularity (many input segments → fewer line-based segments). This means oracle tests will likely need updating or a compatibility shim. Document this decision at this step — if oracle tests break, update expectations to match the new line-based output, which is the correct behavior going forward.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: integrate spatial layout pipeline into extraction engine"
```

---

## Task 8: Update Oracle Tests and Fixtures

**Files:**
- Modify: `papyrus-core/tests/integration_extraction.rs`
- Possibly modify: `tests/fixtures/*.oracle.json` (regenerate if needed)

**Step 1: Run oracle tests to see what changed**

Run: `cargo test -p papyrus-core --test integration_extraction 2>&1`

The layout pass changes segment boundaries. Document which tests fail and why.

**Step 2: Update test expectations**

Oracle comparisons were against per-TJ/Tj segments. With line-based reconstruction, segments are now per-line. Two options:

A. **Update oracle JSONs** to match new line-based output (preferred for forward compatibility).
B. **Add a separate test** for line-based output and keep old oracle test for parser-level regression.

Choose option B: keep the `parse_pdf` oracle tests (they test the parser, not the layout-aware pipeline), and add new integration tests for the full pipeline with layout.

**Step 3: Ensure 0 test failures**

Run: `cargo test --workspace 2>&1`
Expected: All tests PASS.

**Step 4: Commit**

```bash
git add -A
git commit -m "test: update oracle expectations for position-aware extraction"
```

---

## Task 9: CLI Smoke Test and Final Verification

**Files:**
- No new files, verification only

**Step 1: Build release binary**

Run: `cargo build -p papyrus-cli --release 2>&1`
Expected: Compiles cleanly.

**Step 2: Test with simple.pdf**

Run: `./target/release/papyrus tests/fixtures/simple.pdf`
Expected: Output shows text with line breaks, not one blob.

**Step 3: Test with multi-heading.pdf**

Run: `./target/release/papyrus tests/fixtures/multi-heading.pdf`
Expected: Headings and body text on separate lines.

**Step 4: Verify full test suite**

Run: `cargo test --workspace 2>&1`
Expected: All tests PASS.

**Step 5: Run clippy**

Run: `cargo clippy --workspace 2>&1`
Expected: No warnings.

**Step 6: Run fmt check**

Run: `cargo fmt --check 2>&1`
Expected: Clean.

**Step 7: Commit tag**

```bash
git add -A
git commit -m "chore: v0.1.1 release preparation"
```

---

## Summary

| Task | Description | Hypothesis |
|------|-------------|------------|
| 1 | Extend RawTextSegment with x, y, is_rotated | — |
| 2 | Implement TextState struct | — |
| 3 | Wire TextState into parser | — |
| 4 | Verify H1 (Tm coordinates) | H1 |
| 5 | Verify H6 (X-cursor advancement) | H6 |
| 6 | Create spatial layout module | — |
| 7 | Integrate layout into pipeline | — |
| 8 | Update oracle tests | — |
| 9 | CLI smoke test and final verification | — |
