#![forbid(unsafe_code)]

use anyhow::Context;
use clap::Parser;
use env_logger::Env;
use jgb_core::{
    ControllerConfig, ControllerInput, GbColorScheme, GbcColorCorrection, HardwareMode,
    HotkeyConfig, InputConfig, RunConfig,
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Parser)]
struct CliArgs {
    /// Path to ROM file
    #[arg(short = 'f', long = "gb-file-path")]
    gb_file_path: String,

    /// Hardware mode (GameBoy/GameBoyColor)
    #[arg(long = "hardware-mode", default_value_t)]
    hardware_mode: HardwareMode,

    /// Disable audio
    #[arg(long = "no-audio", default_value_t = true)]
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
    /// directory (32-bit floating point, stereo, 48000Hz)
    #[arg(long = "audio-debugging-enabled", default_value_t)]
    audio_debugging_enabled: bool,

    /// Enable hack that samples audio at a slightly higher rate than actual hardware; this is less
    /// accurate but can reduce video choppiness when audio sync is enabled
    #[arg(long = "audio-60hz", default_value_t)]
    audio_60hz: bool,

    /// GB color palette (BlackAndWhite / GreenTint / LimeGreen)
    #[arg(long = "color-scheme", default_value_t)]
    color_scheme: GbColorScheme,

    /// GBC color correction mode (None / GbcLcd)
    #[arg(long, default_value_t)]
    gbc_color_correction: GbcColorCorrection,

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

    /// Fast forward hotkey (default Tab)
    #[arg(long)]
    hotkey_fast_forward: Option<String>,

    /// Up controller input ("button N" / "axis N +" / "axis N -")
    #[arg(long)]
    controller_up: Option<String>,

    /// Down controller input ("button N" / "axis N +" / "axis N -")
    #[arg(long)]
    controller_down: Option<String>,

    /// Left controller input ("button N" / "axis N +" / "axis N -")
    #[arg(long)]
    controller_left: Option<String>,

    /// Right controller input ("button N" / "axis N +" / "axis N -")
    #[arg(long)]
    controller_right: Option<String>,

    /// A controller input ("button N" / "axis N +" / "axis N -")
    #[arg(long)]
    controller_a: Option<String>,

    /// B controller input ("button N" / "axis N +" / "axis N -")
    #[arg(long)]
    controller_b: Option<String>,

    /// Start controller input ("button N" / "axis N +" / "axis N -")
    #[arg(long)]
    controller_start: Option<String>,

    /// Select controller input ("button N" / "axis N +" / "axis N -")
    #[arg(long)]
    controller_select: Option<String>,

    /// Controller axis deadzone on a scale of 0 to 32767
    #[arg(long, default_value_t = 5000)]
    controller_deadzone: u16,

    /// Disable controller rumble
    #[arg(long = "no-controller-rumble", default_value_t = true, action = clap::ArgAction::SetFalse)]
    controller_rumble: bool,
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
            toggle_fullscreen: self.hotkey_toggle_fullscreen.clone().or(default.toggle_fullscreen),
            save_state: self.hotkey_save_state.clone().or(default.save_state),
            load_state: self.hotkey_load_state.clone().or(default.load_state),
            fast_forward: self.hotkey_fast_forward.clone().or(default.fast_forward),
        }
    }

    fn controller_config(&self) -> Result<ControllerConfig, anyhow::Error> {
        let default = ControllerConfig::default();
        let config = ControllerConfig {
            up: parse_controller_input(self.controller_up.as_ref())?.or(default.up),
            down: parse_controller_input(self.controller_down.as_ref())?.or(default.down),
            left: parse_controller_input(self.controller_left.as_ref())?.or(default.left),
            right: parse_controller_input(self.controller_right.as_ref())?.or(default.right),
            a: parse_controller_input(self.controller_a.as_ref())?.or(default.a),
            b: parse_controller_input(self.controller_b.as_ref())?.or(default.b),
            start: parse_controller_input(self.controller_start.as_ref())?.or(default.start),
            select: parse_controller_input(self.controller_select.as_ref())?.or(default.select),
            axis_deadzone: self.controller_deadzone,
            rumble_enabled: self.controller_rumble,
        };
        Ok(config)
    }
}

fn parse_controller_input(
    option: Option<&String>,
) -> Result<Option<ControllerInput>, anyhow::Error> {
    let input_option = match option {
        Some(input_str) => Some(
            input_str
                .parse()
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("failed to parse controller input: {input_str}"))?,
        ),
        None => None,
    };
    Ok(input_option)
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = CliArgs::parse();

    let input_config = args.input_config();
    let hotkey_config = args.hotkey_config();
    let controller_config = args.controller_config()?;

    let run_config = RunConfig {
        gb_file_path: args.gb_file_path,
        hardware_mode: args.hardware_mode,
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
        color_scheme: args.color_scheme,
        gbc_color_correction: args.gbc_color_correction,
        input_config,
        hotkey_config,
        controller_config,
    };

    if let Err(err) = jgb_core::run(&run_config, Arc::new(AtomicBool::new(false))) {
        log::error!("emulator terminated with error: {err}");
        return Err(err.into());
    }

    Ok(())
}
