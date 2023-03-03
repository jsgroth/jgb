//
// Cartridge header addresses
//

pub const ENTRY_POINT: u16 = 0x0100;
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

//
// I/O register addresses
//

pub const JOYPAD_REGISTER: u16 = 0xFF00;

// Divider
pub const DIV_REGISTER: u16 = 0xFF04;

// Timer counter
pub const TIMA_REGISTER: u16 = 0xFF05;

// Timer modulo
pub const TMA_REGISTER: u16 = 0xFF06;

// Timer control
pub const TAC_REGISTER: u16 = 0xFF07;

// LCD control
pub const LCDC_REGISTER: u16 = 0xFF40;

// LCD status
pub const STAT_REGISTER: u16 = 0xFF41;

// Viewport Y position
pub const SCY_REGISTER: u16 = 0xFF42;

// Viewport X position
pub const SCX_REGISTER: u16 = 0xFF43;

// LCD Y coordinate
pub const LY_REGISTER: u16 = 0xFF44;

// LY compare
pub const LYC_REGISTER: u16 = 0xFF45;

// OAM DMA source address & start
pub const DMA_REGISTER: u16 = 0xFF46;

// BG palette data
pub const BGP_REGISTER: u16 = 0xFF47;

// OBJ palette 0/1 data
pub const OBP0_REGISTER: u16 = 0xFF48;
pub const OBP1_REGISTER: u16 = 0xFF49;

// Window Y position
pub const WY_REGISTER: u16 = 0xFF4A;

// Window X position + 7
pub const WX_REGISTER: u16 = 0xFF4B;
