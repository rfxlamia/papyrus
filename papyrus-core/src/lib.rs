pub mod ast;
pub mod detector;
pub mod layout;
pub mod parser;
pub mod renderer;

use std::collections::HashMap;

use ast::ConversionResult;
use detector::{build_document, DetectorConfig};

/// A configured extraction engine.
///
/// Construct via [`Papyrus::builder`] to customise detection thresholds,
/// or call the top-level [`convert`] function for zero-configuration extraction.
#[derive(Debug, Clone)]
pub struct Papyrus {
    config: DetectorConfig,
}

/// Builder for [`Papyrus`].
///
/// All settings have sensible defaults via [`DetectorConfig::default`]; only
/// set the values you want to override.
#[derive(Debug, Clone)]
pub struct PapyrusBuilder {
    config: DetectorConfig,
}

impl PapyrusBuilder {
    /// Minimum font-size ratio over the computed body size to treat a segment
    /// as a heading. Must be less than `1.4` (the fixed level-3 boundary).
    /// Default: `1.2`.
    pub fn heading_size_ratio(mut self, ratio: f32) -> Self {
        self.config.heading_size_ratio = ratio;
        self
    }

    /// Enable or disable bold detection from font name / descriptor metrics.
    /// When `false`, all spans have `bold = false`. Default: `true`.
    pub fn detect_bold(mut self, enabled: bool) -> Self {
        self.config.detect_bold = enabled;
        self
    }

    /// Enable or disable italic detection from font name / descriptor metrics.
    /// When `false`, all spans have `italic = false`. Default: `true`.
    pub fn detect_italic(mut self, enabled: bool) -> Self {
        self.config.detect_italic = enabled;
        self
    }

    /// Consume the builder and return a configured [`Papyrus`] engine.
    pub fn build(self) -> Papyrus {
        Papyrus {
            config: self.config,
        }
    }
}

impl Papyrus {
    /// Return a [`PapyrusBuilder`] pre-loaded with default settings.
    pub fn builder() -> PapyrusBuilder {
        PapyrusBuilder {
            config: DetectorConfig::default(),
        }
    }

    /// Extract structured content from `pdf_bytes`.
    ///
    /// Parsing and detection are best-effort: any problems are captured as
    /// [`ast::Warning`] values in the returned [`ConversionResult`] rather
    /// than surfaced as errors.
    pub fn extract(&self, pdf_bytes: &[u8]) -> ConversionResult {
        extract_with_config(pdf_bytes, &self.config)
    }
}

/// Extract structured content from `pdf_bytes` using default settings.
///
/// Equivalent to `Papyrus::builder().build().extract(pdf_bytes)`.
pub fn convert(pdf_bytes: &[u8]) -> ConversionResult {
    extract_with_config(pdf_bytes, &DetectorConfig::default())
}

/// Core single-pass extraction: load PDF once, resolve fonts and text per page
/// in one pass, then run the detector.
///
/// This is the shared implementation for both [`Papyrus::extract`] and
/// [`convert`]. Keeping it here avoids a redundant `Papyrus::builder().build()`
/// allocation in the hot path.
fn extract_with_config(pdf_bytes: &[u8], config: &DetectorConfig) -> ConversionResult {
    use ast::{DocumentMetadata, Warning};

    let mut all_warnings: Vec<Warning> = Vec::new();

    // Step 1: Load PDF — one load for the entire extraction.
    let (doc_opt, load_warnings) = parser::load_pdf(pdf_bytes);
    all_warnings.extend(load_warnings);

    let doc = match doc_opt {
        Some(d) => d,
        None => {
            let (document, _) = build_document(
                Vec::new(),
                &HashMap::new(),
                config,
                DocumentMetadata {
                    title: None,
                    author: None,
                    page_count: 0,
                },
            );
            return ConversionResult {
                document,
                warnings: all_warnings,
            };
        }
    };

    // Step 2: Metadata.
    let pages = doc.get_pages();
    let page_count = pages.len();
    let (title, author) = parser::extract_doc_info_pub(&doc);
    let metadata = DocumentMetadata {
        title,
        author,
        page_count,
    };

    // Step 3: Per-page font resolution + text extraction in a single pass.
    // Fonts are keyed by (page_number, resource_name) to avoid cross-page
    // collisions when two pages share the same resource name (e.g., both use
    // "F1" for different physical fonts).
    let mut page_fonts_map: HashMap<(usize, Vec<u8>), parser::FontInfo> = HashMap::new();
    let mut all_segments: Vec<parser::RawTextSegment> = Vec::new();

    let mut page_numbers: Vec<u32> = pages.keys().copied().collect();
    page_numbers.sort();

    for &page_num in &page_numbers {
        let page_number = page_num as usize;

        let (fonts, font_warnings) = parser::resolve_fonts_for_page(&doc, page_number);
        all_warnings.extend(font_warnings);

        // Store fonts under (page, resource_name) key.
        for (resource_name, font_info) in fonts {
            page_fonts_map.insert((page_number, resource_name), font_info);
        }

        let (segments, extract_warnings) =
            parser::extract_text_segments_for_page(&doc, page_number, &HashMap::new());
        all_warnings.extend(extract_warnings);

        // Detect image-only pages (no text segments after parsing)
        if segments.is_empty() {
            all_warnings.push(Warning::ImageOnlyPage { page: page_number });
        }

        // Detect and quarantine rotated text
        let rotated = layout::collect_rotated(&segments);
        if !rotated.is_empty() {
            all_warnings.push(Warning::RotatedTextDetected {
                page: page_number,
                segment_count: rotated.len(),
            });
        }

        all_segments.extend(segments);
    }

    // Step 4: Spatial layout — reconstruct lines from position data.
    // Group segments per page, apply Y-grouping and X-gap analysis,
    // produce one segment per line with proper text reconstruction.
    let layout_segments = apply_spatial_layout(&all_segments, &page_fonts_map);

    // Build a flat resource-name → FontInfo map for build_document.
    // Since segments carry their page number, we look up the correct font
    // per (page, resource_name) and flatten into a per-segment map.
    let segment_fonts =
        build_segment_font_map(&layout_segments, &page_fonts_map, &mut all_warnings);

    // Step 5: Detect structure and build AST.
    let (document, detector_warnings) =
        build_document(layout_segments, &segment_fonts, config, metadata);
    all_warnings.extend(detector_warnings);

    ConversionResult {
        document,
        warnings: all_warnings,
    }
}

