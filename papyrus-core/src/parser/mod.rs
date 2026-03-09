use std::collections::HashMap;

use crate::ast::{DocumentMetadata, Warning};

/// Font metadata resolved from a PDF font dictionary.
#[derive(Debug, Clone, PartialEq)]
pub struct FontInfo {
    /// The normalized font name (subset prefix stripped).
    pub name: String,
    /// Optional font size from the font descriptor (diagnostic only; Tf state is authoritative).
    pub size: Option<f32>,
    /// FontWeight from the font descriptor (e.g., 700 = bold). None if not present.
    pub font_weight: Option<f32>,
    /// ItalicAngle from the font descriptor. Non-zero indicates italic/oblique. None if not present.
    pub italic_angle: Option<f32>,
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
    /// Cursor X position (user-space units) before this segment was printed.
    pub x: f32,
    /// Baseline Y position (user-space units) from the text matrix.
    pub y: f32,
    /// True when the text matrix has non-zero rotation (b != 0 or c != 0).
    pub is_rotated: bool,
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

/// Extract FontWeight and ItalicAngle from a font's FontDescriptor dictionary.
///
/// Returns `(font_weight, italic_angle)` — either may be None if the descriptor
/// is absent or the key is missing.
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

    let font_weight = desc.get(b"FontWeight").ok().and_then(extract_number);
    let italic_angle = desc.get(b"ItalicAngle").ok().and_then(extract_number);

    (font_weight, italic_angle)
}

/// Strip a 6-uppercase-letter subset prefix (e.g., "ABCDEF+Helvetica-Bold" -> "Helvetica-Bold").
///
/// PDF subset fonts embed the original font name after a 6-uppercase-letter tag
/// and a `+` separator. Stripping this prefix recovers the canonical font name
/// for pattern matching.
pub(crate) fn strip_subset_prefix(name: &str) -> &str {
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
    }

    (fonts, warnings)
}

/// WinAnsiEncoding lookup table for bytes 0x80–0x9F.
///
/// These bytes differ from ISO-8859-1: WinAnsi maps them to printable characters
/// (smart quotes, em dashes, euro sign, etc.) while ISO-8859-1 maps them to C1
/// control characters. PDF spec §D.1 defines this mapping.
///
/// Index 0 = byte 0x80, index 31 = byte 0x9F.
const WINANSI_0X80_TO_0X9F: [char; 32] = [
    '\u{20AC}', // 0x80 — Euro sign
    '\u{FFFD}', // 0x81 — undefined, use replacement
    '\u{201A}', // 0x82 — Single Low-9 Quotation Mark
    '\u{0192}', // 0x83 — Latin Small Letter F With Hook
    '\u{201E}', // 0x84 — Double Low-9 Quotation Mark
    '\u{2026}', // 0x85 — Horizontal Ellipsis
    '\u{2020}', // 0x86 — Dagger
    '\u{2021}', // 0x87 — Double Dagger
    '\u{02C6}', // 0x88 — Modifier Letter Circumflex Accent
    '\u{2030}', // 0x89 — Per Mille Sign
    '\u{0160}', // 0x8A — Latin Capital Letter S With Caron
    '\u{2039}', // 0x8B — Single Left-Pointing Angle Quotation Mark
    '\u{0152}', // 0x8C — Latin Capital Ligature OE
    '\u{FFFD}', // 0x8D — undefined, use replacement
    '\u{017D}', // 0x8E — Latin Capital Letter Z With Caron
    '\u{FFFD}', // 0x8F — undefined, use replacement
    '\u{FFFD}', // 0x90 — undefined, use replacement
    '\u{2018}', // 0x91 — Left Single Quotation Mark
    '\u{2019}', // 0x92 — Right Single Quotation Mark
    '\u{201C}', // 0x93 — Left Double Quotation Mark
    '\u{201D}', // 0x94 — Right Double Quotation Mark
    '\u{2022}', // 0x95 — Bullet
    '\u{2013}', // 0x96 — En Dash
    '\u{2014}', // 0x97 — Em Dash
    '\u{02DC}', // 0x98 — Small Tilde
    '\u{2122}', // 0x99 — Trade Mark Sign
    '\u{0161}', // 0x9A — Latin Small Letter S With Caron
    '\u{203A}', // 0x9B — Single Right-Pointing Angle Quotation Mark
    '\u{0153}', // 0x9C — Latin Small Ligature OE
    '\u{FFFD}', // 0x9D — undefined, use replacement
    '\u{017E}', // 0x9E — Latin Small Letter Z With Caron
    '\u{0178}', // 0x9F — Latin Capital Letter Y With Diaeresis
];

