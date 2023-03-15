#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    HBlank { scanline: u8, cycle: u32 },
    VBlank { cycle: u32 },
    ScanningOAM { scanline: u8, cycle: u32 },
    Rendering { scanline: u8, pixel: u8, cycle: u32 },
}

const SCREEN_WIDTH: u8 = 160;
const SCREEN_HEIGHT: u8 = 144;

#[derive(Debug, Clone)]
pub struct PpuState {
    mode: Mode,
    frame_buffer: [[u8; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
}
