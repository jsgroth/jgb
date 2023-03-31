#![forbid(unsafe_code)]

mod apu;
mod audio;
mod config;
mod cpu;
mod debug;
mod eventloop;
mod graphics;
mod input;
mod memory;
mod ppu;
mod startup;
mod timer;

use crate::cpu::CpuRegisters;
use crate::memory::AddressSpace;
use thiserror::Error;

use crate::apu::ApuState;
use crate::eventloop::RunError;
use crate::ppu::PpuState;
use crate::startup::StartupError;
pub use config::{InputConfig, RunConfig};

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

pub struct EmulationState {
    address_space: AddressSpace,
    cpu_registers: CpuRegisters,
    ppu_state: PpuState,
    apu_state: ApuState,
}

/// Initialize the emulator using the given configs and then run until it terminates.
pub fn run(run_config: &RunConfig) -> Result<(), EmulationError> {
    let emulation_state = startup::init_emulation_state(run_config)?;

    let sdl_state = startup::init_sdl_state(run_config, &emulation_state)?;

    eventloop::run(emulation_state, sdl_state, run_config)?;

    Ok(())
}
