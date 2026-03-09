use indicatif::{ProgressBar, ProgressStyle};
use papyrus_core::Papyrus;
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

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

#[derive(Debug, Default, Clone)]
pub struct BatchSummary {
    pub converted: usize,
    pub failed: usize,
    pub warnings: Vec<(PathBuf, papyrus_core::ast::Warning)>,
}

impl BatchSummary {
    pub fn exit_code(&self) -> i32 {
        if self.converted > 0 {
            0
        } else {
            1
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    Pipe,
    File(PathBuf),
    Directory(PathBuf),
}

/// Classifies the input path as pipe mode (stdin), file, or directory.
///
/// # Arguments
/// * `input` - Path to classify. Use "-" for stdin/pipe mode.
///
/// # Returns
/// * `InputKind::Pipe` if input is "-"
/// * `InputKind::File` if input is a file
/// * `InputKind::Directory` if input is a directory
///
/// # Errors
/// Returns `io::Error` if the path cannot be accessed or metadata cannot be read.
pub fn classify_input(input: &Path) -> io::Result<InputKind> {
    // Note: Simplified from original plan which required non-TTY stdin to force pipe mode.
    // Current implementation only uses explicit "-" for pipe mode, which is more intuitive
    // and aligns with standard Unix CLI conventions (explicit stdin marker).
    if input == Path::new("-") {
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

#[cfg(test)]
pub(crate) fn workspace_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Converts a single PDF file to Markdown.
///
/// # Arguments
/// * `input` - Path to the input PDF file
/// * `output` - Optional path to write the output. If `None`, output is not written to disk.
/// * `cfg` - Conversion configuration (heading ratio, style detection, quiet mode)
///
/// # Returns
/// `ConversionSummary` containing success status and any warnings encountered.
///
/// # Errors
/// Returns `io::Error` if the file cannot be read or written.
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

/// Discovers all PDF files in a directory (non-recursive).
///
/// # Arguments
/// * `input_dir` - Directory to search for PDF files
///
/// # Returns
/// Sorted vector of paths to PDF files (case-insensitive .pdf or .PDF extension).
///
/// # Errors
/// Returns `io::Error` if the directory cannot be read.
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

/// Maps an input PDF file path to its corresponding output Markdown path.
///
/// # Arguments
/// * `input_root` - Root directory of the input file
/// * `input_file` - Path to the input PDF file
/// * `output` - Optional output directory. If `None`, uses `input_root`.
///
/// # Returns
/// Path with the same stem as the input file but with `.md` extension.
///
/// # Errors
/// Returns `io::Error` if the file stem cannot be determined.
pub fn target_path(
    input_root: &Path,
    input_file: &Path,
    output: Option<&Path>,
) -> io::Result<PathBuf> {
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

/// Converts all PDF files in a directory to Markdown with progress indication.
///
/// # Arguments
/// * `input_dir` - Directory containing PDF files to convert
/// * `output_dir` - Optional output directory. If `None`, writes to `input_dir`.
/// * `cfg` - Conversion configuration
///
/// # Returns
/// `BatchSummary` with counts of converted/failed files and collected warnings.
/// Exit code is 0 if at least one file converted successfully, 1 if all failed.
///
/// # Errors
/// Returns `io::Error` if directories cannot be accessed or created.
pub fn convert_directory(
    input_dir: &Path,
    output_dir: Option<&Path>,
    cfg: ConvertConfig,
) -> io::Result<BatchSummary> {
    let files = discover_pdf_files(input_dir)?;
    if let Some(dir) = output_dir {
        std::fs::create_dir_all(dir)?;
    }

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("[{pos}/{len}] Converting {msg}...")
            .expect("valid progress bar template"),
    );

    let mut summary = BatchSummary::default();
    for file in files {
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        pb.set_message(name);
        let target = target_path(input_dir, &file, output_dir)?;
        match convert_file(&file, Some(&target), cfg) {
            Ok(result) => {
                summary.converted += 1;
                for warning in result.warnings {
                    summary.warnings.push((file.clone(), warning));
                }
            }
            Err(_) => {
                summary.failed += 1;
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();
    Ok(summary)
}

/// Converts PDF from a reader (stdin) to Markdown written to a writer (stdout).
///
/// # Arguments
/// * `reader` - Input stream containing PDF bytes
/// * `writer` - Output stream for Markdown text
/// * `cfg` - Conversion configuration
///
/// # Returns
/// `ConversionSummary` containing success status and warnings.
///
/// # Errors
/// Returns `io::Error` if reading or writing fails.
pub fn convert_pipe<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    cfg: ConvertConfig,
) -> io::Result<ConversionSummary> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;

    let result = build_engine(cfg).extract(&bytes);
    writer.write_all(result.to_markdown().as_bytes())?;
    writer.flush()?;

    Ok(ConversionSummary {
        succeeded: true,
        warnings: result.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::{classify_input, ConvertConfig, InputKind};
    use std::path::Path;

    #[test]
    fn dash_input_is_pipe_mode() {
        let mode = classify_input(Path::new("-")).unwrap();
        assert!(matches!(mode, InputKind::Pipe));
    }

    #[test]
    fn file_path_is_file_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let test_file = tmp.path().join("test.pdf");
        std::fs::write(&test_file, b"%PDF").unwrap();
        let mode = classify_input(&test_file).unwrap();
        assert!(matches!(mode, InputKind::File(_)));
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

        let result = convert_file(
            &fixture,
            Some(&out),
            ConvertConfig::from_flags(1.2, false, false, false),
        )
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

    #[test]
    fn convert_directory_returns_success_when_at_least_one_file_converts() {
        use super::{convert_directory, workspace_fixture};
        let input = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();

        std::fs::copy(
            workspace_fixture("simple.pdf"),
            input.path().join("simple.pdf"),
        )
        .unwrap();
        std::fs::write(input.path().join("bad.pdf"), b"not a real pdf").unwrap();

        let summary = convert_directory(
            input.path(),
            Some(output.path()),
            ConvertConfig::from_flags(1.2, false, false, false),
        )
        .unwrap();

        assert!(summary.converted >= 1);
        assert!(summary.failed <= 1);
        assert_eq!(summary.exit_code(), 0);
    }

    #[test]
    fn convert_directory_returns_failure_when_all_files_fail() {
        use super::convert_directory;
        let input = tempfile::tempdir().unwrap();
        // Create a directory instead of a file to cause read failure
        std::fs::create_dir(input.path().join("bad.pdf")).unwrap();

        let summary = convert_directory(
            input.path(),
            None,
            ConvertConfig::from_flags(1.2, false, false, false),
        )
        .unwrap();

        assert_eq!(summary.converted, 0);
        assert_eq!(summary.exit_code(), 1);
    }

    #[test]
    fn convert_pipe_reads_stdin_and_writes_markdown_to_stdout() {
        use super::{convert_pipe, workspace_fixture};
        let bytes = std::fs::read(workspace_fixture("simple.pdf")).unwrap();
        let mut stdin = std::io::Cursor::new(bytes);
        let mut stdout = Vec::<u8>::new();

        let summary = convert_pipe(
            &mut stdin,
            &mut stdout,
            ConvertConfig::from_flags(1.2, false, false, false),
        )
        .unwrap();

        let markdown = String::from_utf8(stdout).unwrap();
        assert!(markdown.contains("Chapter 1"));
        assert!(summary.succeeded);
    }
}
