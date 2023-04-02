use crate::ppu::PpuState;
use crate::{ppu, RunConfig};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};
use sdl2::video::{FullscreenType, Window};
use sdl2::IntegerOrSdlError;
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
    #[error("error updating frame texture: {msg}")]
    Texture { msg: String },
    #[error("error copying frame texture to renderer: {msg}")]
    CopyToCanvas { msg: String },
}

/// Create an SDL2 renderer from the given SDL2 window, with VSync enabled and with the display area
/// initialized to all white pixels.
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
    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.clear();
    canvas.present();

    Ok(canvas)
}

// GB colors range from 0-3 with 0 being white and 3 being black
// In this pixel format, 0/0/0 = black and 255/255/255 = white, so map [0,3] to [255,0]
const GB_COLOR_TO_RGB: [u8; 4] = [255, 170, 85, 0];

/// Render the current frame to the SDL2 window, overwriting all previously displayed data.
///
/// With VSync enabled this function will block until the next screen refresh.
pub fn render_frame(
    ppu_state: &PpuState,
    canvas: &mut WindowCanvas,
    texture: &mut Texture,
    run_config: &RunConfig,
) -> Result<(), GraphicsError> {
    let frame_buffer = ppu_state.frame_buffer();

    texture
        .with_lock(None, |pixels, pitch| {
            for (i, scanline) in frame_buffer.iter().enumerate() {
                for (j, gb_color) in scanline.iter().copied().enumerate() {
                    let color = GB_COLOR_TO_RGB[gb_color as usize];

                    pixels[i * pitch + 3 * j] = color;
                    pixels[i * pitch + 3 * j + 1] = color;
                    pixels[i * pitch + 3 * j + 2] = color;
                }
            }
        })
        .map_err(|msg| GraphicsError::Texture { msg })?;

    let dst_rect = if run_config.force_integer_scaling {
        let (w, h) = canvas.window().size();
        determine_integer_scale_rect(w, h)
    } else {
        None
    };

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas
        .copy(texture, None, dst_rect)
        .map_err(|msg| GraphicsError::CopyToCanvas { msg })?;
    canvas.present();

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
