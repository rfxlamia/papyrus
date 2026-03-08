use papyrus_core::ast::Node;
use papyrus_core::{convert, Papyrus};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("workspace root")
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn extract_simple_pdf_has_heading_and_paragraph_nodes() {
    let bytes = std::fs::read(fixture_path("simple.pdf")).expect("fixture exists");
    let result = convert(&bytes);

    assert!(
        result
            .document
            .nodes
            .iter()
            .any(|n| matches!(n, Node::Heading { .. })),
        "expected at least one Heading node in simple.pdf"
    );
    assert!(
        result
            .document
            .nodes
            .iter()
            .any(|n| matches!(n, Node::Paragraph { .. })),
        "expected at least one Paragraph node in simple.pdf"
    );
}

#[test]
fn builder_overrides_change_heading_and_bold_detection() {
    let bytes = std::fs::read(fixture_path("multi-heading.pdf")).expect("fixture exists");

    let default_result = Papyrus::builder().build().extract(&bytes);
    let strict_result = Papyrus::builder()
        .heading_size_ratio(2.0)
        .detect_bold(false)
        .build()
        .extract(&bytes);

    let default_heading_count = default_result
        .document
        .nodes
        .iter()
        .filter(|n| matches!(n, Node::Heading { .. }))
        .count();
    let strict_heading_count = strict_result
        .document
        .nodes
        .iter()
        .filter(|n| matches!(n, Node::Heading { .. }))
        .count();

    // A stricter ratio should never produce *more* headings than the default.
    assert!(
        strict_heading_count <= default_heading_count,
        "strict ratio ({strict_heading_count} headings) exceeded default ({default_heading_count} headings)"
    );

    // detect_bold(false) must suppress bold on every span.
    for node in &strict_result.document.nodes {
        if let Node::Heading { spans, .. } | Node::Paragraph { spans } = node {
            for span in spans {
                assert!(
                    !span.bold,
                    "span {:?} should not be bold when detect_bold is false",
                    span.text
                );
            }
        }
    }
}

#[test]
fn extract_corrupted_pdf_is_best_effort_and_non_panicking() {
    let bytes = std::fs::read(fixture_path("corrupted.pdf")).expect("fixture exists");
    let result = convert(&bytes);

    // Corrupted file cannot be parsed; metadata should reflect zero pages.
    assert_eq!(
        result.document.metadata.page_count, 0,
        "corrupted PDF should report 0 pages"
    );
    assert!(
        !result.warnings.is_empty(),
        "corrupted PDF should produce at least one warning"
    );
}
