pub mod ast;
pub mod detector;
pub mod parser;
pub mod renderer;

use std::collections::HashMap;

use ast::ConversionResult;
use detector::{build_document, DetectorConfig};

/// A configured extraction engine.
///
/// Construct via [`Papyrus::builder`] to customise detection thresholds,
/// or call the top-level [`convert`] function for zero-configuration extraction.
#[derive(Debug, Clone)]
pub struct Papyrus {
    config: DetectorConfig,
}

/// Builder for [`Papyrus`].
///
/// All settings have sensible defaults via [`DetectorConfig::default`]; only
/// set the values you want to override.
#[derive(Debug, Clone)]
pub struct PapyrusBuilder {
    config: DetectorConfig,
}

impl PapyrusBuilder {
    /// Minimum font-size ratio over the computed body size to treat a segment
    /// as a heading. Must be less than `1.4` (the fixed level-3 boundary).
    /// Default: `1.2`.
    pub fn heading_size_ratio(mut self, ratio: f32) -> Self {
        self.config.heading_size_ratio = ratio;
        self
    }

    /// Enable or disable bold detection from font name / descriptor metrics.
    /// When `false`, all spans have `bold = false`. Default: `true`.
    pub fn detect_bold(mut self, enabled: bool) -> Self {
        self.config.detect_bold = enabled;
        self
    }

    /// Enable or disable italic detection from font name / descriptor metrics.
    /// When `false`, all spans have `italic = false`. Default: `true`.
    pub fn detect_italic(mut self, enabled: bool) -> Self {
        self.config.detect_italic = enabled;
        self
    }

    /// Consume the builder and return a configured [`Papyrus`] engine.
    pub fn build(self) -> Papyrus {
        Papyrus {
            config: self.config,
        }
    }
}

impl Papyrus {
    /// Return a [`PapyrusBuilder`] pre-loaded with default settings.
    pub fn builder() -> PapyrusBuilder {
        PapyrusBuilder {
            config: DetectorConfig::default(),
        }
    }

    /// Extract structured content from `pdf_bytes`.
    ///
    /// Parsing and detection are best-effort: any problems are captured as
    /// [`ast::Warning`] values in the returned [`ConversionResult`] rather
    /// than surfaced as errors.
    pub fn extract(&self, pdf_bytes: &[u8]) -> ConversionResult {
        let (segments, metadata, mut warnings) = parser::parse_pdf(pdf_bytes);
        let fonts = collect_fonts_for_segments(pdf_bytes, &segments);
        let (document, detector_warnings) =
            build_document(segments, &fonts, &self.config, metadata);
        warnings.extend(detector_warnings);
        ConversionResult { document, warnings }
    }
}

/// Extract structured content from `pdf_bytes` using default settings.
///
/// Equivalent to `Papyrus::builder().build().extract(pdf_bytes)`.
pub fn convert(pdf_bytes: &[u8]) -> ConversionResult {
    Papyrus::builder().build().extract(pdf_bytes)
}

/// Resolve font metadata for every page that appears in `segments`.
///
/// Loads the PDF once and queries each unique page number. Font resource names
/// from different pages that share a name will be merged (last-page wins for
/// duplicate resource keys, which is safe because resource names are
/// per-page-local in the PDF spec).
fn collect_fonts_for_segments(
    pdf_bytes: &[u8],
    segments: &[parser::RawTextSegment],
) -> HashMap<Vec<u8>, parser::FontInfo> {
    let mut fonts = HashMap::new();
    let (doc_opt, _) = parser::load_pdf(pdf_bytes);
    let Some(doc) = doc_opt else {
        return fonts;
    };

    // Deduplicate page numbers so we only call resolve_fonts_for_page once per page.
    let mut pages = segments.iter().map(|s| s.page_number).collect::<Vec<_>>();
    pages.sort_unstable();
    pages.dedup();

    for page in pages {
        let (page_fonts, _) = parser::resolve_fonts_for_page(&doc, page);
        fonts.extend(page_fonts);
    }

    fonts
}
