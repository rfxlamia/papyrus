# Phase 3 Smart Outline and Public API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement smart heading/formatting detection, AST construction, and the public `Papyrus` builder API so `extract()` returns structured `Document` output (not just raw parser segments).

**Architecture:** Keep `parser::parse_pdf` as the low-level extractor, and build Phase 3 behavior in `detector` plus `lib.rs` orchestration. `detector` owns heuristics (`compute_body_size`, heading mapping, bold/italic inference, segment grouping), while `Papyrus::extract` coordinates parser + detector and preserves best-effort warnings. Extend `FontInfo` with descriptor-derived metadata so bold/italic fallback can use PDF font descriptors when names are ambiguous.

**Tech Stack:** Rust 2021, Cargo workspace tests, `lopdf` (parser integration), existing fixture PDFs in `tests/fixtures`.

---

**Execution Notes**
- Work in a dedicated worktree created before execution (`@brainstorming` context assumption).
- Use `@test-driven-development` for every task.
- Use `@verification-before-completion` before claiming task completion.
- Use `@systematic-debugging` immediately if any test fails unexpectedly.
- Keep parser warnings additive and preserve 1-based page numbering.

### Task 1: Replace Detector Stub with Phase 3 Public Contracts

**Files:**
- Modify: `papyrus-core/src/detector/mod.rs:1-4`
- Modify: `papyrus-core/tests/module_surface.rs:1-76`
- Test: `papyrus-core/tests/module_surface.rs`

**Step 1: Write the failing test**

```rust
// papyrus-core/tests/module_surface.rs
use papyrus_core::detector::{ClassifiedSegment, DetectorConfig, SegmentClass};

#[test]
fn detector_surface_exposes_phase3_types() {
    let cfg = DetectorConfig::default();
    assert_eq!(cfg.heading_size_ratio, 1.2);
    assert!(cfg.detect_bold);
    assert!(cfg.detect_italic);

    let class = SegmentClass::Heading(2);
    assert!(matches!(class, SegmentClass::Heading(2)));

    let _ = std::mem::size_of::<ClassifiedSegment>();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core detector_surface_exposes_phase3_types -v`  
Expected: FAIL with unresolved imports from `papyrus_core::detector`.

**Step 3: Write minimal implementation**

```rust
// papyrus-core/src/detector/mod.rs
use std::collections::HashMap;

use crate::ast::{Document, DocumentMetadata, Node, Span, Warning};
use crate::parser::{FontInfo, RawTextSegment};

#[derive(Debug, Clone, PartialEq)]
pub struct DetectorConfig {
    pub heading_size_ratio: f32,
    pub detect_bold: bool,
    pub detect_italic: bool,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            heading_size_ratio: 1.2,
            detect_bold: true,
            detect_italic: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassifiedSegment {
    pub segment: RawTextSegment,
    pub classification: SegmentClass,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SegmentClass {
    Heading(u8),
    Body,
}

pub fn compute_body_size(_segments: &[RawTextSegment]) -> f32 {
    12.0
}

pub fn detect_headings(
    segments: &[RawTextSegment],
    _body_size: f32,
    _heading_size_ratio: f32,
) -> Vec<ClassifiedSegment> {
    segments
        .iter()
        .cloned()
        .map(|segment| ClassifiedSegment {
            segment,
            classification: SegmentClass::Body,
        })
        .collect()
}

pub fn detect_formatting(_font_name: &str, _font_info: &FontInfo) -> (bool, bool) {
    (false, false)
}

pub fn build_document(
    _segments: Vec<RawTextSegment>,
    _fonts: &HashMap<Vec<u8>, FontInfo>,
    _config: &DetectorConfig,
    metadata: DocumentMetadata,
) -> (Document, Vec<Warning>) {
    (
        Document {
            metadata,
            nodes: Vec::new(),
        },
        Vec::new(),
    )
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core detector_surface_exposes_phase3_types -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/detector/mod.rs papyrus-core/tests/module_surface.rs
git commit -m "feat(detector): add phase-3 detector contracts and config"
```

### Task 2: Implement `compute_body_size` Mode Heuristic

