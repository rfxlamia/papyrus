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
