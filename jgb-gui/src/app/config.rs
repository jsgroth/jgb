use anyhow::Context;
use jgb_core::{HotkeyConfig, InputConfig};
use sdl2::keyboard::Keycode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FullscreenMode {
    Exclusive,
    Borderless,
}

impl Default for FullscreenMode {
    fn default() -> Self {
        Self::Borderless
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppInputConfig {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub a: String,
    pub b: String,
    pub start: String,
    pub select: String,
}

impl AppInputConfig {
    pub fn to_input_config(&self) -> InputConfig {
        InputConfig {
            up_keycode: self.up.clone(),
            down_keycode: self.down.clone(),
            left_keycode: self.left.clone(),
            right_keycode: self.right.clone(),
            a_keycode: self.a.clone(),
            b_keycode: self.b.clone(),
            start_keycode: self.start.clone(),
            select_keycode: self.select.clone(),
        }
    }
}

impl Default for AppInputConfig {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppHotkeyConfig {
    pub exit: Option<String>,
    pub toggle_fullscreen: Option<String>,
    pub save_state: Option<String>,
    pub load_state: Option<String>,
}

impl AppHotkeyConfig {
    pub fn to_hotkey_config(&self) -> HotkeyConfig {
        HotkeyConfig {
            exit_keycode: self.exit.clone(),
            toggle_fullscreen_keycode: self.toggle_fullscreen.clone(),
            save_state_keycode: self.save_state.clone(),
            load_state_keycode: self.load_state.clone(),
        }
    }
}

impl Default for AppHotkeyConfig {
    fn default() -> Self {
        Self {
            exit: Some(Keycode::Escape.name()),
            toggle_fullscreen: Some(Keycode::F9.name()),
            save_state: Some(Keycode::F5.name()),
            load_state: Some(Keycode::F6.name()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "true_fn")]
    pub vsync_enabled: bool,

    #[serde(default)]
    pub launch_in_fullscreen: bool,

    #[serde(default)]
    pub fullscreen_mode: FullscreenMode,

    #[serde(default = "true_fn")]
    pub force_integer_scaling: bool,

    #[serde(default = "true_fn")]
    pub audio_enabled: bool,

    #[serde(default = "true_fn")]
    pub audio_sync_enabled: bool,

    #[serde(default = "true_fn")]
    pub audio_60hz_hack_enabled: bool,

    #[serde(default = "default_window_width")]
    pub window_width: u32,

    #[serde(default = "default_window_height")]
    pub window_height: u32,

    pub rom_search_dir: Option<String>,

    #[serde(default)]
    pub input: AppInputConfig,

    #[serde(default)]
    pub hotkeys: AppHotkeyConfig,
}

// #[serde(default)] requires a function
fn true_fn() -> bool {
    true
}

fn default_window_width() -> u32 {
    4 * 160
}

fn default_window_height() -> u32 {
    4 * 144
}

impl Default for AppConfig {
    fn default() -> Self {
        // Hack to ensure that defaults are always identical between Default::default() and the
        // serde defaults
        toml::from_str("").expect("deserializing AppConfig from empty string should always succeed")
    }
}

impl AppConfig {
    pub fn from_toml_file<P>(path: P) -> Result<Self, anyhow::Error>
    where
        P: AsRef<Path>,
    {
        let config_str = fs::read_to_string(path.as_ref()).with_context(|| {
            format!(
                "error reading TOML config file from '{}'",
                path.as_ref().display()
            )
        })?;
        let config: Self = toml::from_str(&config_str).with_context(|| {
            format!(
                "error parsing app config from TOML file at '{}'",
                path.as_ref().display()
            )
        })?;

        Ok(config)
    }

    pub fn save_to_file<P>(&self, path: P) -> Result<(), anyhow::Error>
    where
        P: AsRef<Path>,
    {
        let config_str =
            toml::to_string_pretty(self).context("error serializing config into TOML")?;
        fs::write(path.as_ref(), config_str).with_context(|| {
            format!("error writing app config to '{}'", path.as_ref().display())
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_config_default_does_not_panic() {
        let app_config = AppConfig::default();
        assert!(app_config.vsync_enabled);
    }
}
