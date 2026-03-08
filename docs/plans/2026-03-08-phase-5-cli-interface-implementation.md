# Phase 5 CLI Interface Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a production-ready `papyrus` CLI binary that supports single-file conversion, batch directory conversion, stdin/stdout pipelines, colored warnings, and correct exit codes.

**Architecture:** Split `papyrus-cli` into small testable modules (`cli`, `warning`, `convert`, `run`) exposed through `src/lib.rs`, while keeping `src/main.rs` as a thin process-exit wrapper. Keep all conversion logic as pure-ish functions that accept paths/readers/writers and return typed summaries so unit and integration tests can validate behavior without brittle stdout parsing. Use strict TDD in each task: write the failing test first, implement minimal code, then verify pass before committing.

**Tech Stack:** Rust 2021, `clap` derive API, `indicatif` progress bar, `owo-colors` terminal styling, `assert_cmd` + `predicates` + `tempfile` for CLI integration tests, existing `papyrus-core` API (`Papyrus::builder().extract().to_markdown()`).

---

**Execution Notes**
- Work in a dedicated worktree prepared before execution (`@brainstorming` context assumption).
- Use `@test-driven-development` on every task.
- Use `@systematic-debugging` immediately when a test fails unexpectedly.
- Use `@verification-before-completion` before claiming Phase 5 done.
- Keep changes DRY/YAGNI: no recursive directory traversal in v1, no async, no global mutable state.

### Task 1: Create CLI Package Surface and Dependencies

**Files:**
- Modify: `papyrus-cli/Cargo.toml`
- Create: `papyrus-cli/src/lib.rs`
- Create: `papyrus-cli/src/cli.rs`
- Test: `papyrus-cli/src/cli.rs` (`#[cfg(test)]`)

**Step 1: Write the failing test**

```rust
// papyrus-cli/src/cli.rs
#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parse_convert_defaults() {
        let cli = Cli::parse_from(["papyrus", "convert", "tests/fixtures/simple.pdf"]);
        match cli.command {
            Commands::Convert {
                input,
                output,
                heading_ratio,
                no_bold,
                no_italic,
                quiet,
            } => {
                assert_eq!(input, PathBuf::from("tests/fixtures/simple.pdf"));
                assert_eq!(output, None);
                assert!((heading_ratio - 1.2).abs() < f32::EPSILON);
                assert!(!no_bold);
                assert!(!no_italic);
                assert!(!quiet);
            }
        }
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli parse_convert_defaults -v`  
Expected: FAIL with unresolved imports (`clap`, `Cli`, `Commands`) because CLI module and dependencies do not exist yet.

**Step 3: Write minimal implementation**

```toml
# papyrus-cli/Cargo.toml
[package]
name = "papyrus-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "papyrus"
path = "src/main.rs"

[dependencies]
papyrus-core = { path = "../papyrus-core" }
clap = { version = "4.5", features = ["derive"] }
indicatif = "0.17"
owo-colors = "4.1"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3.13"
```

```rust
// papyrus-cli/src/lib.rs
pub mod cli;
```

```rust
// papyrus-cli/src/cli.rs
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "papyrus", about = "PDF to Markdown conversion engine")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Convert PDF to Markdown
    Convert {
        /// Input PDF file or directory. Use "-" for stdin.
        input: PathBuf,

        /// Output file or directory (default: stdout for file/stdin mode)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Heading size ratio threshold (default: 1.2)
        #[arg(long, default_value_t = 1.2)]
        heading_ratio: f32,

        /// Disable bold detection
        #[arg(long)]
        no_bold: bool,

        /// Disable italic detection
        #[arg(long)]
        no_italic: bool,

        /// Suppress warning output
        #[arg(long)]
        quiet: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parse_convert_defaults() {
        let cli = Cli::parse_from(["papyrus", "convert", "tests/fixtures/simple.pdf"]);
        match cli.command {
            Commands::Convert {
                input,
                output,
                heading_ratio,
                no_bold,
                no_italic,
                quiet,
            } => {
                assert_eq!(input, PathBuf::from("tests/fixtures/simple.pdf"));
                assert_eq!(output, None);
                assert!((heading_ratio - 1.2).abs() < f32::EPSILON);
                assert!(!no_bold);
                assert!(!no_italic);
                assert!(!quiet);
            }
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli parse_convert_defaults -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-cli/Cargo.toml papyrus-cli/src/lib.rs papyrus-cli/src/cli.rs
git commit -m "feat(cli): add clap command surface and dependencies"
```

