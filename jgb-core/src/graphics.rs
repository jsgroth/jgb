use crate::ppu::PpuState;
use sdl2::pixels::Color;
use sdl2::render::{Texture, WindowCanvas};
use sdl2::video::Window;
use sdl2::IntegerOrSdlError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("error updating frame texture: {msg}")]
    Texture { msg: String },
    #[error("error copying frame texture to renderer: {msg}")]
    CopyToCanvas { msg: String },
}

pub fn create_renderer(window: Window) -> Result<WindowCanvas, IntegerOrSdlError> {
    let mut canvas = window.into_canvas().present_vsync().build()?;
    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.clear();
    canvas.present();

    Ok(canvas)
}

pub fn render_frame(
    ppu_state: &PpuState,
    canvas: &mut WindowCanvas,
    texture: &mut Texture,
) -> Result<(), RenderError> {
    let frame_buffer = ppu_state.frame_buffer();

    texture
        .with_lock(None, |pixels, pitch| {
            for (i, scanline) in frame_buffer.iter().enumerate() {
                for (j, gb_color) in scanline.iter().copied().enumerate() {
                    // GB colors range from 0-3 with 0 being white and 3 being black
                    // In this pixel format, 0/0/0 = black and 255/255/255 = white, so map [0,3] to [255,0]
                    let color = 255 - (f64::from(gb_color) / 3.0 * 255.0).round() as u8;

                    pixels[i * pitch + 3 * j] = color;
                    pixels[i * pitch + 3 * j + 1] = color;
                    pixels[i * pitch + 3 * j + 2] = color;
                }
            }
        })
        .map_err(|msg| RenderError::Texture { msg })?;

    canvas.clear();
    canvas
        .copy(texture, None, None)
        .map_err(|msg| RenderError::CopyToCanvas { msg })?;
    canvas.present();

    Ok(())
}
