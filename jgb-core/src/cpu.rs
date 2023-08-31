pub(crate) mod instructions;
mod registers;

#[cfg(test)]
mod tests;

use crate::memory::ioregisters::IoRegister;
use crate::memory::AddressSpace;
use crate::ppu::PpuState;
pub use registers::{CgbSpeedMode, CpuRegisters};
use serde::{Deserialize, Serialize};

/// The number of clock cycles required to execute the interrupt service routine.
pub const ISR_CYCLES_REQUIRED: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    GameBoy,
    GameBoyColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptType {
    VBlank,
    LcdStatus,
    Timer,
    Serial,
    Joypad,
}

impl InterruptType {
    /// Return the handler address that the CPU should jump to in the interrupt service routine
    /// when this interrupt type is triggered.
    pub fn handler_address(self) -> u16 {
        match self {
            Self::VBlank => 0x0040,
            Self::LcdStatus => 0x0048,
            Self::Timer => 0x0050,
            Self::Serial => 0x0058,
            Self::Joypad => 0x0060,
        }
    }

    /// Return the bit mask for this interrupt type used in the IE and IF registers.
    pub fn bit(self) -> u8 {
        match self {
            Self::VBlank => 0x01,
            Self::LcdStatus => 0x02,
            Self::Timer => 0x04,
            Self::Serial => 0x08,
            Self::Joypad => 0x10,
        }
    }
}

/// Determine whether an interrupt has triggered.
///
/// An interrupt triggers when the IME flag is set (interrupt master flag), the last instruction was
/// not EI (enable interrupts), and at least one interrupt type is set in both the IE register
/// (enabled interrupts) and the IF register (requested interrupts).
pub fn interrupt_triggered(cpu_registers: &CpuRegisters, address_space: &AddressSpace) -> bool {
    cpu_registers.ime
        && !cpu_registers.interrupt_delay
        && interrupt_triggered_no_ime_check(address_space)
}

pub fn interrupt_triggered_no_ime_check(address_space: &AddressSpace) -> bool {
    let ie_value = address_space.get_ie_register();
    let if_value = address_space.get_io_registers().read_register(IoRegister::IF);

    ie_value & if_value != 0
}

/// Execute the CPU's interrupt service routine.
///
/// The routine disables the IME flag and then functionally executes CALL N where N is the handler
/// address for the highest priority requested & enabled interrupt type. It also un-halts the CPU
/// if it was previously halted.
///
/// # Panics
///
/// This function will panic if there are no interrupt types that are both enabled and requested.
/// It should only be called if [`interrupt_triggered`] returns true.
pub fn execute_interrupt_service_routine(
    cpu_registers: &mut CpuRegisters,
    address_space: &mut AddressSpace,
    ppu_state: &PpuState,
) {
    cpu_registers.sp -= 2;
    address_space.write_address_u16(cpu_registers.sp, cpu_registers.pc, ppu_state);

    let ie_value = address_space.get_ie_register();
    let interrupt_type = address_space
        .get_io_registers_mut()
        .interrupt_flags()
        .highest_priority_interrupt(ie_value)
        .expect("execute_interrupt_service_routine should only be called when an interrupt has triggered");

    log::trace!(
        "Interrupt type {interrupt_type:?} triggered, replacing previous PC of {:04X} with {:04X}",
        cpu_registers.pc,
        interrupt_type.handler_address()
    );

    cpu_registers.pc = interrupt_type.handler_address();

    address_space.get_io_registers_mut().interrupt_flags().clear(interrupt_type);
    cpu_registers.ime = false;
    cpu_registers.halted = false;
}
