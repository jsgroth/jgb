use crate::AppConfig;
use egui::{Color32, Grid, Ui};
use jgb_core::{ControllerConfig, ControllerInput, HotkeyConfig, InputConfig};
use sdl2::event::Event;
use sdl2::joystick::Joystick;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::ttf;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputType {
    Keyboard,
    Controller { deadzone: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPress {
    Keyboard(Keycode),
    Controller(ControllerInput),
}

pub type InputThread = JoinHandle<anyhow::Result<Option<(ConfigurableInput, InputPress)>>>;

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

struct SingleInput<'a, T> {
    input: ConfigurableInput,
    current_input: Option<T>,
    input_type: InputType,
    field_to_clear: Option<&'a mut Option<T>>,
}

impl<'a, T: std::fmt::Display> SingleInput<'a, T> {
    fn new(input: ConfigurableInput, current_input: Option<T>) -> Self {
        Self {
            input,
            current_input,
            input_type: InputType::Keyboard,
            field_to_clear: None,
        }
    }

    fn add_clear_button(mut self, field_to_clear: &'a mut Option<T>) -> Self {
        self.field_to_clear = Some(field_to_clear);
        self
    }

    fn take_controller_input(mut self, deadzone: u16) -> Self {
        self.input_type = InputType::Controller { deadzone };
        self
    }

    #[must_use]
    fn ui(self, ui: &mut Ui) -> Option<InputThread> {
        ui.label(format!("{}:", self.input.to_str()));

        let button_text = match self.current_input {
            Some(current_input) => current_input.to_string(),
            None => "<None>".into(),
        };
        let thread = if ui.button(button_text).clicked() {
            Some(spawn_input_thread(self.input, self.input_type))
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

pub struct KeyboardSettingsWidget<'a> {
    input_config: &'a InputConfig,
}

impl<'a> KeyboardSettingsWidget<'a> {
    pub fn new(input_config: &'a InputConfig) -> Self {
        Self { input_config }
    }

    #[must_use]
    pub fn ui(self, ui: &mut Ui) -> Option<InputThread> {
        Grid::new("keyboard_settings_grid")
            .show(ui, |ui| {
                [
                    SingleInput::new(ConfigurableInput::Up, Some(self.input_config.up.clone()))
                        .ui(ui),
                    SingleInput::new(
                        ConfigurableInput::Down,
                        Some(self.input_config.down.clone()),
                    )
                    .ui(ui),
                    SingleInput::new(
                        ConfigurableInput::Left,
                        Some(self.input_config.left.clone()),
                    )
                    .ui(ui),
                    SingleInput::new(
                        ConfigurableInput::Right,
                        Some(self.input_config.right.clone()),
                    )
                    .ui(ui),
                    SingleInput::new(ConfigurableInput::A, Some(self.input_config.a.clone()))
                        .ui(ui),
                    SingleInput::new(ConfigurableInput::B, Some(self.input_config.b.clone()))
                        .ui(ui),
                    SingleInput::new(
                        ConfigurableInput::Start,
                        Some(self.input_config.start.clone()),
                    )
                    .ui(ui),
                    SingleInput::new(
                        ConfigurableInput::Select,
                        Some(self.input_config.select.clone()),
                    )
                    .ui(ui),
                ]
                .into_iter()
                .reduce(Option::or)
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
    pub fn ui(self, ui: &mut Ui) -> Option<InputThread> {
        Grid::new("hotkey_settings_grid")
            .show(ui, |ui| {
                [
                    SingleInput::new(
                        ConfigurableInput::HotkeyExit,
                        self.hotkey_config.exit.clone(),
                    )
                    .add_clear_button(&mut self.hotkey_config.exit)
                    .ui(ui),
                    SingleInput::new(
                        ConfigurableInput::HotkeyToggleFullscreen,
                        self.hotkey_config.toggle_fullscreen.clone(),
                    )
                    .add_clear_button(&mut self.hotkey_config.toggle_fullscreen)
                    .ui(ui),
                    SingleInput::new(
                        ConfigurableInput::HotkeySaveState,
                        self.hotkey_config.save_state.clone(),
                    )
                    .add_clear_button(&mut self.hotkey_config.save_state)
                    .ui(ui),
                    SingleInput::new(
                        ConfigurableInput::HotkeyLoadState,
                        self.hotkey_config.load_state.clone(),
                    )
                    .add_clear_button(&mut self.hotkey_config.load_state)
                    .ui(ui),
                ]
                .into_iter()
                .reduce(Option::or)
                .unwrap_or(None)
            })
            .inner
    }
}

pub struct ControllerSettingsWidget<'a> {
    controller_config: &'a mut ControllerConfig,
    deadzone_text: &'a mut String,
}

impl<'a> ControllerSettingsWidget<'a> {
    pub fn new(controller_config: &'a mut ControllerConfig, deadzone_text: &'a mut String) -> Self {
        Self {
            controller_config,
            deadzone_text,
        }
    }

    #[must_use]
    pub fn ui(self, ui: &mut Ui) -> Option<InputThread> {
        let deadzone = self.controller_config.axis_deadzone;
        let thread = Grid::new("controller_settings_grid")
            .show(ui, |ui| {
                [
                    SingleInput::new(ConfigurableInput::Up, self.controller_config.up)
                        .take_controller_input(deadzone)
                        .add_clear_button(&mut self.controller_config.up)
                        .ui(ui),
                    SingleInput::new(ConfigurableInput::Down, self.controller_config.down)
                        .take_controller_input(deadzone)
                        .add_clear_button(&mut self.controller_config.down)
                        .ui(ui),
                    SingleInput::new(ConfigurableInput::Left, self.controller_config.left)
                        .take_controller_input(deadzone)
                        .add_clear_button(&mut self.controller_config.left)
                        .ui(ui),
                    SingleInput::new(ConfigurableInput::Right, self.controller_config.right)
                        .take_controller_input(deadzone)
                        .add_clear_button(&mut self.controller_config.right)
                        .ui(ui),
                    SingleInput::new(ConfigurableInput::A, self.controller_config.a)
                        .take_controller_input(deadzone)
                        .add_clear_button(&mut self.controller_config.a)
                        .ui(ui),
                    SingleInput::new(ConfigurableInput::B, self.controller_config.b)
                        .take_controller_input(deadzone)
                        .add_clear_button(&mut self.controller_config.b)
                        .ui(ui),
                    SingleInput::new(ConfigurableInput::Start, self.controller_config.start)
                        .take_controller_input(deadzone)
                        .add_clear_button(&mut self.controller_config.start)
                        .ui(ui),
                    SingleInput::new(ConfigurableInput::Select, self.controller_config.select)
                        .take_controller_input(deadzone)
                        .add_clear_button(&mut self.controller_config.select)
                        .ui(ui),
                ]
                .into_iter()
                .reduce(Option::or)
                .unwrap_or(None)
            })
            .inner;

        let deadzone_invalid = ui
            .horizontal(|ui| {
                ui.label("Axis deadzone:");
                if !ui.text_edit_singleline(self.deadzone_text).has_focus() {
                    match self.deadzone_text.parse::<u16>() {
                        Ok(deadzone @ 0..=32767) => {
                            self.controller_config.axis_deadzone = deadzone;
                            false
                        }
                        _ => true,
                    }
                } else {
                    false
                }
            })
            .inner;
        if deadzone_invalid {
            ui.colored_label(
                Color32::RED,
                "Deadzone must be an integer between 0 and 32767",
            );
        }

        thread
    }
}

pub fn handle_input_thread_result(thread: InputThread, config: &mut AppConfig) {
    let (button, input_press) = match thread.join().unwrap() {
        Ok(Some((button, input_press))) => (button, input_press),
        Ok(None) => {
            // No change
            return;
        }
        Err(err) => {
            log::error!("key input thread terminated with error: {err}");
            return;
        }
    };

    match input_press {
        InputPress::Keyboard(keycode) => {
            let input_str = keycode.name();
            match button {
                ConfigurableInput::Up => {
                    config.input.up = input_str;
                }
                ConfigurableInput::Down => {
                    config.input.down = input_str;
                }
                ConfigurableInput::Left => {
                    config.input.left = input_str;
                }
                ConfigurableInput::Right => {
                    config.input.right = input_str;
                }
                ConfigurableInput::A => {
                    config.input.a = input_str;
                }
                ConfigurableInput::B => {
                    config.input.b = input_str;
                }
                ConfigurableInput::Start => {
                    config.input.start = input_str;
                }
                ConfigurableInput::Select => {
                    config.input.select = input_str;
                }
                ConfigurableInput::HotkeyExit => {
                    config.hotkeys.exit = Some(input_str);
                }
                ConfigurableInput::HotkeyToggleFullscreen => {
                    config.hotkeys.toggle_fullscreen = Some(input_str);
                }
                ConfigurableInput::HotkeySaveState => {
                    config.hotkeys.save_state = Some(input_str);
                }
                ConfigurableInput::HotkeyLoadState => {
                    config.hotkeys.load_state = Some(input_str);
                }
            }
        }
        InputPress::Controller(controller_input) => match button {
            ConfigurableInput::Up => {
                config.controller.up = Some(controller_input);
            }
            ConfigurableInput::Down => {
                config.controller.down = Some(controller_input);
            }
            ConfigurableInput::Left => {
                config.controller.left = Some(controller_input);
            }
            ConfigurableInput::Right => {
                config.controller.right = Some(controller_input);
            }
            ConfigurableInput::A => {
                config.controller.a = Some(controller_input);
            }
            ConfigurableInput::B => {
                config.controller.b = Some(controller_input);
            }
            ConfigurableInput::Start => {
                config.controller.start = Some(controller_input);
            }
            ConfigurableInput::Select => {
                config.controller.select = Some(controller_input);
            }
            ConfigurableInput::HotkeyExit
            | ConfigurableInput::HotkeyToggleFullscreen
            | ConfigurableInput::HotkeySaveState
            | ConfigurableInput::HotkeyLoadState => {
                panic!("should never attempt to set a hotkey to a controller input");
            }
        },
    }
}

#[must_use]
fn spawn_input_thread(button: ConfigurableInput, input_type: InputType) -> InputThread {
    thread::spawn(move || {
        let sdl = sdl2::init().map_err(anyhow::Error::msg)?;
        let video = sdl.video().map_err(anyhow::Error::msg)?;
        let joystick = sdl.joystick().map_err(anyhow::Error::msg)?;

        let window_title = match input_type {
            InputType::Keyboard => "Press a key...",
            InputType::Controller { .. } => "Press a button...",
        };

        let window = video.window(window_title, 400, 100).build()?;
        let mut canvas = window.into_canvas().build()?;

        let ttf_context = ttf::init()?;
        let font = ttf_context
            .load_font("fonts/IBMPlexMono-Bold.ttf", 40)
            .map_err(anyhow::Error::msg)?;
        let rendered_text = font.render(window_title).solid(Color::RGB(255, 255, 255))?;

        let texture_creator = canvas.texture_creator();
        let font_texture = rendered_text.as_texture(&texture_creator)?;
        canvas
            .copy(&font_texture, None, None)
            .map_err(anyhow::Error::msg)?;

        canvas.present();

        let mut joysticks: Vec<Joystick> = Vec::new();

        let mut event_pump = sdl.event_pump().map_err(anyhow::Error::msg)?;
        loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::KeyDown {
                        keycode: Some(keycode),
                        ..
                    } => {
                        if input_type == InputType::Keyboard {
                            return Ok(Some((button, InputPress::Keyboard(keycode))));
                        }
                    }
                    Event::JoyDeviceAdded { which, .. } => {
                        if matches!(input_type, InputType::Controller { .. }) {
                            joysticks.push(joystick.open(which)?);
                        }
                    }
                    Event::JoyButtonDown { button_idx, .. } => {
                        return Ok(Some((
                            button,
                            InputPress::Controller(ControllerInput::Button(button_idx)),
                        )));
                    }
                    Event::JoyAxisMotion {
                        axis_idx, value, ..
                    } => {
                        if let InputType::Controller { deadzone } = input_type {
                            if value.saturating_abs() as u16 >= deadzone {
                                let input = if value > 0 {
                                    ControllerInput::AxisPositive(axis_idx)
                                } else {
                                    ControllerInput::AxisNegative(axis_idx)
                                };
                                return Ok(Some((button, InputPress::Controller(input))));
                            }
                        }
                    }
                    Event::Quit { .. } => {
                        return Ok(None);
                    }
                    _ => {}
                }
            }

            thread::sleep(Duration::from_millis(1));
        }
    })
}
