use crate::config::ColorScheme;
use crate::cpu::ExecutionMode;
use crate::ppu::{FrameBuffer, PpuState};
use crate::{ppu, HardwareMode, RunConfig};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Texture, TextureCreator, TextureValueError, WindowCanvas};
use sdl2::ttf::{Font, FontError};
use sdl2::video::{FullscreenType, Window};
use sdl2::IntegerOrSdlError;
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
const GB_COLOR_TO_RGB_GREEN_TINT: [[u8; 3]; 4] = [
    [0xAE, 0xD2, 0x8D],
    [0x75, 0x9C, 0x68],
    [0x40, 0x5E, 0x2D],
    [0x0C, 0x1E, 0x09],
];

// Render with an intense green tint that somewhat mimics the original Game Boy LCD screen
const GB_COLOR_TO_RGB_LIME_GREEN: [[u8; 3]; 4] = [
    [0x80, 0xA6, 0x08],
    [0x5D, 0x7F, 0x07],
    [0x25, 0x5C, 0x1A],
    [0x00, 0x32, 0x00],
];

fn palette_for(color_scheme: ColorScheme) -> [[u8; 3]; 4] {
    match color_scheme {
        ColorScheme::BlackAndWhite => GB_COLOR_TO_RGB_BW,
        ColorScheme::GreenTint => GB_COLOR_TO_RGB_GREEN_TINT,
        ColorScheme::LimeGreen => GB_COLOR_TO_RGB_LIME_GREEN,
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
        window
            .set_fullscreen(fullscreen_mode)
            .map_err(|msg| GraphicsError::Fullscreen { msg })?;
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
) -> impl FnOnce(&mut [u8], usize) + '_ {
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

fn normalize_gbc_color(value: u16) -> u8 {
    GBC_COLOR_TO_8_BIT[value as usize]
}

fn gbc_texture_updater(frame_buffer: &FrameBuffer) -> impl FnOnce(&mut [u8], usize) + '_ {
    move |pixels, pitch| {
        for (i, scanline) in frame_buffer.iter().enumerate() {
            for (j, gbc_color) in scanline.iter().copied().enumerate() {
                // R = lowest 5 bits, G = next 5 bits, B = next 5 bits; highest bit unused
                let r = normalize_gbc_color(gbc_color & 0x001F);
                let g = normalize_gbc_color((gbc_color & 0x03E0) >> 5);
                let b = normalize_gbc_color((gbc_color & 0x7C00) >> 10);

                pixels[i * pitch + 3 * j] = r;
                pixels[i * pitch + 3 * j + 1] = g;
                pixels[i * pitch + 3 * j + 2] = b;
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
        Self {
            text,
            end_time: SystemTime::now() + duration,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.end_time <= SystemTime::now()
    }
}

pub struct RenderFrameArgs<
    'ttf,
    'ppu,
    'canvas,
    'tx_creator,
    'texture,
    'font,
    'font_rwops,
    'modals,
    'cfg,
    T,
> {
    pub execution_mode: ExecutionMode,
    pub ppu_state: &'ppu PpuState,
    pub canvas: &'canvas mut WindowCanvas,
    pub texture_creator: &'tx_creator TextureCreator<T>,
    pub texture: &'texture mut GbFrameTexture<'tx_creator>,
    pub font: &'font Font<'ttf, 'font_rwops>,
    pub modals: &'modals [Modal],
    pub run_config: &'cfg RunConfig,
}

/// Render the current frame to the SDL2 window, overwriting all previously displayed data.
///
/// With VSync enabled this function will block until the next screen refresh.
pub fn render_frame<T>(args: RenderFrameArgs<T>) -> Result<(), GraphicsError> {
    let RenderFrameArgs {
        execution_mode,
        ppu_state,
        canvas,
        texture_creator,
        texture,
        font,
        modals,
        run_config,
    } = args;

    let frame_buffer = ppu_state.frame_buffer();

    match execution_mode {
        ExecutionMode::GameBoy => texture
            .0
            .with_lock(
                None,
                gb_texture_updater(frame_buffer, palette_for(run_config.color_scheme)),
            )
            .map_err(|msg| GraphicsError::Texture { msg })?,
        ExecutionMode::GameBoyColor => texture
            .0
            .with_lock(None, gbc_texture_updater(frame_buffer))
            .map_err(|msg| GraphicsError::Texture { msg })?,
    }

    let dst_rect = if run_config.force_integer_scaling {
        let (w, h) = canvas.window().size();
        determine_integer_scale_rect(w, h)
    } else {
        None
    };

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas
        .copy(&texture.0, None, dst_rect)
        .map_err(|msg| GraphicsError::CopyToCanvas { msg })?;

    render_modals(canvas, texture_creator, font, modals)?;

    canvas.present();

    Ok(())
}

fn render_modals<T>(
    canvas: &mut WindowCanvas,
    texture_creator: &TextureCreator<T>,
    font: &Font,
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
            .copy(
                &texture,
                None,
                Rect::new(20, y, surface.width(), surface.height()),
            )
            .map_err(|msg| GraphicsError::CopyToCanvas { msg })?;

        canvas.set_draw_color(Color::RGBA(50, 50, 50, 200));
        canvas
            .draw_rect(Rect::new(20, y, surface.width(), surface.height()))
            .map_err(|msg| GraphicsError::CopyToCanvas { msg })?;

        y += surface.height() as i32 + 20;
    }

    Ok(())
}

fn determine_integer_scale_rect(w: u32, h: u32) -> Option<Rect> {
    let screen_width: u32 = ppu::SCREEN_WIDTH.into();
    let screen_height: u32 = ppu::SCREEN_HEIGHT.into();

    let Some(scale) = (1..)
        .take_while(|&scale| scale * screen_width <= w && scale * screen_height <= h)
        .last()
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