**Files:**
- Modify: `papyrus-core/src/detector/mod.rs`
- Test: `papyrus-core/src/detector/mod.rs` (`#[cfg(test)]` section)

**Step 1: Write the failing test**

```rust
#[test]
fn compute_body_size_uses_mode_with_smaller_tie_breaker() {
    let segments = vec![
        seg("a", 12.0),
        seg("b", 12.0),
        seg("c", 14.0),
        seg("d", 14.0),
        seg("e", 10.0),
        seg("f", 10.0),
    ];

    assert_eq!(compute_body_size(&segments), 10.0);
}

#[test]
fn compute_body_size_returns_default_on_empty_segments() {
    assert_eq!(compute_body_size(&[]), 12.0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core compute_body_size_ -v`  
Expected: FAIL because stub always returns default.

**Step 3: Write minimal implementation**

```rust
pub fn compute_body_size(segments: &[RawTextSegment]) -> f32 {
    if segments.is_empty() {
        return 12.0;
    }

    let mut counts: HashMap<i32, usize> = HashMap::new();
    for segment in segments {
        let key = (segment.font_size * 100.0).round() as i32;
        *counts.entry(key).or_insert(0) += 1;
    }

    let mut best_key = 1200;
    let mut best_count = 0usize;

    for (key, count) in counts {
        if count > best_count || (count == best_count && key < best_key) {
            best_key = key;
            best_count = count;
        }
    }

    best_key as f32 / 100.0
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core compute_body_size_ -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/detector/mod.rs
git commit -m "feat(detector): compute body font size using mode"
```

### Task 3: Implement Heading Classification (`detect_headings`)

**Files:**
- Modify: `papyrus-core/src/detector/mod.rs`
- Test: `papyrus-core/src/detector/mod.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn detect_headings_maps_ratios_to_levels_and_boundaries() {
    let body = 10.0;
    let segments = vec![
        seg("h1", 20.0),
        seg("h2", 17.0),
        seg("h3", 14.0),
        seg("h4", 12.0),
        seg("body", 11.99),
    ];

    let classes = detect_headings(&segments, body, 1.2)
        .into_iter()
        .map(|c| c.classification)
        .collect::<Vec<_>>();

    assert_eq!(classes[0], SegmentClass::Heading(1));
    assert_eq!(classes[1], SegmentClass::Heading(2));
    assert_eq!(classes[2], SegmentClass::Heading(3));
    assert_eq!(classes[3], SegmentClass::Heading(4));
    assert_eq!(classes[4], SegmentClass::Body);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core detect_headings_maps_ratios_to_levels_and_boundaries -v`  
Expected: FAIL because current implementation marks all segments as `Body`.

**Step 3: Write minimal implementation**

```rust
pub fn detect_headings(
    segments: &[RawTextSegment],
    body_size: f32,
    heading_size_ratio: f32,
) -> Vec<ClassifiedSegment> {
    let safe_body = if body_size > 0.0 { body_size } else { 12.0 };

    segments
        .iter()
        .cloned()
        .map(|segment| {
            let ratio = segment.font_size / safe_body;
            let classification = if ratio >= 2.0 {
                SegmentClass::Heading(1)
            } else if ratio >= 1.7 {
                SegmentClass::Heading(2)
            } else if ratio >= 1.4 {
                SegmentClass::Heading(3)
            } else if ratio >= heading_size_ratio {
                SegmentClass::Heading(4)
            } else {
                SegmentClass::Body
            };

            ClassifiedSegment {
                segment,
                classification,
            }
        })
        .collect()
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core detect_headings_maps_ratios_to_levels_and_boundaries -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/detector/mod.rs
git commit -m "feat(detector): classify headings by font ratio thresholds"
```

### Task 4: Extend `FontInfo` with Descriptor Metrics for Formatting Fallback

**Files:**
- Modify: `papyrus-core/src/parser/mod.rs:7-12,67-153`
- Modify: `papyrus-core/tests/module_surface.rs:36-76`
- Test: `papyrus-core/src/parser/mod.rs` and `papyrus-core/tests/module_surface.rs`

**Step 1: Write the failing test**

