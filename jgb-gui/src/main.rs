use clap::Parser;
use eframe::NativeOptions;
use egui::{Pos2, Vec2};
use env_logger::Env;
use jgb_gui::{AppConfig, JgbApp};
use sdl2::rect::Rect;
use std::path::PathBuf;
use std::{env, process};

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

fn get_display_resolution() -> Result<Option<Rect>, String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;

    if video.num_video_displays()? > 1 {
        Ok(None)
    } else {
        Ok(Some(video.display_bounds(0)?))
    }
}

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = GuiArgs::parse();

    let config_path = args
        .config_path
        .map_or_else(default_config_path, PathBuf::from);

    log::info!("reading app config from '{}'", config_path.display());

    let app_config = AppConfig::from_toml_file(&config_path).unwrap_or_else(|err| {
        log::warn!(
            "error reading TOML app config from '{}', using default config: {}",
            config_path.display(),
            err
        );
        AppConfig::default()
    });

    let display_resolution = get_display_resolution().unwrap_or_else(|err| {
        log::error!("error retrieving display resolution: {err}");
        process::exit(1);
    });

    // Manually center window because NativeOptions.centered doesn't appear to work on all platforms
    let initial_window_width = 600;
    let initial_window_height = 500;

    let initial_window_pos = if let Some(display_resolution) = display_resolution {
        log::info!(
            "Read primary display resolution as {}x{}",
            display_resolution.width(),
            display_resolution.height(),
        );

        let initial_window_x =
            display_resolution.x() + (display_resolution.width() - initial_window_width) as i32 / 2;
        let initial_window_y = display_resolution.y()
            + (display_resolution.height() - initial_window_height) as i32 / 2;

        Some(Pos2::new(initial_window_x as f32, initial_window_y as f32))
    } else {
        log::info!("System has more than 1 display device, not attempting to center window");
        None
    };

    let options = NativeOptions {
        initial_window_size: Some(Vec2::new(
            initial_window_width as f32,
            initial_window_height as f32,
        )),
        initial_window_pos,
        ..NativeOptions::default()
    };

    let app = JgbApp::new(app_config, config_path);

    eframe::run_native("jgb", options, Box::new(|_cc| Box::new(app)))
}
