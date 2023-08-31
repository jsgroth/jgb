use crate::apu::ApuState;
use crate::config::RunConfig;
use crate::cpu::{CpuRegisters, ExecutionMode};
use crate::debug::FileApuDebugSink;
use crate::graphics::GraphicsError;
use crate::input::AccelerometerState;
use crate::memory::{AddressSpace, Cartridge, CartridgeLoadError};
use crate::ppu::PpuState;
use crate::{audio, graphics, HardwareMode};
use sdl2::audio::AudioQueue;
use sdl2::event::EventType;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::ttf::Sdl2TtfContext;
use sdl2::video::{WindowBuildError, WindowContext};
use sdl2::{
    ttf, AudioSubsystem, EventPump, GameControllerSubsystem, JoystickSubsystem, Sdl, VideoSubsystem,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::rc::Rc;
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
    #[error("SDL2 TTF initialization error: {source}")]
    SdlTtfInit {
        #[from]
        source: ttf::InitError,
    },
}

impl From<String> for StartupError {
    fn from(value: String) -> Self {
        Self::GenericSdl { sdl_error: value }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ControllerStates {
    // Set in the MBC5 mapper, read in the main loop to set gamepad rumble
    pub rumble_motor_on: Rc<RefCell<bool>>,
    // Set in the main loop based on input events, read in the MBC7 mapper when latching state
    pub accelerometer_state: Rc<RefCell<AccelerometerState>>,
}

#[derive(Serialize, Deserialize)]
pub struct EmulationState {
    pub execution_mode: ExecutionMode,
    pub address_space: AddressSpace,
    pub cpu_registers: CpuRegisters,
    pub ppu_state: PpuState,
    pub apu_state: ApuState,
    #[serde(skip)]
    pub controller_states: ControllerStates,
}

pub struct SdlState {
    pub sdl: Sdl,
    pub video: VideoSubsystem,
    pub audio: AudioSubsystem,
    pub audio_playback_queue: Option<AudioQueue<f32>>,
    pub joystick_subsystem: JoystickSubsystem,
    pub controller_subsystem: GameControllerSubsystem,
    pub canvas: WindowCanvas,
    pub texture_creator: TextureCreator<WindowContext>,
    pub event_pump: EventPump,
    pub ttf_ctx: Sdl2TtfContext,
}

pub fn init_emulation_state(run_config: &RunConfig) -> Result<EmulationState, StartupError> {
    let controller_states = ControllerStates::default();

    let cartridge = match Cartridge::from_file(&run_config.gb_file_path, controller_states.clone())
    {
        Ok(cartridge) => cartridge,
        Err(err) => {
            return Err(StartupError::FileRead {
                file_path: run_config.gb_file_path.clone(),
                source: err,
            });
        }
    };

    let execution_mode = match run_config.hardware_mode {
        HardwareMode::GameBoy => ExecutionMode::GameBoy,
        HardwareMode::GameBoyColor => {
            if cartridge.supports_cgb_mode() {
                ExecutionMode::GameBoyColor
            } else {
                log::info!(concat!(
                    "GBC hardware mode was specified but cartridge does not support ",
                    "CGB mode enhancements, running in GB mode",
                ));
                ExecutionMode::GameBoy
            }
        }
    };

    let address_space = AddressSpace::new(cartridge, execution_mode);
    let cpu_registers = CpuRegisters::new(execution_mode);
    let ppu_state = PpuState::new(execution_mode);
    let apu_state = if run_config.audio_enabled && run_config.audio_debugging_enabled {
        let debug_sink =
            FileApuDebugSink::new().map_err(|err| StartupError::AudioDebugInit { source: err })?;
        ApuState::new_with_debug_sink(Box::new(debug_sink))
    } else {
        ApuState::new()
    };

    Ok(EmulationState {
        execution_mode,
        address_space,
        cpu_registers,
        ppu_state,
        apu_state,
        controller_states,
    })
}

#[allow(clippy::if_then_some_else_none)]
pub fn init_sdl_state(run_config: &RunConfig) -> Result<SdlState, StartupError> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let audio = sdl.audio()?;
    let joystick_subsystem = sdl.joystick()?;
    let controller_subsystem = sdl.game_controller()?;

    let ttf_ctx = ttf::init()?;

    // Hide mouse cursor
    sdl.mouse().show_cursor(false);

    let window_title = get_window_title(&run_config.gb_file_path)?;
    let window =
        video.window(&window_title, run_config.window_width, run_config.window_height).build()?;

    let canvas = graphics::create_renderer(window, run_config)?;
    let texture_creator = canvas.texture_creator();

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
        joystick_subsystem,
        controller_subsystem,
        canvas,
        texture_creator,
        event_pump,
        ttf_ctx,
    })
}

fn get_window_title(gb_file_path: &str) -> Result<String, StartupError> {
    let file_name = Path::new(gb_file_path).file_name().and_then(OsStr::to_str);
    match file_name {
        Some(file_name) => Ok(format!("jgb - {file_name}")),
        None => Err(StartupError::FileName { file_path: gb_file_path.into() }),
    }
}
