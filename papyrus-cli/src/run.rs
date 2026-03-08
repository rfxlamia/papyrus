use crate::cli::{Cli, Commands};
use crate::convert::{classify_input, convert_directory, convert_file, convert_pipe, ConvertConfig, InputKind};
use crate::warning::render_warning_lines;
use std::io::{self, Write};

/// Exit code returned for fatal I/O errors.
const FATAL_IO_EXIT_CODE: i32 = 1;

/// Executes the CLI command and returns the appropriate exit code.
///
/// # Arguments
/// * `cli` - Parsed command-line arguments
///
/// # Returns
/// Exit code: 0 for success, 1 for I/O errors, 2 for invalid arguments (handled by clap).
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
            match classify_input(&input) {
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
                            FATAL_IO_EXIT_CODE
                        }
                    }
                }
                Ok(InputKind::File(path)) => {
                    // When no output file specified, write to stdout
                    if output.is_none() {
                        match std::fs::File::open(&path) {
                            Ok(mut file) => {
                                let mut stdout = io::stdout().lock();
                                match convert_pipe(&mut file, &mut stdout, cfg) {
                                    Ok(summary) => {
                                        for line in render_warning_lines(&summary.warnings, cfg.quiet) {
                                            eprintln!("{line}");
                                        }
                                        0
                                    }
                                    Err(err) => {
                                        let _ = writeln!(io::stderr(), "error: {err}");
                                        FATAL_IO_EXIT_CODE
                                    }
                                }
                            }
                            Err(err) => {
                                let _ = writeln!(io::stderr(), "error: {err}");
                                FATAL_IO_EXIT_CODE
                            }
                        }
                    } else {
                        match convert_file(&path, output.as_deref(), cfg) {
                            Ok(summary) => {
                                for line in render_warning_lines(&summary.warnings, cfg.quiet) {
                                    eprintln!("{line}");
                                }
                                0
                            }
                            Err(err) => {
                                let _ = writeln!(io::stderr(), "error: {err}");
                                FATAL_IO_EXIT_CODE
                            }
                        }
                    }
                }
                Ok(InputKind::Directory(path)) => {
                    match convert_directory(&path, output.as_deref(), cfg) {
                        Ok(summary) => {
                            for (file, warning) in &summary.warnings {
                                for line in render_warning_lines(std::slice::from_ref(warning), cfg.quiet) {
                                    eprintln!("{}: {}", file.display(), line);
                                }
                            }
                            summary.exit_code()
                        }
                        Err(err) => {
                            let _ = writeln!(io::stderr(), "error: {err}");
                            FATAL_IO_EXIT_CODE
                        }
                    }
                }
                Err(err) => {
                    let _ = writeln!(io::stderr(), "error: {err}");
                    FATAL_IO_EXIT_CODE
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FATAL_IO_EXIT_CODE;
    
    #[test]
    fn fatal_io_uses_exit_code_1() {
        assert_eq!(FATAL_IO_EXIT_CODE, 1);
    }
}
