use clap::Parser;
use papyrus_cli::cli::Cli;
use papyrus_cli::run::run_cli;

fn main() {
    let cli = Cli::parse();
    std::process::exit(run_cli(cli));
}
