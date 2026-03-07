#[derive(Debug, Clone, PartialEq)]
pub struct ConversionResult {
    pub document: Document,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub metadata: DocumentMetadata,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Heading { level: u8, spans: Vec<Span> },
    Paragraph { spans: Vec<Span> },
    RawText(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub font_size: f32,
    pub font_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    MissingFontMetrics { font_name: String, page: usize },
    UnreadableTextStream { page: usize, detail: String },
    UnsupportedEncoding { encoding: String, page: usize },
    MalformedPdfObject { detail: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_result_keeps_warnings_and_raw_text() {
        let result = ConversionResult {
            document: Document {
                metadata: DocumentMetadata {
                    title: None,
                    author: None,
                    page_count: 0,
                },
                nodes: vec![Node::RawText("fallback".to_string())],
            },
            warnings: vec![Warning::MalformedPdfObject {
                detail: "broken object".to_string(),
            }],
        };

        assert_eq!(result.document.nodes.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn raw_text_variant_round_trips() {
        let node = Node::RawText("unclassified".to_string());
        match node {
            Node::RawText(s) => assert_eq!(s, "unclassified"),
            _ => panic!("expected raw text"),
        }
    }
}
