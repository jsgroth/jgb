use sdl2::keyboard::Keycode;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputConfig {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub a: String,
    pub b: String,
    pub start: String,
    pub select: String,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            up: Keycode::Up.name(),
            down: Keycode::Down.name(),
            left: Keycode::Left.name(),
            right: Keycode::Right.name(),
            a: Keycode::Z.name(),
            b: Keycode::X.name(),
            start: Keycode::Return.name(),
            select: Keycode::RShift.name(),
        }
    }
}

impl std::fmt::Display for InputConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Up={}, Down={}, Left={}, Right={}, A={}, B={}, Start={}, Select={}",
            self.up, self.down, self.left, self.right, self.a, self.b, self.start, self.select
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub exit: Option<String>,
    pub toggle_fullscreen: Option<String>,
    pub save_state: Option<String>,
    pub load_state: Option<String>,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            exit: Some(Keycode::Escape.name()),
            toggle_fullscreen: Some(Keycode::F9.name()),
            save_state: Some(Keycode::F5.name()),
            load_state: Some(Keycode::F6.name()),
        }
    }
}

impl std::fmt::Display for HotkeyConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Exit={}, ToggleFullscreen={}, SaveState={}, LoadState={}",
            fmt_option(self.exit.as_ref()),
            fmt_option(self.toggle_fullscreen.as_ref()),
            fmt_option(self.save_state.as_ref()),
            fmt_option(self.load_state.as_ref())
        )
    }
}

fn fmt_option<T: std::fmt::Display>(option: Option<&T>) -> String {
    match option {
        Some(value) => format!("{value}"),
        None => "<None>".into(),
    }
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub gb_file_path: String,
    pub audio_enabled: bool,
    pub sync_to_audio: bool,
    pub vsync_enabled: bool,
    pub launch_fullscreen: bool,
    pub borderless_fullscreen: bool,
    pub force_integer_scaling: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub audio_debugging_enabled: bool,
    pub audio_60hz: bool,
    pub input_config: InputConfig,
    pub hotkey_config: HotkeyConfig,
}

impl std::fmt::Display for RunConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "gb_file_path: {}", self.gb_file_path)?;
        writeln!(f, "audio_enabled: {}", self.audio_enabled)?;
        writeln!(f, "sync_to_audio: {}", self.sync_to_audio)?;
        writeln!(f, "vsync_enabled: {}", self.vsync_enabled)?;
        writeln!(f, "launch_fullscreen: {}", self.launch_fullscreen)?;
        writeln!(f, "borderless_fullscreen: {}", self.borderless_fullscreen)?;
        writeln!(f, "force_integer_scaling: {}", self.force_integer_scaling)?;
        writeln!(f, "window_width: {}", self.window_width)?;
        writeln!(f, "window_height: {}", self.window_height)?;
        writeln!(
            f,
            "audio_debugging_enabled: {}",
            self.audio_debugging_enabled
        )?;
        writeln!(f, "audio_60hz: {}", self.audio_60hz)?;
        writeln!(f, "input_config: {}", self.input_config)?;
        writeln!(f, "hotkey_config: {}", self.hotkey_config)?;

        Ok(())
    }
}