/// Decode a single byte using WinAnsiEncoding.
fn winansi_byte_to_char(b: u8) -> char {
    if b < 0x80 {
        b as char
    } else if b <= 0x9F {
        WINANSI_0X80_TO_0X9F[(b - 0x80) as usize]
    } else {
        // 0xA0–0xFF: same as ISO-8859-1 (direct Unicode codepoint)
        b as char
    }
}

/// Decode raw PDF string bytes to UTF-8.
///
/// Handles UTF-16BE (with or without BOM) and falls back to WinAnsiEncoding.
fn decode_pdf_string(bytes: &[u8]) -> String {
    // Check for UTF-16BE BOM (0xFE 0xFF)
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM — skip BOM bytes
        return decode_utf16be(&bytes[2..]);
    }

    // Heuristic for UTF-16BE without BOM: if first byte is 0x00 and length is even,
    // it's likely UTF-16BE (common in CIDFont text). Note: per PDF spec §7.9.2.2,
    // UTF-16BE is formally identified only by the BOM. This heuristic is a pragmatic
    // best-effort for spec-violating PDFs.
    if bytes.len() >= 2 && bytes.len().is_multiple_of(2) && bytes[0] == 0x00 {
        return decode_utf16be(bytes);
    }

    // Default: WinAnsiEncoding (PDF spec §D.1)
    bytes.iter().map(|&b| winansi_byte_to_char(b)).collect()
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

/// Estimate the width of a text string in user-space units.
///
/// Uses char count * font_size * 0.6 as a rough approximation.
/// Will be replaced by actual font metrics in v0.1.2 (H3).
fn estimate_string_width(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * 0.6
}

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

