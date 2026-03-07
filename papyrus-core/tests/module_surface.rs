use papyrus_core::{detector, parser, renderer};

#[test]
fn module_surfaces_are_linked() {
    let parsed = parser::parse_pdf_bytes(b"%PDF-1.7");
    let detected = detector::detect_structure(parsed);
    let markdown = renderer::render_markdown(&detected);
    assert!(markdown.is_empty());
}
