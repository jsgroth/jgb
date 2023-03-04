mod config;
mod cpu;
mod initialization;
mod lcd;
mod memory;

use crate::cpu::CpuRegisters;
use crate::memory::AddressSpace;
use std::error::Error;

pub use config::{PersistentConfig, RunConfig};

pub struct EmulationState {
    address_space: AddressSpace,
    cpu_registers: CpuRegisters,
}

pub fn run(
    persistent_config: PersistentConfig,
    run_config: RunConfig,
) -> Result<(), Box<dyn Error>> {
    let emulation_state = initialization::initialize(persistent_config, run_config)?;

    cpu::run(emulation_state)?;

    Ok(())
}
