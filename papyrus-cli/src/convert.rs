use std::io;
use std::path::{Path, PathBuf};
use papyrus_core::Papyrus;

#[derive(Debug, Clone, Copy)]
pub struct ConvertConfig {
    pub heading_ratio: f32,
    pub detect_bold: bool,
    pub detect_italic: bool,
    pub quiet: bool,
}

impl ConvertConfig {
    pub fn from_flags(heading_ratio: f32, no_bold: bool, no_italic: bool, quiet: bool) -> Self {
        Self {
            heading_ratio,
            detect_bold: !no_bold,
            detect_italic: !no_italic,
            quiet,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ConversionSummary {
    pub succeeded: bool,
    pub warnings: Vec<papyrus_core::ast::Warning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    Pipe,
    File(PathBuf),
    Directory(PathBuf),
}

pub fn classify_input(input: &Path, stdin_is_terminal: bool) -> io::Result<InputKind> {
    if input == Path::new("-") || !stdin_is_terminal {
        return Ok(InputKind::Pipe);
    }
    let meta = std::fs::metadata(input)?;
    if meta.is_dir() {
        Ok(InputKind::Directory(input.to_path_buf()))
    } else {
        Ok(InputKind::File(input.to_path_buf()))
    }
}

fn build_engine(cfg: ConvertConfig) -> Papyrus {
    Papyrus::builder()
        .heading_size_ratio(cfg.heading_ratio)
        .detect_bold(cfg.detect_bold)
        .detect_italic(cfg.detect_italic)
        .build()
}

pub fn workspace_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join(name)
}

pub fn convert_file(
    input: &Path,
    output: Option<&Path>,
    cfg: ConvertConfig,
) -> io::Result<ConversionSummary> {
    let bytes = std::fs::read(input)?;
    let result = build_engine(cfg).extract(&bytes);
    let markdown = result.to_markdown();

    if let Some(out_path) = output {
        std::fs::write(out_path, markdown)?;
    }

    Ok(ConversionSummary {
        succeeded: true,
        warnings: result.warnings,
    })
}

pub fn discover_pdf_files(input_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = vec![];
    for entry in std::fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("pdf") {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

pub fn target_path(input_root: &Path, input_file: &Path, output: Option<&Path>) -> io::Result<PathBuf> {
    let stem = input_file
        .file_stem()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing file stem"))?;

    let mut target = match output {
        Some(output_root) => output_root.join(stem),
        None => input_root.join(stem),
    };
    target.set_extension("md");
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::{classify_input, ConvertConfig, InputKind};
    use std::path::Path;

    #[test]
    fn dash_input_is_pipe_mode() {
        let mode = classify_input(Path::new("-"), true).unwrap();
        assert!(matches!(mode, InputKind::Pipe));
    }

    #[test]
    fn non_tty_stdin_forces_pipe_mode() {
        let mode = classify_input(Path::new("tests/fixtures/simple.pdf"), false).unwrap();
        assert!(matches!(mode, InputKind::Pipe));
    }

    #[test]
    fn convert_config_disables_styles_from_flags() {
        let cfg = ConvertConfig::from_flags(2.0, true, true, false);
        assert_eq!(cfg.heading_ratio, 2.0);
        assert!(!cfg.detect_bold);
        assert!(!cfg.detect_italic);
        assert!(!cfg.quiet);
    }

    #[test]
    fn convert_file_writes_markdown_to_output_file() {
        use super::{convert_file, workspace_fixture};
        let fixture = workspace_fixture("simple.pdf");
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("simple.md");

        let result = convert_file(&fixture, Some(&out), ConvertConfig::from_flags(1.2, false, false, false))
            .unwrap();

        assert!(result.succeeded);
        let markdown = std::fs::read_to_string(&out).unwrap();
        assert!(markdown.contains("Chapter 1"));
    }

    #[test]
    fn convert_file_missing_input_returns_not_found() {
        use super::convert_file;
        let err = convert_file(
            Path::new("tests/fixtures/does-not-exist.pdf"),
            None,
            ConvertConfig::from_flags(1.2, false, false, false),
        )
        .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn discover_pdf_files_is_non_recursive_and_sorted() {
        use super::discover_pdf_files;
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.pdf"), b"%PDF-1.7").unwrap();
        std::fs::write(tmp.path().join("b.PDF"), b"%PDF-1.7").unwrap();
        std::fs::write(tmp.path().join("notes.txt"), b"nope").unwrap();
        std::fs::create_dir(tmp.path().join("nested")).unwrap();
        std::fs::write(tmp.path().join("nested").join("c.pdf"), b"%PDF-1.7").unwrap();

        let files = discover_pdf_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("a.pdf"));
        assert!(files[1].ends_with("b.PDF"));
    }

    #[test]
    fn target_path_maps_to_output_dir_with_md_extension() {
        use super::target_path;
        use std::path::PathBuf;
        let input_root = Path::new("/tmp/in");
        let input_file = Path::new("/tmp/in/report.pdf");
        let output_root = Path::new("/tmp/out");

        let target = target_path(input_root, input_file, Some(output_root)).unwrap();
        assert_eq!(target, PathBuf::from("/tmp/out/report.md"));
    }
}
