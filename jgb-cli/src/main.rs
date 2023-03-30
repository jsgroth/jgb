use clap::Parser;
use jgb_core::{PersistentConfig, RunConfig};
use std::error::Error;

#[derive(Parser)]
struct Cli {
    /// Path to ROM file
    #[arg(short = 'f', long = "gb-file-path")]
    gb_file_path: String,

    /// Enable audio
    #[arg(short = 'a', long = "audio-enabled", default_value_t = false)]
    audio_enabled: bool,

    /// Disable audio sync; can reduce video choppiness but may cause audio skips
    #[arg(long = "no-audio-sync", default_value_t = true, action = clap::ArgAction::SetFalse)]
    sync_to_audio: bool,

    /// Disable VSync; can cause choppy video or screen tearing
    #[arg(long = "no-vsync", default_value_t = true, action = clap::ArgAction::SetFalse)]
    vsync_enabled: bool,

    /// Display window width
    #[arg(short = 'w', long = "window-width", default_value_t = 640)]
    window_width: u32,

    /// Display window height
    #[arg(short = 'l', long = "window-height", default_value_t = 576)]
    window_height: u32,

    /// Turn on audio debugging; samples will be written to raw PCM files in the current working
    /// directory (signed 16-bit stereo, 48000Hz)
    #[arg(long = "audio-debugging-enabled", default_value_t = false)]
    audio_debugging_enabled: bool,

    /// Disable hack that samples audio at a slightly higher rate than actual hardware; this is more
    /// accurate but can cause video choppiness when audio sync is enabled
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
