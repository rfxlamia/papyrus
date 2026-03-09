use papyrus_core::ast::{ConversionResult, Document, DocumentMetadata, Node, Span};
use papyrus_core::detector::{ClassifiedSegment, DetectorConfig, SegmentClass};
use papyrus_core::{convert, parser, renderer, Papyrus};

#[test]
fn renderer_surface_exposes_document_entrypoint() {
    let doc = Document {
        metadata: DocumentMetadata {
            title: None,
            author: None,
            page_count: 0,
        },
        nodes: vec![],
    };

    let markdown = renderer::render_document(&doc);
    assert!(markdown.is_empty());
}

#[test]
fn module_surfaces_are_linked() {
    // Phase 2: parse_pdf replaces parse_pdf_bytes
    let (segments, metadata, warnings) = parser::parse_pdf(b"%PDF-1.7");
    // Stub should return empty results
    assert!(segments.is_empty());
    assert_eq!(metadata.page_count, 0);
    assert!(metadata.title.is_none());
    assert!(metadata.author.is_none());

    // renderer::render_document is covered by renderer_surface_exposes_document_entrypoint;
    // confirm it compiles and links correctly here too.
    let markdown = renderer::render_document(&Document {
        metadata: DocumentMetadata {
            title: None,
            author: None,
            page_count: 0,
        },
        nodes: vec![],
    });
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
            | papyrus_core::ast::Warning::UnsupportedEncoding { .. }
            | papyrus_core::ast::Warning::RotatedTextDetected { .. }
            | papyrus_core::ast::Warning::ImageOnlyPage { .. } => {}
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
        x: 0.0,
        y: 0.0,
        is_rotated: false,
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
        x: 0.0,
        y: 0.0,
        is_rotated: false,
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

#[test]
fn markdown_api_methods_delegate_to_renderer_output() {
    let document = Document {
        metadata: DocumentMetadata {
            title: None,
            author: None,
            page_count: 1,
        },
        nodes: vec![Node::Paragraph {
            spans: vec![Span {
                text: "phase4".to_string(),
                bold: false,
                italic: false,
                font_size: 12.0,
                font_name: None,
            }],
        }],
    };

    let result = ConversionResult {
        document: document.clone(),
        warnings: vec![],
    };

    assert_eq!(document.to_markdown(), "phase4\n");
    assert_eq!(result.to_markdown(), "phase4\n");
}

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
