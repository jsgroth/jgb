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
    pub fn from_opcode_bits(bits: u8) -> Option<Self> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl CpuRegisters {
    pub fn af(&self) -> u16 {
        u16::from_be_bytes([self.accumulator, self.flags])
    }

    pub fn bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }

    pub fn de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }

    pub fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    pub fn set_hl(&mut self, hl: u16) {
        let [h, l] = hl.to_be_bytes();
        self.h = h;
        self.l = l;
    }

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

    pub fn read_register_pair(&self, register_pair: CpuRegisterPair) -> u16 {
        match register_pair {
            CpuRegisterPair::AF => self.af(),
            CpuRegisterPair::BC => self.bc(),
            CpuRegisterPair::DE => self.de(),
            CpuRegisterPair::HL => self.hl(),
            CpuRegisterPair::SP => self.sp,
        }
    }

    pub fn set_register_pair(&mut self, register_pair: CpuRegisterPair, value: u16) {
        match register_pair {
            CpuRegisterPair::AF => {
                let [a, f] = value.to_be_bytes();
                self.accumulator = a;
                self.flags = f;
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

    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.flags =
            (u8::from(z) << 7) | (u8::from(n) << 6) | (u8::from(h) << 5) | (u8::from(c) << 4);
    }

    pub fn set_some_flags(&mut self, z: Option<bool>, n: Option<bool>, h: Option<bool>, c: Option<bool>) {
        match z {
            Some(true) => {
                self.flags |= 1 << 7;
            }
            Some(false) => {
                self.flags &= !(1 << 7);
            }
            None => {}
        }

        match n {
            Some(true) => {
                self.flags |= 1 << 6;
            }
            Some(false) => {
                self.flags &= !(1 << 6);
            }
            None => {}
        }

        match h {
            Some(true) => {
                self.flags |= 1 << 5;
            }
            Some(false) => {
                self.flags &= !(1 << 5);
            }
            None => {}
        }

        match c {
            Some(true) => {
                self.flags |= 1 << 4;
            }
            Some(false) => {
                self.flags &= !(1 << 4);
            }
            None => {}
        }
    }

    pub fn zero_flag(&self) -> bool {
        self.flags & 0x80 != 0
    }

    pub fn carry_flag(&self) -> bool {
        self.flags & 0x10 != 0
    }
}
