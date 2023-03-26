use crate::config::{PersistentConfig, RunConfig};
use crate::cpu::CpuRegisters;
use crate::memory::{AddressSpace, Cartridge, CartridgeLoadError};
use crate::ppu::PpuState;
use crate::{graphics, EmulationState};
use sdl2::render::WindowCanvas;
use sdl2::video::WindowBuildError;
use sdl2::{
    AudioSubsystem, EventPump, GameControllerSubsystem, IntegerOrSdlError, Sdl, VideoSubsystem,
};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StartupError {
    #[error("error loading cartridge from {file_path}: {source}")]
    FileRead {
        file_path: String,
        #[source]
        source: CartridgeLoadError,
    },
    #[error("unable to get file name from path: {file_path}")]
    FileName { file_path: String },
    #[error("SDL2 error: {sdl_error}")]
    GenericSdl { sdl_error: String },
    #[error("error building SDL2 window: {source}")]
    SdlWindowBuild {
        #[from]
        source: WindowBuildError,
    },
    #[error("error building SDL2 canvas: {source}")]
    SdlCanvasBuild {
        #[from]
        source: IntegerOrSdlError,
    },
}

impl From<String> for StartupError {
    fn from(value: String) -> Self {
        Self::GenericSdl { sdl_error: value }
    }
}

pub struct SdlState {
    pub sdl: Sdl,
    pub video: VideoSubsystem,
    pub audio: AudioSubsystem,
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
            return Err(StartupError::FileRead {
                file_path: run_config.gb_file_path.clone(),
                source: err,
            })
        }
    };

    let address_space = AddressSpace::new(cartridge);
    let cpu_registers = CpuRegisters::new();
    let ppu_state = PpuState::new();

    Ok(EmulationState {
        address_space,
        cpu_registers,
        ppu_state,
    })
}

pub fn init_sdl_state(
    _: &PersistentConfig,
    run_config: &RunConfig,
) -> Result<SdlState, StartupError> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let audio = sdl.audio()?;
    let game_controller = sdl.game_controller()?;

    let window_title = get_window_title(&run_config.gb_file_path)?;
    let window = video
        .window(
            &window_title,
            run_config.window_width,
            run_config.window_height,
        )
        .build()?;

    let canvas = graphics::create_renderer(window)?;

    let event_pump = sdl.event_pump()?;

    Ok(SdlState {
        sdl,
        video,
        audio,
        game_controller,
        canvas,
        event_pump,
    })
}

fn get_window_title(gb_file_path: &str) -> Result<String, StartupError> {
    let file_name = Path::new(gb_file_path).file_name().and_then(|s| s.to_str());
    match file_name {
        Some(file_name) => Ok(format!("jgb - {file_name}")),
        None => Err(StartupError::FileName {
            file_path: gb_file_path.into(),
        }),
    }
}
