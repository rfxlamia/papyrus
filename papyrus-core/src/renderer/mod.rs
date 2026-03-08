use crate::ast::Document;

pub fn render_document(_document: &Document) -> String {
    String::new()
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
}
