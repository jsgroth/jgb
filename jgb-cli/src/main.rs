use clap::Parser;
use jgb_core::{PersistentConfig, RunConfig};
use std::error::Error;

#[derive(Parser)]
struct Cli {
    #[arg(short = 'f', long = "gb-file-path")]
    gb_file_path: String,
    #[arg(short = 'a', long = "audio-enabled", default_value_t = false)]
    audio_enabled: bool,
    #[arg(long = "no-audio-sync", default_value_t = true, action = clap::ArgAction::SetFalse)]
    sync_to_audio: bool,
    #[arg(long = "no-vsync", default_value_t = true, action = clap::ArgAction::SetFalse)]
    vsync_enabled: bool,
    #[arg(short = 'w', long = "window-width", default_value_t = 640)]
    window_width: u32,
    #[arg(short = 'l', long = "window-height", default_value_t = 576)]
    window_height: u32,
    #[arg(long = "audio-debugging-enabled", default_value_t = false)]
    audio_debugging_enabled: bool,
    #[arg(long = "no-audio-60hz", default_value_t = true, action = clap::ArgAction::SetFalse)]
    audio_60hz: bool,
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
        audio_debugging_enabled: args.audio_debugging_enabled,
        audio_60hz: args.audio_60hz,
    };

    jgb_core::run(persistent_config, run_config)
}
