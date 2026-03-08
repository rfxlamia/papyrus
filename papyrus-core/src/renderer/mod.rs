use crate::ast::{Document, Node, Span};

// ── Private helpers ──────────────────────────────────────────────────────────

fn escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// ── Crate-internal rendering ─────────────────────────────────────────────────

pub(crate) fn render_span(span: &Span) -> String {
    if !span.bold && !span.italic {
        // Plain spans: preserve text as-is (inter-word spaces are intentional),
        // but return empty if there is truly no content at all.
        if span.text.is_empty() {
            return String::new();
        }
        return escape_text(&span.text);
    }

    // Formatted spans: trim surrounding whitespace before wrapping in markers
    // so we never produce "  **bold**  " which CommonMark parsers may reject.
    let core = span.text.trim();

    if core.is_empty() {
        return String::new();
    }

    let marker = match (span.bold, span.italic) {
        (true, true) => "***",
        (true, false) => "**",
        (false, true) => "*",
        (false, false) => unreachable!("plain spans return early above"),
    };

    format!("{marker}{}{marker}", escape_text(core))
}

fn render_spans(spans: &[Span]) -> String {
    spans
        .iter()
        .map(render_span)
        .filter(|s| !s.is_empty())
        .collect::<String>()
}

fn trim_trailing_ws_per_line(input: &str) -> String {
    input
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_node(node: &Node) -> String {
    match node {
        Node::Heading { level, spans } => {
            let hashes = "#".repeat((*level).clamp(1, 6) as usize);
            let text = trim_trailing_ws_per_line(&render_spans(spans));
            format!("{hashes} {text}\n\n")
        }
        Node::Paragraph { spans } => {
            let text = trim_trailing_ws_per_line(&render_spans(spans));
            format!("{text}\n\n")
        }
        Node::RawText(text) => {
            let cleaned = trim_trailing_ws_per_line(text);
            format!("{cleaned}\n\n")
        }
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

pub fn render_document(document: &Document) -> String {
    let body = document
        .nodes
        .iter()
        .map(render_node)
        .collect::<String>()
        .trim_start_matches('\n')
        .trim_end_matches('\n')
        .to_string();

    if body.is_empty() {
        String::new()
    } else {
        format!("{body}\n")
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Node, Span};

    fn span(text: &str, bold: bool, italic: bool) -> Span {
        Span {
            text: text.to_string(),
            bold,
            italic,
            font_size: 12.0,
            font_name: None,
        }
    }

    // ── escape_text ──────────────────────────────────────────────────────────

    #[test]
    fn escape_text_escapes_all_commonmark_special_chars() {
        let raw = r"\`*_{}[]()#+-.!|";
        let escaped = escape_text(raw);
        assert_eq!(escaped, r"\\\`\*\_\{\}\[\]\(\)\#\+\-\.\!\|");
    }

    #[test]
    fn escape_text_leaves_safe_text_unchanged() {
        assert_eq!(escape_text("Papyrus Renderer 123"), "Papyrus Renderer 123");
    }

    // ── render_span ──────────────────────────────────────────────────────────

    #[test]
    fn render_span_supports_plain_bold_italic_and_bold_italic() {
        assert_eq!(render_span(&span("plain", false, false)), "plain");
        assert_eq!(render_span(&span("bold", true, false)), "**bold**");
        assert_eq!(render_span(&span("italic", false, true)), "*italic*");
        assert_eq!(render_span(&span("both", true, true)), "***both***");
    }

    #[test]
    fn render_span_plain_preserves_whitespace_for_inter_word_spacing() {
        // Space-only plain spans are intentional inter-word separators.
        assert_eq!(render_span(&span(" ", false, false)), " ");
        assert_eq!(render_span(&span("  hello  ", false, false)), "  hello  ");
    }

    #[test]
    fn render_span_drops_empty_and_whitespace_only_formatted_output() {
        assert_eq!(render_span(&span("", true, false)), "");
        assert_eq!(render_span(&span("   ", true, true)), "");
    }

    #[test]
    fn render_span_trims_surrounding_whitespace_before_applying_markers() {
        assert_eq!(
            render_span(&span("  bold me  ", true, false)),
            "**bold me**"
        );
        assert_eq!(render_span(&span("\tbold\t", false, true)), "*bold*");
    }

    // ── render_node ──────────────────────────────────────────────────────────

    #[test]
    fn render_node_heading_uses_hash_prefix_and_blank_line() {
        let node = Node::Heading {
            level: 3,
            spans: vec![span("Heading", false, false)],
        };
        assert_eq!(render_node(&node), "### Heading\n\n");
    }

    #[test]
    fn render_node_paragraph_joins_spans_without_extra_spaces() {
        let node = Node::Paragraph {
            spans: vec![
                span("Hello", false, false),
                span(" ", false, false),
                span("world", true, false),
            ],
        };
        assert_eq!(render_node(&node), "Hello **world**\n\n");
    }

    #[test]
    fn render_node_raw_text_passthrough_appends_blank_line() {
        assert_eq!(render_node(&Node::RawText("raw".to_string())), "raw\n\n");
    }

    // ── render_document ──────────────────────────────────────────────────────

    #[test]
    fn render_document_has_single_trailing_newline_for_non_empty_docs() {
        let doc = Document {
            metadata: crate::ast::DocumentMetadata {
                title: None,
                author: None,
                page_count: 1,
            },
            nodes: vec![
                Node::Heading {
                    level: 1,
                    spans: vec![span("Title", false, false)],
                },
                Node::Paragraph {
                    spans: vec![span("Body", false, false)],
                },
            ],
        };

        let markdown = render_document(&doc);
        assert_eq!(markdown, "# Title\n\nBody\n");
        assert!(markdown.ends_with('\n'));
        assert!(!markdown.ends_with("\n\n"));
    }

    #[test]
    fn render_document_empty_doc_is_empty_string() {
        let doc = Document {
            metadata: crate::ast::DocumentMetadata {
                title: None,
                author: None,
                page_count: 0,
            },
            nodes: vec![],
        };

        assert_eq!(render_document(&doc), "");
    }
}