```rust
// papyrus-core/tests/module_surface.rs
#[test]
fn parser_font_info_exposes_descriptor_metrics() {
    let font = parser::FontInfo {
        name: "Helvetica".to_string(),
        size: None,
        font_weight: Some(700.0),
        italic_angle: Some(-12.0),
    };

    assert_eq!(font.font_weight, Some(700.0));
    assert_eq!(font.italic_angle, Some(-12.0));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core parser_font_info_exposes_descriptor_metrics -v`  
Expected: FAIL because `font_weight` and `italic_angle` fields do not exist.

**Step 3: Write minimal implementation**

```rust
// parser::FontInfo
pub struct FontInfo {
    pub name: String,
    pub size: Option<f32>,
    pub font_weight: Option<f32>,
    pub italic_angle: Option<f32>,
}

fn extract_font_descriptor_metrics(
    doc: &lopdf::Document,
    font_dict: &lopdf::Dictionary,
) -> (Option<f32>, Option<f32>) {
    let descriptor = font_dict
        .get(b"FontDescriptor")
        .ok()
        .and_then(|obj| match obj {
            lopdf::Object::Reference(id) => doc.get_object(*id).ok(),
            other => Some(other),
        })
        .and_then(|obj| obj.as_dict().ok());

    let Some(desc) = descriptor else {
        return (None, None);
    };

    let font_weight = desc
        .get(b"FontWeight")
        .ok()
        .and_then(extract_number);
    let italic_angle = desc
        .get(b"ItalicAngle")
        .ok()
        .and_then(extract_number);

    (font_weight, italic_angle)
}

// inside resolve_fonts_for_page loop
let (font_weight, italic_angle) = extract_font_descriptor_metrics(doc, &font_dict);

fonts.insert(
    resource_name,
    FontInfo {
        name: base_font_name,
        size: None,
        font_weight,
        italic_angle,
    },
);
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core parser_font_info_exposes_descriptor_metrics -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/parser/mod.rs papyrus-core/tests/module_surface.rs
git commit -m "feat(parser): capture font descriptor metrics in font info"
```

### Task 5: Implement `detect_formatting` Heuristics (Name + Descriptor Fallback)

**Files:**
- Modify: `papyrus-core/src/detector/mod.rs`
- Test: `papyrus-core/src/detector/mod.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn detect_formatting_reads_font_name_patterns_and_subset_prefix() {
    let info = font_info("ignored", None, None);

    assert_eq!(detect_formatting("Arial-Bold", &info), (true, false));
    assert_eq!(detect_formatting("TimesNewRoman-Italic", &info), (false, true));
    assert_eq!(detect_formatting("ABCDEF+Helvetica-BoldOblique", &info), (true, true));
}

#[test]
fn detect_formatting_falls_back_to_descriptor_metrics() {
    let info = font_info("mystery-font", Some(700.0), Some(-10.0));
    assert_eq!(detect_formatting("CustomFont-Regular", &info), (true, true));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core detect_formatting_ -v`  
Expected: FAIL because current implementation returns `(false, false)`.

**Step 3: Write minimal implementation**

```rust
fn normalize_font_name(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.len() >= 7
        && lower.as_bytes()[6] == b'+'
        && lower[..6].bytes().all(|b| b.is_ascii_lowercase())
    {
        lower[7..].to_string()
    } else {
        lower
    }
}

pub fn detect_formatting(font_name: &str, font_info: &FontInfo) -> (bool, bool) {
    let normalized = normalize_font_name(font_name);

    let has_bold_combo = normalized.contains("bolditalic") || normalized.contains("boldoblique");
    let mut bold = has_bold_combo || normalized.contains("bold");
    let mut italic = has_bold_combo
        || normalized.contains("italic")
        || normalized.contains("oblique");

    if !bold {
        bold = font_info.font_weight.map(|w| w > 600.0).unwrap_or(false);
    }

    if !italic {
        italic = font_info
            .italic_angle
            .map(|angle| angle.abs() > f32::EPSILON)
            .unwrap_or(false);
    }

    (bold, italic)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core detect_formatting_ -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/detector/mod.rs
git commit -m "feat(detector): implement bold italic detection heuristics"
```

### Task 6: Implement `build_document` (Grouping + RawText Fallback)

