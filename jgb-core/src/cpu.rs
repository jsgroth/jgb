pub(crate) mod instructions;
mod registers;

#[cfg(test)]
mod tests;

use crate::memory::ioregisters::IoRegister;
use crate::memory::AddressSpace;
use crate::ppu::PpuState;
pub use registers::CpuRegisters;

pub const ISR_CYCLES_REQUIRED: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptType {
    VBlank,
    LcdStatus,
    Timer,
    Joypad,
    // serial not implemented
}

impl InterruptType {
    pub fn handler_address(self) -> u16 {
        match self {
            Self::VBlank => 0x0040,
            Self::LcdStatus => 0x0048,
            Self::Timer => 0x0050,
            Self::Joypad => 0x0060,
        }
    }

    pub fn bit(self) -> u8 {
        match self {
            Self::VBlank => 0x01,
            Self::LcdStatus => 0x02,
            Self::Timer => 0x04,
            Self::Joypad => 0x10,
        }
    }
}

pub fn interrupt_triggered(cpu_registers: &CpuRegisters, address_space: &AddressSpace) -> bool {
    let ie_value = address_space.get_ie_register();
    let if_value = address_space
        .get_io_registers()
        .read_register(IoRegister::IF);

    cpu_registers.ime && !cpu_registers.interrupt_delay && (ie_value & if_value != 0)
}

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
        "Interrupt type {interrupt_type:?} triggered, replacing previous PC of {:04X} with {:04x}",
        cpu_registers.pc,
        interrupt_type.handler_address()
    );

    cpu_registers.pc = interrupt_type.handler_address();

    address_space
        .get_io_registers_mut()
        .interrupt_flags()
        .clear(interrupt_type);
    cpu_registers.ime = false;
}
