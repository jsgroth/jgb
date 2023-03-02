use clap::Parser;
use jgb_core::{PersistentConfig, RunConfig};
use std::error::Error;

#[derive(Parser)]
struct Cli {
    #[arg(short = 'f', long = "gb_file_path")]
    gb_file_path: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let persistent_config = PersistentConfig {};
    let run_config = RunConfig {
        gb_file_path: args.gb_file_path,
    };

    jgb_core::run(persistent_config, run_config)
}
