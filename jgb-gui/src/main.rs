#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, rust_2018_idioms)]
// Remove pedantic lints that are very likely to produce false positives or that I disagree with
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::if_not_else,
    clippy::inline_always,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::stable_sort_primitive,
    clippy::struct_excessive_bools,
    clippy::unreadable_literal
)]

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

// Returns Ok(None) if more than one display is connected
fn get_display_resolution() -> Result<Option<Rect>, String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;

    if video.num_video_displays()? > 1 {
        Ok(None)
    } else {
        Ok(Some(video.display_bounds(0)?))
    }
}

// Override winit scale factor to 1 if it looks like we're on a Steam Deck
fn steam_deck_dpi_hack() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;

    let primary_display_name = video.display_name(0)?;
    let (_, primary_display_hdpi, _) = video.display_dpi(0)?;
    let primary_display_bounds = video.display_bounds(0)?;

    log::info!("Primary display name: {primary_display_name}");

    if primary_display_name.as_str() == "ANX7530 U 3\""
        && primary_display_hdpi > 500.0
        && primary_display_bounds.w == 1280
        && primary_display_bounds.h == 800
    {
        log::info!("Assuming running on Steam Deck, overriding scale factor to 1 because otherwise it will default to 4.5");
        env::set_var("WINIT_X11_SCALE_FACTOR", "1");
    }

    Ok(())
}

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    steam_deck_dpi_hack().expect("checking video display information should not fail");

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

    let initial_window_width = 600;
    let initial_window_height = 500;

    // Manually center window because NativeOptions.centered doesn't appear to work on all platforms
    let initial_window_pos = if let Some(display_resolution) = display_resolution {
        log::info!(
            "Read primary display resolution as {}x{}",
            display_resolution.width(),
            display_resolution.height(),
        );

        let initial_window_x = (display_resolution.width() - initial_window_width) as i32 / 2;
        let initial_window_y = (display_resolution.height() - initial_window_height) as i32 / 2;

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