### Task 2: Implement Warning Renderer with Quiet Support

**Files:**
- Modify: `papyrus-cli/src/lib.rs`
- Create: `papyrus-cli/src/warning.rs`
- Test: `papyrus-cli/src/warning.rs` (`#[cfg(test)]`)

**Step 1: Write the failing test**

```rust
// papyrus-cli/src/warning.rs
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli warning::tests -v`  
Expected: FAIL because `warning` module/functions do not exist.

**Step 3: Write minimal implementation**

```rust
// papyrus-cli/src/lib.rs
pub mod cli;
pub mod warning;
```

```rust
// papyrus-cli/src/warning.rs
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli warning::tests -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-cli/src/lib.rs papyrus-cli/src/warning.rs
git commit -m "feat(cli): add colored warning renderer with quiet mode"
```

### Task 3: Add Conversion Config and Input Classification

**Files:**
- Modify: `papyrus-cli/src/lib.rs`
- Create: `papyrus-cli/src/convert.rs`
- Test: `papyrus-cli/src/convert.rs` (`#[cfg(test)]`)

**Step 1: Write the failing test**

```rust
// papyrus-cli/src/convert.rs
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
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli convert::tests -v`  
Expected: FAIL because `convert` module types/functions are missing.

**Step 3: Write minimal implementation**

```rust
// papyrus-cli/src/lib.rs
pub mod cli;
pub mod convert;
pub mod warning;
```

```rust
// papyrus-cli/src/convert.rs
use std::io;
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
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli convert::tests -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-cli/src/lib.rs papyrus-cli/src/convert.rs
git commit -m "feat(cli): add conversion config and input classification"
```

### Task 4: Implement Single-File Conversion and Output Routing

**Files:**
- Modify: `papyrus-cli/src/convert.rs`
- Test: `papyrus-cli/src/convert.rs` (`#[cfg(test)]`)

**Step 1: Write the failing test**

```rust
#[test]
fn convert_file_writes_markdown_to_output_file() {
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
    let err = convert_file(
        Path::new("tests/fixtures/does-not-exist.pdf"),
        None,
        ConvertConfig::from_flags(1.2, false, false, false),
    )
    .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli convert_file_ -v`  
Expected: FAIL because `convert_file` and `ConversionSummary` are not implemented.

**Step 3: Write minimal implementation**

```rust
use papyrus_core::Papyrus;

#[derive(Debug, Default, Clone)]
pub struct ConversionSummary {
    pub succeeded: bool,
    pub warnings: Vec<papyrus_core::ast::Warning>,
}

fn build_engine(cfg: ConvertConfig) -> Papyrus {
    Papyrus::builder()
        .heading_size_ratio(cfg.heading_ratio)
        .detect_bold(cfg.detect_bold)
        .detect_italic(cfg.detect_italic)
        .build()
}

pub fn workspace_fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli convert_file_ -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-cli/src/convert.rs
git commit -m "feat(cli): implement single-file conversion flow"
```

### Task 5: Implement Batch File Discovery and Output Mapping

**Files:**
- Modify: `papyrus-cli/src/convert.rs`
- Test: `papyrus-cli/src/convert.rs` (`#[cfg(test)]`)

**Step 1: Write the failing test**

```rust
#[test]
fn discover_pdf_files_is_non_recursive_and_sorted() {
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
    let input_root = Path::new("/tmp/in");
    let input_file = Path::new("/tmp/in/report.pdf");
    let output_root = Path::new("/tmp/out");

    let target = target_path(input_root, input_file, Some(output_root)).unwrap();
    assert_eq!(target, PathBuf::from("/tmp/out/report.md"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli discover_pdf_files_ -v`  
Expected: FAIL because helper functions do not exist yet.

**Step 3: Write minimal implementation**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli discover_pdf_files_ target_path_maps_to_output_dir_with_md_extension -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-cli/src/convert.rs
git commit -m "feat(cli): add batch pdf discovery and output mapping helpers"
```

### Task 6: Implement Batch Directory Conversion with Partial Failure Semantics

**Files:**
- Modify: `papyrus-cli/src/convert.rs`
- Test: `papyrus-cli/src/convert.rs` (`#[cfg(test)]`)

**Step 1: Write the failing test**

