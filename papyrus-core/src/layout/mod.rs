//! Spatial layout analysis for positioning-aware text reconstruction.
//!
//! Groups raw text segments into lines based on Y-proximity, sorts by
//! reading order (Y descending, X ascending), and provides line/paragraph
//! break detection.

use crate::parser::RawTextSegment;

/// Group segments into lines based on Y-proximity.
///
/// Segments with `|y1 - y2| < font_size * 0.5` are considered the same line.
/// Rotated segments are excluded. Lines are sorted Y-descending (top of page
/// first), segments within each line sorted X-ascending (left to right).
pub fn group_into_lines<'a>(
    segments: &'a [RawTextSegment],
    body_font_size: f32,
) -> Vec<Vec<&'a RawTextSegment>> {
    let tolerance = body_font_size * 0.5;

    // Filter out rotated segments
    let mut normal: Vec<&RawTextSegment> = segments.iter().filter(|s| !s.is_rotated).collect();

    if normal.is_empty() {
        return Vec::new();
    }

    // Sort by Y descending (top first), then X ascending
    normal.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Group into lines by Y-proximity
    let mut lines: Vec<Vec<&RawTextSegment>> = Vec::new();
    let mut current_line: Vec<&RawTextSegment> = vec![normal[0]];
    let mut current_y = normal[0].y;

    for seg in &normal[1..] {
        if (current_y - seg.y).abs() <= tolerance {
            current_line.push(seg);
        } else {
            // Sort current line by X before finalizing
            current_line
                .sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            lines.push(current_line);
            current_line = vec![seg];
            current_y = seg.y;
        }
    }
    // Don't forget the last line
    current_line.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    lines.push(current_line);

    lines
}

/// Collect rotated segments from a page (excluded from layout pipeline).
pub fn collect_rotated<'a>(segments: &'a [RawTextSegment]) -> Vec<&'a RawTextSegment> {
    segments.iter().filter(|s| s.is_rotated).collect()
}

/// Reconstruct text for a single line with X-gap word spacing.
///
/// Inserts a space between adjacent segments when the X-gap exceeds
/// `space_width * 0.8`. Uses `font_size * 0.3` as fallback space width.
pub fn reconstruct_line_text(line: &[&RawTextSegment]) -> String {
    if line.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut prev_end_x: Option<f32> = None;

    for seg in line {
        if let Some(prev_x) = prev_end_x {
            let gap = seg.x - prev_x;
            let space_width = seg.font_size * 0.3; // fallback, replaced by font metrics in v0.1.2
            if gap > space_width * 0.8 {
                result.push(' ');
            }
        }
        result.push_str(&seg.text);
        // Estimate end X of this segment
        let width = seg.text.chars().count() as f32 * seg.font_size * 0.6;
        prev_end_x = Some(seg.x + width);
    }

    result
}

/// Detect paragraph break between two consecutive lines.
///
/// Returns true when the Y-gap between line_above and line_below exceeds
/// `median_line_height * 1.5`.
pub fn is_paragraph_break(line_above_y: f32, line_below_y: f32, median_line_height: f32) -> bool {
    let gap = (line_above_y - line_below_y).abs();
    gap > median_line_height * 1.5
}

