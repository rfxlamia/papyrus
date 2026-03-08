use crate::ast::{Document, Node, Span};

// ── Private helpers ──────────────────────────────────────────────────────────

/// Escape all CommonMark structurally-significant characters in inline content.
///
/// Escapes (CommonMark spec §2.4):
/// - Punctuation that can form emphasis, code, links, headings, lists, etc.
/// - `<`, `>` — prevent autolinks (§6.6) and raw HTML blocks (§6.11)
/// - `&` — prevent HTML entity references (§2.5)
fn escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' | '<' | '>' | '&' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// ── Crate-internal rendering ─────────────────────────────────────────────────

/// Render a single span to its CommonMark inline representation.
///
/// Plain spans preserve whitespace verbatim (inter-word spaces from the PDF
/// extractor are intentional). Formatted spans trim surrounding whitespace
/// before applying markers so `"  **bold**  "` — which CommonMark parsers
/// reject as a valid emphasis run per §6.2 — is never emitted.
///
/// When a formatted span's content is whitespace-only (e.g., `" "` in bold
/// from a spacing glyph in the content stream), the span collapses to a
/// single space `" "` rather than empty string. This preserves inter-word
/// boundaries that would otherwise fuse adjacent words:
/// `"Click"` + `" "(bold)` + `"here"(bold)` → `"Click **here**"` not
/// `"Click**here**"`.
pub(crate) fn render_span(span: &Span) -> String {
    if !span.bold && !span.italic {
        // Plain spans: preserve text verbatim for spacing; return empty only
        // for truly empty source text.
        if span.text.is_empty() {
            return String::new();
        }
        return escape_text(&span.text);
    }

    // Formatted spans: trim to avoid whitespace-wrapped markers.
    let core = span.text.trim();

    if core.is_empty() {
        // Source had only whitespace — preserve one space as a word separator
        // so adjacent plain spans are not fused by the filter in render_spans.
        return if span.text.is_empty() {
            String::new()
        } else {
            " ".to_string()
        };
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

/// Render a single AST node to its CommonMark block representation.
///
/// Empty headings and paragraphs (all spans resolve to whitespace) produce
/// an empty string rather than `"### \n\n"` or `"\n\n"`, so the document
/// normalisation step in `render_document` can strip them cleanly.
pub(crate) fn render_node(node: &Node) -> String {
    match node {
        Node::Heading { level, spans } => {
            let text = trim_trailing_ws_per_line(&render_spans(spans));
            if text.is_empty() {
                return String::new();
            }
            let hashes = "#".repeat((*level).clamp(1, 6) as usize);
            format!("{hashes} {text}\n\n")
        }
        Node::Paragraph { spans } => {
            let text = trim_trailing_ws_per_line(&render_spans(spans));
            if text.is_empty() {
                return String::new();
            }
            format!("{text}\n\n")
        }
        Node::RawText(text) => {
            // RawText is a best-effort fallback from unresolved fonts; content
            // is passed through without escaping (it may already be plain text
            // that should not be double-escaped).
            let cleaned = trim_trailing_ws_per_line(text);
            if cleaned.is_empty() {
                return String::new();
            }
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
    use crate::ast::DocumentMetadata;

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
        // Original set
        let raw = r"\`*_{}[]()#+-.!|";
        let escaped = escape_text(raw);
        assert_eq!(escaped, r"\\\`\*\_\{\}\[\]\(\)\#\+\-\.\!\|");
    }

    #[test]
    fn escape_text_escapes_html_structural_chars() {
        // <, >, & must be escaped to prevent autolinks, raw HTML, and entity refs
        assert_eq!(escape_text("<"), r"\<");
        assert_eq!(escape_text(">"), r"\>");
        assert_eq!(escape_text("&"), r"\&");
        assert_eq!(escape_text("A < B & C > D"), r"A \< B \& C \> D");
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
    fn render_span_drops_empty_formatted_output_but_preserves_spacing() {
        // Truly empty source → empty output
        assert_eq!(render_span(&span("", true, false)), "");
        assert_eq!(render_span(&span("", true, true)), "");
        // Whitespace-only source → single space (word boundary preservation)
        assert_eq!(render_span(&span(" ", true, false)), " ");
        assert_eq!(render_span(&span("   ", true, true)), " ");
        assert_eq!(render_span(&span("\t", false, true)), " ");
    }

    #[test]
    fn render_span_trims_surrounding_whitespace_before_applying_markers() {
        assert_eq!(
            render_span(&span("  bold me  ", true, false)),
            "**bold me**"
        );
        assert_eq!(render_span(&span("\tbold\t", false, true)), "*bold*");
    }

    #[test]
    fn render_span_escapes_inner_text_without_escaping_markers() {
        assert_eq!(render_span(&span("A*B", true, false)), "**A\\*B**");
    }

    #[test]
    fn render_span_escapes_html_chars_in_plain_and_formatted() {
        assert_eq!(render_span(&span("a < b", false, false)), r"a \< b");
        assert_eq!(render_span(&span("a > b", false, false)), r"a \> b");
        assert_eq!(render_span(&span("a & b", false, false)), r"a \& b");
        assert_eq!(render_span(&span("x < y", true, false)), r"**x \< y**");
    }

    #[test]
    fn render_spans_preserves_inter_word_space_from_formatted_whitespace_span() {
        // This is the critical inter-word fusing regression test.
        // PDF extractors emit inter-word spaces as separate spans that inherit
        // the current font's bold/italic state.
        let spans = vec![
            span("Click", false, false),
            span(" ", true, false), // bold space from PDF — must become " " not ""
            span("here", true, false),
        ];
        // Expected: "Click **here**" (space preserved, consecutive bold fused)
        // The bold space collapses to " " which render_spans keeps.
        // "Click" + " " + "**here**" = "Click **here**"
        let result = render_spans(&spans);
        assert_eq!(result, "Click **here**");
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
    fn render_node_heading_level_clamping() {
        // Level 0 → clamp to 1 → "#"
        let h0 = Node::Heading {
            level: 0,
            spans: vec![span("X", false, false)],
        };
        assert_eq!(render_node(&h0), "# X\n\n");
        // Level 7 → clamp to 6 → "######"
        let h7 = Node::Heading {
            level: 7,
            spans: vec![span("X", false, false)],
        };
        assert_eq!(render_node(&h7), "###### X\n\n");
    }

    #[test]
    fn render_node_empty_heading_produces_empty_string() {
        // All-whitespace spans collapse; heading should not emit "### \n\n"
        let node = Node::Heading {
            level: 3,
            spans: vec![span("", true, false), span("   ", false, true)],
        };
        assert_eq!(render_node(&node), "");
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
    fn render_node_empty_paragraph_produces_empty_string() {
        let node = Node::Paragraph {
            spans: vec![span("", true, false)],
        };
        assert_eq!(render_node(&node), "");
    }

    #[test]
    fn render_node_raw_text_passthrough_appends_blank_line() {
        assert_eq!(render_node(&Node::RawText("raw".to_string())), "raw\n\n");
    }

    #[test]
    fn render_node_empty_raw_text_produces_empty_string() {
        assert_eq!(render_node(&Node::RawText(String::new())), "");
        assert_eq!(render_node(&Node::RawText("   ".to_string())), "");
    }

    // ── render_document ──────────────────────────────────────────────────────

    #[test]
    fn render_document_has_single_trailing_newline_for_non_empty_docs() {
        let doc = Document {
            metadata: DocumentMetadata {
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
            metadata: DocumentMetadata {
                title: None,
                author: None,
                page_count: 0,
            },
            nodes: vec![],
        };
        assert_eq!(render_document(&doc), "");
    }

    #[test]
    fn render_document_skips_empty_nodes_cleanly() {
        // An all-whitespace heading sandwiched between real content must not
        // produce extra blank lines in the output.
        let doc = Document {
            metadata: DocumentMetadata {
                title: None,
                author: None,
                page_count: 1,
            },
            nodes: vec![
                Node::Paragraph {
                    spans: vec![span("Before", false, false)],
                },
                Node::Heading {
                    level: 2,
                    spans: vec![span("   ", false, false)],
                },
                Node::Paragraph {
                    spans: vec![span("After", false, false)],
                },
            ],
        };
        let markdown = render_document(&doc);
        assert_eq!(markdown, "Before\n\nAfter\n");
    }
}
