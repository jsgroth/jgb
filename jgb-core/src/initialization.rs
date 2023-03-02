use crate::config::{PersistentConfig, RunConfig};
use crate::cpu::registers::CpuRegisters;
use crate::memory::{AddressSpace, Cartridge, VRam};
use crate::EmulationState;
use std::error::Error;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StartupError {
    #[error("error reading file at {file_path}: {source}")]
    FileReadError {
        file_path: String,
        #[source]
        source: io::Error,
    },
}

pub fn initialize(
    _: PersistentConfig,
    run_config: RunConfig,
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

    let address_space = AddressSpace::new(cartridge, VRam {});
    let cpu_registers = CpuRegisters::new();

    Ok(EmulationState {
        address_space,
        cpu_registers,
    })
}
