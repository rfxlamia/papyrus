use papyrus_core::ast::Warning;
use papyrus_core::parser;
use serde::Deserialize;
use std::path::PathBuf;

/// Oracle JSON structure mirroring extract_oracle.py output.
#[derive(Debug, Deserialize)]
struct Oracle {
    pages: Vec<OraclePage>,
}

#[derive(Debug, Deserialize)]
struct OraclePage {
    /// 0-based page number in the oracle
    page_number: usize,
    blocks: Vec<OracleBlock>,
}

#[derive(Debug, Deserialize)]
struct OracleBlock {
    text: String,
    font_name: String,
    font_size: f64,
    #[allow(dead_code)]
    is_bold: bool,
    #[allow(dead_code)]
    is_italic: bool,
}

/// Flatten oracle blocks into ordered (1-based page_number, text, font_name, font_size) tuples.
fn flatten_oracle(oracle: &Oracle) -> Vec<(usize, &str, &str, f64)> {
    let mut flat = Vec::new();
    for page in &oracle.pages {
        // Oracle page_number is 0-based; parser is 1-based
        let page_num = page.page_number + 1;
        for block in &page.blocks {
            flat.push((
                page_num,
                block.text.as_str(),
                block.font_name.as_str(),
                block.font_size,
            ));
        }
    }
    flat
}

/// Resolve fixture path relative to workspace root.
fn fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("workspace root")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Load oracle JSON for a given fixture base name (e.g., "simple").
fn load_oracle(base_name: &str) -> Oracle {
    let path = fixture_path(&format!("{}.oracle.json", base_name));
    let data = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("oracle {} must exist at {:?}: {}", base_name, path, e));
    serde_json::from_str(&data)
        .unwrap_or_else(|e| panic!("oracle {} is invalid JSON: {}", base_name, e))
}

/// Load PDF fixture bytes.
fn load_fixture_bytes(name: &str) -> Vec<u8> {
    let path = fixture_path(name);
    std::fs::read(&path)
        .unwrap_or_else(|e| panic!("fixture {} must exist at {:?}: {}", name, path, e))
}

/// Core oracle comparison for a fixture.
///
/// Compares per-segment ordered (page_number, normalized_text) sequence,
/// segment count, per-index font names (normalized), per-index font sizes
/// (within tolerance), and metadata.
fn assert_oracle_match(fixture_name: &str, base_name: &str) {
    let bytes = load_fixture_bytes(fixture_name);
    let oracle = load_oracle(base_name);
    let (segments, metadata, warnings) = parser::parse_pdf(&bytes);

    let expected = flatten_oracle(&oracle);

    // 1. Segment count must match
    assert_eq!(
        segments.len(),
        expected.len(),
        "[{}] segment count mismatch: got {}, expected {}.\nSegments: {:?}\nExpected: {:?}",
        base_name,
        segments.len(),
        expected.len(),
        segments
            .iter()
            .map(|s| (&s.text, s.page_number))
            .collect::<Vec<_>>(),
        expected.iter().map(|e| (e.1, e.0)).collect::<Vec<_>>(),
    );

    // 2. Per-index comparison: page_number, text, font_name, font_size
    for (i, (seg, &(exp_page, exp_text, exp_font, exp_size))) in
        segments.iter().zip(expected.iter()).enumerate()
    {
        // Page number (1-based)
        assert_eq!(
            seg.page_number, exp_page,
            "[{}] segment {} page mismatch: got {}, expected {}",
            base_name, i, seg.page_number, exp_page,
        );

        // Text (exact match after trimming whitespace)
        assert_eq!(
            seg.text.trim(),
            exp_text.trim(),
            "[{}] segment {} text mismatch",
            base_name,
            i,
        );

        // Font name: resolve resource name through font map to get base font name
        // We re-resolve fonts for this segment's page to look up the resource name
        let font_name = resolve_font_name_for_segment(&bytes, seg);
        assert_eq!(
            font_name, exp_font,
            "[{}] segment {} font name mismatch: got {:?}, expected {:?}",
            base_name, i, font_name, exp_font,
        );

        // Font size within tolerance (abs_diff <= 0.1)
        let size_diff = (seg.font_size as f64 - exp_size).abs();
        assert!(
            size_diff <= 0.1,
            "[{}] segment {} font size mismatch: got {}, expected {}, diff={}",
            base_name,
            i,
            seg.font_size,
            exp_size,
            size_diff,
        );
    }

    // 3. Metadata: page_count
    let expected_page_count = oracle.pages.len();
    assert_eq!(
        metadata.page_count, expected_page_count,
        "[{}] page_count mismatch: got {}, expected {}",
        base_name, metadata.page_count, expected_page_count,
    );

    // 4. No critical warnings for valid PDFs
    for w in &warnings {
        if let Warning::UnreadableTextStream { page, detail } = w {
            panic!(
                "[{}] unexpected UnreadableTextStream on page {}: {}",
                base_name, page, detail
            );
        }
    }
}

