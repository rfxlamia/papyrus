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
                    // When no output file specified, write to stdout
                    if output.is_none() {
                        let mut stdout = io::stdout().lock();
                        match convert_pipe(&mut std::fs::File::open(&path).unwrap(), &mut stdout, cfg) {
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
                                fatal_io_exit_code()
                            }
                        }
                    }
                }
                Ok(InputKind::Directory(path)) => {
                    match convert_directory(&path, output.as_deref(), cfg) {
                        Ok(summary) => {
                            for (file, warning) in &summary.warnings {
                                for line in render_warning_lines(&[warning.clone()], cfg.quiet) {
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