/// Compute the median Y-gap between consecutive lines.
///
/// Returns the median inter-line gap, or `body_font_size * 1.2` as fallback
/// when fewer than 2 lines exist.
pub fn compute_median_line_height(lines: &[Vec<&RawTextSegment>], body_font_size: f32) -> f32 {
    if lines.len() < 2 {
        return body_font_size * 1.2;
    }

    let mut gaps: Vec<f32> = Vec::new();
    for pair in lines.windows(2) {
        let y_above = pair[0].first().map(|s| s.y).unwrap_or(0.0);
        let y_below = pair[1].first().map(|s| s.y).unwrap_or(0.0);
        let gap = (y_above - y_below).abs();
        if gap > 0.0 {
            gaps.push(gap);
        }
    }

    if gaps.is_empty() {
        return body_font_size * 1.2;
    }

    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    gaps[gaps.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::RawTextSegment;

    fn seg_at(text: &str, x: f32, y: f32, font_size: f32) -> RawTextSegment {
        RawTextSegment {
            text: text.to_string(),
            font_resource_name: b"F1".to_vec(),
            font_size,
            page_number: 1,
            x,
            y,
            is_rotated: false,
        }
    }

    #[test]
    fn group_lines_by_y_proximity() {
        let segments = vec![
            seg_at("Hello", 72.0, 700.0, 12.0),
            seg_at("World", 120.0, 700.0, 12.0),
            seg_at("Second line", 72.0, 686.0, 12.0),
        ];
        let lines = group_into_lines(&segments, 12.0);
        assert_eq!(lines.len(), 2, "should detect 2 lines");
        assert_eq!(lines[0].len(), 2, "first line should have 2 segments");
        assert_eq!(lines[1].len(), 1, "second line should have 1 segment");
    }

    #[test]
    fn lines_sorted_y_descending_x_ascending() {
        let segments = vec![
            seg_at("B", 200.0, 700.0, 12.0),
            seg_at("A", 72.0, 700.0, 12.0),
            seg_at("C", 72.0, 686.0, 12.0),
        ];
        let lines = group_into_lines(&segments, 12.0);
        // First line (Y=700) comes first (higher Y = higher on page)
        assert_eq!(lines[0][0].text, "A"); // X=72 before X=200
        assert_eq!(lines[0][1].text, "B");
        assert_eq!(lines[1][0].text, "C"); // second line
    }

    #[test]
    fn rotated_segments_excluded_from_lines() {
        let mut rotated = seg_at("WATERMARK", 300.0, 400.0, 24.0);
        rotated.is_rotated = true;
        let segments = vec![seg_at("Normal", 72.0, 700.0, 12.0), rotated];
        let lines = group_into_lines(&segments, 12.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0][0].text, "Normal");
    }

    #[test]
    fn collect_rotated_filters_correctly() {
        let mut rotated = seg_at("WATERMARK", 300.0, 400.0, 24.0);
        rotated.is_rotated = true;
        let segments = vec![seg_at("Normal", 72.0, 700.0, 12.0), rotated];
        let rot = collect_rotated(&segments);
        assert_eq!(rot.len(), 1);
        assert_eq!(rot[0].text, "WATERMARK");
    }

    #[test]
    fn reconstruct_line_text_joins_segments() {
        let segments = vec![
            seg_at("Hello", 72.0, 700.0, 12.0),
            seg_at("World", 130.0, 700.0, 12.0),
        ];
        let refs: Vec<&RawTextSegment> = segments.iter().collect();
        let text = reconstruct_line_text(&refs);
        assert!(
            text.contains("Hello") && text.contains("World"),
            "line text should contain both segments: {:?}",
            text
        );
    }

    #[test]
    fn paragraph_break_detection() {
        let median = 14.0;
        // Normal line gap (14pt) — not a paragraph break
        assert!(!is_paragraph_break(700.0, 686.0, median));
        // Large gap (28pt > 21pt threshold) — paragraph break
        assert!(is_paragraph_break(700.0, 672.0, median));
    }

    #[test]
    fn compute_median_line_height_with_lines() {
        let segments = vec![
            seg_at("Line 1", 72.0, 700.0, 12.0),
            seg_at("Line 2", 72.0, 686.0, 12.0),
            seg_at("Line 3", 72.0, 672.0, 12.0),
        ];
        let lines = group_into_lines(&segments, 12.0);
        let median = compute_median_line_height(&lines, 12.0);
        assert!((median - 14.0).abs() < 0.1, "median line height should be ~14pt, got {}", median);
    }

    #[test]
    fn compute_median_line_height_fallback() {
        let lines: Vec<Vec<&RawTextSegment>> = Vec::new();
        let median = compute_median_line_height(&lines, 12.0);
        assert!((median - 14.4).abs() < 0.1, "fallback should be font_size * 1.2");
    }

    #[test]
    fn superscript_stays_on_same_line() {
        // Superscript baseline is ~4pt above normal 12pt text
        // Tolerance is 12.0 * 0.5 = 6.0, so 4pt shift should stay on same line
        let segments = vec![
            seg_at("Main text", 72.0, 700.0, 12.0),
            seg_at("[1]", 200.0, 704.0, 8.0), // superscript, Y shifted up 4pt
        ];
        let lines = group_into_lines(&segments, 12.0);
        assert_eq!(lines.len(), 1, "superscript should stay on same line");
    }
}
