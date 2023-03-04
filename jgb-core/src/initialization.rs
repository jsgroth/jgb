use crate::config::{PersistentConfig, RunConfig};
use crate::cpu::CpuRegisters;
use crate::memory::{AddressSpace, Cartridge, CartridgeLoadError};
use crate::EmulationState;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StartupError {
    #[error("error loading cartridge from {file_path}: {source}")]
    FileReadError {
        file_path: String,
        #[source]
        source: CartridgeLoadError,
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

    let address_space = AddressSpace::new(cartridge);
    let cpu_registers = CpuRegisters::new();

    Ok(EmulationState {
        address_space,
        cpu_registers,
    })
}
