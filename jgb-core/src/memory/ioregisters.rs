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
    WAVE0,
    WAVE1,
    WAVE2,
    WAVE3,
    WAVE4,
    WAVE5,
    WAVE6,
    WAVE7,
    WAVE8,
    WAVE9,
    WAVE10,
    WAVE11,
    WAVE12,
    WAVE13,
    WAVE14,
    WAVE15,
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
            0xFF30 => Self::WAVE0,
            0xFF31 => Self::WAVE1,
            0xFF32 => Self::WAVE2,
            0xFF33 => Self::WAVE3,
            0xFF34 => Self::WAVE4,
            0xFF35 => Self::WAVE5,
            0xFF36 => Self::WAVE6,
            0xFF37 => Self::WAVE7,
            0xFF38 => Self::WAVE8,
            0xFF39 => Self::WAVE9,
            0xFF3A => Self::WAVE10,
            0xFF3B => Self::WAVE11,
            0xFF3C => Self::WAVE12,
            0xFF3D => Self::WAVE13,
            0xFF3E => Self::WAVE14,
            0xFF3F => Self::WAVE15,
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
            Self::WAVE0 => 0xFF30,
            Self::WAVE1 => 0xFF31,
            Self::WAVE2 => 0xFF32,
            Self::WAVE3 => 0xFF33,
            Self::WAVE4 => 0xFF34,
            Self::WAVE5 => 0xFF35,
            Self::WAVE6 => 0xFF36,
            Self::WAVE7 => 0xFF37,
            Self::WAVE8 => 0xFF38,
            Self::WAVE9 => 0xFF39,
            Self::WAVE10 => 0xFF3A,
            Self::WAVE11 => 0xFF3B,
            Self::WAVE12 => 0xFF3C,
            Self::WAVE13 => 0xFF3D,
            Self::WAVE14 => 0xFF3E,
            Self::WAVE15 => 0xFF3F,
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

    pub fn is_readable(self) -> bool {
        match self {
            Self::NR13 | Self::NR23 | Self::NR31 | Self::NR33 | Self::NR41 => false,
            _ => true,
        }
    }

    pub fn is_writable(self) -> bool {
        match self {
            Self::LY => false,
            _ => true,
        }
    }
}

#[derive(Debug)]
pub struct IoRegisters {
    contents: [u8; 0x80],
}

impl IoRegisters {
    pub fn new() -> Self {
        Self {
            contents: [0; 0x80],
        }
    }

    pub fn read_address(&self, address: u16) -> u8 {
        let Some(register) = IoRegister::from_address(address) else { return 0xFF; };

        if !register.is_readable() {
            return 0xFF;
        }

        let byte = self.contents[(address - 0xFF00) as usize];
        match register {
            IoRegister::JOYP => (byte & 0x0F) | 0xC0,
            IoRegister::STAT => byte | 0x80,
            _ => byte,
        }
    }

    pub fn write_address(&mut self, address: u16, value: u8) {
        let Some(register) = IoRegister::from_address(address) else { return; };

        if !register.is_writable() {
            return;
        }

        let relative_addr = (address - 0xFF00) as usize;
        match register {
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
            _ => {
                self.contents[relative_addr] = value;
            }
        }
    }
}
