use clap::Parser;
use eframe::NativeOptions;
use env_logger::Env;
use jgb_gui::{AppConfig, JgbApp};
use std::env;
use std::path::PathBuf;

#[derive(Parser)]
struct GuiArgs {
    /// Path to config file; defaults to '<cwd>/jgb-config.toml'
    #[arg(long = "config")]
    config_path: Option<String>,
}

// Panics on error
fn default_config_path() -> PathBuf {
    let cwd = env::current_dir().expect("cannot determine current working directory");
    cwd.join("jgb-config.toml")
}

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = GuiArgs::parse();

    let config_path = args
        .config_path
        .map(PathBuf::from)
        .unwrap_or_else(default_config_path);

    log::info!("reading app config from '{}'", config_path.display());

    let app_config = AppConfig::from_toml_file(&config_path).unwrap_or_else(|err| {
        log::warn!(
            "error reading TOML app config from '{}', using default config: {}",
            config_path.display(),
            err
        );
        AppConfig::default()
    });

    let app = JgbApp::new(app_config, config_path);

    eframe::run_native(
        "jgb",
        NativeOptions::default(),
        Box::new(|_cc| Box::new(app)),
    )
}
