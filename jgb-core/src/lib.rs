#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
// Remove pedantic lints that are very likely to produce false positives or that I disagree with
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::if_not_else)]
#![allow(clippy::inline_always)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::similar_names)]
#![allow(clippy::single_match_else)]
#![allow(clippy::stable_sort_primitive)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::unreadable_literal)]

mod apu;
mod audio;
mod config;
mod cpu;
mod debug;
mod eventloop;
pub mod font;
mod graphics;
mod input;
mod memory;
mod ppu;
mod serialize;
mod startup;
mod timer;

use std::sync::{Arc, Mutex};
use thiserror::Error;

use crate::eventloop::RunError;
use crate::startup::StartupError;
pub use config::{
    ColorScheme, ControllerConfig, ControllerInput, HardwareMode, HatDirection, HotkeyConfig,
    InputConfig, RunConfig,
};

#[derive(Error, Debug)]
pub enum EmulationError {
    #[error("error initializing emulator: {source}")]
    Startup {
        #[from]
        source: StartupError,
    },
    #[error("runtime error: {source}")]
    Runtime {
        #[from]
        source: RunError,
    },
}

/// Initialize the emulator using the given configs and then run until it terminates or until
/// `quit_signal` is set to true.
///
/// # Errors
///
/// This function will return an error if emulation terminates unexpectedly.
pub fn run(run_config: &RunConfig, quit_signal: Arc<Mutex<bool>>) -> Result<(), EmulationError> {
    let emulation_state = startup::init_emulation_state(run_config)?;

    let sdl_state = startup::init_sdl_state(run_config)?;

    eventloop::run(emulation_state, sdl_state, run_config, quit_signal)?;

    Ok(())
}