**Files:**
- Modify: `papyrus-core/src/detector/mod.rs`
- Test: `papyrus-core/src/detector/mod.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn build_document_groups_consecutive_classification_and_preserves_spans() {
    let segments = vec![
        seg_with_font("Chapter 1", b"F1", 24.0, 1),
        seg_with_font("Intro", b"F1", 24.0, 1),
        seg_with_font("Body A", b"F2", 12.0, 1),
        seg_with_font("Body B", b"F2", 12.0, 1),
    ];

    let fonts = map_fonts([
        (b"F1".to_vec(), font_info("Helvetica-Bold", Some(700.0), None)),
        (b"F2".to_vec(), font_info("Helvetica", None, None)),
    ]);

    let cfg = DetectorConfig::default();
    let metadata = DocumentMetadata {
        title: Some("Demo".to_string()),
        author: None,
        page_count: 1,
    };

    let (doc, warnings) = build_document(segments, &fonts, &cfg, metadata.clone());

    assert!(warnings.is_empty());
    assert_eq!(doc.metadata, metadata);
    assert_eq!(doc.nodes.len(), 2);
    assert!(matches!(doc.nodes[0], Node::Heading { level: 1, .. }));
    assert!(matches!(doc.nodes[1], Node::Paragraph { .. }));
}

#[test]
fn build_document_uses_raw_text_when_font_is_missing() {
    let segments = vec![seg_with_font("Unknown font", b"FX", 12.0, 1)];
    let cfg = DetectorConfig::default();

    let (doc, warnings) = build_document(
        segments,
        &HashMap::new(),
        &cfg,
        DocumentMetadata {
            title: None,
            author: None,
            page_count: 1,
        },
    );

    assert_eq!(doc.nodes, vec![Node::RawText("Unknown font".to_string())]);
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], Warning::MissingFontMetrics { .. }));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core build_document_ -v`  
Expected: FAIL because current implementation returns empty node list.

**Step 3: Write minimal implementation**

```rust
pub fn build_document(
    segments: Vec<RawTextSegment>,
    fonts: &HashMap<Vec<u8>, FontInfo>,
    config: &DetectorConfig,
    metadata: DocumentMetadata,
) -> (Document, Vec<Warning>) {
    let mut warnings = Vec::new();
    let body_size = compute_body_size(&segments);
    let classified = detect_headings(&segments, body_size, config.heading_size_ratio);

    let mut nodes = Vec::new();
    let mut current_kind: Option<SegmentClass> = None;
    let mut current_spans: Vec<Span> = Vec::new();

    for item in classified {
        let font = match fonts.get(&item.segment.font_resource_name) {
            Some(font) => font,
            None => {
                warnings.push(Warning::MissingFontMetrics {
                    font_name: String::from_utf8_lossy(&item.segment.font_resource_name).to_string(),
                    page: item.segment.page_number,
                });
                nodes.push(Node::RawText(item.segment.text));
                current_kind = None;
                current_spans.clear();
                continue;
            }
        };

        let (mut bold, mut italic) = detect_formatting(&font.name, font);
        if !config.detect_bold {
            bold = false;
        }
        if !config.detect_italic {
            italic = false;
        }

        let span = Span {
            text: item.segment.text,
            bold,
            italic,
            font_size: item.segment.font_size,
            font_name: Some(font.name.clone()),
        };

        // flush when classification changes, then append current span
        // Heading(level) => Node::Heading
        // Body => Node::Paragraph
    }

    // final flush

    (
        Document { metadata, nodes },
        warnings,
    )
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core build_document_ -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/detector/mod.rs
git commit -m "feat(detector): build document nodes from classified segments"
```

### Task 7: Add `PapyrusBuilder`, `Papyrus::extract`, and `convert` Public API

**Files:**
- Modify: `papyrus-core/src/lib.rs:1-4`
- Modify: `papyrus-core/tests/module_surface.rs`
- Test: `papyrus-core/tests/module_surface.rs`

**Step 1: Write the failing test**

