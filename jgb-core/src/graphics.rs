use crate::config::GbColorScheme;
use crate::cpu::ExecutionMode;
use crate::ppu::{FrameBuffer, PpuState};
use crate::{ppu, GbcColorCorrection, HardwareMode, RunConfig};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Texture, TextureCreator, TextureValueError, WindowCanvas};
use sdl2::ttf::{Font, FontError};
use sdl2::video::{FullscreenType, Window};
use sdl2::IntegerOrSdlError;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GraphicsError {
    #[error("error setting fullscreen mode: {msg}")]
    Fullscreen { msg: String },
    #[error("error creating renderer: {source}")]
    CreateRenderer {
        #[from]
        source: IntegerOrSdlError,
    },
    #[error("error creating texture: {source}")]
    CreateTexture {
        #[from]
        source: TextureValueError,
    },
    #[error("error updating frame texture: {msg}")]
    Texture { msg: String },
    #[error("error copying frame texture to renderer: {msg}")]
    CopyToCanvas { msg: String },
    #[error("error rendering font: {source}")]
    FontRender {
        #[from]
        source: FontError,
    },
}

// GB colors range from 0-3 with 0 being "white" and 3 being "black"

// 0/0/0 = black and 255/255/255 = white, so linearly map [0,3] to [255,0]
const GB_COLOR_TO_RGB_BW: [[u8; 3]; 4] =
    [[255, 255, 255], [170, 170, 170], [85, 85, 85], [0, 0, 0]];

// Render with a light green tint
const GB_COLOR_TO_RGB_GREEN_TINT: [[u8; 3]; 4] =
    [[0xAE, 0xD2, 0x8D], [0x75, 0x9C, 0x68], [0x40, 0x5E, 0x2D], [0x0C, 0x1E, 0x09]];

// Render with an intense green tint that somewhat mimics the original Game Boy LCD screen
const GB_COLOR_TO_RGB_LIME_GREEN: [[u8; 3]; 4] =
    [[0x80, 0xA6, 0x08], [0x5D, 0x7F, 0x07], [0x25, 0x5C, 0x1A], [0x00, 0x32, 0x00]];

fn palette_for(color_scheme: GbColorScheme) -> [[u8; 3]; 4] {
    match color_scheme {
        GbColorScheme::BlackAndWhite => GB_COLOR_TO_RGB_BW,
        GbColorScheme::GreenTint => GB_COLOR_TO_RGB_GREEN_TINT,
        GbColorScheme::LimeGreen => GB_COLOR_TO_RGB_LIME_GREEN,
    }
}

/// Create an SDL2 renderer from the given SDL2 window, with VSync optionally enabled and with the
/// display area initialized to all white pixels.
pub fn create_renderer(
    mut window: Window,
    run_config: &RunConfig,
) -> Result<WindowCanvas, GraphicsError> {
    if run_config.launch_fullscreen {
        let fullscreen_mode = if run_config.borderless_fullscreen {
            FullscreenType::Desktop
        } else {
            FullscreenType::True
        };
        window.set_fullscreen(fullscreen_mode).map_err(|msg| GraphicsError::Fullscreen { msg })?;
    }

    let mut canvas_builder = window.into_canvas();
    if run_config.vsync_enabled {
        canvas_builder = canvas_builder.present_vsync();
    }

    let mut canvas = canvas_builder.build()?;

    // Set initial color based on the color scheme's "white"
    let [r, g, b] = match run_config.hardware_mode {
        HardwareMode::GameBoy => palette_for(run_config.color_scheme)[0],
        HardwareMode::GameBoyColor => [255, 255, 255],
    };

    canvas.set_draw_color(Color::RGB(r, g, b));
    canvas.clear();
    canvas.present();

    Ok(canvas)
}

/// Newtype wrapper around an SDL2 Texture, specifically to use for rendering PPU output
pub struct GbFrameTexture<'a>(Texture<'a>);

impl<'a> GbFrameTexture<'a> {
    pub fn create<T>(texture_creator: &'a TextureCreator<T>) -> Result<Self, GraphicsError> {
        let texture = texture_creator.create_texture_streaming(
            PixelFormatEnum::RGB24,
            ppu::SCREEN_WIDTH.into(),
            ppu::SCREEN_HEIGHT.into(),
        )?;
        Ok(Self(texture))
    }
}

