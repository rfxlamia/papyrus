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
