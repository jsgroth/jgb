use std::fmt::Formatter;

pub struct PersistentConfig {}

pub struct RunConfig {
    pub gb_file_path: String,
    pub audio_enabled: bool,
    pub sync_to_audio: bool,
    pub vsync_enabled: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub audio_debugging_enabled: bool,
    pub audio_60hz: bool,
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

        Ok(())
    }
}
