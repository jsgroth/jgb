use clap::Parser;
use jgb_core::{PersistentConfig, RunConfig};
use std::error::Error;

#[derive(Parser)]
struct Cli {
    #[arg(short = 'f', long = "gb_file_path")]
    gb_file_path: String,
    #[arg(short = 'a', long = "audio_enabled", default_value_t = false)]
    audio_enabled: bool,
    #[arg(long = "no-sync-to-audio", default_value_t = true, action = clap::ArgAction::SetFalse)]
    sync_to_audio: bool,
    #[arg(long = "no-vsync", default_value_t = true, action = clap::ArgAction::SetFalse)]
    vsync_enabled: bool,
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
        audio_enabled: args.audio_enabled,
        sync_to_audio: args.sync_to_audio,
        vsync_enabled: args.vsync_enabled,
        window_width: args.window_width,
        window_height: args.window_height,
    };

    jgb_core::run(persistent_config, run_config)
}
