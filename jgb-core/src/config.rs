use sdl2::keyboard::Keycode;
use std::fmt::Formatter;

#[derive(Debug, Clone)]
pub struct InputConfig {
    pub up_keycode: String,
    pub down_keycode: String,
    pub left_keycode: String,
    pub right_keycode: String,
    pub a_keycode: String,
    pub b_keycode: String,
    pub start_keycode: String,
    pub select_keycode: String,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            up_keycode: Keycode::Up.name(),
            down_keycode: Keycode::Down.name(),
            left_keycode: Keycode::Left.name(),
            right_keycode: Keycode::Right.name(),
            a_keycode: Keycode::Z.name(),
            b_keycode: Keycode::X.name(),
            start_keycode: Keycode::Return.name(),
            select_keycode: Keycode::RShift.name(),
        }
    }
}

impl std::fmt::Display for InputConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Up={}, Down={}, Left={}, Right={}, A={}, B={}, Start={}, Select={}",
            self.up_keycode,
            self.down_keycode,
            self.left_keycode,
            self.right_keycode,
            self.a_keycode,
            self.b_keycode,
            self.start_keycode,
            self.select_keycode
        )
    }
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub gb_file_path: String,
    pub audio_enabled: bool,
    pub sync_to_audio: bool,
    pub vsync_enabled: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub audio_debugging_enabled: bool,
    pub audio_60hz: bool,
    pub input_config: InputConfig,
}

impl std::fmt::Display for RunConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "gb_file_path: {}", self.gb_file_path)?;
        writeln!(f, "audio_enabled: {}", self.audio_enabled)?;
        writeln!(f, "sync_to_audio: {}", self.sync_to_audio)?;
        writeln!(f, "vsync_enabled: {}", self.vsync_enabled)?;
        writeln!(f, "window_width: {}", self.window_width)?;
        writeln!(f, "window_height: {}", self.window_height)?;
        writeln!(
            f,
            "audio_debugging_enabled: {}",
            self.audio_debugging_enabled
        )?;
        writeln!(f, "audio_60hz: {}", self.audio_60hz)?;
        writeln!(f, "input_config: {}", self.input_config)?;

        Ok(())
    }
}
