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
    let page_id = match pages.get(&(page_number as u32)) {
        Some(id) => *id,
        None => return (fonts, warnings),
    };

    // Use lopdf's built-in font resolution
    let page_fonts = match doc.get_page_fonts(page_id) {
        Ok(f) => f,
        Err(_) => return (fonts, warnings),
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

        // Try to extract font size from FontDescriptor (optional/diagnostic only)
        let descriptor_size = font_dict
            .get(b"FontDescriptor")
            .ok()
            .and_then(|obj| {
                // Dereference if it's a reference
                match obj {
                    lopdf::Object::Reference(id) => doc.get_object(*id).ok(),
                    _ => Some(obj),
                }
            })
            .and_then(|obj| obj.as_dict().ok())
            .and_then(|desc| {
                // Try to get a representative size from the descriptor
                // (this is diagnostic only; Tf state is authoritative for font_size)
                desc.get(b"FontSize").ok().and_then(|o| o.as_float().ok())
            });

        fonts.insert(
            resource_name,
            FontInfo {
                name: base_font_name,
                size: descriptor_size,
            },
        );
    }

    (fonts, warnings)
}

/// Extract raw text segments from a page's content stream.
///
/// Processes Tf, Tj, TJ, BT, and ET operators.
pub fn extract_text_segments_for_page(
    _doc: &lopdf::Document,
    _page_number: usize,
    _fonts: &HashMap<Vec<u8>, FontInfo>,
) -> (Vec<RawTextSegment>, Vec<Warning>) {
    (Vec::new(), Vec::new())
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
    use crate::ast::Warning;
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
    fn resolve_fonts_for_page_strips_subset_prefix() {
        // Subset prefixes are 6 uppercase letters followed by +
        // Our fixtures may or may not have them, but if they do, names should be stripped.
        let doc = load_fixture("simple.pdf");
        let (fonts, _) = resolve_fonts_for_page(&doc, 1);
        for fi in fonts.values() {
            // No font name should start with a 6-uppercase-letter prefix
            assert!(
                !has_subset_prefix(&fi.name),
                "font name should have subset prefix stripped: {}",
                fi.name
            );
        }
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

    /// Check if a font name has a subset prefix (6 uppercase ASCII letters + '+')
    fn has_subset_prefix(name: &str) -> bool {
        if name.len() < 7 {
            return false;
        }
        let prefix = &name[..7];
        prefix.ends_with('+') && prefix[..6].chars().all(|c| c.is_ascii_uppercase())
    }
}
