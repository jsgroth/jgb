use sdl2::rwops::RWops;
use sdl2::ttf::{Font, Sdl2TtfContext};

const FONT_BYTES: &[u8] = include_bytes!("../../fonts/IBMPlexMono-Bold.ttf");

/// Load the IBM Plex Mono Bold font from within the executable.
///
/// # Errors
///
/// This function will return an error if it is unable to load the font.
pub fn load_font(ttf_ctx: &Sdl2TtfContext, point_size: u16) -> Result<Font<'_, 'static>, String> {
    let rwops = RWops::from_bytes(FONT_BYTES)?;
    ttf_ctx.load_font_from_rwops(rwops, point_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::ttf;

    #[test]
    fn can_load_font() {
        let ttf_context = ttf::init().unwrap();
        load_font(&ttf_context, 20).expect("loading font should not fail");
    }
}
