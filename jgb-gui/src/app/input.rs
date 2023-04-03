use crate::AppConfig;
use egui::{Grid, Ui};
use jgb_core::{HotkeyConfig, InputConfig};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub type KeyInputThread = JoinHandle<Result<(ConfigurableInput, Keycode), anyhow::Error>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurableInput {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
    HotkeyExit,
    HotkeyToggleFullscreen,
    HotkeySaveState,
    HotkeyLoadState,
}

impl ConfigurableInput {
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
            Self::HotkeyExit => "Exit",
            Self::HotkeyToggleFullscreen => "Toggle Fullscreen",
            Self::HotkeySaveState => "Save State",
            Self::HotkeyLoadState => "Load State",
        }
    }
}

struct SingleKeyInput<'a> {
    input: ConfigurableInput,
    current_keycode: String,
    field_to_clear: Option<&'a mut Option<String>>,
}

impl<'a> SingleKeyInput<'a> {
    fn new(input: ConfigurableInput, current_keycode: String) -> Self {
        Self {
            input,
            current_keycode,
            field_to_clear: None,
        }
    }

    fn add_clear_button(mut self, field_to_clear: &'a mut Option<String>) -> Self {
        self.field_to_clear = Some(field_to_clear);
        self
    }

    #[must_use]
    fn ui(self, ui: &mut Ui) -> Option<KeyInputThread> {
        ui.label(format!("{}:", self.input.to_str()));

        let thread = if ui.button(self.current_keycode).clicked() {
            Some(spawn_key_input_thread(self.input))
        } else {
            None
        };

        if let Some(field_to_clear) = self.field_to_clear {
            if ui.button("Clear").clicked() {
                *field_to_clear = None;
            }
        }

        ui.end_row();

        thread
    }
}

pub struct InputSettingsWidget<'a> {
    input_config: &'a InputConfig,
}

impl<'a> InputSettingsWidget<'a> {
    pub fn new(input_config: &'a InputConfig) -> Self {
        Self { input_config }
    }

    #[must_use]
    pub fn ui(self, ui: &mut Ui) -> Option<KeyInputThread> {
        Grid::new("input_settings_grid")
            .show(ui, |ui| {
                [
                    SingleKeyInput::new(ConfigurableInput::Up, self.input_config.up.clone()).ui(ui),
                    SingleKeyInput::new(ConfigurableInput::Down, self.input_config.down.clone())
                        .ui(ui),
                    SingleKeyInput::new(ConfigurableInput::Left, self.input_config.left.clone())
                        .ui(ui),
                    SingleKeyInput::new(ConfigurableInput::Right, self.input_config.right.clone())
                        .ui(ui),
                    SingleKeyInput::new(ConfigurableInput::A, self.input_config.a.clone()).ui(ui),
                    SingleKeyInput::new(ConfigurableInput::B, self.input_config.b.clone()).ui(ui),
                    SingleKeyInput::new(ConfigurableInput::Start, self.input_config.start.clone())
                        .ui(ui),
                    SingleKeyInput::new(
                        ConfigurableInput::Select,
                        self.input_config.select.clone(),
                    )
                    .ui(ui),
                ]
                .into_iter()
                .reduce(|a, b| a.or(b))
                .unwrap_or(None)
            })
            .inner
    }
}

pub struct HotkeySettingsWidget<'a> {
    hotkey_config: &'a mut HotkeyConfig,
}

impl<'a> HotkeySettingsWidget<'a> {
    pub fn new(hotkey_config: &'a mut HotkeyConfig) -> Self {
        Self { hotkey_config }
    }

    #[must_use]
    pub fn ui(self, ui: &mut Ui) -> Option<KeyInputThread> {
        Grid::new("hotkey_settings_grid")
            .show(ui, |ui| {
                [
                    SingleKeyInput::new(
                        ConfigurableInput::HotkeyExit,
                        self.hotkey_config.exit.clone().unwrap_or("<None>".into()),
                    )
                    .add_clear_button(&mut self.hotkey_config.exit)
                    .ui(ui),
                    SingleKeyInput::new(
                        ConfigurableInput::HotkeyToggleFullscreen,
                        self.hotkey_config
                            .toggle_fullscreen
                            .clone()
                            .unwrap_or("<None>".into()),
                    )
                    .add_clear_button(&mut self.hotkey_config.toggle_fullscreen)
                    .ui(ui),
                    SingleKeyInput::new(
                        ConfigurableInput::HotkeySaveState,
                        self.hotkey_config
                            .save_state
                            .clone()
                            .unwrap_or("<None>".into()),
                    )
                    .add_clear_button(&mut self.hotkey_config.save_state)
                    .ui(ui),
                    SingleKeyInput::new(
                        ConfigurableInput::HotkeyLoadState,
                        self.hotkey_config
                            .load_state
                            .clone()
                            .unwrap_or("<None>".into()),
                    )
                    .add_clear_button(&mut self.hotkey_config.load_state)
                    .ui(ui),
                ]
                .into_iter()
                .reduce(|a, b| a.or(b))
                .unwrap_or(None)
            })
            .inner
    }
}

pub fn handle_key_input_thread_result(thread: KeyInputThread, config: &mut AppConfig) {
    let (button, keycode) = match thread.join().unwrap() {
        Ok((button, keycode)) => (button, keycode),
        Err(err) => {
            log::error!("key input thread terminated with error: {err}");
            return;
        }
    };

    let name = keycode.name();
    match button {
        ConfigurableInput::Up => {
            config.input.up = name;
        }
        ConfigurableInput::Down => {
            config.input.down = name;
        }
        ConfigurableInput::Left => {
            config.input.left = name;
        }
        ConfigurableInput::Right => {
            config.input.right = name;
        }
        ConfigurableInput::A => {
            config.input.a = name;
        }
        ConfigurableInput::B => {
            config.input.b = name;
        }
        ConfigurableInput::Start => {
            config.input.start = name;
        }
        ConfigurableInput::Select => {
            config.input.select = name;
        }
        ConfigurableInput::HotkeyExit => {
            config.hotkeys.exit = Some(name);
        }
        ConfigurableInput::HotkeyToggleFullscreen => {
            config.hotkeys.toggle_fullscreen = Some(name);
        }
        ConfigurableInput::HotkeySaveState => {
            config.hotkeys.save_state = Some(name);
        }
        ConfigurableInput::HotkeyLoadState => {
            config.hotkeys.load_state = Some(name);
        }
    }
}

#[must_use]
fn spawn_key_input_thread(button: ConfigurableInput) -> KeyInputThread {
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
