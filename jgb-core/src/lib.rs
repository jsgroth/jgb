use crate::cpu::registers::CpuRegisters;
use crate::memory::AddressSpace;
use std::error::Error;

mod config;
mod cpu;
mod initialization;
mod memory;

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

    Ok(())
}
