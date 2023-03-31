use anyhow::{Context, Result};
use clap::Parser;
use env_logger::Env;
use jgb_core::{InputConfig, RunConfig};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
struct CliArgs {
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

    /// Path to TOML input config file. Must have top-level keys 'up', 'left', 'down', 'right', 'a',
    /// 'b', 'start', 'select'
    #[arg(long = "input-config")]
    input_config_path: Option<String>,
}

#[derive(Deserialize)]
struct TomlInputConfig {
    up: String,
    down: String,
    left: String,
    right: String,
    a: String,
    b: String,
    start: String,
    select: String,
}

impl TomlInputConfig {
    fn into_input_config(self) -> InputConfig {
        InputConfig {
            up_keycode: self.up,
            down_keycode: self.down,
            left_keycode: self.left,
            right_keycode: self.right,
            a_keycode: self.a,
            b_keycode: self.b,
            start_keycode: self.start,
            select_keycode: self.select,
        }
    }
}

fn parse_input_config(path: &str) -> Result<InputConfig> {
    let config = fs::read_to_string(Path::new(path))
        .with_context(|| format!("failed to read input config from {path}"))?;
    let toml_config: TomlInputConfig = toml::from_str(&config)
        .with_context(|| format!("failed to parse input config from {path}"))?;
    Ok(toml_config.into_input_config())
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = CliArgs::parse();

    let input_config = match args.input_config_path {
        Some(input_config_path) => parse_input_config(&input_config_path)?,
        None => InputConfig::default(),
    };

    let run_config = RunConfig {
        gb_file_path: args.gb_file_path,
        audio_enabled: args.audio_enabled,
        sync_to_audio: args.sync_to_audio,
        vsync_enabled: args.vsync_enabled,
        window_width: args.window_width,
        window_height: args.window_height,
        audio_debugging_enabled: args.audio_debugging_enabled,
        audio_60hz: args.audio_60hz,
        input_config,
    };

    if let Err(err) = jgb_core::run(&run_config, Arc::new(Mutex::new(false))) {
        log::error!("emulator terminated with error: {err}");
        return Err(err.into());
    }

    Ok(())
}
