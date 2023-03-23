use crate::cpu::InterruptType;
use crate::memory::{address, AddressSpace};
use std::collections::VecDeque;

mod queue;

type FrameBuffer = [[u8; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize];

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

#[derive(Debug, Clone)]
struct SortedOamData(Vec<OamSpriteData>);

impl SortedOamData {
    fn from_array_vec(mut v: Vec<OamSpriteData>) -> Self {
        v.sort_by_key(|data| data.x_pos);

        Self(v)
    }
}

#[derive(Debug, Clone)]
enum State {
    HBlank {
        scanline: u8,
        dot: u32,
    },
    VBlank {
        scanline: u8,
        dot: u32,
    },
    ScanningOAM {
        scanline: u8,
        dot: u32,
        sprites: Vec<OamSpriteData>,
    },
    RenderingScanline {
        scanline: u8,
        pixel: u8,
        dot: u32,
        sprites: SortedOamData,
        bg_pixel_queue: VecDeque<u8>,
        sprite_pixel_queue: VecDeque<u8>,
    },
}

impl State {
    fn scanline(&self) -> u8 {
        match self {
            &Self::VBlank { scanline, .. }
            | &Self::HBlank { scanline, .. }
            | &Self::ScanningOAM { scanline, .. }
            | &Self::RenderingScanline { scanline, .. } => scanline,
        }
    }
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

#[derive(Debug, Clone)]
pub struct PpuState {
    enabled: bool,
    state: State,
    oam_dma_status: Option<OamDmaStatus>,
    frame_buffer: FrameBuffer,
    last_stat_interrupt_line: bool,
}

impl PpuState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            state: State::VBlank {
                scanline: SCREEN_HEIGHT + 1,
                dot: 0,
            },
            oam_dma_status: None,
            frame_buffer: [[0; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
            last_stat_interrupt_line: false,
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

const SCREEN_WIDTH: u8 = 160;
const SCREEN_HEIGHT: u8 = 144;

const DOTS_PER_M_CYCLE: u32 = 4;
const DOTS_PER_SCANLINE: u32 = 456;
const OAM_SCAN_DOTS: u32 = 80;
const MIN_RENDER_DOTS: u32 = 172;

const LAST_VBLANK_SCANLINE: u8 = 153;

const MAX_SPRITES_PER_SCANLINE: usize = 10;

const DUMMY_STATE: State = State::VBlank {
    scanline: 0,
    dot: 0,
};

pub fn tick_m_cycle(ppu_state: &mut PpuState, address_space: &mut AddressSpace) {
    let enabled = address_space.get_io_registers().lcdc().lcd_enabled();
    ppu_state.enabled = enabled;

    if !enabled {
        return;
    }

    let old_state = std::mem::replace(&mut ppu_state.state, DUMMY_STATE);
    let new_state = process_state(
        old_state,
        address_space,
        &ppu_state.oam_dma_status,
        &mut ppu_state.frame_buffer,
    );

    let scanline = new_state.scanline();

    address_space
        .get_io_registers_mut()
        .privileged_set_ly(scanline);

    if let &State::VBlank {
        scanline: 144,
        dot: 0,
    } = &new_state
    {
        address_space
            .get_io_registers_mut()
            .interrupt_flags()
            .set(InterruptType::VBlank);
    }

    ppu_state.state = new_state;

    todo!("update LY, STAT, IF")
}

fn process_state(
    state: State,
    address_space: &AddressSpace,
    oam_dma_status: &Option<OamDmaStatus>,
    pixel_buffer: &mut FrameBuffer,
) -> State {
    match state {
        State::VBlank { scanline, dot } => vblank_next_state(scanline, dot),
        State::HBlank { scanline, dot } => hblank_next_state(scanline, dot),
        State::ScanningOAM {
            scanline,
            dot,
            sprites,
        } => process_scanning_oam_state(scanline, dot, sprites, address_space, oam_dma_status),
        State::RenderingScanline {
            scanline,
            pixel,
            dot,
            sprites,
            bg_pixel_queue,
            sprite_pixel_queue,
        } => process_render_state(
            scanline,
            pixel,
            dot,
            sprites,
            bg_pixel_queue,
            sprite_pixel_queue,
            pixel_buffer,
        ),
    }
}

fn vblank_next_state(scanline: u8, dot: u32) -> State {
    let new_dot = dot + DOTS_PER_M_CYCLE;
    if new_dot == DOTS_PER_SCANLINE {
        if scanline == LAST_VBLANK_SCANLINE {
            State::ScanningOAM {
                scanline: 0,
                dot: 0,
                sprites: Vec::new(),
            }
        } else {
            State::VBlank {
                scanline: scanline + 1,
                dot: 0,
            }
        }
    } else {
        State::VBlank {
            scanline,
            dot: new_dot,
        }
    }
}

fn hblank_next_state(scanline: u8, dot: u32) -> State {
    let new_dot = dot + DOTS_PER_M_CYCLE;
    if new_dot == DOTS_PER_SCANLINE {
        if scanline == SCREEN_HEIGHT - 1 {
            State::VBlank {
                scanline: scanline + 1,
                dot: 0,
            }
        } else {
            State::ScanningOAM {
                scanline: scanline + 1,
                dot: 0,
                sprites: Vec::new(),
            }
        }
    } else {
        State::HBlank {
            scanline,
            dot: new_dot,
        }
    }
}

fn process_scanning_oam_state(
    scanline: u8,
    dot: u32,
    mut sprites: Vec<OamSpriteData>,
    address_space: &AddressSpace,
    oam_dma_status: &Option<OamDmaStatus>,
) -> State {
    // PPU effectively can't read OAM while an OAM DMA transfer is in progress
    if oam_dma_status.is_none() {
        // PPU reads 2 OAM entries every M-cycle (4 dots)
        scan_oam(&mut sprites, address_space, scanline, dot);
        scan_oam(&mut sprites, address_space, scanline, dot + 2);
    }

    let new_dot = dot + DOTS_PER_M_CYCLE;
    if new_dot == OAM_SCAN_DOTS {
        State::RenderingScanline {
            scanline,
            pixel: 0,
            dot: new_dot,
            sprites: SortedOamData::from_array_vec(sprites),
            bg_pixel_queue: VecDeque::new(),
            sprite_pixel_queue: VecDeque::new(),
        }
    } else {
        State::ScanningOAM {
            scanline,
            dot: new_dot,
            sprites,
        }
    }
}

fn scan_oam(
    sprites: &mut Vec<OamSpriteData>,
    address_space: &AddressSpace,
    scanline: u8,
    dot: u32,
) {
    if sprites.len() == MAX_SPRITES_PER_SCANLINE {
        return;
    }

    let oam_offset: u16 = (dot * 2)
        .try_into()
        .expect("dot values should never be large enough for (dot * 2) to overflow a u16");
    let obj_address = address::OAM_START + oam_offset;

    let y_pos = address_space.ppu_read_address_u8(obj_address);

    let sprite_height = address_space.get_io_registers().lcdc().sprite_height();

    let top_scanline = i32::from(y_pos) - i32::from(sprite_height);
    let bottom_scanline = i32::from(y_pos) - 1;
    if (top_scanline..=bottom_scanline).contains(&scanline.into()) {
        let x_pos = address_space.ppu_read_address_u8(obj_address + 1);
        let tile_index = address_space.ppu_read_address_u8(obj_address + 2);
        let flags = address_space.ppu_read_address_u8(obj_address + 3);

        sprites.push(OamSpriteData {
            y_pos,
            x_pos,
            tile_index,
            flags,
        });
    }
}

fn process_render_state(
    scanline: u8,
    pixel: u8,
    dot: u32,
    sprites: SortedOamData,
    bg_pixel_buffer: VecDeque<u8>,
    sprite_pixel_buffer: VecDeque<u8>,
    frame_buffer: &mut FrameBuffer,
) -> State {
    todo!()
}
