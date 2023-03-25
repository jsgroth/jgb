use crate::memory::ioregisters::{IoRegister, IoRegisters};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApuState {
    enabled: bool,
}

impl ApuState {
    pub fn new() -> Self {
        Self { enabled: true }
    }
}

const ALL_AUDIO_REGISTERS: [IoRegister; 21] = [
    IoRegister::NR10,
    IoRegister::NR11,
    IoRegister::NR12,
    IoRegister::NR13,
    IoRegister::NR14,
    IoRegister::NR21,
    IoRegister::NR22,
    IoRegister::NR23,
    IoRegister::NR24,
    IoRegister::NR30,
    IoRegister::NR31,
    IoRegister::NR32,
    IoRegister::NR33,
    IoRegister::NR34,
    IoRegister::NR41,
    IoRegister::NR42,
    IoRegister::NR43,
    IoRegister::NR44,
    IoRegister::NR50,
    IoRegister::NR51,
    IoRegister::NR52,
];

pub fn tick_m_cycle(apu_state: &mut ApuState, io_registers: &mut IoRegisters) {
    let apu_enabled = io_registers.apu_read_register(IoRegister::NR52) & 0x80 != 0;

    // If the APU was just disabled, clear all audio registers
    if apu_state.enabled && !apu_enabled {
        for audio_register in ALL_AUDIO_REGISTERS {
            io_registers.apu_write_register(audio_register, 0x00);
        }
    }
    apu_state.enabled = apu_enabled;

    if !apu_enabled {
        // TODO remove clippy allow once there is more to this method
        #[allow(clippy::needless_return)]
        return;
    }
}
