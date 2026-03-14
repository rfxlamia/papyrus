use owo_colors::OwoColorize;
use papyrus_core::ast::Warning;

/// Formats a single warning with colored output.
///
/// # Arguments
/// * `warning` - The warning to format
///
/// # Returns
/// Formatted string with ANSI color codes (yellow "Warning:", cyan page numbers).
pub fn format_warning(warning: &Warning) -> String {
    let prefix = "Warning:".yellow().to_string();
    match warning {
        Warning::MissingFontMetrics { font_name, page } => {
            format!(
                "{prefix} Missing font metrics for \"{font_name}\" on {}",
                format!("page {page}").cyan()
            )
        }
        Warning::UnreadableTextStream { page, detail } => {
            format!(
                "{prefix} Unreadable text stream on {} ({detail})",
                format!("page {page}").cyan()
            )
        }
        Warning::UnsupportedEncoding { encoding, page } => {
            format!(
                "{prefix} Unsupported encoding \"{encoding}\" on {}",
                format!("page {page}").cyan()
            )
        }
        Warning::MalformedPdfObject { detail } => {
            format!("{prefix} Malformed PDF object ({detail})")
        }
        Warning::RotatedTextDetected { page, segment_count } => {
            format!(
                "{prefix} {segment_count} rotated text segment(s) on {}",
                format!("page {page}").cyan()
            )
        }
        Warning::ImageOnlyPage { page } => {
            format!(
                "{prefix} Image-only page detected: {}",
                format!("page {page}").cyan()
            )
        }
    }
}

/// Renders a list of warnings as formatted strings.
///
/// # Arguments
/// * `warnings` - Slice of warnings to render
/// * `quiet` - If true, suppresses all output and returns empty vector
///
/// # Returns
/// Vector of formatted warning strings, or empty vector if quiet mode is enabled.
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

    #[test]
    fn formats_rotated_text_detected_warning() {
        let line = format_warning(&Warning::RotatedTextDetected {
            page: 5,
            segment_count: 7,
        });
        assert!(line.contains("Warning:"));
        assert!(line.contains("7 rotated text segment(s)"));
        assert!(line.contains("page 5"));
        // Ensure the page text is colored (cyan) via ANSI escape sequence.
        assert!(line.contains("\u{1b}[36m"));
    }

    #[test]
    fn formats_image_only_page_warning() {
        let line = format_warning(&Warning::ImageOnlyPage { page: 8 });
        assert!(line.contains("Warning:"));
        assert!(line.contains("Image-only page detected:"));
        assert!(line.contains("page 8"));
        // Ensure the page text is colored (cyan) via ANSI escape sequence.
        assert!(line.contains("\u{1b}[36m"));
    }
}
