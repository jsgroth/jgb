use crate::memory::address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuRegister {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

impl CpuRegister {
    /// Parses a CPU register out of bits 3-5 in the given byte.
    ///
    /// Returns None if those bits are 110 which is not a valid register code.
    pub fn from_mid_opcode_bits(bits: u8) -> Option<Self> {
        match bits & 0x38 {
            0x00 => Some(Self::B),
            0x08 => Some(Self::C),
            0x10 => Some(Self::D),
            0x18 => Some(Self::E),
            0x20 => Some(Self::H),
            0x28 => Some(Self::L),
            0x38 => Some(Self::A),
            _ => None,
        }
    }

    /// Parses a CPU register out of bits 0-2 in the given byte.
    ///
    /// Returns None if those bits are 110 which is not a valid register code.
    pub fn from_low_opcode_bits(bits: u8) -> Option<Self> {
        match bits & 0x07 {
            0x00 => Some(Self::B),
            0x01 => Some(Self::C),
            0x02 => Some(Self::D),
            0x03 => Some(Self::E),
            0x04 => Some(Self::H),
            0x05 => Some(Self::L),
            0x07 => Some(Self::A),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuRegisterPair {
    AF,
    BC,
    DE,
    HL,
    SP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZFlag(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NFlag(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HFlag(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CFlag(pub bool);

impl ZFlag {
    const BIT: u8 = 1 << 7;

    fn to_bit(self) -> u8 {
        if self.0 {
            Self::BIT
        } else {
            0
        }
    }
}

impl NFlag {
    const BIT: u8 = 1 << 6;

    fn to_bit(self) -> u8 {
        if self.0 {
            Self::BIT
        } else {
            0
        }
    }
}

impl HFlag {
    const BIT: u8 = 1 << 5;

    fn to_bit(self) -> u8 {
        if self.0 {
            Self::BIT
        } else {
            0
        }
    }
}

impl CFlag {
    const BIT: u8 = 1 << 4;

    fn to_bit(self) -> u8 {
        if self.0 {
            Self::BIT
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuRegisters {
    pub accumulator: u8,
    pub flags: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub ime: bool,
    // Tracks whether the previous instruction was EI
    pub interrupt_delay: bool,
    pub halted: bool,
}

impl CpuRegisters {
    /// Creates a new `CpuRegisters` value with all fields initialized to reasonable values.
    ///
    /// In particular, the program counter is initialized to 0x0100 (entry point in cartridge ROM),
    /// and the stack pointer is initialized to 0xFFFE (last address in HRAM).
    pub fn new() -> Self {
        Self {
            accumulator: 0x01,
            flags: 0x00,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            pc: address::ENTRY_POINT,
            sp: address::HRAM_END,
            ime: false,
            interrupt_delay: false,
            halted: false,
        }
    }

    /// Read the A and F registers together as a 16-bit value.
    pub fn af(&self) -> u16 {
        u16::from_be_bytes([self.accumulator, self.flags])
    }

    /// Read the B and C registers together as a 16-bit value.
    pub fn bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }

    /// Read the D and E registers together as a 16-bit value.
    pub fn de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }

    /// Read the H and L registers together as a 16-bit value.
    pub fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    /// Set the H and L registers together as a 16-bit value. The first byte is assigned to H
    /// and the second byte is assigned to L.
    pub fn set_hl(&mut self, hl: u16) {
        let [h, l] = hl.to_be_bytes();
        self.h = h;
        self.l = l;
    }

    /// Read the value from the given register.
    pub fn read_register(&self, register: CpuRegister) -> u8 {
        match register {
            CpuRegister::A => self.accumulator,
            CpuRegister::B => self.b,
            CpuRegister::C => self.c,
            CpuRegister::D => self.d,
            CpuRegister::E => self.e,
            CpuRegister::H => self.h,
            CpuRegister::L => self.l,
        }
    }

    /// Assign a value to the given register.
    pub fn set_register(&mut self, register: CpuRegister, value: u8) {
        match register {
            CpuRegister::A => {
                self.accumulator = value;
            }
            CpuRegister::B => {
                self.b = value;
            }
            CpuRegister::C => {
                self.c = value;
            }
            CpuRegister::D => {
                self.d = value;
            }
            CpuRegister::E => {
                self.e = value;
            }
            CpuRegister::H => {
                self.h = value;
            }
            CpuRegister::L => {
                self.l = value;
            }
        }
    }

    /// Obtain a mutable reference to the given register's value.
    pub fn get_register_mut(&mut self, register: CpuRegister) -> &mut u8 {
        match register {
            CpuRegister::A => &mut self.accumulator,
            CpuRegister::B => &mut self.b,
            CpuRegister::C => &mut self.c,
            CpuRegister::D => &mut self.d,
            CpuRegister::E => &mut self.e,
            CpuRegister::H => &mut self.h,
            CpuRegister::L => &mut self.l,
        }
    }

    /// Read the given pair of registers as a 16-bit value, except for SP which is a 16-bit
    /// register and is read directly.
    pub fn read_register_pair(&self, register_pair: CpuRegisterPair) -> u16 {
        match register_pair {
            CpuRegisterPair::AF => self.af(),
            CpuRegisterPair::BC => self.bc(),
            CpuRegisterPair::DE => self.de(),
            CpuRegisterPair::HL => self.hl(),
            CpuRegisterPair::SP => self.sp,
        }
    }

    /// Assign a 16-bit value to the given pair of registers, except for SP which is a 16-bit
    /// register and is assigned directly.
    pub fn set_register_pair(&mut self, register_pair: CpuRegisterPair, value: u16) {
        match register_pair {
            CpuRegisterPair::AF => {
                let [a, f] = value.to_be_bytes();
                self.accumulator = a;
                // Lower 4 bits of flags register are unused
                self.flags = f & 0xF0;
            }
            CpuRegisterPair::BC => {
                let [b, c] = value.to_be_bytes();
                self.b = b;
                self.c = c;
            }
            CpuRegisterPair::DE => {
                let [d, e] = value.to_be_bytes();
                self.d = d;
                self.e = e;
            }
            CpuRegisterPair::HL => {
                self.set_hl(value);
            }
            CpuRegisterPair::SP => {
                self.sp = value;
            }
        }
    }

    /// Set all four flags in the F register.
    pub fn set_flags(&mut self, z: ZFlag, n: NFlag, h: HFlag, c: CFlag) {
        self.flags = z.to_bit() | n.to_bit() | h.to_bit() | c.to_bit();
    }

    /// Set any number of flags in the F register.
    ///
    /// Some(true) will set the flag to 1, Some(false) will set the flag to 0, and None will leave
    /// the flag unchanged.
    pub fn set_some_flags(
        &mut self,
        z: Option<ZFlag>,
        n: Option<NFlag>,
        h: Option<HFlag>,
        c: Option<CFlag>,
    ) {
        match z {
            Some(ZFlag(true)) => {
                self.flags |= ZFlag::BIT;
            }
            Some(ZFlag(false)) => {
                self.flags &= !ZFlag::BIT;
            }
            None => {}
        }

        match n {
            Some(NFlag(true)) => {
                self.flags |= NFlag::BIT;
            }
            Some(NFlag(false)) => {
                self.flags &= !NFlag::BIT;
            }
            None => {}
        }

        match h {
            Some(HFlag(true)) => {
                self.flags |= HFlag::BIT;
            }
            Some(HFlag(false)) => {
                self.flags &= !HFlag::BIT;
            }
            None => {}
        }

        match c {
            Some(CFlag(true)) => {
                self.flags |= CFlag::BIT;
            }
            Some(CFlag(false)) => {
                self.flags &= !CFlag::BIT;
            }
            None => {}
        }
    }

    /// Return whether or not the Z flag (last result zero) is currently set in the F register.
    pub fn z_flag(&self) -> bool {
        self.flags & ZFlag::BIT != 0
    }

    /// Return whether or not the N flag (last op subtraction) is currently set in the F register.
    pub fn n_flag(&self) -> bool {
        self.flags & NFlag::BIT != 0
    }

    /// Return whether or not the H flag (half carry) is currently set in the F register.
    pub fn h_flag(&self) -> bool {
        self.flags & HFlag::BIT != 0
    }

    /// Return whether or not the C flag (carry) is currently set in the F register.
    pub fn c_flag(&self) -> bool {
        self.flags & CFlag::BIT != 0
    }
}
