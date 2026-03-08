use crate::ast::{Document, Span};

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
    let core = span.text.trim();

    if core.is_empty() {
        return String::new();
    }

    if !span.bold && !span.italic {
        return escape_text(core);
    }

    let marker = match (span.bold, span.italic) {
        (true, true) => "***",
        (true, false) => "**",
        (false, true) => "*",
        (false, false) => unreachable!("plain spans return early above"),
    };

    format!("{marker}{}{marker}", escape_text(core))
}

// ── Public API ───────────────────────────────────────────────────────────────

pub fn render_document(_document: &Document) -> String {
    String::new()
}

// ── Tests ────────────────────────────────────────────────────────────────────

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
    fn escape_text_escapes_all_commonmark_special_chars() {
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
        // plain spans also trim surrounding whitespace
        assert_eq!(render_span(&span("  plain  ", false, false)), "plain");
    }

    #[test]
    fn render_span_drops_empty_formatted_output() {
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
}
