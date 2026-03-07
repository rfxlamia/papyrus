use std::collections::HashMap;

use crate::ast::{DocumentMetadata, Warning};

/// Font metadata resolved from a PDF font dictionary.
#[derive(Debug, Clone, PartialEq)]
pub struct FontInfo {
    /// The normalized font name (subset prefix stripped).
    pub name: String,
    /// Optional font size from the font descriptor (diagnostic only; Tf state is authoritative).
    pub size: Option<f32>,
}

/// A single text segment extracted from a PDF content stream.
#[derive(Debug, Clone, PartialEq)]
pub struct RawTextSegment {
    /// Decoded UTF-8 text content.
    pub text: String,
    /// The font resource name as it appears in the content stream (e.g., b"F1").
    pub font_resource_name: Vec<u8>,
    /// Font size from the current Tf text state.
    pub font_size: f32,
    /// 1-based page number.
    pub page_number: usize,
}

/// Load PDF bytes into a lopdf Document, mapping all failures to warnings.
///
/// Returns `(None, warnings)` on failure, `(Some(doc), warnings)` on success.
pub fn load_pdf(bytes: &[u8]) -> (Option<lopdf::Document>, Vec<Warning>) {
    if bytes.is_empty() {
        return (
            None,
            vec![Warning::MalformedPdfObject {
                detail: "empty PDF bytes".to_string(),
            }],
        );
    }

    match lopdf::Document::load_mem(bytes) {
        Ok(doc) => (Some(doc), Vec::new()),
        Err(e) => (
            None,
            vec![Warning::MalformedPdfObject {
                detail: format!("failed to load PDF: {}", e),
            }],
        ),
    }
}

/// Strip a 6-uppercase-letter subset prefix (e.g., "ABCDEF+Helvetica-Bold" -> "Helvetica-Bold").
fn strip_subset_prefix(name: &str) -> &str {
    if name.len() >= 7
        && name.as_bytes()[6] == b'+'
        && name[..6].bytes().all(|b| b.is_ascii_uppercase())
    {
        &name[7..]
    } else {
        name
    }
}

/// Resolve font dictionaries for a given page.
///
/// Returns a map of font resource name (e.g., b"F1") to FontInfo, plus any warnings.
/// `page_number` is 1-based.
pub fn resolve_fonts_for_page(
    doc: &lopdf::Document,
    page_number: usize,
) -> (HashMap<Vec<u8>, FontInfo>, Vec<Warning>) {
    let mut fonts = HashMap::new();
    let mut warnings = Vec::new();

    // Get the page ObjectId from the 1-based page number
    let pages = doc.get_pages();
    let page_num_u32 = match u32::try_from(page_number) {
        Ok(n) => n,
        Err(_) => {
            warnings.push(Warning::MalformedPdfObject {
                detail: format!("page number {} exceeds u32 range", page_number),
            });
            return (fonts, warnings);
        }
    };
    let page_id = match pages.get(&page_num_u32) {
        Some(id) => *id,
        None => {
            warnings.push(Warning::MalformedPdfObject {
                detail: format!(
                    "page {} not found (document has {} pages)",
                    page_number,
                    pages.len()
                ),
            });
            return (fonts, warnings);
        }
    };

    // Use lopdf's built-in font resolution
    let page_fonts = match doc.get_page_fonts(page_id) {
        Ok(f) => f,
        Err(e) => {
            warnings.push(Warning::MalformedPdfObject {
                detail: format!(
                    "failed to read font resources for page {}: {}",
                    page_number, e
                ),
            });
            return (fonts, warnings);
        }
    };

    for (resource_name, font_dict) in page_fonts {
        // Extract BaseFont name
        let base_font_name = match font_dict.get(b"BaseFont") {
            Ok(obj) => match obj.as_name() {
                Ok(name_bytes) => {
                    let raw_name = String::from_utf8_lossy(name_bytes).to_string();
                    strip_subset_prefix(&raw_name).to_string()
                }
                Err(_) => {
                    warnings.push(Warning::MissingFontMetrics {
                        font_name: "<unknown>".to_string(),
                        page: page_number,
                    });
                    continue;
                }
            },
            Err(_) => {
                warnings.push(Warning::MissingFontMetrics {
                    font_name: "<unknown>".to_string(),
                    page: page_number,
                });
                continue;
            }
        };

        // Note: FontInfo.size is diagnostic only — Tf state is authoritative for
        // RawTextSegment.font_size. Standard PDF font descriptors don't carry a
        // /FontSize key, so we leave this as None. It may be populated by future
        // heuristics if needed.

        fonts.insert(
            resource_name,
            FontInfo {
                name: base_font_name,
                size: None,
            },
        );
    }

    (fonts, warnings)
}