```rust
// papyrus-core/tests/module_surface.rs
use papyrus_core::{convert, Papyrus};

#[test]
fn public_api_builder_and_convert_are_wired() {
    let engine = Papyrus::builder()
        .heading_size_ratio(1.5)
        .detect_bold(false)
        .detect_italic(false)
        .build();

    let result = engine.extract(b"this is not a pdf");
    assert_eq!(result.document.metadata.page_count, 0);
    assert!(!result.warnings.is_empty());

    let default_result = convert(b"this is not a pdf");
    assert!(!default_result.warnings.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core public_api_builder_and_convert_are_wired -v`  
Expected: FAIL with unresolved `Papyrus`/`convert` items.

**Step 3: Write minimal implementation**

```rust
// papyrus-core/src/lib.rs
pub mod ast;
pub mod detector;
pub mod parser;
pub mod renderer;

use std::collections::HashMap;

use ast::{ConversionResult, Document, DocumentMetadata};
use detector::{build_document, DetectorConfig};

#[derive(Debug, Clone)]
pub struct Papyrus {
    config: DetectorConfig,
}

#[derive(Debug, Clone)]
pub struct PapyrusBuilder {
    config: DetectorConfig,
}

impl PapyrusBuilder {
    pub fn heading_size_ratio(mut self, ratio: f32) -> Self {
        self.config.heading_size_ratio = ratio;
        self
    }

    pub fn detect_bold(mut self, enabled: bool) -> Self {
        self.config.detect_bold = enabled;
        self
    }

    pub fn detect_italic(mut self, enabled: bool) -> Self {
        self.config.detect_italic = enabled;
        self
    }

    pub fn build(self) -> Papyrus {
        Papyrus { config: self.config }
    }
}

impl Papyrus {
    pub fn builder() -> PapyrusBuilder {
        PapyrusBuilder {
            config: DetectorConfig::default(),
        }
    }

    pub fn extract(&self, pdf_bytes: &[u8]) -> ConversionResult {
        let (segments, metadata, mut warnings) = parser::parse_pdf(pdf_bytes);
        let fonts = collect_fonts_for_segments(pdf_bytes, &segments);
        let (document, detector_warnings) = build_document(segments, &fonts, &self.config, metadata);
        warnings.extend(detector_warnings);
        ConversionResult { document, warnings }
    }
}

fn collect_fonts_for_segments(
    pdf_bytes: &[u8],
    segments: &[parser::RawTextSegment],
) -> HashMap<Vec<u8>, parser::FontInfo> {
    let mut fonts = HashMap::new();
    let (doc_opt, _) = parser::load_pdf(pdf_bytes);
    let Some(doc) = doc_opt else {
        return fonts;
    };

    let mut pages = segments.iter().map(|s| s.page_number).collect::<Vec<_>>();
    pages.sort_unstable();
    pages.dedup();

    for page in pages {
        let (page_fonts, _) = parser::resolve_fonts_for_page(&doc, page);
        fonts.extend(page_fonts);
    }

    fonts
}

pub fn convert(pdf_bytes: &[u8]) -> ConversionResult {
    Papyrus::builder().build().extract(pdf_bytes)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core public_api_builder_and_convert_are_wired -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/lib.rs papyrus-core/tests/module_surface.rs
git commit -m "feat(core): expose papyrus builder extract and convert api"
```

### Task 8: Add Full-Pipeline Integration Tests for Phase 3 Behavior

**Files:**
- Create: `papyrus-core/tests/integration_phase3_pipeline.rs`
- Modify: `papyrus-core/tests/module_surface.rs` (remove obsolete detector stub assertions if still present)
- Test: `papyrus-core/tests/integration_phase3_pipeline.rs`

**Step 1: Write the failing test**

