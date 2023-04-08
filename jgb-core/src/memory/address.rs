//
// Cartridge header addresses
//

pub const ENTRY_POINT: u16 = 0x0100;
pub const CGB_SUPPORT: u16 = 0x0143;
pub const MAPPER: u16 = 0x0147;
pub const RAM_SIZE: u16 = 0x0149;

//
// Address space boundaries
//

pub const ROM_START: u16 = 0x0000;
pub const ROM_END: u16 = 0x7FFF;

pub const VRAM_START: u16 = 0x8000;
pub const VRAM_END: u16 = 0x9FFF;

pub const EXTERNAL_RAM_START: u16 = 0xA000;
pub const EXTERNAL_RAM_END: u16 = 0xBFFF;

pub const WORKING_RAM_START: u16 = 0xC000;
pub const WORKING_RAM_END: u16 = 0xDFFF;

pub const CGB_BANK_0_WORKING_RAM_END: u16 = 0xCFFF;

pub const ECHO_RAM_START: u16 = 0xE000;
pub const ECHO_RAM_END: u16 = 0xFDFF;

pub const OAM_START: u16 = 0xFE00;
pub const OAM_END: u16 = 0xFE9F;

pub const UNUSABLE_START: u16 = 0xFEA0;
pub const UNUSABLE_END: u16 = 0xFEFF;

pub const IO_REGISTERS_START: u16 = 0xFF00;
pub const IO_REGISTERS_END: u16 = 0xFF7F;

pub const HRAM_START: u16 = 0xFF80;
pub const HRAM_END: u16 = 0xFFFE;

pub const IE_REGISTER: u16 = 0xFFFF;
