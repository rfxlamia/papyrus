use std::collections::HashMap;

use crate::ast::{Document, DocumentMetadata, Node, Span, Warning};
use crate::parser::{strip_subset_prefix, FontInfo, RawTextSegment};

/// Configuration for the structure-detection pass.
///
/// All thresholds are relative to the computed body font size.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectorConfig {
    /// Minimum font-size ratio over body size to classify a segment as a heading.
    /// Defaults to `1.2`. Must be less than the fixed level-3 boundary (`1.4`).
    pub heading_size_ratio: f32,
    /// Whether to detect bold formatting from font name and descriptor metrics.
    /// When `false`, all spans have `bold = false` regardless of font data.
    pub detect_bold: bool,
    /// Whether to detect italic formatting from font name and descriptor metrics.
    /// When `false`, all spans have `italic = false` regardless of font data.
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

/// A `RawTextSegment` paired with its structural classification.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassifiedSegment {
    pub segment: RawTextSegment,
    pub classification: SegmentClass,
}

/// The structural role of a text segment within the document.
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentClass {
    /// A heading at the given level (1 = largest, 4 = smallest recognised heading).
    Heading(u8),
    /// Regular body text.
    Body,
}

/// Compute the dominant ("body") font size across all segments using the mode.
///
/// Font sizes are bucketed at 0.01-point precision. On a tie, the smallest size
/// wins — body text is typically the smallest repeated size in a document.
/// Returns `12.0` when `segments` is empty.
pub fn compute_body_size(segments: &[RawTextSegment]) -> f32 {
    if segments.is_empty() {
        return 12.0;
    }

    let mut counts: HashMap<i32, usize> = HashMap::new();
    for segment in segments {
        let key = (segment.font_size * 100.0).round() as i32;
        *counts.entry(key).or_insert(0) += 1;
    }

    // best_key starts at 1200 (12pt) so the very first entry always wins the
    // count comparison (best_count == 0 < any real count), making the seed
    // value irrelevant in practice.
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

/// Classify each segment as `Body` or a heading level based on its font-size
/// ratio relative to `body_size`.
///
/// Fixed level boundaries (ratios are relative to `body_size`):
/// - ≥ 2.0 → `Heading(1)`
/// - ≥ 1.7 → `Heading(2)`
/// - ≥ 1.4 → `Heading(3)`
/// - ≥ `heading_size_ratio` → `Heading(4)`
/// - otherwise → `Body`
///
/// If `body_size` is zero or negative, falls back to 12.0 pt.
pub fn detect_headings(
    segments: Vec<RawTextSegment>,
    body_size: f32,
    heading_size_ratio: f32,
) -> Vec<ClassifiedSegment> {
    let safe_body = if body_size > 0.0 { body_size } else { 12.0 };

    segments
        .into_iter()
        .map(|segment| {
            let ratio = segment.font_size / safe_body;
            let classification = if ratio >= 2.0 {
                // Level 1: at least double the body size
                SegmentClass::Heading(1)
            } else if ratio >= 1.7 {
                // Level 2: 70%+ larger than body
                SegmentClass::Heading(2)
            } else if ratio >= 1.4 {
                // Level 3: 40%+ larger than body
                SegmentClass::Heading(3)
            } else if ratio >= heading_size_ratio {
                // Level 4: exceeds the configurable minimum heading ratio
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

/// Determine bold and italic flags from the font resource name and descriptor metrics.
///
/// Detection order:
/// 1. Lowercase font name (subset prefix stripped) is scanned for `"bold"`,
///    `"italic"`, `"oblique"`, and combined forms like `"bolditalic"`.
/// 2. If bold is not found via name, `FontInfo::font_weight > 600` is used as
///    a fallback.
/// 3. If italic is not found via name, a non-zero `FontInfo::italic_angle` is
///    used as a fallback.
///
/// Returns `(bold, italic)`.
pub fn detect_formatting(font_name: &str, font_info: &FontInfo) -> (bool, bool) {
    // Normalise: strip PDF subset prefix then lowercase for case-insensitive matching.
    let stripped = strip_subset_prefix(font_name);
    let normalized = stripped.to_lowercase();

    // Combined forms must be checked first to avoid double-counting
    // (e.g., "BoldOblique" contains both "bold" and "oblique").
    let has_bold_combo = normalized.contains("bolditalic") || normalized.contains("boldoblique");
    let mut bold = has_bold_combo || normalized.contains("bold");
    let mut italic =
        has_bold_combo || normalized.contains("italic") || normalized.contains("oblique");

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

/// Flush the accumulated spans into an AST node and clear the accumulators.
///
/// No-op when `spans` is empty.
fn flush_group(kind: &Option<SegmentClass>, spans: Vec<Span>, nodes: &mut Vec<Node>) {
    if spans.is_empty() {
        return;
    }
    match kind {
        Some(SegmentClass::Heading(level)) => {
            nodes.push(Node::Heading {
                level: *level,
                spans,
            });
        }
        _ => {
            nodes.push(Node::Paragraph { spans });
        }
    }
}

/// Build an AST `Document` from raw segments, font metadata, and configuration.
///
/// Algorithm:
/// 1. Compute the body font size (mode of all segment sizes).
/// 2. Classify every segment as a heading level or body text.
/// 3. Group consecutive segments with the same classification into a single
///    `Node::Heading` or `Node::Paragraph`.
/// 4. Segments whose font resource name is absent from `fonts` are emitted as
///    `Node::RawText` and contribute a `Warning::MissingFontMetrics`.
///
/// The returned `Vec<Warning>` is empty when all fonts are resolved.
pub fn build_document(
    segments: Vec<RawTextSegment>,
    fonts: &HashMap<Vec<u8>, FontInfo>,
    config: &DetectorConfig,
    metadata: DocumentMetadata,
) -> (Document, Vec<Warning>) {
    let mut warnings = Vec::new();
    let body_size = compute_body_size(&segments);
    let classified = detect_headings(segments, body_size, config.heading_size_ratio);

    let mut nodes = Vec::new();
    let mut current_kind: Option<SegmentClass> = None;
    let mut current_spans: Vec<Span> = Vec::new();

    for item in classified {
        let font = match fonts.get(&item.segment.font_resource_name) {
            Some(font) => font,
            None => {
                // Flush any pending group before emitting RawText so it lands
                // as its own node rather than merging into a preceding group.
                flush_group(
                    &current_kind,
                    std::mem::take(&mut current_spans),
                    &mut nodes,
                );
                current_kind = None;

                warnings.push(Warning::MissingFontMetrics {
                    font_name: String::from_utf8_lossy(&item.segment.font_resource_name)
                        .to_string(),
                    page: item.segment.page_number,
                });
                nodes.push(Node::RawText(item.segment.text));
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

        // Flush the current group when the classification changes.
        let same_class = match (&current_kind, &item.classification) {
            (Some(SegmentClass::Heading(a)), SegmentClass::Heading(b)) => a == b,
            (Some(SegmentClass::Body), SegmentClass::Body) => true,
            (None, _) => true, // first item — nothing to flush
            _ => false,
        };

        if !same_class {
            flush_group(
                &current_kind,
                std::mem::take(&mut current_spans),
                &mut nodes,
            );
        }

        current_kind = Some(item.classification);
        current_spans.push(span);
    }

    // Final flush for any trailing group.
    flush_group(
        &current_kind,
        std::mem::take(&mut current_spans),
        &mut nodes,
    );

    (Document { metadata, nodes }, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str, font_size: f32) -> RawTextSegment {
        RawTextSegment {
            text: text.to_string(),
            font_resource_name: b"F1".to_vec(),
            font_size,
            page_number: 1,
            x: 0.0,
            y: 0.0,
            is_rotated: false,
        }
    }

    fn seg_with_font(
        text: &str,
        font_resource: &[u8],
        font_size: f32,
        page: usize,
    ) -> RawTextSegment {
        RawTextSegment {
            text: text.to_string(),
            font_resource_name: font_resource.to_vec(),
            font_size,
            page_number: page,
            x: 0.0,
            y: 0.0,
            is_rotated: false,
        }
    }

    fn font_info(name: &str, font_weight: Option<f32>, italic_angle: Option<f32>) -> FontInfo {
        FontInfo {
            name: name.to_string(),
            size: None,
            font_weight,
            italic_angle,
        }
    }

    fn map_fonts<const N: usize>(entries: [(Vec<u8>, FontInfo); N]) -> HashMap<Vec<u8>, FontInfo> {
        entries.into_iter().collect()
    }

    // ── compute_body_size ──────────────────────────────────────────────────────

    #[test]
    fn compute_body_size_uses_mode_with_smaller_tie_breaker() {
        // Three sizes each appear twice: smallest (10.0) should win the tie.
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

    // ── detect_headings ────────────────────────────────────────────────────────

    #[test]
    fn detect_headings_maps_ratios_to_levels_and_boundaries() {
        let body = 10.0;
        // Exact boundary values per spec: 2.0, 1.7, 1.4, and heading_size_ratio (1.2).
        let segments = vec![
            seg("h1", 20.0),    // ratio 2.0 → Heading(1)
            seg("h2", 17.0),    // ratio 1.7 → Heading(2)
            seg("h3", 14.0),    // ratio 1.4 → Heading(3)
            seg("h4", 12.0),    // ratio 1.2 → Heading(4)
            seg("body", 11.99), // ratio < 1.2 → Body
        ];

        let classes = detect_headings(segments, body, 1.2)
            .into_iter()
            .map(|c| c.classification)
            .collect::<Vec<_>>();

        assert_eq!(classes[0], SegmentClass::Heading(1));
        assert_eq!(classes[1], SegmentClass::Heading(2));
        assert_eq!(classes[2], SegmentClass::Heading(3));
        assert_eq!(classes[3], SegmentClass::Heading(4));
        assert_eq!(classes[4], SegmentClass::Body);
    }

    // ── detect_formatting ──────────────────────────────────────────────────────

    #[test]
    fn detect_formatting_reads_font_name_patterns_and_subset_prefix() {
        let info = font_info("ignored", None, None);

        assert_eq!(detect_formatting("Arial-Bold", &info), (true, false));
        assert_eq!(
            detect_formatting("TimesNewRoman-Italic", &info),
            (false, true)
        );
        assert_eq!(
            detect_formatting("ABCDEF+Helvetica-BoldOblique", &info),
            (true, true)
        );
    }

    #[test]
    fn detect_formatting_falls_back_to_descriptor_metrics() {
        let info = font_info("mystery-font", Some(700.0), Some(-10.0));
        assert_eq!(detect_formatting("CustomFont-Regular", &info), (true, true));
    }

    // ── build_document ─────────────────────────────────────────────────────────

    #[test]
    fn build_document_groups_consecutive_classification_and_preserves_spans() {
        let segments = vec![
            seg_with_font("Chapter 1", b"F1", 24.0, 1),
            seg_with_font("Intro", b"F1", 24.0, 1),
            seg_with_font("Body A", b"F2", 12.0, 1),
            seg_with_font("Body B", b"F2", 12.0, 1),
        ];

        let fonts = map_fonts([
            (
                b"F1".to_vec(),
                font_info("Helvetica-Bold", Some(700.0), None),
            ),
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

        // Verify heading node: level and span texts
        match &doc.nodes[0] {
            Node::Heading { level, spans } => {
                assert_eq!(*level, 1);
                assert_eq!(spans.len(), 2);
                assert_eq!(spans[0].text, "Chapter 1");
                assert_eq!(spans[1].text, "Intro");
                assert!(spans[0].bold);
            }
            other => panic!("expected Heading, got {:?}", other),
        }

        // Verify paragraph node: span texts
        match &doc.nodes[1] {
            Node::Paragraph { spans } => {
                assert_eq!(spans.len(), 2);
                assert_eq!(spans[0].text, "Body A");
                assert_eq!(spans[1].text, "Body B");
                assert!(!spans[0].bold);
            }
            other => panic!("expected Paragraph, got {:?}", other),
        }
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
}