```rust
#[test]
fn convert_directory_returns_success_when_at_least_one_file_converts() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    std::fs::copy(workspace_fixture("simple.pdf"), input.path().join("simple.pdf")).unwrap();
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
    let input = tempfile::tempdir().unwrap();
    std::fs::write(input.path().join("bad.pdf"), b"").unwrap();

    let summary = convert_directory(
        input.path(),
        None,
        ConvertConfig::from_flags(1.2, false, false, false),
    )
    .unwrap();

    assert_eq!(summary.converted, 0);
    assert_eq!(summary.exit_code(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli convert_directory_returns_ -v`  
Expected: FAIL because `convert_directory` and batch summary API are missing.

**Step 3: Write minimal implementation**

```rust
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug, Default, Clone)]
pub struct BatchSummary {
    pub converted: usize,
    pub failed: usize,
    pub warnings: Vec<(PathBuf, papyrus_core::ast::Warning)>,
}

impl BatchSummary {
    pub fn exit_code(&self) -> i32 {
        if self.converted > 0 { 0 } else { 1 }
    }
}

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
            .unwrap()
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli convert_directory_returns_ -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-cli/src/convert.rs
git commit -m "feat(cli): add batch directory conversion and exit semantics"
```

### Task 7: Implement Pipe Mode (stdin -> stdout)

**Files:**
- Modify: `papyrus-cli/src/convert.rs`
- Test: `papyrus-cli/src/convert.rs` (`#[cfg(test)]`)

**Step 1: Write the failing test**

```rust
#[test]
fn convert_pipe_reads_stdin_and_writes_markdown_to_stdout() {
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli convert_pipe_reads_stdin_and_writes_markdown_to_stdout -v`  
Expected: FAIL because `convert_pipe` is not implemented.

**Step 3: Write minimal implementation**

```rust
use std::io::{Read, Write};

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
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli convert_pipe_reads_stdin_and_writes_markdown_to_stdout -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-cli/src/convert.rs
git commit -m "feat(cli): add pipe conversion mode"
```

### Task 8: Add Runner Module and Exit Code Mapping

**Files:**
- Modify: `papyrus-cli/src/lib.rs`
- Create: `papyrus-cli/src/run.rs`
- Test: `papyrus-cli/src/run.rs` (`#[cfg(test)]`)

**Step 1: Write the failing test**

```rust
// papyrus-cli/src/run.rs
#[cfg(test)]
mod tests {
    use super::fatal_io_exit_code;
    #[test]
    fn fatal_io_uses_exit_code_1() {
        assert_eq!(fatal_io_exit_code(), 1);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli fatal_io_uses_exit_code_1 -v`  
Expected: FAIL because `run` module is missing.

**Step 3: Write minimal implementation**

```rust
// papyrus-cli/src/lib.rs
pub mod cli;
pub mod convert;
pub mod run;
pub mod warning;
```

```rust
// papyrus-cli/src/run.rs
use crate::cli::{Cli, Commands};
use crate::convert::{classify_input, convert_directory, convert_file, convert_pipe, ConvertConfig, InputKind};
use crate::warning::render_warning_lines;
use std::io::{self, IsTerminal, Write};

pub fn fatal_io_exit_code() -> i32 {
    1
}

pub fn run_cli(cli: Cli) -> i32 {
    match cli.command {
        Commands::Convert {
            input,
            output,
            heading_ratio,
            no_bold,
            no_italic,
            quiet,
        } => {
            let cfg = ConvertConfig::from_flags(heading_ratio, no_bold, no_italic, quiet);
            match classify_input(&input, io::stdin().is_terminal()) {
                Ok(InputKind::Pipe) => {
                    let mut stdin = io::stdin().lock();
                    let mut stdout = io::stdout().lock();
                    match convert_pipe(&mut stdin, &mut stdout, cfg) {
                        Ok(summary) => {
                            for line in render_warning_lines(&summary.warnings, cfg.quiet) {
                                eprintln!("{line}");
                            }
                            0
                        }
                        Err(err) => {
                            let _ = writeln!(io::stderr(), "error: {err}");
                            fatal_io_exit_code()
                        }
                    }
                }
                Ok(InputKind::File(path)) => {
                    match convert_file(&path, output.as_deref(), cfg) {
                        Ok(summary) => {
                            for line in render_warning_lines(&summary.warnings, cfg.quiet) {
                                eprintln!("{line}");
                            }
                            0
                        }
                        Err(err) => {
                            let _ = writeln!(io::stderr(), "error: {err}");
                            fatal_io_exit_code()
                        }
                    }
                }
                Ok(InputKind::Directory(path)) => {
                    match convert_directory(&path, output.as_deref(), cfg) {
                        Ok(summary) => {
                            for (file, warning) in summary.warnings {
                                for line in render_warning_lines(&[warning], cfg.quiet) {
                                    eprintln!("{}: {}", file.display(), line);
                                }
                            }
                            summary.exit_code()
                        }
                        Err(err) => {
                            let _ = writeln!(io::stderr(), "error: {err}");
                            fatal_io_exit_code()
                        }
                    }
                }
                Err(err) => {
                    let _ = writeln!(io::stderr(), "error: {err}");
                    fatal_io_exit_code()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fatal_io_exit_code;
    #[test]
    fn fatal_io_uses_exit_code_1() {
        assert_eq!(fatal_io_exit_code(), 1);
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli fatal_io_uses_exit_code_1 -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-cli/src/lib.rs papyrus-cli/src/run.rs
git commit -m "feat(cli): wire command runner and exit code mapping"
```

