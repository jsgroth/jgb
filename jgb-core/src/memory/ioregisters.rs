mod lcdc;

use crate::cpu::InterruptType;
use crate::memory::address;
pub use lcdc::{AddressRange, Lcdc, SpriteMode, TileDataRange};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoRegister {
    JOYP,
    SB,
    SC,
    DIV,
    TIMA,
    TMA,
    TAC,
    IF,
    NR10,
    NR11,
    NR12,
    NR13,
    NR14,
    NR21,
    NR22,
    NR23,
    NR24,
    NR30,
    NR31,
    NR32,
    NR33,
    NR34,
    NR41,
    NR42,
    NR43,
    NR44,
    NR50,
    NR51,
    NR52,
    LCDC,
    STAT,
    SCY,
    SCX,
    LY,
    LYC,
    DMA,
    BGP,
    OBP0,
    OBP1,
    WY,
    WX,
}

impl IoRegister {
    /// Return the hardware register corresponding to the given address.
    pub fn from_address(address: u16) -> Option<Self> {
        let register = match address {
            0xFF00 => Self::JOYP,
            0xFF01 => Self::SB,
            0xFF02 => Self::SC,
            0xFF04 => Self::DIV,
            0xFF05 => Self::TIMA,
            0xFF06 => Self::TMA,
            0xFF07 => Self::TAC,
            0xFF0F => Self::IF,
            0xFF10 => Self::NR10,
            0xFF11 => Self::NR11,
            0xFF12 => Self::NR12,
            0xFF13 => Self::NR13,
            0xFF14 => Self::NR14,
            0xFF16 => Self::NR21,
            0xFF17 => Self::NR22,
            0xFF18 => Self::NR23,
            0xFF19 => Self::NR24,
            0xFF1A => Self::NR30,
            0xFF1B => Self::NR31,
            0xFF1C => Self::NR32,
            0xFF1D => Self::NR33,
            0xFF1E => Self::NR34,
            0xFF20 => Self::NR41,
            0xFF21 => Self::NR42,
            0xFF22 => Self::NR43,
            0xFF23 => Self::NR44,
            0xFF24 => Self::NR50,
            0xFF25 => Self::NR51,
            0xFF26 => Self::NR52,
            0xFF40 => Self::LCDC,
            0xFF41 => Self::STAT,
            0xFF42 => Self::SCY,
            0xFF43 => Self::SCX,
            0xFF44 => Self::LY,
            0xFF45 => Self::LYC,
            0xFF46 => Self::DMA,
            0xFF47 => Self::BGP,
            0xFF48 => Self::OBP0,
            0xFF49 => Self::OBP1,
            0xFF4A => Self::WY,
            0xFF4B => Self::WX,
            _ => return None,
        };

        Some(register)
    }

    /// Return the address for this hardware register.
    pub fn to_address(self) -> u16 {
        match self {
            Self::JOYP => 0xFF00,
            Self::SB => 0xFF01,
            Self::SC => 0xFF02,
            Self::DIV => 0xFF04,
            Self::TIMA => 0xFF05,
            Self::TMA => 0xFF06,
            Self::TAC => 0xFF07,
            Self::IF => 0xFF0F,
            Self::NR10 => 0xFF10,
            Self::NR11 => 0xFF11,
            Self::NR12 => 0xFF12,
            Self::NR13 => 0xFF13,
            Self::NR14 => 0xFF14,
            Self::NR21 => 0xFF16,
            Self::NR22 => 0xFF17,
            Self::NR23 => 0xFF18,
            Self::NR24 => 0xFF19,
            Self::NR30 => 0xFF1A,
            Self::NR31 => 0xFF1B,
            Self::NR32 => 0xFF1C,
            Self::NR33 => 0xFF1D,
            Self::NR34 => 0xFF1E,
            Self::NR41 => 0xFF20,
            Self::NR42 => 0xFF21,
            Self::NR43 => 0xFF22,
            Self::NR44 => 0xFF23,
            Self::NR50 => 0xFF24,
            Self::NR51 => 0xFF25,
            Self::NR52 => 0xFF26,
            Self::LCDC => 0xFF40,
            Self::STAT => 0xFF41,
            Self::SCY => 0xFF42,
            Self::SCX => 0xFF43,
            Self::LY => 0xFF44,
            Self::LYC => 0xFF45,
            Self::DMA => 0xFF46,
            Self::BGP => 0xFF47,
            Self::OBP0 => 0xFF48,
            Self::OBP1 => 0xFF49,
            Self::WY => 0xFF4A,
            Self::WX => 0xFF4B,
        }
    }

