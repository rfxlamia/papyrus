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
pub fn load_pdf(_bytes: &[u8]) -> (Option<lopdf::Document>, Vec<Warning>) {
    (None, Vec::new())
}

/// Resolve font dictionaries for a given page.
///
/// Returns a map of font resource name (e.g., b"F1") to FontInfo, plus any warnings.
pub fn resolve_fonts_for_page(
    _doc: &lopdf::Document,
    _page_number: usize,
) -> (HashMap<Vec<u8>, FontInfo>, Vec<Warning>) {
    (HashMap::new(), Vec::new())
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