/// Build a `font_resource_name → FontInfo` map for use in `build_document`.
///
/// Iterates over all segments and looks up each `(page_number, resource_name)`
/// pair from the pre-resolved `page_fonts_map`. The result is a flat map keyed
/// only by `resource_name` (matching `build_document`'s lookup key).
///
/// **Known limitation:** `build_document` keys fonts by resource name alone, so
/// if two pages use the same resource name (e.g., `F1`) for different physical
/// fonts, the last writer wins. This matches the behaviour of `parser::parse_pdf`
/// and is acceptable for the current single-pass architecture. A future
/// improvement would thread the page number through to `build_document`.
///
/// Missing entries emit `Warning::MissingFontMetrics`, deduplicated per
/// resource name to avoid warning spam on multi-segment pages.
fn build_segment_font_map(
    segments: &[parser::RawTextSegment],
    page_fonts_map: &HashMap<(usize, Vec<u8>), parser::FontInfo>,
    warnings: &mut Vec<ast::Warning>,
) -> HashMap<Vec<u8>, parser::FontInfo> {
    let mut result: HashMap<Vec<u8>, parser::FontInfo> = HashMap::new();
    let mut warned: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();

    for segment in segments {
        let key = (segment.page_number, segment.font_resource_name.clone());
        match page_fonts_map.get(&key) {
            Some(font_info) => {
                // Last-page-wins on collision, consistent with parse_pdf behaviour.
                result.insert(segment.font_resource_name.clone(), font_info.clone());
            }
            None => {
                if warned.insert(segment.font_resource_name.clone()) {
                    warnings.push(ast::Warning::MissingFontMetrics {
                        font_name: String::from_utf8_lossy(&segment.font_resource_name).to_string(),
                        page: segment.page_number,
                    });
                }
            }
        }
    }

    result
}

/// Apply spatial layout analysis to transform raw segments into line-based segments.
///
/// Groups segments per page by Y-proximity, reconstructs line text with X-gap
/// word spacing, and inserts paragraph breaks where Y-gaps exceed the threshold.
/// Rotated segments are quarantined and appended after normal text per page.
fn apply_spatial_layout(
    all_segments: &[parser::RawTextSegment],
    _page_fonts_map: &HashMap<(usize, Vec<u8>), parser::FontInfo>,
) -> Vec<parser::RawTextSegment> {
    use detector::compute_body_size;

    if all_segments.is_empty() {
        return Vec::new();
    }

    let body_size = compute_body_size(all_segments);

    // Group segments by page number
    let mut pages: std::collections::BTreeMap<usize, Vec<&parser::RawTextSegment>> =
        std::collections::BTreeMap::new();
    for seg in all_segments {
        pages.entry(seg.page_number).or_default().push(seg);
    }

    let mut result = Vec::new();

    for (&page_number, page_segments) in &pages {
        // Collect owned references for layout functions
        let owned_segs: Vec<parser::RawTextSegment> =
            page_segments.iter().map(|s| (*s).clone()).collect();

        let lines = layout::group_into_lines(&owned_segs, body_size);
        let rotated = layout::collect_rotated(&owned_segs);

        if lines.is_empty() && rotated.is_empty() {
            continue;
        }

        let median_height = layout::compute_median_line_height(&lines, body_size);

        for (i, line) in lines.iter().enumerate() {
            // Get the dominant font info from the first segment in the line
            let first_seg = line[0];

            // Reconstruct text for this line
            let line_text = layout::reconstruct_line_text(line);

            if line_text.trim().is_empty() {
                continue;
            }

            // Detect paragraph break before this line (not before the first line)
            if i > 0 {
                let prev_y = lines[i - 1].first().map(|s| s.y).unwrap_or(0.0);
                let curr_y = line.first().map(|s| s.y).unwrap_or(0.0);
                if layout::is_paragraph_break(prev_y, curr_y, median_height) {
                    // Insert an empty paragraph marker segment
                    result.push(parser::RawTextSegment {
                        text: String::new(),
                        font_resource_name: first_seg.font_resource_name.clone(),
                        font_size: first_seg.font_size,
                        page_number,
                        x: first_seg.x,
                        y: first_seg.y,
                        is_rotated: false,
                    });
                }
            }

            result.push(parser::RawTextSegment {
                text: line_text,
                font_resource_name: first_seg.font_resource_name.clone(),
                font_size: first_seg.font_size,
                page_number,
                x: first_seg.x,
                y: first_seg.y,
                is_rotated: false,
            });
        }

        // Append quarantined rotated text at end of page
        for seg in &rotated {
            result.push(parser::RawTextSegment {
                text: seg.text.clone(),
                font_resource_name: seg.font_resource_name.clone(),
                font_size: seg.font_size,
                page_number,
                x: seg.x,
                y: seg.y,
                is_rotated: true,
            });
        }
    }

    result
}
