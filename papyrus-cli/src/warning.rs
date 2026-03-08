use owo_colors::OwoColorize;
use papyrus_core::ast::Warning;

pub fn format_warning(warning: &Warning) -> String {
    let prefix = "Warning:".yellow().to_string();
    match warning {
        Warning::MissingFontMetrics { font_name, page } => {
            format!("{prefix} Missing font metrics for \"{font_name}\" on {}", format!("page {page}").cyan())
        }
        Warning::UnreadableTextStream { page, detail } => {
            format!("{prefix} Unreadable text stream on {} ({detail})", format!("page {page}").cyan())
        }
        Warning::UnsupportedEncoding { encoding, page } => {
            format!("{prefix} Unsupported encoding \"{encoding}\" on {}", format!("page {page}").cyan())
        }
        Warning::MalformedPdfObject { detail } => {
            format!("{prefix} Malformed PDF object ({detail})")
        }
    }
}

pub fn render_warning_lines(warnings: &[Warning], quiet: bool) -> Vec<String> {
    if quiet {
        return vec![];
    }
    warnings.iter().map(format_warning).collect()
}

#[cfg(test)]
mod tests {
    use super::{format_warning, render_warning_lines};
    use papyrus_core::ast::Warning;

    #[test]
    fn formats_missing_font_metrics_warning() {
        let line = format_warning(&Warning::MissingFontMetrics {
            font_name: "ComicSans".to_string(),
            page: 3,
        });
        assert!(line.contains("Warning:"));
        assert!(line.contains("ComicSans"));
        assert!(line.contains("page 3"));
    }

    #[test]
    fn quiet_mode_suppresses_warnings() {
        let warnings = vec![Warning::MalformedPdfObject {
            detail: "broken xref".to_string(),
        }];
        let lines = render_warning_lines(&warnings, true);
        assert!(lines.is_empty());
    }
}
