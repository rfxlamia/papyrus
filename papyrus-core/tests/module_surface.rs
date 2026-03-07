use papyrus_core::{detector, parser, renderer};

#[test]
fn module_surfaces_are_linked() {
    // Phase 2: parse_pdf replaces parse_pdf_bytes
    let (segments, metadata, warnings) = parser::parse_pdf(b"%PDF-1.7");
    // Stub should return empty results
    assert!(segments.is_empty());
    assert_eq!(metadata.page_count, 0);
    assert!(metadata.title.is_none());
    assert!(metadata.author.is_none());

    // detector and renderer stubs still work with their current signatures
    let detected = detector::detect_structure(vec![]);
    let markdown = renderer::render_markdown(&detected);
    assert!(markdown.is_empty());

    // Incomplete PDF header may produce MalformedPdfObject warnings once
    // parse_pdf is fully wired — assert shape is valid if any exist.
    for w in &warnings {
        // All warnings must be one of the known Warning variants
        match w {
            papyrus_core::ast::Warning::MalformedPdfObject { detail } => {
                assert!(
                    !detail.is_empty(),
                    "MalformedPdfObject detail must not be empty"
                );
            }
            papyrus_core::ast::Warning::MissingFontMetrics { .. }
            | papyrus_core::ast::Warning::UnreadableTextStream { .. }
            | papyrus_core::ast::Warning::UnsupportedEncoding { .. } => {}
        }
    }
}

#[test]
fn parser_types_are_constructible() {
    let font_info = parser::FontInfo {
        name: "Helvetica".to_string(),
        size: Some(12.0),
    };
    assert_eq!(font_info.name, "Helvetica");
    assert_eq!(font_info.size, Some(12.0));

    let segment = parser::RawTextSegment {
        text: "Hello".to_string(),
        font_resource_name: b"F1".to_vec(),
        font_size: 12.0,
        page_number: 1,
    };
    assert_eq!(segment.text, "Hello");
    assert_eq!(segment.font_resource_name, b"F1");
    assert_eq!(segment.font_size, 12.0);
    assert_eq!(segment.page_number, 1);
}

#[test]
fn parser_types_derive_clone_and_debug() {
    let font_info = parser::FontInfo {
        name: "Helvetica".to_string(),
        size: Some(12.0),
    };
    let cloned = font_info.clone();
    assert_eq!(font_info, cloned);
    let _ = format!("{:?}", font_info);

    let segment = parser::RawTextSegment {
        text: "Hello".to_string(),
        font_resource_name: b"F1".to_vec(),
        font_size: 12.0,
        page_number: 1,
    };
    let cloned = segment.clone();
    assert_eq!(segment, cloned);
    let _ = format!("{:?}", segment);
}
