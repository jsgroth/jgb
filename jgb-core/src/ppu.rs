use crate::memory::{address, AddressSpace};
use crate::ppu::queue::ArrayQueue;
use tinyvec::ArrayVec;

mod queue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    HBlank,
    VBlank,
    ScanningOAM,
    RenderingScanline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct OamSpriteData {
    x_pos: u8,
    y_pos: u8,
    tile_index: u8,
    flags: u8,
}

const SPRITES_PER_SCANLINE: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    HBlank {
        scanline: u8,
        cycle: u32,
    },
    VBlank {
        scanline: u8,
        cycle: u32,
    },
    ScanningOAM {
        scanline: u8,
        cycle: u32,
        sprites: ArrayVec<[OamSpriteData; SPRITES_PER_SCANLINE]>,
    },
    RenderingScanline {
        scanline: u8,
        pixel: u8,
        cycle: u32,
        sprites: ArrayVec<[OamSpriteData; SPRITES_PER_SCANLINE]>,
        bg_pixel_buffer: ArrayQueue<u8>,
        sprite_pixel_buffer: ArrayQueue<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OamDmaStatus {
    pub source_high_bits: u16,
    pub current_low_bits: u16,
}

impl OamDmaStatus {
    pub fn current_source_address(self) -> u16 {
        self.source_high_bits | self.current_low_bits
    }

    pub fn current_dest_address(self) -> u16 {
        address::OAM_START | self.current_low_bits
    }

    pub fn increment(self) -> Option<Self> {
        if self.current_low_bits == 0x009F {
            None
        } else {
            Some(Self {
                source_high_bits: self.source_high_bits,
                current_low_bits: self.current_low_bits + 1,
            })
        }
    }
}

const SCREEN_WIDTH: u8 = 160;
const SCREEN_HEIGHT: u8 = 144;

#[derive(Debug, Clone)]
pub struct PpuState {
    enabled: bool,
    state: State,
    oam_dma_status: Option<OamDmaStatus>,
    frame_buffer: [[u8; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
}

impl PpuState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            state: State::VBlank {
                scanline: SCREEN_HEIGHT + 1,
                cycle: 0,
            },
            oam_dma_status: None,
            frame_buffer: [[0; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn mode(&self) -> Mode {
        match self.state {
            State::HBlank { .. } => Mode::HBlank,
            State::VBlank { .. } => Mode::VBlank,
            State::ScanningOAM { .. } => Mode::ScanningOAM,
            State::RenderingScanline { .. } => Mode::RenderingScanline,
        }
    }

    pub fn oam_dma_status(&self) -> Option<OamDmaStatus> {
        self.oam_dma_status
    }
}

const DOTS_PER_M_CYCLE: u32 = 4;
const CYCLES_PER_SCANLINE: u32 = 456;

pub fn tick_m_cycle(ppu_state: &mut PpuState, address_space: &mut AddressSpace) {
    let lcdc = address_space.get_io_registers().lcdc();

    let enabled = lcdc.lcd_enabled();
    ppu_state.enabled = enabled;

    if !enabled {
        return;
    }

    todo!()
}