### Task 9: Replace Main Entry Point with Thin Process Wrapper

**Files:**
- Modify: `papyrus-cli/src/main.rs`
- Test: `papyrus-cli/tests/cli_integration.rs` (created in Task 10)

**Step 1: Write the failing test**

```rust
// papyrus-cli/tests/cli_integration.rs
use assert_cmd::Command;

#[test]
fn invalid_arguments_exit_with_code_2() {
    Command::cargo_bin("papyrus")
        .unwrap()
        .arg("convert")
        .assert()
        .code(2);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli invalid_arguments_exit_with_code_2 -v`  
Expected: FAIL because `main.rs` still prints stub text and does not invoke clap parser.

**Step 3: Write minimal implementation**

```rust
// papyrus-cli/src/main.rs
use clap::Parser;
use papyrus_cli::cli::Cli;
use papyrus_cli::run::run_cli;

fn main() {
    let cli = Cli::parse();
    std::process::exit(run_cli(cli));
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli invalid_arguments_exit_with_code_2 -v`  
Expected: PASS (clap handles invalid args and exits with code `2`).

**Step 5: Commit**

```bash
git add papyrus-cli/src/main.rs
git commit -m "feat(cli): replace stub main with clap-driven entrypoint"
```

### Task 10: Add End-to-End CLI Integration Tests for All Required Modes

**Files:**
- Create: `papyrus-cli/tests/cli_integration.rs`
- Modify: `papyrus-cli/src/convert.rs` (if needed for stdout behavior when `-o` absent)
- Modify: `papyrus-cli/src/run.rs` (if needed to print markdown to stdout in file mode)
- Test: `papyrus-cli/tests/cli_integration.rs`

**Step 1: Write the failing test**

```rust
// papyrus-cli/tests/cli_integration.rs
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn single_file_to_output_file() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("out.md");

    Command::cargo_bin("papyrus")
        .unwrap()
        .args(["convert", fixture_path("simple.pdf").to_str().unwrap(), "-o", out.to_str().unwrap()])
        .assert()
        .success();

    let markdown = fs::read_to_string(out).unwrap();
    assert!(markdown.contains("Chapter 1"));
}

#[test]
fn stdout_mode_without_output_flag() {
    Command::cargo_bin("papyrus")
        .unwrap()
        .args(["convert", fixture_path("simple.pdf").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Chapter 1"));
}

#[test]
fn batch_mode_writes_multiple_files() {
    let input = tempdir().unwrap();
    let output = tempdir().unwrap();

    fs::copy(fixture_path("simple.pdf"), input.path().join("simple.pdf")).unwrap();
    fs::copy(fixture_path("multi-page.pdf"), input.path().join("multi-page.pdf")).unwrap();

    Command::cargo_bin("papyrus")
        .unwrap()
        .args(["convert", input.path().to_str().unwrap(), "-o", output.path().to_str().unwrap()])
        .assert()
        .success();

    assert!(output.path().join("simple.md").exists());
    assert!(output.path().join("multi-page.md").exists());
}

#[test]
fn invalid_input_returns_exit_code_1() {
    Command::cargo_bin("papyrus")
        .unwrap()
        .args(["convert", "tests/fixtures/does-not-exist.pdf"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("error:"));
}

#[test]
fn custom_flags_change_output() {
    let default = Command::cargo_bin("papyrus")
        .unwrap()
        .args(["convert", fixture_path("bold-italic.pdf").to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let no_bold = Command::cargo_bin("papyrus")
        .unwrap()
        .args(["convert", fixture_path("bold-italic.pdf").to_str().unwrap(), "--no-bold", "--heading-ratio", "2.0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_ne!(default, no_bold);
}

#[test]
fn pipe_mode_reads_stdin_and_writes_stdout() {
    let bytes = fs::read(fixture_path("simple.pdf")).unwrap();
    Command::cargo_bin("papyrus")
        .unwrap()
        .args(["convert", "-"])
        .write_stdin(bytes)
        .assert()
        .success()
        .stdout(predicate::str::contains("Chapter 1"));
}

#[test]
fn warning_output_visible_and_quiet_suppresses() {
    Command::cargo_bin("papyrus")
        .unwrap()
        .args(["convert", fixture_path("corrupted.pdf").to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning:"));

    Command::cargo_bin("papyrus")
        .unwrap()
        .args(["convert", fixture_path("corrupted.pdf").to_str().unwrap(), "--quiet"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning:").not());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli --test cli_integration -v`  
