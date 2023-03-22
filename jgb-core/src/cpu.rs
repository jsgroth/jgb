pub(crate) mod instructions;
mod registers;

#[cfg(test)]
mod tests;

pub use registers::CpuRegisters;

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
}
