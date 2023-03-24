use crate::cpu::InterruptType;
use crate::memory::ioregisters::{IoRegister, IoRegisters, SpriteMode, TileDataRange};
use crate::memory::{address, AddressSpace};
use std::collections::VecDeque;

type FrameBuffer = [[u8; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    HBlank,
    VBlank,
    ScanningOAM,
    RenderingScanline,
}

impl Mode {
    fn flag_bits(self) -> u8 {
        match self {
            Self::HBlank => 0x00,
            Self::VBlank => 0x01,
            Self::ScanningOAM => 0x02,
            Self::RenderingScanline => 0x03,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct OamSpriteData {
    x_pos: u8,
    y_pos: u8,
    tile_index: u8,
    flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SortedOamData(Vec<OamSpriteData>);

impl SortedOamData {
    fn from_vec(mut v: Vec<OamSpriteData>) -> Self {
        v.sort_by_key(|data| data.x_pos);

        Self(v)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpritePalette {
    ObjPalette0,
    ObjPalette1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueuedObjPixel {
    color_id: u8,
    obj_palette: SpritePalette,
    bg_over_obj: bool,
}

impl QueuedObjPixel {
    const TRANSPARENT: Self = Self {
        color_id: 0x00,
        obj_palette: SpritePalette::ObjPalette0,
        bg_over_obj: true,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
        bg_fetcher_x: u8,
        sprite_fetcher_x: u8,
        dot: u32,
        sprites: SortedOamData,
        bg_pixel_queue: VecDeque<u8>,
        sprite_pixel_queue: VecDeque<QueuedObjPixel>,
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

    pub fn mode(&self) -> Mode {
        match self {
            State::HBlank { .. } => Mode::HBlank,
            State::VBlank { .. } => Mode::VBlank,
            State::ScanningOAM { .. } => Mode::ScanningOAM,
            State::RenderingScanline { .. } => Mode::RenderingScanline,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OamDmaStatus {
    pub source_high_bits: u16,
    pub current_low_bits: u16,
}

impl OamDmaStatus {
    pub fn new(source_high_bits: u8) -> Self {
        Self {
            source_high_bits: u16::from(source_high_bits) << 8,
            current_low_bits: 0x00,
        }
    }

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
        self.state.mode()
    }

    pub fn oam_dma_status(&self) -> Option<OamDmaStatus> {
        self.oam_dma_status
    }

    pub fn frame_buffer(&self) -> &FrameBuffer {
        &self.frame_buffer
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

const VBLANK_START: State = State::VBlank {
    scanline: SCREEN_HEIGHT,
    dot: 0,
};

pub fn progress_oam_dma_transfer(ppu_state: &mut PpuState, address_space: &mut AddressSpace) {
    if ppu_state.oam_dma_status.is_none()
        && address_space
            .get_io_registers()
            .oam_dma_transfer_requested()
    {
        address_space
            .get_io_registers_mut()
            .clear_oam_dma_transfer_request();

        let source_high_bits = address_space
            .get_io_registers()
            .read_register(IoRegister::DMA);
        if source_high_bits <= 0xDF {
            ppu_state.oam_dma_status = Some(OamDmaStatus::new(source_high_bits));
        }
    }

    let Some(oam_dma_status) = ppu_state.oam_dma_status else { return; };

    address_space.copy_byte(
        oam_dma_status.current_source_address(),
        oam_dma_status.current_dest_address(),
    );
    ppu_state.oam_dma_status = oam_dma_status.increment();
}

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

    log::trace!("new PPU state: {new_state:?}");

    let scanline = new_state.scanline();
    let new_mode = new_state.mode();

    address_space
        .get_io_registers_mut()
        .privileged_set_ly(scanline);

    update_stat_register(address_space.get_io_registers_mut(), scanline, new_mode);

    if new_state == VBLANK_START {
        address_space
            .get_io_registers_mut()
            .interrupt_flags()
            .set(InterruptType::VBlank);
    }

    let stat_interrupt_line =
        compute_stat_interrupt_line(address_space.get_io_registers(), scanline, new_mode);
    if !ppu_state.last_stat_interrupt_line && stat_interrupt_line {
        address_space
            .get_io_registers_mut()
            .interrupt_flags()
            .set(InterruptType::LcdStatus);
    }

    ppu_state.state = new_state;
    ppu_state.last_stat_interrupt_line = stat_interrupt_line;
}

fn update_stat_register(io_registers: &mut IoRegisters, scanline: u8, mode: Mode) {
    let lyc_match = scanline == io_registers.read_register(IoRegister::LYC);

    let mode_bits = mode.flag_bits();

    let existing_stat = io_registers.privileged_read_stat() & 0xF8;
    let new_stat = existing_stat | (u8::from(lyc_match) << 2) | mode_bits;

    io_registers.privileged_set_stat(new_stat);
}

fn compute_stat_interrupt_line(io_registers: &IoRegisters, scanline: u8, mode: Mode) -> bool {
    let stat = io_registers.privileged_read_stat();

    let lyc_source = stat & 0x40 != 0;
    let scanning_oam_source = stat & 0x20 != 0;
    let vblank_source = stat & 0x10 != 0;
    let hblank_source = stat & 0x08 != 0;

    let lyc_match = scanline == io_registers.read_register(IoRegister::LYC);

    (lyc_source && lyc_match)
        || (scanning_oam_source && mode == Mode::ScanningOAM)
        || (vblank_source && mode == Mode::VBlank)
        || (hblank_source && mode == Mode::HBlank)
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
        State::RenderingScanline { .. } => process_render_state(state, address_space, pixel_buffer),
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
            bg_fetcher_x: 0,
            sprite_fetcher_x: 0,
            dot: new_dot,
            sprites: SortedOamData::from_vec(sprites),
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

    let sprite_height = address_space
        .get_io_registers()
        .lcdc()
        .sprite_mode()
        .height();

    let top_scanline = i32::from(y_pos) - 16;
    let bottom_scanline = top_scanline + i32::from(sprite_height);
    if (top_scanline..bottom_scanline).contains(&scanline.into()) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TileData(u8, u8);

// This function is not even remotely cycle-accurate but it attempts to approximate the pixel queue
// behavior of actual hardware
fn process_render_state(
    state: State,
    address_space: &AddressSpace,
    frame_buffer: &mut FrameBuffer,
) -> State {
    let State::RenderingScanline {
        scanline,
        mut pixel,
        mut bg_fetcher_x,
        mut sprite_fetcher_x,
        dot,
        sprites,
        mut bg_pixel_queue,
        mut sprite_pixel_queue,
    } = state else {
        panic!("process render_state only accepts RenderingScanline state, was: {state:?}");
    };

    if pixel == SCREEN_WIDTH {
        if dot >= MIN_RENDER_DOTS {
            return State::HBlank {
                scanline,
                dot: dot + DOTS_PER_M_CYCLE,
            };
        }
        return State::RenderingScanline {
            scanline,
            pixel,
            bg_fetcher_x,
            sprite_fetcher_x,
            dot: dot + DOTS_PER_M_CYCLE,
            sprites,
            bg_pixel_queue,
            sprite_pixel_queue,
        };
    }

    log::trace!(
        "LCDC: {:02X}",
        address_space
            .get_io_registers()
            .read_register(IoRegister::LCDC)
    );

    let lcdc = address_space.get_io_registers().lcdc();
    let bg_enabled = lcdc.bg_enabled();
    let sprites_enabled = lcdc.sprites_enabled();
    let window_tile_map_area = lcdc.window_tile_map_area();
    let bg_tile_data_area = lcdc.bg_tile_data_area();
    let bg_tile_map_area = lcdc.bg_tile_map_area();
    let window_enabled = lcdc.window_enabled();
    let sprite_mode = lcdc.sprite_mode();

    let bg_palette = address_space
        .get_io_registers()
        .read_register(IoRegister::BGP);
    let obj_palette_0 = address_space
        .get_io_registers()
        .read_register(IoRegister::OBP0);
    let obj_palette_1 = address_space
        .get_io_registers()
        .read_register(IoRegister::OBP1);

    if bg_pixel_queue.len() >= 8 && sprite_pixel_queue.len() >= 8 {
        while !bg_pixel_queue.is_empty() && !sprite_pixel_queue.is_empty() && pixel < SCREEN_WIDTH {
            let bg_pixel = bg_pixel_queue.pop_front().unwrap();
            let sprite_pixel = sprite_pixel_queue.pop_front().unwrap();

            // Discard BG pixel if BG is disabled
            let bg_pixel = if bg_enabled { bg_pixel } else { 0x00 };

            let pixel_color = if sprite_pixel.color_id == 0x00
                || (sprite_pixel.bg_over_obj && bg_pixel != 0x00)
            {
                get_bg_pixel_color(bg_pixel, bg_palette)
            } else {
                let sprite_palette = match sprite_pixel.obj_palette {
                    SpritePalette::ObjPalette0 => obj_palette_0,
                    SpritePalette::ObjPalette1 => obj_palette_1,
                };
                get_obj_pixel_color(sprite_pixel.color_id, sprite_palette)
            };

            log::trace!("bg_pixel={bg_pixel}, sprite_pixel={sprite_pixel:?}, bg_palette={bg_palette:02X}, obj_palette_0={obj_palette_0:02X}, obj_palette_1={obj_palette_1:02X}, pixel_color={pixel_color}");

            frame_buffer[scanline as usize][pixel as usize] = pixel_color;
            pixel += 1;
        }

        return State::RenderingScanline {
            scanline,
            pixel,
            bg_fetcher_x,
            sprite_fetcher_x,
            dot: dot + DOTS_PER_M_CYCLE,
            sprites,
            bg_pixel_queue,
            sprite_pixel_queue,
        };
    }

    let window_y = address_space
        .get_io_registers()
        .read_register(IoRegister::WY);
    let window_x_plus_7 = address_space
        .get_io_registers()
        .read_register(IoRegister::WX);

    while bg_pixel_queue.len() < 8 {
        if window_enabled && scanline >= window_y && bg_fetcher_x + 7 >= window_x_plus_7 {
            log::trace!("Inside window at x={bg_fetcher_x}, y={scanline}");

            // Clear any existing BG pixels if we just entered the window
            if bg_fetcher_x + 7 == window_x_plus_7 {
                bg_pixel_queue.clear();
            }

            let window_tile_x: u16 = ((bg_fetcher_x + 7 - window_x_plus_7) / 8).into();
            let window_tile_y: u16 = ((scanline - window_y) / 8).into();
            let tile_map_offset = 32 * window_tile_y + window_tile_x;
            let tile_index =
                address_space.ppu_read_address_u8(window_tile_map_area.start + tile_map_offset);

            let tile_address = get_bg_tile_address(bg_tile_data_area, tile_index);

            let y: u16 = ((scanline - window_y) % 8).into();
            let tile_data_0 = address_space.ppu_read_address_u8(tile_address + 2 * y);
            let tile_data_1 = address_space.ppu_read_address_u8(tile_address + 2 * y + 1);

            let mut x = (bg_fetcher_x + 7 - window_x_plus_7) % 8;
            while x < 8 {
                let pixel_color_id = get_pixel_color_id(TileData(tile_data_0, tile_data_1), x);
                bg_pixel_queue.push_back(pixel_color_id);

                x += 1;
                bg_fetcher_x += 1;
            }
        } else {
            let viewport_y = address_space
                .get_io_registers()
                .read_register(IoRegister::SCY);
            let viewport_x = address_space
                .get_io_registers()
                .read_register(IoRegister::SCX);

            log::trace!("Viewport at x={viewport_x}, y={viewport_y}");

            let bg_y = viewport_y.wrapping_add(scanline);
            let bg_x = viewport_x.wrapping_add(bg_fetcher_x);

            let bg_tile_y: u16 = (bg_y / 8).into();
            let bg_tile_x: u16 = (bg_x / 8).into();
            let tile_map_offset = 32 * bg_tile_y + bg_tile_x;
            let tile_index =
                address_space.ppu_read_address_u8(bg_tile_map_area.start + tile_map_offset);

            log::trace!(
                "Reading tile index at x={bg_tile_x}, y={bg_tile_y} using tile map address {:04X}",
                bg_tile_map_area.start + tile_map_offset
            );

            let tile_address = get_bg_tile_address(bg_tile_data_area, tile_index);

            log::trace!("Reading tile data from address {tile_address:04X}");

            let y: u16 = (bg_y % 8).into();
            let tile_data_0 = address_space.ppu_read_address_u8(tile_address + 2 * y);
            let tile_data_1 = address_space.ppu_read_address_u8(tile_address + 2 * y + 1);

            let mut x = bg_x % 8;
            while x < 8
                && (!window_enabled || scanline < window_y || (bg_fetcher_x + 7) < window_x_plus_7)
            {
                let pixel_color_id = get_pixel_color_id(TileData(tile_data_0, tile_data_1), x);
                bg_pixel_queue.push_back(pixel_color_id);

                x += 1;
                bg_fetcher_x += 1;
            }
        }
    }

    let sprite_tiles = lookup_sprite_tiles(&sprites.0, address_space, scanline);
    let sprites_with_tiles: Vec<_> = sprites.0.iter().copied().zip(sprite_tiles).collect();
    while sprite_pixel_queue.len() < 8 {
        if !sprites_enabled {
            sprite_pixel_queue.push_back(QueuedObjPixel::TRANSPARENT);
            sprite_fetcher_x += 1;
            continue;
        }

        let overlapping_sprites = find_overlapping_sprites(&sprites_with_tiles, sprite_fetcher_x);
        if overlapping_sprites.is_empty() {
            sprite_pixel_queue.push_back(QueuedObjPixel::TRANSPARENT);
            sprite_fetcher_x += 1;
            continue;
        }

        let pixel_to_queue = overlapping_sprites
            .into_iter()
            .find_map(|(sprite_data, tile_data)| {
                let bg_over_obj = sprite_data.flags & 0x80 != 0;
                let flip_x = sprite_data.flags & 0x20 != 0;
                let obj_palette = if sprite_data.flags & 0x10 != 0 {
                    SpritePalette::ObjPalette1
                } else {
                    SpritePalette::ObjPalette0
                };

                let x = if flip_x {
                    7 - (sprite_fetcher_x + 8 - sprite_data.x_pos)
                } else {
                    sprite_fetcher_x + 8 - sprite_data.x_pos
                };

                let pixel_color_id = get_pixel_color_id(tile_data, x);
                if pixel_color_id != 0x00 {
                    Some(QueuedObjPixel {
                        color_id: pixel_color_id,
                        obj_palette,
                        bg_over_obj,
                    })
                } else {
                    None
                }
            })
            .unwrap_or(QueuedObjPixel::TRANSPARENT);

        sprite_pixel_queue.push_back(pixel_to_queue);
        sprite_fetcher_x += 1;
    }

    State::RenderingScanline {
        scanline,
        pixel,
        bg_fetcher_x,
        sprite_fetcher_x,
        dot: dot + DOTS_PER_M_CYCLE,
        sprites,
        bg_pixel_queue,
        sprite_pixel_queue,
    }
}

fn lookup_sprite_tiles(
    sprites: &[OamSpriteData],
    address_space: &AddressSpace,
    scanline: u8,
) -> Vec<TileData> {
    let sprite_mode = address_space.get_io_registers().lcdc().sprite_mode();

    let mut sprite_tiles = Vec::with_capacity(sprites.len());
    for sprite in sprites {
        let flip_y = sprite.flags & 0x40 != 0;

        let sprite_y = scanline + 16 - sprite.y_pos;

        let (tile_index, y) = match sprite_mode {
            SpriteMode::Single => {
                let y = if flip_y { 7 - sprite_y } else { sprite_y };
                (sprite.tile_index, y)
            }
            SpriteMode::Stacked => {
                let y = if flip_y { 15 - sprite_y } else { sprite_y };
                let tile_index = if y < 8 {
                    sprite.tile_index & 0xFE
                } else {
                    (sprite.tile_index & 0xFE) + 1
                };
                (tile_index, y % 8)
            }
        };

        let y: u16 = y.into();

        let tile_address = 0x8000 + 16 * u16::from(tile_index);

        let tile_data_0 = address_space.ppu_read_address_u8(tile_address + 2 * y);
        let tile_data_1 = address_space.ppu_read_address_u8(tile_address + 2 * y + 1);

        sprite_tiles.push(TileData(tile_data_0, tile_data_1));
    }
    sprite_tiles
}

fn find_overlapping_sprites(
    sprites: &[(OamSpriteData, TileData)],
    x: u8,
) -> Vec<(OamSpriteData, TileData)> {
    sprites
        .iter()
        .filter(|&(sprite_data, _)| {
            (sprite_data.x_pos..sprite_data.x_pos.saturating_add(8)).contains(&(x + 8))
        })
        .copied()
        .collect()
}

fn get_bg_tile_address(bg_tile_data_area: TileDataRange, tile_index: u8) -> u16 {
    match bg_tile_data_area {
        TileDataRange::Block0 => {
            // Intentionally wrap [128, 255] to [-128, -1]
            let signed_tile_index = tile_index as i8;
            (i32::from(bg_tile_data_area.start_address()) + 16 * i32::from(signed_tile_index))
                as u16
        }
        TileDataRange::Block1 => bg_tile_data_area.start_address() + 16 * u16::from(tile_index),
    }
}

fn get_bg_pixel_color(pixel: u8, palette: u8) -> u8 {
    (palette >> (pixel * 2)) & 0x03
}

fn get_obj_pixel_color(pixel: u8, palette: u8) -> u8 {
    // 0x00 in OBJ pixels means transparent, ignore palette
    if pixel == 0x00 {
        0x00
    } else {
        (palette >> (pixel * 2)) & 0x03
    }
}

fn get_pixel_color_id(tile_data: TileData, x: u8) -> u8 {
    let bit_mask = 1 << (7 - x);
    u8::from(tile_data.1 & bit_mask != 0) << 1 | u8::from(tile_data.0 & bit_mask != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::memory::Cartridge;

    #[test]
    fn oam_dma_transfer_basic_test() {
        let mut address_space = AddressSpace::new(Cartridge::new(vec![0; 0x150]).unwrap());
        let mut ppu_state = PpuState::new();

        progress_oam_dma_transfer(&mut ppu_state, &mut address_space);
        assert_eq!(None, ppu_state.oam_dma_status);

        address_space.write_address_u8(0xC500, 0x78, &ppu_state);
        address_space.write_address_u8(0xC555, 0x12, &ppu_state);
        address_space.write_address_u8(0xC59F, 0x34, &ppu_state);
        address_space.write_address_u8(0xC5A0, 0x56, &ppu_state);

        address_space
            .get_io_registers_mut()
            .write_register(IoRegister::DMA, 0xC5);

        progress_oam_dma_transfer(&mut ppu_state, &mut address_space);
        assert!(ppu_state.oam_dma_status.is_some());

        for _ in 0..158 {
            progress_oam_dma_transfer(&mut ppu_state, &mut address_space);
            assert!(ppu_state.oam_dma_status.is_some());
        }

        progress_oam_dma_transfer(&mut ppu_state, &mut address_space);
        assert_eq!(None, ppu_state.oam_dma_status);

        assert_eq!(0x78, address_space.read_address_u8(0xFE00, &ppu_state));
        assert_eq!(0x12, address_space.read_address_u8(0xFE55, &ppu_state));
        assert_eq!(0x34, address_space.read_address_u8(0xFE9F, &ppu_state));
    }

    #[test]
    fn scan_oam_basic_test() {
        let mut address_space = AddressSpace::new(Cartridge::new(vec![0; 0x150]).unwrap());
        let ppu_state = PpuState::new();

        address_space.write_address_u8(address::OAM_START + 40, 53, &ppu_state);
        address_space.write_address_u8(address::OAM_START + 41, 20, &ppu_state);
        address_space.write_address_u8(address::OAM_START + 42, 0xC3, &ppu_state);
        address_space.write_address_u8(address::OAM_START + 43, 0x30, &ppu_state);

        let mut sprites = Vec::new();

        scan_oam(&mut sprites, &address_space, 40, 20);

        assert_eq!(
            sprites,
            vec![OamSpriteData {
                y_pos: 53,
                x_pos: 20,
                tile_index: 0xC3,
                flags: 0x30,
            }]
        );

        // Scanline 45 is past the bottom of the sprite
        scan_oam(&mut sprites, &address_space, 45, 20);
        assert_eq!(1, sprites.len());
    }
}