fn gb_texture_updater(
    frame_buffer: &FrameBuffer,
    palette: [[u8; 3]; 4],
) -> impl Fn(&mut [u8], usize) + '_ {
    move |pixels, pitch| {
        for (i, scanline) in frame_buffer.iter().enumerate() {
            for (j, gb_color) in scanline.iter().copied().enumerate() {
                let start = i * pitch + 3 * j;
                pixels[start..start + 3].copy_from_slice(&palette[usize::from(gb_color)]);
            }
        }
    }
}

// Pre-computed so as to avoid needing to do floating point arithmetic while rendering
// GBC_COLOR_TO_8_BIT[i] = (f64::from(i) * 255.0 / 31.0).round() as u8
const GBC_COLOR_TO_8_BIT: [u8; 32] = [
    0, 8, 16, 25, 33, 41, 49, 58, 66, 74, 82, 90, 99, 107, 115, 123, 132, 140, 148, 156, 165, 173,
    181, 189, 197, 206, 214, 222, 230, 239, 247, 255,
];

fn parse_gbc_color(gbc_color: u16) -> [u16; 3] {
    // R = lowest 5 bits, G = next 5 bits, B = next 5 bits; highest bit unused
    [gbc_color & 0x001F, (gbc_color & 0x03E0) >> 5, (gbc_color & 0x7C00) >> 10]
}

fn normalize_gbc_color(value: u16) -> u8 {
    GBC_COLOR_TO_8_BIT[value as usize]
}

fn gbc_texture_updater_raw_colors(frame_buffer: &FrameBuffer) -> impl Fn(&mut [u8], usize) + '_ {
    move |pixels, pitch| {
        for (i, scanline) in frame_buffer.iter().enumerate() {
            for (j, gbc_color) in scanline.iter().copied().enumerate() {
                let [r, g, b] = parse_gbc_color(gbc_color).map(normalize_gbc_color);

                pixels[i * pitch + 3 * j] = r;
                pixels[i * pitch + 3 * j + 1] = g;
                pixels[i * pitch + 3 * j + 2] = b;
            }
        }
    }
}

// Indexed by R then G then B
struct ColorCorrectionTable([[[[u8; 3]; 32]; 32]; 32]);

impl ColorCorrectionTable {
    fn create() -> Self {
        let mut table = [[[[0; 3]; 32]; 32]; 32];

        for (r, r_row) in table.iter_mut().enumerate() {
            for (g, g_row) in r_row.iter_mut().enumerate() {
                for (b, value) in g_row.iter_mut().enumerate() {
                    let rf = r as f64;
                    let gf = g as f64;
                    let bf = b as f64;

                    // Based on this public domain shader:
                    // https://github.com/libretro/common-shaders/blob/master/handheld/shaders/color/gbc-color.cg
                    let corrected_r = ((0.78824 * rf + 0.12157 * gf) * 255.0 / 31.0).round() as u8;
                    let corrected_g =
                        ((0.025 * rf + 0.72941 * gf + 0.275 * bf) * 255.0 / 31.0).round() as u8;
                    let corrected_b =
                        ((0.12039 * rf + 0.12157 * gf + 0.82 * bf) * 255.0 / 31.0).round() as u8;

                    value.copy_from_slice(&[corrected_r, corrected_g, corrected_b]);
                }
            }
        }

        Self(table)
    }
}

fn gbc_texture_updater_corrected_colors(
    frame_buffer: &FrameBuffer,
) -> impl Fn(&mut [u8], usize) + '_ {
    static COLOR_CORRECTION_TABLE: OnceLock<ColorCorrectionTable> = OnceLock::new();
    let color_correction_table = COLOR_CORRECTION_TABLE.get_or_init(ColorCorrectionTable::create);

    move |pixels, pitch| {
        for (i, scanline) in frame_buffer.iter().enumerate() {
            for (j, gbc_color) in scanline.iter().copied().enumerate() {
                let [r, g, b] = parse_gbc_color(gbc_color);

                let corrected_colors = color_correction_table.0[r as usize][g as usize][b as usize];

                let start = i * pitch + 3 * j;
                pixels[start..start + 3].copy_from_slice(&corrected_colors);
            }
        }
    }
}

pub const FONT_SIZE: u16 = 16;

#[derive(Debug, Clone)]
pub struct Modal {
    text: String,
    end_time: SystemTime,
}

impl Modal {
    pub fn new(text: String, duration: Duration) -> Self {
        Self { text, end_time: SystemTime::now() + duration }
    }

    pub fn is_finished(&self) -> bool {
        self.end_time <= SystemTime::now()
    }
}

