use crate::apu::ApuState;
use crate::config::RunConfig;
use crate::cpu::CpuRegisters;
use crate::debug::FileApuDebugSink;
use crate::graphics::GraphicsError;
use crate::memory::{AddressSpace, Cartridge, CartridgeLoadError};
use crate::ppu::PpuState;
use crate::{audio, graphics};
use sdl2::audio::AudioQueue;
use sdl2::event::EventType;
use sdl2::render::WindowCanvas;
use sdl2::video::WindowBuildError;
use sdl2::{AudioSubsystem, EventPump, GameControllerSubsystem, Sdl, VideoSubsystem};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::io;
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
    #[error("error initializing audio debugging sink: {source}")]
    AudioDebugInit {
        #[source]
        source: io::Error,
    },
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
        source: GraphicsError,
    },
    #[error("SDL2 audio initialization error: {msg}")]
    SdlAudioInit { msg: String },
}

impl From<String> for StartupError {
    fn from(value: String) -> Self {
        Self::GenericSdl { sdl_error: value }
    }
}

#[derive(Serialize, Deserialize)]
pub struct EmulationState {
    pub address_space: AddressSpace,
    pub cpu_registers: CpuRegisters,
    pub ppu_state: PpuState,
    pub apu_state: ApuState,
}

pub struct SdlState {
    pub sdl: Sdl,
    pub video: VideoSubsystem,
    pub audio: AudioSubsystem,
    pub audio_playback_queue: Option<AudioQueue<i16>>,
    pub game_controller: GameControllerSubsystem,
    pub canvas: WindowCanvas,
    pub event_pump: EventPump,
}

pub fn init_emulation_state(run_config: &RunConfig) -> Result<EmulationState, StartupError> {
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
    let apu_state = if run_config.audio_enabled && run_config.audio_debugging_enabled {
        let debug_sink =
            FileApuDebugSink::new().map_err(|err| StartupError::AudioDebugInit { source: err })?;
        ApuState::new_with_debug_sink(Box::new(debug_sink))
    } else {
        ApuState::new()
    };

    Ok(EmulationState {
        address_space,
        cpu_registers,
        ppu_state,
        apu_state,
    })
}

pub fn init_sdl_state(run_config: &RunConfig) -> Result<SdlState, StartupError> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let audio = sdl.audio()?;
    let game_controller = sdl.game_controller()?;

    // Hide mouse cursor
    sdl.mouse().show_cursor(false);

    let window_title = get_window_title(&run_config.gb_file_path)?;
    let window = video
        .window(
            &window_title,
            run_config.window_width,
            run_config.window_height,
        )
        .build()?;

    let canvas = graphics::create_renderer(window, run_config)?;

    let mut event_pump = sdl.event_pump()?;

    // Disable extremely frequent events that are not used
    event_pump.disable_event(EventType::MouseMotion);

    let audio_playback_queue = if run_config.audio_enabled {
        let audio_playback_queue =
            audio::initialize(&audio).map_err(|msg| StartupError::SdlAudioInit { msg })?;
        Some(audio_playback_queue)
    } else {
        None
    };

    Ok(SdlState {
        sdl,
        video,
        audio,
        audio_playback_queue,
        game_controller,
        canvas,
        event_pump,
    })
}

fn get_window_title(gb_file_path: &str) -> Result<String, StartupError> {
    let file_name = Path::new(gb_file_path).file_name().and_then(OsStr::to_str);
    match file_name {
        Some(file_name) => Ok(format!("jgb - {file_name}")),
        None => Err(StartupError::FileName {
            file_path: gb_file_path.into(),
        }),
    }
}
