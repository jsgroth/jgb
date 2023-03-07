use clap::Parser;
use jgb_core::{PersistentConfig, RunConfig};
use std::error::Error;

#[derive(Parser)]
struct Cli {
    #[arg(short = 'f', long = "gb_file_path")]
    gb_file_path: String,
    #[arg(short = 'w', long = "window_width", default_value_t = 640)]
    window_width: u32,
    #[arg(short = 'l', long = "window_height", default_value_t = 576)]
    window_height: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args = Cli::parse();

    let persistent_config = PersistentConfig {};
    let run_config = RunConfig {
        gb_file_path: args.gb_file_path,
        window_width: args.window_width,
        window_height: args.window_height,
    };

    jgb_core::run(persistent_config, run_config)
}