/// Decode raw PDF string bytes to UTF-8.
///
/// Handles UTF-16BE (with or without BOM) and falls back to WinAnsi/Latin-1.
/// Emits `Warning::UnsupportedEncoding` only on truly unknown encodings (not in this phase).
fn decode_pdf_string(bytes: &[u8]) -> String {
    // Check for UTF-16BE BOM (0xFE 0xFF)
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM — skip BOM bytes
        return decode_utf16be(&bytes[2..]);
    }

    // Check for UTF-16BE without BOM: heuristic — if first byte is 0x00 and
    // length is even, it's likely UTF-16BE (common in CIDFont text)
    if bytes.len() >= 2 && bytes.len() % 2 == 0 && bytes[0] == 0x00 {
        return decode_utf16be(bytes);
    }

    // Default: WinAnsi / Latin-1 (ISO 8859-1) — each byte maps directly to a Unicode codepoint
    bytes.iter().map(|&b| b as char).collect()
}

/// Decode UTF-16BE bytes into a String.
fn decode_utf16be(bytes: &[u8]) -> String {
    let u16_iter = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
    char::decode_utf16(u16_iter)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Extract raw text segments from a page's content stream.
///
/// Processes Tf, Tj, TJ, BT, and ET operators.
/// `page_number` is 1-based.
pub fn extract_text_segments_for_page(
    doc: &lopdf::Document,
    page_number: usize,
    _fonts: &HashMap<Vec<u8>, FontInfo>,
) -> (Vec<RawTextSegment>, Vec<Warning>) {
    let mut segments = Vec::new();
    let mut warnings = Vec::new();

    // Get page ObjectId
    let pages = doc.get_pages();
    let page_num_u32 = match u32::try_from(page_number) {
        Ok(n) => n,
        Err(_) => return (segments, warnings),
    };
    let page_id = match pages.get(&page_num_u32) {
        Some(id) => *id,
        None => return (segments, warnings),
    };

    // Decode content stream
    let content = match doc.get_and_decode_page_content(page_id) {
        Ok(c) => c,
        Err(e) => {
            warnings.push(Warning::UnreadableTextStream {
                page: page_number,
                detail: format!("failed to decode content stream: {}", e),
            });
            return (segments, warnings);
        }
    };

    // Text state machine
    let mut current_font_resource: Option<Vec<u8>> = None;
    let mut current_font_size: Option<f32> = None;
    let mut tf_set_in_text_object = false;

    for op in content.operations.iter() {
        match op.operator.as_str() {
            "BT" => {
                // Begin text object — reset state
                current_font_resource = None;
                current_font_size = None;
                tf_set_in_text_object = false;
            }
            "ET" => {
                // End text object — reset state
                current_font_resource = None;
                current_font_size = None;
                tf_set_in_text_object = false;
            }
            "Tf" => {
                // Set font: operands are [Name, Number]
                if op.operands.len() >= 2 {
                    if let Some(name_bytes) = extract_name(&op.operands[0]) {
                        current_font_resource = Some(name_bytes);
                    }
                    if let Some(size) = extract_number(&op.operands[1]) {
                        current_font_size = Some(size);
                    }
                    tf_set_in_text_object = true;
                }
            }
            "Tj" => {
                // Show string: operand is [String]
                if let Some(text_bytes) = op.operands.first().and_then(extract_string_bytes) {
                    let text = decode_pdf_string(&text_bytes);
                    if !text.is_empty() {
                        let (font_res, font_sz) = get_text_state_or_default(
                            &current_font_resource,
                            current_font_size,
                            tf_set_in_text_object,
                            page_number,
                            &mut warnings,
                        );
                        segments.push(RawTextSegment {
                            text,
                            font_resource_name: font_res,
                            font_size: font_sz,
                            page_number,
                        });
                    }
                }
            }
            "TJ" => {
                // Show array of strings/numbers: operand is [Array]
                if let Some(lopdf::Object::Array(arr)) = op.operands.first() {
                    let mut combined = String::new();
                    for item in arr {
                        if let Some(bytes) = extract_string_bytes(item) {
                            combined.push_str(&decode_pdf_string(&bytes));
                        }
                        // Numeric entries are kerning adjustments — skip them
                    }
                    if !combined.is_empty() {
                        let (font_res, font_sz) = get_text_state_or_default(
                            &current_font_resource,
                            current_font_size,
                            tf_set_in_text_object,
                            page_number,
                            &mut warnings,
                        );
                        segments.push(RawTextSegment {
                            text: combined,
                            font_resource_name: font_res,
                            font_size: font_sz,
                            page_number,
                        });
                    }
                }
            }
            _ => {
                // Ignore all other operators
            }
        }
    }

    (segments, warnings)
}

/// Extract font resource name or defaults if Tf not yet set.
/// Emits a warning on first use without Tf.
fn get_text_state_or_default(
    current_font_resource: &Option<Vec<u8>>,
    current_font_size: Option<f32>,
    tf_set: bool,
    page_number: usize,
    warnings: &mut Vec<Warning>,
) -> (Vec<u8>, f32) {
    if tf_set {
        (
            current_font_resource
                .clone()
                .unwrap_or_else(|| b"<unknown>".to_vec()),
            current_font_size.unwrap_or(0.0),
        )
    } else {
        warnings.push(Warning::MalformedPdfObject {
            detail: format!("text state not set before Tj/TJ on page {}", page_number),
        });
        (b"<unknown>".to_vec(), 0.0)
    }
}

/// Extract a Name value (bytes) from a lopdf Object.
fn extract_name(obj: &lopdf::Object) -> Option<Vec<u8>> {
    match obj {
        lopdf::Object::Name(n) => Some(n.clone()),
        _ => None,
    }
}

/// Extract a numeric value (f32) from a lopdf Object.
fn extract_number(obj: &lopdf::Object) -> Option<f32> {
    match obj {
        lopdf::Object::Real(f) => Some(*f),
        lopdf::Object::Integer(i) => Some(*i as f32),
        _ => None,
    }
}

/// Extract raw bytes from a String object.
fn extract_string_bytes(obj: &lopdf::Object) -> Option<Vec<u8>> {
    match obj {
        lopdf::Object::String(bytes, _) => Some(bytes.clone()),
        _ => None,
    }
}

/// End-to-end PDF parsing: load, extract metadata, resolve fonts, extract text segments.
///
/// Never panics. Returns empty results with warnings on failure.
pub fn parse_pdf(bytes: &[u8]) -> (Vec<RawTextSegment>, DocumentMetadata, Vec<Warning>) {
    let _ = bytes;
    (
        Vec::new(),
        DocumentMetadata {
            title: None,
            author: None,
            page_count: 0,
        },
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Helper to resolve fixture paths relative to the workspace root.
    fn fixture_path(name: &str) -> PathBuf {
        // CARGO_MANIFEST_DIR points to papyrus-core/
        // Fixtures are at ../tests/fixtures/
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .expect("workspace root")
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    // ── load_pdf tests ──

    #[test]
    fn load_pdf_empty_bytes_returns_none_with_warning() {
        let (doc, warnings) = load_pdf(b"");
        assert!(doc.is_none(), "empty bytes should not produce a document");
        assert!(!warnings.is_empty(), "should emit at least one warning");
        match &warnings[0] {
            Warning::MalformedPdfObject { detail } => {
                assert!(!detail.is_empty(), "detail should be non-empty");
            }
            other => panic!("expected MalformedPdfObject, got {:?}", other),
        }
    }

    #[test]
    fn load_pdf_invalid_header_returns_none_with_warning() {
        let (doc, warnings) = load_pdf(b"this is not a PDF");
        assert!(
            doc.is_none(),
            "invalid header should not produce a document"
        );
        assert!(!warnings.is_empty(), "should emit at least one warning");
        match &warnings[0] {
            Warning::MalformedPdfObject { detail } => {
                assert!(!detail.is_empty(), "detail should be non-empty");
            }
            other => panic!("expected MalformedPdfObject, got {:?}", other),
        }
    }

    #[test]
    fn load_pdf_corrupted_fixture_returns_none_with_warning() {
        let path = fixture_path("corrupted.pdf");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("corrupted.pdf fixture must exist at {:?}: {}", path, e));
        let (doc, warnings) = load_pdf(&bytes);
        assert!(doc.is_none(), "corrupted PDF should not produce a document");
        assert!(!warnings.is_empty(), "should emit at least one warning");
        match &warnings[0] {
            Warning::MalformedPdfObject { detail } => {
                assert!(!detail.is_empty(), "detail should be non-empty");
            }
            other => panic!("expected MalformedPdfObject, got {:?}", other),
        }
    }

    #[test]
    fn load_pdf_valid_simple_fixture_returns_some() {
        let path = fixture_path("simple.pdf");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("simple.pdf fixture must exist at {:?}: {}", path, e));
        let (doc, warnings) = load_pdf(&bytes);
        assert!(doc.is_some(), "valid PDF should produce a document");
        // A valid, well-formed PDF should not produce MalformedPdfObject warnings
        for w in &warnings {
            match w {
                Warning::MalformedPdfObject { .. } => {
                    panic!("valid PDF should not produce MalformedPdfObject warning");
                }
                _ => {}
            }
        }
    }

    // ── resolve_fonts_for_page tests ──

    /// Helper to load a fixture PDF for font resolution tests.
    fn load_fixture(name: &str) -> lopdf::Document {
        let path = fixture_path(name);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("fixture {} must exist at {:?}: {}", name, path, e));
        let (doc, _) = load_pdf(&bytes);
        doc.expect("fixture should be a valid PDF")
    }

    #[test]
    fn resolve_fonts_for_page_simple_returns_font_entries() {
        let doc = load_fixture("simple.pdf");
        let (fonts, warnings) = resolve_fonts_for_page(&doc, 1);
        // simple.pdf uses Helvetica — there should be at least one font
        assert!(
            !fonts.is_empty(),
            "simple.pdf page 1 should have font entries"
        );
        // Check that at least one font has "Helvetica" in its name
        let has_helvetica = fonts.values().any(|f| f.name.contains("Helvetica"));
        assert!(
            has_helvetica,
            "simple.pdf should have a Helvetica font, got: {:?}",
            fonts
        );
        // No warnings expected for a well-formed page
        let malformed_warnings: Vec<_> = warnings
            .iter()
            .filter(|w| matches!(w, Warning::MissingFontMetrics { .. }))
            .collect();
        assert!(
            malformed_warnings.is_empty(),
            "well-formed page should not produce MissingFontMetrics warnings"
        );
    }

    #[test]
    fn resolve_fonts_for_page_bold_italic_returns_bold_and_italic_fonts() {
        let doc = load_fixture("bold-italic.pdf");
        let (fonts, _) = resolve_fonts_for_page(&doc, 1);
        assert!(
            !fonts.is_empty(),
            "bold-italic.pdf page 1 should have font entries"
        );
        // Should have a Bold font and an Oblique/Italic font
        let names: Vec<&str> = fonts.values().map(|f| f.name.as_str()).collect();
        let has_bold = names.iter().any(|n| n.to_lowercase().contains("bold"));
        let has_italic = names
            .iter()
            .any(|n| n.to_lowercase().contains("oblique") || n.to_lowercase().contains("italic"));
        assert!(
            has_bold,
            "bold-italic.pdf should have a Bold font, got: {:?}",
            names
        );
        assert!(
            has_italic,
            "bold-italic.pdf should have an Oblique/Italic font, got: {:?}",
            names
        );
    }

    #[test]
    fn resolve_fonts_for_page_preserves_resource_names_as_keys() {
        let doc = load_fixture("simple.pdf");
        let (fonts, _) = resolve_fonts_for_page(&doc, 1);
        // Keys should be font resource names like b"F1", b"Helv", etc.
        for key in fonts.keys() {
            assert!(
                !key.is_empty(),
                "font resource name key should not be empty"
            );
        }
    }

    // ── extract_text_segments_for_page tests ──

    #[test]
    fn extract_text_segments_for_page_simple_returns_segments() {
        let doc = load_fixture("simple.pdf");
        let (fonts, _) = resolve_fonts_for_page(&doc, 1);
        let (segments, warnings) = extract_text_segments_for_page(&doc, 1, &fonts);
        assert!(
            !segments.is_empty(),
            "simple.pdf page 1 should produce text segments"
        );
        // All segments should be page 1
        for seg in &segments {
            assert_eq!(seg.page_number, 1, "all segments should be page 1");
        }
        // Combined text should contain "Chapter 1" (from oracle)
        let combined: String = segments.iter().map(|s| s.text.as_str()).collect();
        assert!(
            combined.contains("Chapter 1"),
            "simple.pdf should contain 'Chapter 1', got: {:?}",
            combined
        );
        // No UnreadableTextStream warnings for well-formed page
        for w in &warnings {
            if let Warning::UnreadableTextStream { .. } = w {
                panic!("well-formed page should not produce UnreadableTextStream");
            }
        }
    }

    #[test]
    fn extract_text_segments_for_page_bold_italic_returns_different_fonts() {
        let doc = load_fixture("bold-italic.pdf");
        let (fonts, _) = resolve_fonts_for_page(&doc, 1);
        let (segments, _) = extract_text_segments_for_page(&doc, 1, &fonts);
        assert!(
            !segments.is_empty(),
            "bold-italic.pdf page 1 should produce text segments"
        );
        // Should have segments with different font resource names (bold vs italic vs regular)
        let unique_fonts: std::collections::HashSet<&Vec<u8>> =
            segments.iter().map(|s| &s.font_resource_name).collect();
        assert!(
            unique_fonts.len() >= 2,
            "bold-italic.pdf should use at least 2 different fonts, got: {:?}",
            unique_fonts
        );
    }

    #[test]
    fn extract_text_segments_for_page_font_size_comes_from_tf_state() {
        let doc = load_fixture("simple.pdf");
        let (fonts, _) = resolve_fonts_for_page(&doc, 1);
        let (segments, _) = extract_text_segments_for_page(&doc, 1, &fonts);
        // All segments should have a positive font size from Tf state
        for seg in &segments {
            assert!(
                seg.font_size > 0.0,
                "font_size should be positive (from Tf), got: {}",
                seg.font_size
            );
        }
    }

    #[test]
    fn extract_text_segments_for_page_preserves_operator_encounter_order() {
        let doc = load_fixture("simple.pdf");
        let (fonts, _) = resolve_fonts_for_page(&doc, 1);
        let (segments, _) = extract_text_segments_for_page(&doc, 1, &fonts);
        // Segments should appear in content stream order
        // For simple.pdf, "Chapter 1" should come before "Body text." (per oracle)
        let combined: String = segments.iter().map(|s| s.text.as_str()).collect();
        if combined.contains("Chapter 1") && combined.contains("Body text.") {
            let chapter_pos = combined.find("Chapter 1").unwrap();
            let body_pos = combined.find("Body text.").unwrap();
            assert!(
                chapter_pos < body_pos,
                "Chapter 1 should come before Body text. in operator order"
            );
        }
    }

    // ── strip_subset_prefix direct tests ──

    #[test]
    fn strip_subset_prefix_strips_valid_prefix() {
        assert_eq!(
            strip_subset_prefix("ABCDEF+Helvetica-Bold"),
            "Helvetica-Bold"
        );
    }

    #[test]
    fn strip_subset_prefix_strips_any_six_uppercase_letters() {
        assert_eq!(strip_subset_prefix("ZZZZZZ+TimesNewRoman"), "TimesNewRoman");
    }

    #[test]
    fn strip_subset_prefix_leaves_non_prefixed_name() {
        assert_eq!(strip_subset_prefix("Helvetica"), "Helvetica");
    }

    #[test]
    fn strip_subset_prefix_leaves_short_names() {
        assert_eq!(strip_subset_prefix("AB+X"), "AB+X");
    }

    #[test]
    fn strip_subset_prefix_leaves_lowercase_prefix() {
        // Lowercase letters before + don't match the subset pattern
        assert_eq!(strip_subset_prefix("abcdef+Font"), "abcdef+Font");
    }

    #[test]
    fn strip_subset_prefix_leaves_mixed_case_prefix() {
        assert_eq!(strip_subset_prefix("ABCDEf+Font"), "ABCDEf+Font");
    }

    #[test]
    fn strip_subset_prefix_leaves_empty_string() {
        assert_eq!(strip_subset_prefix(""), "");
    }
}