/// Resolve the font base name for a segment by re-running font resolution on its page.
fn resolve_font_name_for_segment(pdf_bytes: &[u8], seg: &parser::RawTextSegment) -> String {
    let (doc_opt, _) = parser::load_pdf(pdf_bytes);
    let doc = doc_opt.expect("should be a valid PDF for font resolution");
    let (fonts, _) = parser::resolve_fonts_for_page(&doc, seg.page_number);
    fonts
        .get(&seg.font_resource_name)
        .map(|fi| fi.name.clone())
        .unwrap_or_else(|| {
            format!(
                "<unresolved:{}>",
                String::from_utf8_lossy(&seg.font_resource_name)
            )
        })
}

// ── Oracle integration tests per fixture ──

#[test]
fn oracle_simple() {
    assert_oracle_match("simple.pdf", "simple");
}

#[test]
fn oracle_bold_italic() {
    assert_oracle_match("bold-italic.pdf", "bold-italic");
}

#[test]
fn oracle_multi_heading() {
    assert_oracle_match("multi-heading.pdf", "multi-heading");
}

#[test]
fn oracle_multi_page() {
    assert_oracle_match("multi-page.pdf", "multi-page");
}

// ── Corrupted PDF test ──

#[test]
fn oracle_corrupted_does_not_panic_and_emits_warning() {
    let bytes = load_fixture_bytes("corrupted.pdf");
    let (segments, metadata, warnings) = parser::parse_pdf(&bytes);

    // Must not panic (if we got here, it didn't)
    assert!(
        segments.is_empty(),
        "corrupted PDF should produce no segments"
    );
    assert_eq!(
        metadata.page_count, 0,
        "corrupted PDF should have page_count=0"
    );

    // Must emit at least one warning
    assert!(
        !warnings.is_empty(),
        "corrupted PDF should produce at least one warning"
    );

    // Warning variant and detail assertions
    match &warnings[0] {
        Warning::MalformedPdfObject { detail } => {
            assert!(
                !detail.is_empty(),
                "MalformedPdfObject detail must not be empty"
            );
            // Detail should contain meaningful context
            assert!(
                detail.contains("failed to load PDF") || detail.contains("empty PDF"),
                "detail should explain the failure: {:?}",
                detail
            );
        }
        other => panic!(
            "expected first warning to be MalformedPdfObject, got {:?}",
            other
        ),
    }
}

// ── Metadata parity tests ──

#[test]
fn oracle_simple_metadata_parity() {
    let bytes = load_fixture_bytes("simple.pdf");
    let (_, metadata, _) = parser::parse_pdf(&bytes);
    assert_eq!(metadata.page_count, 1);
    // PyMuPDF-generated fixtures may or may not have title/author
    // We just verify the fields exist and are consistent
}

#[test]
fn oracle_multi_page_metadata_parity() {
    let bytes = load_fixture_bytes("multi-page.pdf");
    let (_, metadata, _) = parser::parse_pdf(&bytes);
    assert_eq!(metadata.page_count, 2);
}
