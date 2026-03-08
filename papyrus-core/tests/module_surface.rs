use papyrus_core::detector::{ClassifiedSegment, DetectorConfig, SegmentClass};
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

    // renderer stub still works with its current signature
    let markdown = renderer::render_markdown(&[]);
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
fn detector_surface_exposes_phase3_types() {
    let cfg = DetectorConfig::default();
    assert_eq!(cfg.heading_size_ratio, 1.2);
    assert!(cfg.detect_bold);
    assert!(cfg.detect_italic);

    let class = SegmentClass::Heading(2);
    assert!(matches!(class, SegmentClass::Heading(2)));

    let _ = std::mem::size_of::<ClassifiedSegment>();
}

#[test]
fn parser_types_are_constructible() {
    let font_info = parser::FontInfo {
        name: "Helvetica".to_string(),
        size: Some(12.0),
        font_weight: None,
        italic_angle: None,
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
        font_weight: None,
        italic_angle: None,
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