```rust
use papyrus_core::ast::Node;
use papyrus_core::{convert, Papyrus};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("workspace root")
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn extract_simple_pdf_has_heading_and_paragraph_nodes() {
    let bytes = std::fs::read(fixture_path("simple.pdf")).expect("fixture exists");
    let result = convert(&bytes);

    assert!(result
        .document
        .nodes
        .iter()
        .any(|n| matches!(n, Node::Heading { .. })));
    assert!(result
        .document
        .nodes
        .iter()
        .any(|n| matches!(n, Node::Paragraph { .. })));
}

#[test]
fn builder_overrides_change_heading_and_bold_detection() {
    let bytes = std::fs::read(fixture_path("multi-heading.pdf")).expect("fixture exists");

    let default = Papyrus::builder().build().extract(&bytes);
    let strict = Papyrus::builder()
        .heading_size_ratio(2.0)
        .detect_bold(false)
        .build()
        .extract(&bytes);

    let default_heading_count = default
        .document
        .nodes
        .iter()
        .filter(|n| matches!(n, Node::Heading { .. }))
        .count();
    let strict_heading_count = strict
        .document
        .nodes
        .iter()
        .filter(|n| matches!(n, Node::Heading { .. }))
        .count();

    assert!(strict_heading_count <= default_heading_count);

    for node in &strict.document.nodes {
        if let Node::Heading { spans, .. } | Node::Paragraph { spans } = node {
            assert!(spans.iter().all(|s| !s.bold));
        }
    }
}

#[test]
fn extract_corrupted_pdf_is_best_effort_and_non_panicking() {
    let bytes = std::fs::read(fixture_path("corrupted.pdf")).expect("fixture exists");
    let result = convert(&bytes);

    assert_eq!(result.document.metadata.page_count, 0);
    assert!(!result.warnings.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core --test integration_phase3_pipeline -v`  
Expected: FAIL until detector grouping/API behavior is complete.

**Step 3: Write minimal implementation**

```rust
// Finalize any remaining gaps from Tasks 1-7:
// - ensure heading classification and grouping produce Heading + Paragraph nodes
// - ensure config flags force bold/italic false
// - ensure corrupted PDFs return ConversionResult with warnings and no panic
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core --test integration_phase3_pipeline -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/tests/integration_phase3_pipeline.rs papyrus-core/tests/module_surface.rs papyrus-core/src/detector/mod.rs papyrus-core/src/lib.rs papyrus-core/src/parser/mod.rs
git commit -m "test(core): add phase-3 extraction pipeline integration coverage"
```

### Task 9: End-to-End Verification and Plan Closure

**Files:**
- Modify: `docs/plans/phase-3-verification-log.md` (create if absent)

**Step 1: Run focused regression suites**

Run: `cargo test -p papyrus-core module_surfaces_are_linked -v`  
Expected: PASS.

**Step 2: Run parser + detector + integration suites**

Run: `cargo test -p papyrus-core parse_pdf -- --nocapture`  
Expected: PASS.

Run: `cargo test -p papyrus-core detect_ -- --nocapture`  
Expected: PASS.

Run: `cargo test -p papyrus-core --test integration_extraction -v`  
Expected: PASS.

Run: `cargo test -p papyrus-core --test integration_phase3_pipeline -v`  
Expected: PASS.

**Step 3: Run full workspace tests**

Run: `cargo test --workspace -v`  
Expected: PASS with no new failures.

**Step 4: Record verification evidence**

```markdown
# docs/plans/phase-3-verification-log.md
- Date: 2026-03-08
- Commands run:
  - cargo test -p papyrus-core module_surfaces_are_linked -v
  - cargo test -p papyrus-core --test integration_phase3_pipeline -v
  - cargo test --workspace -v
- Result: all green
- Notes: warnings are preserved in best-effort flow
```

**Step 5: Commit**

```bash
git add docs/plans/phase-3-verification-log.md
git commit -m "docs: record phase-3 verification evidence"
```

---

## Definition of Done Checklist

- [ ] `compute_body_size()` returns mode font size with deterministic tie-breaker.
- [ ] `detect_headings()` maps ratio boundaries exactly (`2.0`, `1.7`, `1.4`, `heading_size_ratio`).
- [ ] `detect_formatting()` supports name-based detection and descriptor fallback.
- [ ] `build_document()` groups consecutive classification blocks into `Node::Heading`/`Node::Paragraph`.
- [ ] Missing font info creates `Node::RawText` and emits `Warning::MissingFontMetrics`.
- [ ] `PapyrusBuilder` supports `heading_size_ratio`, `detect_bold`, `detect_italic`.
- [ ] `Papyrus::extract()` orchestrates parser + detector and aggregates warnings.
- [ ] `convert()` uses default configuration.
- [ ] New Phase 3 integration tests pass against fixtures.
- [ ] `cargo test --workspace -v` is fully green.
