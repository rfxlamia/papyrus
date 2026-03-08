use papyrus_core::ast::{Document, DocumentMetadata, Node, Span};
use pulldown_cmark::{Event, Parser, Tag};

fn span(text: &str, bold: bool, italic: bool) -> Span {
    Span {
        text: text.to_string(),
        bold,
        italic,
        font_size: 12.0,
        font_name: None,
    }
}

#[test]
fn markdown_round_trip_matches_ast_shape() {
    let doc = Document {
        metadata: DocumentMetadata {
            title: None,
            author: None,
            page_count: 1,
        },
        nodes: vec![
            Node::Heading {
                level: 2,
                spans: vec![span("Phase 4", false, false)],
            },
            Node::Paragraph {
                spans: vec![
                    span("bold", true, false),
                    span(" and ", false, false),
                    span("italic", false, true),
                ],
            },
        ],
    };

    let markdown = doc.to_markdown();
    let events = Parser::new(&markdown).collect::<Vec<_>>();

    let heading_count = events
        .iter()
        .filter(|event| matches!(event, Event::Start(Tag::Heading { .. })))
        .count();
    let paragraph_count = events
        .iter()
        .filter(|event| matches!(event, Event::Start(Tag::Paragraph)))
        .count();
    let strong_count = events
        .iter()
        .filter(|event| matches!(event, Event::Start(Tag::Strong)))
        .count();
    let emphasis_count = events
        .iter()
        .filter(|event| matches!(event, Event::Start(Tag::Emphasis)))
        .count();

    assert_eq!(heading_count, 1);
    assert_eq!(paragraph_count, 1);
    assert_eq!(strong_count, 1);
    assert_eq!(emphasis_count, 1);
    assert!(markdown.ends_with('\n'));
}
