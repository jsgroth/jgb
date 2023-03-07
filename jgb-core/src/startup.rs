use crate::config::{PersistentConfig, RunConfig};
use crate::cpu::CpuRegisters;
use crate::memory::{AddressSpace, Cartridge, CartridgeLoadError};
use crate::EmulationState;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Canvas, Texture, TextureCreator, TextureValueError, WindowCanvas};
use sdl2::video::{Window, WindowBuildError, WindowContext};
use sdl2::{EventPump, GameControllerSubsystem, IntegerOrSdlError, Sdl, VideoSubsystem};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StartupError {
    #[error("error loading cartridge from {file_path}: {source}")]
    FileReadError {
        file_path: String,
        #[source]
        source: CartridgeLoadError,
    },
    #[error("unable to get file name from path: {file_path}")]
    FileNameError { file_path: String },
    #[error("SDL2 error: {sdl_error}")]
    GenericSdlError { sdl_error: String },
    #[error("error building SDL2 window: {source}")]
    SdlWindowBuildError {
        #[from]
        source: WindowBuildError,
    },
    #[error("error building SDL2 canvas: {source}")]
    SdlCanvasBuildError {
        #[from]
        source: IntegerOrSdlError,
    },
    #[error("error creating SDL2 window texture: {source}")]
    SdlTextureValueError {
        #[from]
        source: TextureValueError,
    },
}

impl From<String> for StartupError {
    fn from(value: String) -> Self {
        Self::GenericSdlError { sdl_error: value }
    }
}

pub struct SdlState {
    pub sdl: Sdl,
    pub video: VideoSubsystem,
    pub game_controller: GameControllerSubsystem,
    pub canvas: WindowCanvas,
    pub event_pump: EventPump,
}

pub fn init_emulation_state(
    _: &PersistentConfig,
    run_config: &RunConfig,
) -> Result<EmulationState, StartupError> {
    let cartridge = match Cartridge::from_file(&run_config.gb_file_path) {
        Ok(cartridge) => cartridge,
        Err(err) => {
            return Err(StartupError::FileReadError {
                file_path: run_config.gb_file_path.clone(),
                source: err,
            })
        }
    };

    let address_space = AddressSpace::new(cartridge);
    let cpu_registers = CpuRegisters::new();

    Ok(EmulationState {
        address_space,
        cpu_registers,
    })
}

pub fn init_sdl_state(
    _: &PersistentConfig,
    run_config: &RunConfig,
) -> Result<SdlState, StartupError> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let game_controller = sdl.game_controller()?;

    let window_title = get_window_title(&run_config.gb_file_path)?;
    let window = video
        .window(
            &window_title,
            run_config.window_width,
            run_config.window_height,
        )
        .build()?;

    let mut canvas = window.into_canvas().present_vsync().build()?;
    canvas.clear();
    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.present();

    let event_pump = sdl.event_pump()?;

    Ok(SdlState {
        sdl,
        video,
        game_controller,
        canvas,
        event_pump,
    })
}

fn get_window_title(gb_file_path: &str) -> Result<String, StartupError> {
    let file_name = Path::new(gb_file_path)
        .file_name()
        .map(|s| s.to_str())
        .flatten();
    match file_name {
        Some(file_name) => Ok(format!("jgb - {file_name}")),
        None => Err(StartupError::FileNameError {
            file_path: gb_file_path.into(),
        }),
    }
}
