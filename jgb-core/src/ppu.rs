pub enum PpuState {
    HBlank { scanline: u8, cycle: u32 },
    VBlank { cycle: u32 },
    ScanningOAM { scanline: u8, cycle: u32 },
    Rendering { scanline: u8, pixel: u8, cycle: u32 },
}