/// Render the current frame to the SDL2 window, overwriting all previously displayed data.
///
/// With VSync enabled this function will block until the next screen refresh.
#[allow(clippy::too_many_arguments)]
pub fn render_frame<T>(
    execution_mode: ExecutionMode,
    ppu_state: &PpuState,
    canvas: &mut WindowCanvas,
    texture_creator: &TextureCreator<T>,
    texture: &mut GbFrameTexture<'_>,
    font: &Font<'_, '_>,
    modals: &[Modal],
    run_config: &RunConfig,
) -> Result<(), GraphicsError> {
    let frame_buffer = ppu_state.frame_buffer();

    // Cludge to avoid a pointless heap allocation via Box. Trying to use `&dyn` without doing this
    // will result in "does not live long enough" errors
    let gb_updater;
    let gbc_raw_updater;
    let gbc_corrected_updater;

    let texture_updater: &dyn Fn(&mut [u8], usize) = match execution_mode {
        ExecutionMode::GameBoy => {
            gb_updater = gb_texture_updater(frame_buffer, palette_for(run_config.color_scheme));
            &gb_updater
        }
        ExecutionMode::GameBoyColor => match run_config.gbc_color_correction {
            GbcColorCorrection::None => {
                gbc_raw_updater = gbc_texture_updater_raw_colors(frame_buffer);
                &gbc_raw_updater
            }
            GbcColorCorrection::GbcLcd => {
                gbc_corrected_updater = gbc_texture_updater_corrected_colors(frame_buffer);
                &gbc_corrected_updater
            }
        },
    };

    texture.0.with_lock(None, texture_updater).map_err(|msg| GraphicsError::Texture { msg })?;

    let dst_rect = if run_config.force_integer_scaling {
        let (w, h) = canvas.window().size();
        determine_integer_scale_rect(w, h)
    } else {
        None
    };

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.copy(&texture.0, None, dst_rect).map_err(|msg| GraphicsError::CopyToCanvas { msg })?;

    render_modals(canvas, texture_creator, font, modals)?;

    canvas.present();

    Ok(())
}

fn render_modals<T>(
    canvas: &mut WindowCanvas,
    texture_creator: &TextureCreator<T>,
    font: &Font<'_, '_>,
    modals: &[Modal],
) -> Result<(), GraphicsError> {
    if modals.is_empty() {
        return Ok(());
    }

    canvas.set_blend_mode(BlendMode::Blend);

    let mut y = 20;
    for modal in modals {
        let surface = font
            .render(&modal.text)
            .shaded(Color::RGBA(255, 255, 255, 255), Color::RGBA(0, 0, 0, 128))?;
        let texture = surface.as_texture(texture_creator)?;

        canvas
            .copy(&texture, None, Rect::new(20, y, surface.width(), surface.height()))
            .map_err(|msg| GraphicsError::CopyToCanvas { msg })?;

        canvas.set_draw_color(Color::RGBA(50, 50, 50, 200));
        canvas
            .draw_rect(Rect::new(20, y, surface.width(), surface.height()))
            .map_err(|msg| GraphicsError::CopyToCanvas { msg })?;

        y += surface.height() as i32 + 20;
    }

    Ok(())
}

#[allow(clippy::maybe_infinite_iter)]
fn determine_integer_scale_rect(w: u32, h: u32) -> Option<Rect> {
    let screen_width: u32 = ppu::SCREEN_WIDTH.into();
    let screen_height: u32 = ppu::SCREEN_HEIGHT.into();

    let Some(scale) =
        (1..).take_while(|&scale| scale * screen_width <= w && scale * screen_height <= h).last()
    else {
        // Give up, display area is too small for 1x scale
        return None;
    };

    let scaled_width = scale * screen_width;
    let scaled_height = scale * screen_height;
    Some(Rect::new(
        ((w - scaled_width) / 2) as i32,
        ((h - scaled_height) / 2) as i32,
        scaled_width,
        scaled_height,
    ))
}

pub fn toggle_fullscreen(
    canvas: &mut WindowCanvas,
    run_config: &RunConfig,
) -> Result<(), GraphicsError> {
    let fullscreen_mode = if run_config.borderless_fullscreen {
        FullscreenType::Desktop
    } else {
        FullscreenType::True
    };

    let current_fullscreen = canvas.window().fullscreen_state();
    let new_fullscreen = match current_fullscreen {
        FullscreenType::Off => fullscreen_mode,
        FullscreenType::True | FullscreenType::Desktop => FullscreenType::Off,
    };
    canvas
        .window_mut()
        .set_fullscreen(new_fullscreen)
        .map_err(|msg| GraphicsError::Fullscreen { msg })
}
