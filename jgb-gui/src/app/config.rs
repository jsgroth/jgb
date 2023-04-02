use anyhow::Context;
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
pub struct AppConfig {
    #[serde(default = "default_vsync_enabled")]
    pub vsync_enabled: bool,

    #[serde(default)]
    pub launch_in_fullscreen: bool,

    #[serde(default)]
    pub fullscreen_mode: FullscreenMode,

    #[serde(default = "default_force_integer_scaling")]
    pub force_integer_scaling: bool,

    #[serde(default = "default_audio_enabled")]
    pub audio_enabled: bool,

    #[serde(default = "default_audio_sync_enabled")]
    pub audio_sync_enabled: bool,

    #[serde(default = "default_audio_60hz_hack_enabled")]
    pub audio_60hz_hack_enabled: bool,

    #[serde(default = "default_window_width")]
    pub window_width: u32,

    #[serde(default = "default_window_height")]
    pub window_height: u32,

    pub rom_search_dir: Option<String>,
}

fn default_vsync_enabled() -> bool {
    true
}

fn default_force_integer_scaling() -> bool {
    true
}

fn default_audio_enabled() -> bool {
    true
}

fn default_audio_sync_enabled() -> bool {
    true
}

fn default_audio_60hz_hack_enabled() -> bool {
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
        Self {
            vsync_enabled: default_vsync_enabled(),
            launch_in_fullscreen: bool::default(),
            fullscreen_mode: FullscreenMode::default(),
            force_integer_scaling: default_force_integer_scaling(),
            audio_enabled: default_audio_enabled(),
            audio_sync_enabled: default_audio_sync_enabled(),
            audio_60hz_hack_enabled: default_audio_60hz_hack_enabled(),
            window_width: default_window_width(),
            window_height: default_window_height(),
            rom_search_dir: Option::default(),
        }
    }
}

impl AppConfig {
    pub fn from_toml_file<P>(path: P) -> Result<Self, anyhow::Error>
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        let config_str = fs::read_to_string(path.as_ref())
            .with_context(|| format!("error reading TOML config file from '{path:?}'"))?;
        let config: Self = toml::from_str(&config_str)
            .with_context(|| format!("error parsing app config from TOML file at '{path:?}'"))?;

        Ok(config)
    }

    pub fn save_to_file<P>(&self, path: P) -> Result<(), anyhow::Error>
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        let config_str =
            toml::to_string_pretty(self).context("error serializing config into TOML")?;
        fs::write(path.as_ref(), config_str)
            .with_context(|| format!("error writing app config to '{path:?}'"))?;

        Ok(())
    }
}