/// Extract raw text segments from a page's content stream.
///
/// Processes Tf, Tj, TJ, BT, and ET operators with position tracking.
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
    let mut warned_no_tf = false;
    let mut text_state = TextState::new();

    for op in content.operations.iter() {
        match op.operator.as_str() {
            "BT" => {
                // Begin text object — reset state
                current_font_resource = None;
                current_font_size = None;
                tf_set_in_text_object = false;
                warned_no_tf = false;
                text_state.reset_for_bt();
            }
            "ET" => {
                // End text object — reset state
                current_font_resource = None;
                current_font_size = None;
                tf_set_in_text_object = false;
                warned_no_tf = false;
            }
            "Tm" => {
                // Set text matrix: operands [a b c d e f]
                if op.operands.len() >= 6 {
                    let vals: Vec<f32> = op.operands.iter().filter_map(extract_number).collect();
                    if vals.len() >= 6 {
                        text_state.set_matrix(vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]);
                    }
                }
            }
            "Td" => {
                // Text position offset: operands [tx ty]
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
                // Text position offset + set leading: operands [tx ty]
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
                            &mut warned_no_tf,
                            page_number,
                            &mut warnings,
                        );
                        let seg_x = text_state.current_x;
                        let seg_y = text_state.current_y;
                        let rotated = text_state.is_rotated();
                        let width = estimate_string_width(&text, font_sz);
                        text_state.advance_x(width);
                        segments.push(RawTextSegment {
                            text,
                            font_resource_name: font_res,
                            font_size: font_sz,
                            page_number,
                            x: seg_x,
                            y: seg_y,
                            is_rotated: rotated,
                        });
                    }
                }
            }
            "TJ" => {
                // Show array of strings/numbers with position-aware tracking.
                // Each string element produces a segment with its own X position.
                // Number elements adjust the X cursor (kerning/word spacing).
                if let Some(lopdf::Object::Array(arr)) = op.operands.first() {
                    let (font_res, font_sz) = get_text_state_or_default(
                        &current_font_resource,
                        current_font_size,
                        tf_set_in_text_object,
                        &mut warned_no_tf,
                        page_number,
                        &mut warnings,
                    );
                    let rotated = text_state.is_rotated();

                    // Collect all string elements with their starting X positions
                    // into a single segment per TJ call, but use position data
                    // to handle spacing correctly.
                    let mut combined = String::new();
                    let mut seg_start_x = text_state.current_x;
                    let mut first_string = true;

                    for item in arr {
                        if let Some(bytes) = extract_string_bytes(item) {
                            let text = decode_pdf_string(&bytes);
                            if first_string {
                                seg_start_x = text_state.current_x;
                                first_string = false;
                            }
                            combined.push_str(&text);
                            let width = estimate_string_width(&text, font_sz);
                            text_state.advance_x(width);
                        } else if let Some(num) = extract_number(item) {
                            // TJ number: adjust cursor position.
                            // Large negative = word space; small = kerning.
                            text_state.adjust_tj(num, font_sz);
                            // If displacement is large enough to be a word space,
                            // insert a space into the combined string.
                            let displacement_pts = (num.abs() / 1000.0) * font_sz;
                            let space_threshold = font_sz * 0.3 * 0.8;
                            if num < 0.0
                                && displacement_pts > space_threshold
                                && !combined.is_empty()
                                && !combined.ends_with(char::is_whitespace)
                            {
                                combined.push(' ');
                            }
                        }
                    }
                    if !combined.is_empty() {
                        segments.push(RawTextSegment {
                            text: combined,
                            font_resource_name: font_res,
                            font_size: font_sz,
                            page_number,
                            x: seg_start_x,
                            y: text_state.current_y,
                            is_rotated: rotated,
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
/// Emits at most one warning per text object when Tf is missing (per plan §5).
fn get_text_state_or_default(
    current_font_resource: &Option<Vec<u8>>,
    current_font_size: Option<f32>,
    tf_set: bool,
    warned_no_tf: &mut bool,
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
        if !*warned_no_tf {
            warnings.push(Warning::MalformedPdfObject {
                detail: format!("text state not set before Tj/TJ on page {}", page_number),
            });
            *warned_no_tf = true;
        }
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
    let mut all_segments = Vec::new();
    let mut all_warnings = Vec::new();

    // Step 1: Load PDF
    let (doc_opt, load_warnings) = load_pdf(bytes);
    all_warnings.extend(load_warnings);

    let doc = match doc_opt {
        Some(d) => d,
        None => {
            return (
                all_segments,
                DocumentMetadata {
                    title: None,
                    author: None,
                    page_count: 0,
                },
                all_warnings,
            );
        }
    };

    // Step 2: Extract metadata
    let pages = doc.get_pages();
    let page_count = pages.len();

    // Try to extract title and author from the document info dictionary
    let (title, author) = extract_doc_info(&doc);

    let metadata = DocumentMetadata {
        title,
        author,
        page_count,
    };

    // Step 3: Per-page extraction — iterate in page order (1-based)
    let mut page_numbers: Vec<u32> = pages.keys().copied().collect();
    page_numbers.sort();

    for &page_num in &page_numbers {
        // Safe: u32 -> usize is always widening on 32-bit+ platforms
        let page_number = page_num as usize;

        // Resolve fonts for this page
        let (fonts, font_warnings) = resolve_fonts_for_page(&doc, page_number);
        all_warnings.extend(font_warnings);

        // Extract text segments for this page
        let (segments, extract_warnings) =
            extract_text_segments_for_page(&doc, page_number, &fonts);
        all_warnings.extend(extract_warnings);

        all_segments.extend(segments);
    }

    (all_segments, metadata, all_warnings)
}

/// Public crate accessor for doc info extraction, used by `lib.rs` single-pass pipeline.
pub(crate) fn extract_doc_info_pub(doc: &lopdf::Document) -> (Option<String>, Option<String>) {
    extract_doc_info(doc)
}

/// Extract title and author from PDF info dictionary.
fn extract_doc_info(doc: &lopdf::Document) -> (Option<String>, Option<String>) {
    // Try the trailer's /Info reference
    let info_dict = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|obj| match obj {
            lopdf::Object::Reference(id) => doc.get_object(*id).ok(),
            _ => Some(obj),
        })
        .and_then(|obj| obj.as_dict().ok());

    let info = match info_dict {
        Some(d) => d,
        None => return (None, None),
    };

    let title = get_info_string(info, b"Title");
    let author = get_info_string(info, b"Author");

    (title, author)
}

/// Extract a non-empty string value from a PDF info dictionary.
fn get_info_string(info: &lopdf::Dictionary, key: &[u8]) -> Option<String> {
    info.get(key).ok().and_then(|obj| match obj {
        lopdf::Object::String(bytes, _) => {
            let s = decode_pdf_string(bytes);
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        _ => None,
    })
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
            if let Warning::MalformedPdfObject { .. } = w {
                panic!("valid PDF should not produce MalformedPdfObject warning");
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

    // ── get_text_state_or_default tests ──

    #[test]
    fn get_text_state_or_default_with_tf_set_returns_current_state() {
        let font_res = Some(b"F1".to_vec());
        let mut warned = false;
        let mut warnings = Vec::new();
        let (res, size) =
            get_text_state_or_default(&font_res, Some(12.0), true, &mut warned, 1, &mut warnings);
        assert_eq!(res, b"F1");
        assert_eq!(size, 12.0);
        assert!(
            warnings.is_empty(),
            "should not emit warning when Tf is set"
        );
        assert!(!warned, "warned flag should remain false");
    }

    #[test]
    fn get_text_state_or_default_without_tf_returns_defaults_and_warns_once() {
        let mut warned = false;
        let mut warnings = Vec::new();

        // First call: should emit warning
        let (res1, size1) =
            get_text_state_or_default(&None, None, false, &mut warned, 1, &mut warnings);
        assert_eq!(res1, b"<unknown>");
        assert_eq!(size1, 0.0);
        assert_eq!(warnings.len(), 1, "should emit exactly one warning");
        match &warnings[0] {
            Warning::MalformedPdfObject { detail } => {
                assert!(detail.contains("text state not set before Tj/TJ"));
            }
            other => panic!("expected MalformedPdfObject, got {:?}", other),
        }
        assert!(warned, "warned flag should be set after first call");

        // Second call: should NOT emit another warning (plan §5: "emit one")
        let (res2, size2) =
            get_text_state_or_default(&None, None, false, &mut warned, 1, &mut warnings);
        assert_eq!(res2, b"<unknown>");
        assert_eq!(size2, 0.0);
        assert_eq!(
            warnings.len(),
            1,
            "should still have exactly one warning after second call"
        );
    }

    #[test]
    fn get_text_state_or_default_warned_resets_across_text_objects() {
        let mut warned = false;
        let mut warnings = Vec::new();

        // First text object: emit one warning
        get_text_state_or_default(&None, None, false, &mut warned, 1, &mut warnings);
        assert_eq!(warnings.len(), 1);

        // Simulate BT: reset warned flag (caller is responsible for this)
        warned = false;

        // Second text object: should emit another warning
        get_text_state_or_default(&None, None, false, &mut warned, 1, &mut warnings);
        assert_eq!(
            warnings.len(),
            2,
            "should have two warnings for two separate text objects"
        );
    }

    // ── parse_pdf tests ──

    #[test]
    fn parse_pdf_simple_returns_metadata_and_segments() {
        let path = fixture_path("simple.pdf");
        let bytes = std::fs::read(&path).unwrap();
        let (segments, metadata, warnings) = parse_pdf(&bytes);

        // Metadata
        assert_eq!(metadata.page_count, 1, "simple.pdf has 1 page");

        // Segments should not be empty
        assert!(!segments.is_empty(), "should produce segments");

        // All segments should be page 1 (1-based)
        for seg in &segments {
            assert_eq!(seg.page_number, 1, "all segments should be page 1");
        }

        // Combined text should contain oracle-expected content
        let combined: String = segments.iter().map(|s| s.text.as_str()).collect();
        assert!(combined.contains("Chapter 1"), "should contain 'Chapter 1'");
        assert!(
            combined.contains("Body text."),
            "should contain 'Body text.'"
        );

        // No critical warnings for valid PDF
        for w in &warnings {
            if let Warning::UnreadableTextStream { .. } = w {
                panic!("valid PDF should not produce UnreadableTextStream");
            }
        }
    }

    #[test]
    fn parse_pdf_failed_load_returns_empty_with_warning() {
        let (segments, metadata, warnings) = parse_pdf(b"not a pdf");
        assert!(
            segments.is_empty(),
            "failed load should produce no segments"
        );
        assert_eq!(
            metadata.page_count, 0,
            "failed load should have page_count=0"
        );
        assert!(metadata.title.is_none(), "failed load should have no title");
        assert!(
            metadata.author.is_none(),
            "failed load should have no author"
        );
        assert!(!warnings.is_empty(), "failed load should produce warnings");
        match &warnings[0] {
            Warning::MalformedPdfObject { detail } => {
                assert!(!detail.is_empty());
            }
            other => panic!("expected MalformedPdfObject, got {:?}", other),
        }
    }

    #[test]
    fn parse_pdf_empty_bytes_returns_empty_with_warning() {
        let (segments, metadata, warnings) = parse_pdf(b"");
        assert!(segments.is_empty());
        assert_eq!(metadata.page_count, 0);
        assert!(!warnings.is_empty());
        match &warnings[0] {
            Warning::MalformedPdfObject { .. } => {}
            other => panic!("expected MalformedPdfObject, got {:?}", other),
        }
    }

    #[test]
    fn parse_pdf_corrupted_fixture_returns_empty_with_warning() {
        let path = fixture_path("corrupted.pdf");
        let bytes = std::fs::read(&path).unwrap();
        let (segments, metadata, warnings) = parse_pdf(&bytes);
        assert!(
            segments.is_empty(),
            "corrupted PDF should produce no segments"
        );
        assert_eq!(metadata.page_count, 0);
        assert!(!warnings.is_empty());
        match &warnings[0] {
            Warning::MalformedPdfObject { detail } => {
                assert!(!detail.is_empty());
            }
            other => panic!("expected MalformedPdfObject, got {:?}", other),
        }
    }

    #[test]
    fn parse_pdf_multi_page_has_1_based_page_numbers() {
        let path = fixture_path("multi-page.pdf");
        let bytes = std::fs::read(&path).unwrap();
        let (segments, metadata, _) = parse_pdf(&bytes);

        assert!(
            metadata.page_count >= 2,
            "multi-page.pdf should have 2+ pages"
        );

        // Check that segments reference pages starting from 1, not 0
        let min_page = segments.iter().map(|s| s.page_number).min().unwrap_or(0);
        assert_eq!(min_page, 1, "minimum page number should be 1 (1-based)");

        // Check that segments span multiple pages
        let max_page = segments.iter().map(|s| s.page_number).max().unwrap_or(0);
        assert!(
            max_page >= 2,
            "multi-page.pdf should have segments from page 2+, got max={}",
            max_page
        );
    }

    #[test]
    fn parse_pdf_aggregates_warnings_in_stable_order() {
        // Calling parse_pdf twice on the same input should produce identical warnings
        let path = fixture_path("simple.pdf");
        let bytes = std::fs::read(&path).unwrap();
        let (_, _, warnings1) = parse_pdf(&bytes);
        let (_, _, warnings2) = parse_pdf(&bytes);
        assert_eq!(
            warnings1.len(),
            warnings2.len(),
            "warnings should be deterministic"
        );
        assert_eq!(
            warnings1, warnings2,
            "warnings should be stable across runs"
        );
    }

    // ── decode_pdf_string tests ──

    #[test]
    fn decode_pdf_string_winansi_ascii() {
        // Pure ASCII bytes should decode to the same string
        let result = decode_pdf_string(b"Hello World");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn decode_pdf_string_winansi_high_latin_range() {
        // Bytes 0xA0+: same as Latin-1 — é (0xE9), ü (0xFC), ñ (0xF1)
        let result = decode_pdf_string(&[0xE9, 0xFC, 0xF1]);
        assert_eq!(result, "\u{00E9}\u{00FC}\u{00F1}");
        assert!(
            !result.contains(char::REPLACEMENT_CHARACTER),
            "valid WinAnsi high-latin should not produce replacement chars"
        );
    }

    #[test]
    fn decode_pdf_string_winansi_0x80_to_0x9f_range() {
        // 0x80 = € (U+20AC), 0x93 = " (U+201C), 0x96 = – (U+2013), 0x97 = — (U+2014)
        let result = decode_pdf_string(&[0x80, 0x93, 0x96, 0x97]);
        assert_eq!(result, "\u{20AC}\u{201C}\u{2013}\u{2014}");
    }

    #[test]
    fn decode_pdf_string_winansi_undefined_bytes_use_replacement() {
        // 0x81 and 0x8D are undefined in WinAnsi — should produce replacement chars
        let result = decode_pdf_string(&[0x81, 0x8D]);
        assert_eq!(result, "\u{FFFD}\u{FFFD}");
    }

    #[test]
    fn decode_pdf_string_utf16be_with_bom() {
        // UTF-16BE BOM (FE FF) followed by "Hi" (0x0048 0x0069)
        let bytes = [0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69];
        let result = decode_pdf_string(&bytes);
        assert_eq!(result, "Hi");
        assert!(
            !result.contains(char::REPLACEMENT_CHARACTER),
            "valid UTF-16BE should not produce replacement chars"
        );
    }

    #[test]
    fn decode_pdf_string_utf16be_without_bom() {
        // UTF-16BE without BOM: starts with 0x00 and even length
        // "AB" = 0x0041 0x0042
        let bytes = [0x00, 0x41, 0x00, 0x42];
        let result = decode_pdf_string(&bytes);
        assert_eq!(result, "AB");
    }

    #[test]
    fn decode_pdf_string_utf16be_with_non_ascii() {
        // UTF-16BE BOM + "café" = c(0x0063) a(0x0061) f(0x0066) é(0x00E9)
        let bytes = [0xFE, 0xFF, 0x00, 0x63, 0x00, 0x61, 0x00, 0x66, 0x00, 0xE9];
        let result = decode_pdf_string(&bytes);
        assert_eq!(result, "caf\u{00E9}");
    }

    #[test]
    fn decode_pdf_string_empty_bytes() {
        let result = decode_pdf_string(b"");
        assert_eq!(result, "");
    }

    #[test]
    fn decode_pdf_string_single_byte_not_utf16() {
        // Single byte can't be UTF-16BE — should fallback to WinAnsi
        let result = decode_pdf_string(&[0x41]);
        assert_eq!(result, "A");
    }

    #[test]
    fn decode_utf16be_invalid_surrogate_uses_replacement() {
        // Unpaired high surrogate: 0xD800
        let bytes = [0xD8, 0x00];
        let result = decode_utf16be(&bytes);
        assert!(
            result.contains(char::REPLACEMENT_CHARACTER),
            "invalid surrogate should produce replacement char, got: {:?}",
            result
        );
    }

    // ── winansi_byte_to_char tests ──

    #[test]
    fn winansi_byte_to_char_ascii_range() {
        assert_eq!(winansi_byte_to_char(0x41), 'A');
        assert_eq!(winansi_byte_to_char(0x20), ' ');
        assert_eq!(winansi_byte_to_char(0x7F), '\x7F');
    }

    #[test]
    fn winansi_byte_to_char_special_range() {
        assert_eq!(winansi_byte_to_char(0x80), '\u{20AC}'); // Euro
        assert_eq!(winansi_byte_to_char(0x91), '\u{2018}'); // Left single quote
        assert_eq!(winansi_byte_to_char(0x92), '\u{2019}'); // Right single quote
        assert_eq!(winansi_byte_to_char(0x93), '\u{201C}'); // Left double quote
        assert_eq!(winansi_byte_to_char(0x94), '\u{201D}'); // Right double quote
        assert_eq!(winansi_byte_to_char(0x96), '\u{2013}'); // En dash
        assert_eq!(winansi_byte_to_char(0x97), '\u{2014}'); // Em dash
        assert_eq!(winansi_byte_to_char(0x99), '\u{2122}'); // TM
    }

    #[test]
    fn winansi_byte_to_char_high_latin_range() {
        assert_eq!(winansi_byte_to_char(0xA0), '\u{00A0}'); // NBSP
        assert_eq!(winansi_byte_to_char(0xE9), '\u{00E9}'); // é
        assert_eq!(winansi_byte_to_char(0xFF), '\u{00FF}'); // ÿ
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

    // ── TextState ──────────────────────────────────────────────────────────────

    #[test]
    fn text_state_tracks_tm_operator() {
        let mut state = TextState::new();
        state.set_matrix(1.0, 0.0, 0.0, 1.0, 72.0, 700.0);
        assert_eq!(state.current_x, 72.0);
        assert_eq!(state.current_y, 700.0);
        assert!(!state.is_rotated());
    }

    #[test]
    fn text_state_detects_rotation() {
        let mut state = TextState::new();
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
        assert_eq!(state.current_x, 72.0);
        assert_eq!(state.current_y, 686.0);
    }

    #[test]
    fn text_state_advance_x_after_tj() {
        let mut state = TextState::new();
        state.set_matrix(1.0, 0.0, 0.0, 1.0, 72.0, 700.0);
        state.advance_x(30.0);
        assert_eq!(state.current_x, 102.0);
    }

    #[test]
    fn text_state_adjust_tj_displacement() {
        let mut state = TextState::new();
        state.set_matrix(1.0, 0.0, 0.0, 1.0, 72.0, 700.0);
        // TJ number -200 with font_size 10 → offset = 200/1000 * 10 = 2.0
        state.adjust_tj(-200.0, 10.0);
        assert_eq!(state.current_x, 74.0); // 72 - (-200/1000*10) = 72 + 2 = 74
    }

    #[test]
    fn text_state_td_upper_sets_tl() {
        let mut state = TextState::new();
        state.set_matrix(1.0, 0.0, 0.0, 1.0, 72.0, 700.0);
        state.apply_td_upper(0.0, -14.0);
        assert_eq!(state.current_y, 686.0);
        // TD sets TL = -ty = -(-14) = 14
        state.apply_t_star();
        assert_eq!(state.current_y, 672.0);
    }

    #[test]
    fn text_state_reset_for_bt() {
        let mut state = TextState::new();
        state.set_matrix(0.0, 1.0, -1.0, 0.0, 100.0, 200.0);
        state.set_tl(14.0);
        state.reset_for_bt();
        assert_eq!(state.current_x, 0.0);
        assert_eq!(state.current_y, 0.0);
        assert!(!state.is_rotated());
        // TL should persist across BT per PDF spec
        state.apply_t_star();
        assert_eq!(state.current_y, -14.0);
    }
}
