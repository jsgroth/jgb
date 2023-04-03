use crate::app::config::AppInputConfig;
use egui::{Grid, Ui};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub type KeyInputThread = JoinHandle<Result<(GbButton, Keycode), anyhow::Error>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GbButton {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

impl GbButton {
    fn to_str(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::A => "A",
            Self::B => "B",
            Self::Start => "Start",
            Self::Select => "Select",
        }
    }
}

struct SingleKeyInput {
    button: GbButton,
    current_keycode: String,
}

impl SingleKeyInput {
    fn new(button: GbButton, current_keycode: String) -> Self {
        Self {
            button,
            current_keycode,
        }
    }

    #[must_use]
    fn ui(self, ui: &mut Ui) -> Option<KeyInputThread> {
        ui.label(format!("{}:", self.button.to_str()));

        let thread = if ui.button(self.current_keycode).clicked() {
            Some(spawn_key_input_thread(self.button))
        } else {
            None
        };

        ui.end_row();

        thread
    }
}

pub struct KeyInputWidget<'a> {
    input_config: &'a AppInputConfig,
}

impl<'a> KeyInputWidget<'a> {
    pub fn new(input_config: &'a AppInputConfig) -> Self {
        Self { input_config }
    }

    #[must_use]
    pub fn ui(self, ui: &mut Ui) -> Option<KeyInputThread> {
        Grid::new("input_settings_grid")
            .show(ui, |ui| {
                SingleKeyInput::new(GbButton::Up, self.input_config.up.clone())
                    .ui(ui)
                    .or(SingleKeyInput::new(GbButton::Down, self.input_config.down.clone()).ui(ui))
                    .or(SingleKeyInput::new(GbButton::Left, self.input_config.left.clone()).ui(ui))
                    .or(
                        SingleKeyInput::new(GbButton::Right, self.input_config.right.clone())
                            .ui(ui),
                    )
                    .or(SingleKeyInput::new(GbButton::A, self.input_config.a.clone()).ui(ui))
                    .or(SingleKeyInput::new(GbButton::B, self.input_config.b.clone()).ui(ui))
                    .or(
                        SingleKeyInput::new(GbButton::Start, self.input_config.start.clone())
                            .ui(ui),
                    )
                    .or(
                        SingleKeyInput::new(GbButton::Select, self.input_config.select.clone())
                            .ui(ui),
                    )
            })
            .inner
    }
}

pub fn handle_key_input_thread_result(thread: KeyInputThread, input_config: &mut AppInputConfig) {
    let (button, keycode) = match thread.join().unwrap() {
        Ok((button, keycode)) => (button, keycode),
        Err(err) => {
            log::error!("key input thread terminated with error: {err}");
            return;
        }
    };

    let config_field = match button {
        GbButton::Up => &mut input_config.up,
        GbButton::Down => &mut input_config.down,
        GbButton::Left => &mut input_config.left,
        GbButton::Right => &mut input_config.right,
        GbButton::A => &mut input_config.a,
        GbButton::B => &mut input_config.b,
        GbButton::Start => &mut input_config.start,
        GbButton::Select => &mut input_config.select,
    };
    *config_field = keycode.name();
}

#[must_use]
fn spawn_key_input_thread(button: GbButton) -> KeyInputThread {
    thread::spawn(move || {
        let sdl = sdl2::init().map_err(anyhow::Error::msg)?;
        let video = sdl.video().map_err(anyhow::Error::msg)?;

        let window = video.window("Press a key...", 200, 100).build()?;
        let mut canvas = window.into_canvas().build()?;
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();
        canvas.present();

        let mut event_pump = sdl.event_pump().map_err(anyhow::Error::msg)?;
        loop {
            for event in event_pump.poll_iter() {
                if let Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } = event
                {
                    return Ok((button, keycode));
                }
            }

            thread::sleep(Duration::from_millis(1));
        }
    })
}
