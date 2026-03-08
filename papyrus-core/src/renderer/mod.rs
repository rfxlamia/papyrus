use crate::ast::{Document, Span};

pub fn render_document(_document: &Document) -> String {
    String::new()
}

pub(crate) fn render_span(span: &Span) -> String {
    if span.text.is_empty() {
        return String::new();
    }

    if !span.bold && !span.italic {
        return escape_text(&span.text);
    }

    let leading_len = span.text.len() - span.text.trim_start().len();
    let trailing_len = span.text.len() - span.text.trim_end().len();
    let leading = &span.text[..leading_len];
    let trailing = &span.text[span.text.len() - trailing_len..];
    let core = span.text.trim();

    if core.is_empty() {
        return String::new();
    }

    let marker = match (span.bold, span.italic) {
        (true, true) => "***",
        (true, false) => "**",
        (false, true) => "*",
        (false, false) => "",
    };

    let escaped_core = escape_text(core);
    format!("{leading}{marker}{escaped_core}{marker}{trailing}")
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;

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
    fn escape_text_escapes_all_phase4_commonmark_special_chars() {
        let raw = r"\`*_{}[]()#+-.!|";
        let escaped = escape_text(raw);
        assert_eq!(escaped, r"\\\`\*\_\{\}\[\]\(\)\#\+\-\.\!\|");
    }

    #[test]
    fn escape_text_leaves_safe_text_unchanged() {
        assert_eq!(escape_text("Papyrus Renderer 123"), "Papyrus Renderer 123");
    }

    #[test]
    fn render_span_supports_plain_bold_italic_and_bold_italic() {
        assert_eq!(render_span(&span("plain", false, false)), "plain");
        assert_eq!(render_span(&span("bold", true, false)), "**bold**");
        assert_eq!(render_span(&span("italic", false, true)), "*italic*");
        assert_eq!(render_span(&span("both", true, true)), "***both***");
    }

    #[test]
    fn render_span_drops_empty_formatted_output() {
        assert_eq!(render_span(&span("", true, false)), "");
        assert_eq!(render_span(&span("   ", true, true)), "");
    }

    #[test]
    fn render_span_trims_whitespace_inside_markers_only() {
        assert_eq!(
            render_span(&span("  bold me  ", true, false)),
            "  **bold me**  "
        );
    }
}
