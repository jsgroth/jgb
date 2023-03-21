use crate::memory::address;

mod queue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    HBlank,
    VBlank,
    ScanningOAM,
    RenderingScanline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    HBlank { scanline: u8, cycle: u32 },
    VBlank { cycle: u32 },
    ScanningOAM { scanline: u8, cycle: u32 },
    RenderingScanline { scanline: u8, pixel: u8, cycle: u32 },
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
            state: State::ScanningOAM {
                scanline: 0,
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