Expected: FAIL on missing stdout behavior and/or missing runner pieces until all Phase 5 wiring is complete.

**Step 3: Write minimal implementation**

```rust
// papyrus-cli/src/run.rs (important delta for stdout mode)
// inside InputKind::File branch:
match convert_file(&path, output.as_deref(), cfg) {
    Ok(summary) => {
        if output.is_none() {
            let bytes = std::fs::read(&path).unwrap_or_default();
            let markdown = papyrus_core::Papyrus::builder()
                .heading_size_ratio(cfg.heading_ratio)
                .detect_bold(cfg.detect_bold)
                .detect_italic(cfg.detect_italic)
                .build()
                .extract(&bytes)
                .to_markdown();
            println!("{markdown}");
        }
        for line in render_warning_lines(&summary.warnings, cfg.quiet) {
            eprintln!("{line}");
        }
        0
    }
    Err(err) => {
        eprintln!("error: {err}");
        1
    }
}
```

```rust
// papyrus-cli/src/convert.rs (alternative cleaner refactor for file mode)
pub fn convert_file_to_markdown(input: &Path, cfg: ConvertConfig) -> io::Result<(String, Vec<papyrus_core::ast::Warning>)> {
    let bytes = std::fs::read(input)?;
    let result = build_engine(cfg).extract(&bytes);
    Ok((result.to_markdown(), result.warnings))
}
```

```rust
// then in run.rs use convert_file_to_markdown to avoid duplicate extraction:
let (markdown, warnings) = convert_file_to_markdown(&path, cfg)?;
if let Some(out_path) = output.as_deref() {
    std::fs::write(out_path, &markdown)?;
} else {
    print!("{markdown}");
}
for line in render_warning_lines(&warnings, cfg.quiet) {
    eprintln!("{line}");
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli --test cli_integration -v`  
Expected: PASS for all required scenarios (single file, stdout, batch, invalid input, custom flags, pipe mode, warnings/quiet).

**Step 5: Commit**

```bash
git add papyrus-cli/tests/cli_integration.rs papyrus-cli/src/convert.rs papyrus-cli/src/run.rs
git commit -m "test(cli): cover phase-5 modes and finalize runtime behavior"
```

### Task 11: Final Verification Gate

**Files:**
- Modify: `docs/plans/phase-5-verification-log.md` (create if missing)

**Step 1: Write the failing check (definition of done script)**

```bash
#!/usr/bin/env bash
set -euo pipefail

cargo test -p papyrus-cli -v
cargo test -p papyrus-core -v
cargo build --release -p papyrus-cli
```

**Step 2: Run verification**

Run: `cargo test -p papyrus-cli -v && cargo test -p papyrus-core -v && cargo build --release -p papyrus-cli`  
Expected: all commands succeed; release binary produced at `target/release/papyrus`.

**Step 3: Record verification log**

```markdown
# Phase 5 Verification Log

- [ ] `cargo test -p papyrus-cli -v`
- [ ] `cargo test -p papyrus-core -v`
- [ ] `cargo build --release -p papyrus-cli`
- [ ] Manual smoke: `target/release/papyrus convert tests/fixtures/simple.pdf`
- [ ] Manual pipe smoke: `cat tests/fixtures/simple.pdf | target/release/papyrus convert -`
```

**Step 4: Commit verification artifacts**

```bash
git add docs/plans/phase-5-verification-log.md
git commit -m "docs: add phase-5 verification log checklist"
```

**Step 5: Handoff**

```bash
echo "Phase 5 implementation complete. Ready for review/merge."
```
