use clap::Parser;
use env_logger::Env;
use jgb_core::{ControllerConfig, HotkeyConfig, InputConfig, RunConfig};
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

    /// Launch in fullscreen
    #[arg(long = "fullscreen")]
    launch_fullscreen: bool,

    /// Disable borderless fullscreen mode
    #[arg(long = "no-borderless", default_value_t = true, action = clap::ArgAction::SetFalse)]
    borderless_fullscreen: bool,

    /// Disable integer scaling
    #[arg(long = "no-integer-scaling", default_value_t = true, action = clap::ArgAction::SetFalse)]
    force_integer_scaling: bool,

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

    /// Up input key (default Up)
    #[arg(long)]
    input_up: Option<String>,

    /// Down input key (default Down)
    #[arg(long)]
    input_down: Option<String>,

    /// Left input key (default Left)
    #[arg(long)]
    input_left: Option<String>,

    /// Right input key (default Right)
    #[arg(long)]
    input_right: Option<String>,

    /// A input key (default Z)
    #[arg(long)]
    input_a: Option<String>,

    /// B input key (default X)
    #[arg(long)]
    input_b: Option<String>,

    /// Start input key (default Return)
    #[arg(long)]
    input_start: Option<String>,

    /// Select input key (default Right Shift)
    #[arg(long)]
    input_select: Option<String>,

    /// Exit hotkey (default Escape)
    #[arg(long)]
    hotkey_exit: Option<String>,

    /// Fullscreen toggle hotkey (default F9)
    #[arg(long)]
    hotkey_toggle_fullscreen: Option<String>,

    /// Save state hotkey (default F5)
    #[arg(long)]
    hotkey_save_state: Option<String>,

    /// Load state hotkey (default F6)
    #[arg(long)]
    hotkey_load_state: Option<String>,
}

impl CliArgs {
    fn input_config(&self) -> InputConfig {
        let default = InputConfig::default();
        InputConfig {
            up: self.input_up.clone().unwrap_or(default.up),
            down: self.input_down.clone().unwrap_or(default.down),
            left: self.input_left.clone().unwrap_or(default.left),
            right: self.input_right.clone().unwrap_or(default.right),
            a: self.input_a.clone().unwrap_or(default.a),
            b: self.input_b.clone().unwrap_or(default.b),
            start: self.input_start.clone().unwrap_or(default.start),
            select: self.input_select.clone().unwrap_or(default.select),
        }
    }

    fn hotkey_config(&self) -> HotkeyConfig {
        let default = HotkeyConfig::default();
        HotkeyConfig {
            exit: self.hotkey_exit.clone().or(default.exit),
            toggle_fullscreen: self
                .hotkey_toggle_fullscreen
                .clone()
                .or(default.toggle_fullscreen),
            save_state: self.hotkey_save_state.clone().or(default.save_state),
            load_state: self.hotkey_load_state.clone().or(default.load_state),
        }
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = CliArgs::parse();

    let input_config = args.input_config();
    let hotkey_config = args.hotkey_config();
    let controller_config = ControllerConfig::default();

    let run_config = RunConfig {
        gb_file_path: args.gb_file_path,
        audio_enabled: args.audio_enabled,
        sync_to_audio: args.sync_to_audio,
        vsync_enabled: args.vsync_enabled,
        launch_fullscreen: args.launch_fullscreen,
        borderless_fullscreen: args.borderless_fullscreen,
        force_integer_scaling: args.force_integer_scaling,
        window_width: args.window_width,
        window_height: args.window_height,
        audio_debugging_enabled: args.audio_debugging_enabled,
        audio_60hz: args.audio_60hz,
        input_config,
        hotkey_config,
        controller_config,
    };

    if let Err(err) = jgb_core::run(&run_config, Arc::new(Mutex::new(false))) {
        log::error!("emulator terminated with error: {err}");
        return Err(err.into());
    }

    Ok(())
}