    /// Return whether or not the CPU is allowed to read this hardware register.
    pub fn is_cpu_readable(self) -> bool {
        !matches!(
            self,
            Self::NR13 | Self::NR23 | Self::NR31 | Self::NR33 | Self::NR41
        )
    }

    /// Return whether or not the CPU is allowed to write to this hardware register.
    pub fn is_cpu_writable(self) -> bool {
        !matches!(self, Self::LY)
    }

    /// Return whether or not this is an audio register.
    pub fn is_audio_register(self) -> bool {
        matches!(
            self,
            Self::NR10
                | Self::NR11
                | Self::NR12
                | Self::NR13
                | Self::NR14
                | Self::NR21
                | Self::NR22
                | Self::NR23
                | Self::NR24
                | Self::NR30
                | Self::NR31
                | Self::NR32
                | Self::NR33
                | Self::NR34
                | Self::NR41
                | Self::NR42
                | Self::NR43
                | Self::NR44
                | Self::NR50
                | Self::NR51
                | Self::NR52
        )
    }
}

/// A convenience view around the IF register.
pub struct InterruptFlags<'a>(&'a mut u8);

impl<'a> InterruptFlags<'a> {
    /// Returns the highest priority requested + enabled interrupt, or None if no enabled interrupts
    /// have been requested.
    pub fn highest_priority_interrupt(&self, ie_value: u8) -> Option<InterruptType> {
        let masked_if = *self.0 & ie_value;
        if masked_if & 0x01 != 0 {
            Some(InterruptType::VBlank)
        } else if masked_if & 0x02 != 0 {
            Some(InterruptType::LcdStatus)
        } else if masked_if & 0x04 != 0 {
            Some(InterruptType::Timer)
        } else if masked_if & 0x10 != 0 {
            Some(InterruptType::Joypad)
        } else {
            None
        }
    }

    #[cfg(test)]
    pub fn get(&self, interrupt_type: InterruptType) -> bool {
        *self.0 & interrupt_type.bit() != 0
    }

    /// Sets the bit for the given interrupt type.
    pub fn set(&mut self, interrupt_type: InterruptType) {
        *self.0 |= interrupt_type.bit();
    }

    /// Clears the bit for the given interrupt type.
    pub fn clear(&mut self, interrupt_type: InterruptType) {
        *self.0 &= !interrupt_type.bit();
    }
}

