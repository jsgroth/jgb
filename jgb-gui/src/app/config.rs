use anyhow::Context;
use jgb_core::{HotkeyConfig, InputConfig};
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
    pub input: InputConfig,

    #[serde(default)]
    pub hotkeys: HotkeyConfig,
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
