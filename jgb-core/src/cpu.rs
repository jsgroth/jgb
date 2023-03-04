mod instructions;
mod registers;

use crate::EmulationState;
use std::error::Error;

pub use registers::CpuRegisters;

pub fn run(emulation_state: EmulationState) -> Result<(), Box<dyn Error>> {
    let EmulationState {
        mut address_space,
        mut cpu_registers,
    } = emulation_state;

    let mut i = 0;
    loop {
        let old_pc = cpu_registers.pc;
        let (instruction, pc) = instructions::parse_next_instruction(&address_space, old_pc)?;

        log::debug!("Preparing to execute instruction [0x{old_pc:04x}]: {instruction:04x?}");
        log::debug!("Updating PC to 0x{pc:04x}");

        cpu_registers.pc = pc;
        instruction.execute(&mut address_space, &mut cpu_registers)?;

        i += 1;

        if i == 30 {
            return Ok(());
        }
    }
}