fn dirty_bit_for_register(io_register: IoRegister) -> Option<u16> {
    let bit = match io_register {
        IoRegister::NR11 => 1 << 0,
        IoRegister::NR13 => 1 << 1,
        IoRegister::NR14 => 1 << 2,
        IoRegister::NR21 => 1 << 3,
        IoRegister::NR23 => 1 << 4,
        IoRegister::NR24 => 1 << 5,
        IoRegister::NR31 => 1 << 6,
        IoRegister::NR33 => 1 << 7,
        IoRegister::NR34 => 1 << 8,
        IoRegister::NR41 => 1 << 9,
        IoRegister::NR44 => 1 << 10,
        IoRegister::DMA => 1 << 11,
        _ => return None,
    };

    Some(bit)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoRegisters {
    contents: [u8; 0x80],
    dirty_bits: u16,
}

impl IoRegisters {
    const JOYP_RELATIVE_ADDR: usize = 0x00;
    const DIV_RELATIVE_ADDR: usize = 0x04;
    const IF_RELATIVE_ADDR: usize = 0x0F;
    const NR52_RELATIVE_ADDR: usize = 0x26;
    const LCDC_RELATIVE_ADDR: usize = 0x40;
    const STAT_RELATIVE_ADDR: usize = 0x41;
    const LY_RELATIVE_ADDR: usize = 0x44;

    pub fn new() -> Self {
        let mut contents = [0; 0x80];

        // JOYP
        contents[0x00] = 0xCF;

        // DIV
        contents[0x04] = 0x18;

        // TAC
        contents[0x07] = 0xF8;

        // IF
        contents[0x0F] = 0xE1;

        // LCDC
        contents[0x40] = 0x91;

        // STAT
        contents[0x41] = 0x81;

        // LY
        contents[0x44] = 0x91;

        // DMA
        contents[0x46] = 0xFF;

        // BGP
        contents[0x47] = 0xFC;

        Self {
            contents,
            dirty_bits: 0x00,
        }
    }

    /// Read the value from the hardware register at the given address. Returns 0xFF if the address
    /// is invalid or the register is not readable by the CPU.
    pub fn read_address(&self, address: u16) -> u8 {
        if is_waveform_address(address) {
            return self.contents[(address - address::IO_REGISTERS_START) as usize];
        }

        let Some(register) = IoRegister::from_address(address) else { return 0xFF; };

        if !register.is_cpu_readable() {
            return 0xFF;
        }

        let byte = self.contents[(address - address::IO_REGISTERS_START) as usize];
        match register {
            IoRegister::JOYP => (byte & 0x0F) | 0xC0,
            IoRegister::STAT | IoRegister::NR10 => byte | 0x80,
            IoRegister::NR11 | IoRegister::NR21 => byte | 0x3F,
            IoRegister::NR30 => byte | 0x7F,
            IoRegister::NR32 => byte | 0x9F,
            IoRegister::NR14 | IoRegister::NR24 | IoRegister::NR34 | IoRegister::NR44 => {
                byte | 0xBF
            }
            IoRegister::NR52 => byte | 0x70,
            _ => byte,
        }
    }

    /// Assign a value to the hardware register at the given address. Does nothing if the address
    /// is invalid or the register is not writable by the CPU.
    pub fn write_address(&mut self, address: u16, value: u8) {
        if is_waveform_address(address) {
            self.contents[(address - address::IO_REGISTERS_START) as usize] = value;
            return;
        }

        let Some(register) = IoRegister::from_address(address) else { return; };

        if !register.is_cpu_writable() {
            return;
        }

        // Audio registers other than NR52 are not writable while the APU is disabled
        let apu_enabled = self.contents[Self::NR52_RELATIVE_ADDR] & 0x80 != 0;
        if !apu_enabled && register.is_audio_register() && register != IoRegister::NR52 {
            return;
        }

        if let Some(bit) = dirty_bit_for_register(register) {
            self.dirty_bits |= bit;
        }

        let relative_addr = (address - 0xFF00) as usize;
        match register {
            IoRegister::DIV => {
                // All writes to DIV reset the value to 0
                self.contents[relative_addr] = 0x00;
            }
            IoRegister::JOYP => {
                let existing_value = self.contents[relative_addr];
                let new_value = existing_value & (value | 0xCF);
                let new_value = new_value | (value & 0x30);
                self.contents[relative_addr] = new_value;
            }
            IoRegister::STAT => {
                let existing_value = self.contents[relative_addr];
                let new_value = existing_value & (value | 0x87);
                let new_value = new_value | (value & 0x78);
                self.contents[relative_addr] = new_value;
            }
            IoRegister::NR52 => {
                let existing_value = self.contents[relative_addr];
                self.contents[relative_addr] = (value & 0x80) | (existing_value & 0x0F);
            }
            _ => {
                self.contents[relative_addr] = value;
            }
        }
    }

    /// Read the value from the given hardware register. Returns 0xFF if the register is not
    /// readable by the CPU.
    pub fn read_register(&self, register: IoRegister) -> u8 {
        self.read_address(register.to_address())
    }

    /// Assign a value to the given hardware register. Does nothing if the register is not
    /// writable by the CPU.
    pub fn write_register(&mut self, register: IoRegister, value: u8) {
        self.write_address(register.to_address(), value);
    }

    /// Read the value of the JOYP register, including bits that the CPU cannot read. Intended to
    /// be used in the code that updates the JOYP register based on current inputs.
    pub fn privileged_read_joyp(&self) -> u8 {
        self.contents[Self::JOYP_RELATIVE_ADDR] | 0xC0
    }

    /// Assign a value to the JOYP register, including bits that the CPU cannot write.
    pub fn privileged_set_joyp(&mut self, value: u8) {
        self.contents[Self::JOYP_RELATIVE_ADDR] = value & 0x3F;
    }

    /// Assign a value to the STAT register (LCD status), including bits that the CPU cannot write.
    /// Should only be used by the PPU.
    pub fn privileged_set_stat(&mut self, value: u8) {
        self.contents[Self::STAT_RELATIVE_ADDR] = value & 0x7F;
    }

    /// Assign a value to the LY register (current scanline), which the CPU cannot normally write
    /// to. Should only be used by the PPU.
    pub fn privileged_set_ly(&mut self, value: u8) {
        self.contents[Self::LY_RELATIVE_ADDR] = value;
    }

    /// Assign a value to the DIV register (timer divider), which is normally always reset to 0x00
    /// when the CPU writes to it. Should only be used by the timer code.
    pub fn privileged_set_div(&mut self, value: u8) {
        self.contents[Self::DIV_RELATIVE_ADDR] = value;
    }

    /// Read an audio register from the perspective of the APU, bypassing CPU access checks (both
    /// register-level and bit-level).
    ///
    /// # Panics
    ///
    /// This method will panic if passed a non-audio register.
    pub fn apu_read_register(&mut self, register: IoRegister) -> u8 {
        if !register.is_audio_register() {
            panic!("apu_read_register can only be used to read audio registers, was: {register:?}");
        }

        self.contents[(register.to_address() - address::IO_REGISTERS_START) as usize]
    }

    /// Assign a value to an audio register from the perspective of the APU, bypassing CPU access
    /// checks (both register-level and bit-level).
    ///
    /// # Panics
    ///
    /// This method will panic if passed a non-audio register.
    pub fn apu_write_register(&mut self, register: IoRegister, value: u8) {
        if !register.is_audio_register() {
            panic!(
                "apu_write_register can only be used to write audio registers, was: {register:?}"
            );
        }

        self.contents[(register.to_address() - address::IO_REGISTERS_START) as usize] = value;
    }

    /// Obtain a read-only view around the LCDC register (LCD control).
    pub fn lcdc(&self) -> Lcdc {
        Lcdc(&self.contents[Self::LCDC_RELATIVE_ADDR])
    }

    /// Obtain a read/write view around the IF register (interrupt request flags).
    pub fn interrupt_flags(&mut self) -> InterruptFlags {
        InterruptFlags(&mut self.contents[Self::IF_RELATIVE_ADDR])
    }

    /// Returns whether or not the given register has been written to.
    ///
    /// # Panics
    ///
    /// Dirty bits are only tracked for the DMA register and specific audio registers. This method
    /// will panic if called for a register for which the dirty bit is not tracked.
    pub fn is_register_dirty(&self, register: IoRegister) -> bool {
        let Some(bit) = dirty_bit_for_register(register) else {
            panic!("dirty bit not tracked for register: {register:?}");
        };

        self.dirty_bits & bit != 0
    }

    /// Clears the dirty bit for the given register.
    ///
    /// # Panics
    ///
    /// Dirty bits are only tracked for the DMA register and specific audio registers. This method
    /// will panic if called for a register for which the dirty bit is not tracked.
    pub fn clear_dirty_bit(&mut self, register: IoRegister) {
        let Some(bit) = dirty_bit_for_register(register) else {
            panic!("dirty bit not tracked for register: {register:?}");
        };

        self.dirty_bits &= !bit;
    }
}

fn is_waveform_address(address: u16) -> bool {
    (0xFF30..=0xFF3F).contains(&address)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_io_registers() -> IoRegisters {
        IoRegisters {
            contents: [0; 0x80],
            dirty_bits: 0x00,
        }
    }

    #[test]
    fn joyp_mask() {
        // Bits 6-7 should be unusable and should always read 1
        // Bits 4-5 should be writable only and should always read 0
        // Bits 0-3 should be readable only, writes should be ignored

        let mut registers = empty_io_registers();

        let joyp_address = IoRegister::JOYP.to_address();

        assert_eq!(0xC0, registers.read_address(joyp_address));

        registers.write_address(joyp_address, 0x00);
        assert_eq!(0xC0, registers.read_address(joyp_address));

        registers.write_address(joyp_address, 0x0F);
        assert_eq!(0xC0, registers.read_address(joyp_address));
        assert_eq!(0x00, registers.privileged_read_joyp() & 0x0F);

        registers.write_address(joyp_address, 0x20);
        assert_eq!(0xC0, registers.read_address(joyp_address));
        assert_eq!(0x20, registers.privileged_read_joyp() & 0x30);

        registers.privileged_set_joyp(0x19);
        assert_eq!(0xC9, registers.read_address(joyp_address));
    }

    #[test]
    fn stat_mask() {
        // Bit 7 should be unusable and should always read 1
        // Bits 3-6 should be both readable and writable
        // Bits 0-2 should be readable only, writes should be ignored

        let mut registers = empty_io_registers();

        let stat_address = IoRegister::STAT.to_address();

        assert_eq!(0x80, registers.read_address(stat_address));

        registers.write_address(stat_address, 0x00);
        assert_eq!(0x80, registers.read_address(stat_address));

        registers.write_address(stat_address, 0x07);
        assert_eq!(0x80, registers.read_address(stat_address));

        registers.write_address(stat_address, 0x28);
        assert_eq!(0xA8, registers.read_address(stat_address));

        registers.privileged_set_stat(0x2F);
        assert_eq!(0xAF, registers.read_address(stat_address));
    }

    #[test]
    fn ly() {
        // CPU should be allowed to read LY but not write LY

        let mut registers = empty_io_registers();

        registers.privileged_set_ly(0x57);
        assert_eq!(0x57, registers.read_register(IoRegister::LY));

        registers.write_register(IoRegister::LY, !0x57);
        assert_eq!(0x57, registers.read_register(IoRegister::LY));
    }
}
