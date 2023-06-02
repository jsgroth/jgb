use crate::config::{ControllerConfig, ControllerInput, HatDirection, InputConfig};
use crate::cpu::InterruptType;
use crate::memory::ioregisters::{IoRegister, IoRegisters};
use crate::HotkeyConfig;
use sdl2::controller::GameController;
use sdl2::joystick::{HatState, Joystick};
use sdl2::keyboard::Keycode;
use sdl2::sensor::SensorType;
use sdl2::{GameControllerSubsystem, IntegerOrSdlError, JoystickSubsystem};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

#[derive(Error, Debug)]
pub enum KeyMapError {
    #[error("invalid keycode in input config: {keycode}")]
    InvalidKeycode { keycode: String },
    #[error("keycode used for multiple buttons: {keycode}")]
    DuplicateKeycode { keycode: String },
}

#[derive(Debug, Error)]
pub enum JoystickError {
    #[error("error opening joystick device: {source}")]
    DeviceOpen {
        #[source]
        source: IntegerOrSdlError,
    },
    #[error("controller input used for multiple buttons: {input}")]
    DuplicateInput { input: ControllerInput },
    #[error("axis deadzone must be at most {}, was: {deadzone}", i16::MAX)]
    InvalidDeadzone { deadzone: u16 },
    #[error("error enabling accelerometer: {source}")]
    AccelerometerEnable {
        #[source]
        source: IntegerOrSdlError,
    },
}

fn try_parse_keycode(s: &str) -> Result<Keycode, KeyMapError> {
    Keycode::from_name(s).ok_or_else(|| KeyMapError::InvalidKeycode { keycode: s.into() })
}

#[derive(Debug, Clone)]
pub struct KeyMap(HashMap<Keycode, Button>);

impl KeyMap {
    pub fn from_config(input_config: &InputConfig) -> Result<Self, KeyMapError> {
        let mut map = HashMap::new();
        for (button, config_input) in [
            (Button::Up, &input_config.up),
            (Button::Down, &input_config.down),
            (Button::Left, &input_config.left),
            (Button::Right, &input_config.right),
            (Button::A, &input_config.a),
            (Button::B, &input_config.b),
            (Button::Start, &input_config.start),
            (Button::Select, &input_config.select),
        ] {
            let keycode = try_parse_keycode(config_input)?;
            if map.insert(keycode, button).is_some() {
                return Err(KeyMapError::DuplicateKeycode {
                    keycode: keycode.name(),
                });
            }
        }

        Ok(Self(map))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotkey {
    Exit,
    ToggleFullscreen,
    SaveState,
    LoadState,
    FastForward,
}

#[derive(Debug, Clone)]
pub struct HotkeyMap(HashMap<Keycode, Hotkey>);

impl HotkeyMap {
    pub fn from_config(hotkey_config: &HotkeyConfig) -> Result<Self, KeyMapError> {
        let mut map = HashMap::new();
        for (hotkey, config_input) in [
            (Hotkey::Exit, hotkey_config.exit.as_ref()),
            (
                Hotkey::ToggleFullscreen,
                hotkey_config.toggle_fullscreen.as_ref(),
            ),
            (Hotkey::SaveState, hotkey_config.save_state.as_ref()),
            (Hotkey::LoadState, hotkey_config.load_state.as_ref()),
            (Hotkey::FastForward, hotkey_config.fast_forward.as_ref()),
        ] {
            if let Some(keycode) = config_input {
                let keycode = try_parse_keycode(keycode)?;
                if map.insert(keycode, hotkey).is_some() {
                    return Err(KeyMapError::DuplicateKeycode {
                        keycode: keycode.name(),
                    });
                }
            }
        }

        Ok(Self(map))
    }
}

#[derive(Debug, Clone)]
pub struct ControllerMap {
    map: HashMap<ControllerInput, Button>,
    axis_deadzone: i16,
}

impl ControllerMap {
    pub fn from_config(controller_config: &ControllerConfig) -> Result<Self, JoystickError> {
        let axis_deadzone: i16 = controller_config.axis_deadzone.try_into().map_err(|_err| {
            JoystickError::InvalidDeadzone {
                deadzone: controller_config.axis_deadzone,
            }
        })?;

        let mut map = HashMap::new();
        for (button, config_input) in [
            (Button::Up, controller_config.up),
            (Button::Down, controller_config.down),
            (Button::Left, controller_config.left),
            (Button::Right, controller_config.right),
            (Button::A, controller_config.a),
            (Button::B, controller_config.b),
            (Button::Start, controller_config.start),
            (Button::Select, controller_config.select),
        ] {
            if let Some(input) = config_input {
                if map.insert(input, button).is_some() {
                    return Err(JoystickError::DuplicateInput { input });
                }
            }
        }

        Ok(Self { map, axis_deadzone })
    }
}

// This struct exists to keep connected Joystick values alive, as SDL will stop generating joystick
// events once the corresponding Joystick value is dropped
pub struct Joysticks<'joy, 'gc> {
    joystick_subsystem: &'joy JoystickSubsystem,
    controller_subsystem: &'gc GameControllerSubsystem,
    joysticks: HashMap<u32, Joystick>,
    controllers: HashMap<u32, GameController>,
}

impl<'joy, 'gc> Joysticks<'joy, 'gc> {
    pub fn new(
        joystick_subsystem: &'joy JoystickSubsystem,
        controller_subsystem: &'gc GameControllerSubsystem,
    ) -> Self {
        Self {
            joystick_subsystem,
            controller_subsystem,
            joysticks: HashMap::new(),
            controllers: HashMap::new(),
        }
    }

    pub fn joy_device_added(&mut self, which: u32) -> Result<(), JoystickError> {
        let joystick = self
            .joystick_subsystem
            .open(which)
            .map_err(|source| JoystickError::DeviceOpen { source })?;
        log::info!(
            "Joystick connected: {} ({})",
            joystick.name(),
            joystick.guid()
        );
        self.joysticks.insert(which, joystick);

        Ok(())
    }

    pub fn joy_device_removed(&mut self, which: u32) {
        if let Some(removed) = self.joysticks.remove(&which) {
            log::info!(
                "Joystick disconnected: {} ({})",
                removed.name(),
                removed.guid()
            );
        }
    }

    pub fn controller_device_added(
        &mut self,
        which: u32,
        accelerometer_enabled: bool,
    ) -> Result<(), JoystickError> {
        if !accelerometer_enabled {
            log::info!(
                "Not opening game controller idx {which} because accelerometer is not enabled"
            );
            return Ok(());
        }

        let controller = self
            .controller_subsystem
            .open(which)
            .map_err(|source| JoystickError::DeviceOpen { source })?;

        log::info!("Game controller connected: {}", controller.name());

        if controller.has_sensor(SensorType::Accelerometer) {
            controller
                .sensor_set_enabled(SensorType::Accelerometer, true)
                .map_err(|err| JoystickError::AccelerometerEnable { source: err })?;
            log::info!("Enabled accelerometer");
        } else {
            log::info!("Controller does not have an accelerometer");
        }

        self.controllers.insert(which, controller);

        Ok(())
    }

    pub fn controller_device_removed(&mut self, which: u32) {
        if let Some(removed) = self.controllers.remove(&which) {
            log::info!("Game controller disconnected: {}", removed.name());
        }
    }

    pub fn set_rumble(&mut self, rumble_motor_on: bool) {
        let rumble_intensity = u16::from(rumble_motor_on) * 0xFFFF;
        for joystick in self.joysticks.values_mut() {
            // Ignore errors because attempting to toggle rumble on a controller that doesn't
            // support rumble will result in an error, and there doesn't appear to be a way to
            // check beforehand that a controller supports rumble
            let _: Result<(), IntegerOrSdlError> =
                joystick.set_rumble(rumble_intensity, rumble_intensity, 50);
        }
    }
}

#[derive(Debug, Clone)]
pub struct JoypadState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    a: bool,
    b: bool,
    start: bool,
    select: bool,
    last_joystick_axis_values: [i16; 256],
}

impl JoypadState {
    pub fn new() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
            a: false,
            b: false,
            start: false,
            select: false,
            // "Map" from axis index to last value
            last_joystick_axis_values: [0; 256],
        }
    }

    fn get_field_mut(&mut self, button: Option<Button>) -> Option<&mut bool> {
        match button {
            Some(Button::Up) => Some(&mut self.up),
            Some(Button::Down) => Some(&mut self.down),
            Some(Button::Left) => Some(&mut self.left),
            Some(Button::Right) => Some(&mut self.right),
            Some(Button::A) => Some(&mut self.a),
            Some(Button::B) => Some(&mut self.b),
            Some(Button::Start) => Some(&mut self.start),
            Some(Button::Select) => Some(&mut self.select),
            None => None,
        }
    }

    pub fn key_down(&mut self, keycode: Keycode, key_map: &KeyMap) {
        if let Some(field) = self.get_field_mut(key_map.0.get(&keycode).copied()) {
            *field = true;
        }
        log::debug!("Key pressed: {keycode}, current state: {self:?}");
    }

    pub fn key_up(&mut self, keycode: Keycode, key_map: &KeyMap) {
        if let Some(field) = self.get_field_mut(key_map.0.get(&keycode).copied()) {
            *field = false;
        }
        log::debug!("Key released: {keycode}, current state: {self:?}");
    }

    pub fn joy_button_down(&mut self, button: u8, controller_map: &ControllerMap) {
        let input = ControllerInput::Button(button);
        if let Some(field) = self.get_field_mut(controller_map.map.get(&input).copied()) {
            *field = true;
        }
        log::debug!("Joy button pressed: {button}, current state: {self:?}");
    }

    pub fn joy_button_up(&mut self, button: u8, controller_map: &ControllerMap) {
        let input = ControllerInput::Button(button);
        if let Some(field) = self.get_field_mut(controller_map.map.get(&input).copied()) {
            *field = false;
        }
        log::debug!("Joy button released: {button}, current state: {self:?}");
    }

    pub fn joy_axis_motion(&mut self, axis: u8, value: i16, controller_map: &ControllerMap) {
        // Apply deadzone, use saturating_abs so as not to leave i16::MIN as a negative number
        let value = if value.saturating_abs() < controller_map.axis_deadzone {
            0
        } else {
            value
        };

        // Don't bother checking anything if the value hasn't changed; JoyAxisMotion events are
        // very frequent on any controller with analog sticks
        if value == self.last_joystick_axis_values[axis as usize] {
            return;
        }
        self.last_joystick_axis_values[axis as usize] = value;

        let (pos_state, neg_state) = match value.cmp(&0) {
            Ordering::Greater => (true, false),
            Ordering::Less => (false, true),
            Ordering::Equal => (false, false),
        };

        let pos_button = controller_map
            .map
            .get(&ControllerInput::AxisPositive(axis))
            .copied();
        let neg_button = controller_map
            .map
            .get(&ControllerInput::AxisNegative(axis))
            .copied();
        if let Some(field) = self.get_field_mut(pos_button) {
            *field = pos_state;
        }
        if let Some(field) = self.get_field_mut(neg_button) {
            *field = neg_state;
        }
        log::debug!("Joy axis motion: axis={axis}, value={value}, current state: {self:?}");
    }

    pub fn hat_motion(&mut self, hat: u8, state: HatState, controller_map: &ControllerMap) {
        let hat_up = matches!(state, HatState::Up | HatState::LeftUp | HatState::RightUp);
        let hat_down = matches!(
            state,
            HatState::Down | HatState::LeftDown | HatState::RightDown
        );
        let hat_left = matches!(
            state,
            HatState::Left | HatState::LeftUp | HatState::LeftDown
        );
        let hat_right = matches!(
            state,
            HatState::Right | HatState::RightUp | HatState::RightDown
        );

        for (state, direction) in [
            (hat_up, HatDirection::Up),
            (hat_down, HatDirection::Down),
            (hat_left, HatDirection::Left),
            (hat_right, HatDirection::Right),
        ] {
            let button = controller_map
                .map
                .get(&ControllerInput::Hat(hat, direction))
                .copied();
            if let Some(button) = self.get_field_mut(button) {
                *button = state;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AccelerometerState {
    pub x: u16,
    pub y: u16,
}

impl Default for AccelerometerState {
    fn default() -> Self {
        Self {
            x: 0x8000,
            y: 0x8000,
        }
    }
}

impl AccelerometerState {
    const ACCELEROMETER_CENTER: f32 = 0x81D0 as f32;
    const ACCELEROMETER_GRAVITY: f32 = 0x70 as f32;
    const GRAVITATIONAL_ACCELERATION_M_S2: f32 = 9.80665;

    pub fn update_from_sdl_values(&mut self, values: [f32; 3]) {
        // GBC accelerometer x and y axes correspond to x and z in SDL's accelerometer definition
        let [x, _, z] = values;

        self.x = Self::sdl_value_to_u16(x);
        self.y = Self::sdl_value_to_u16(z);
    }

    fn sdl_value_to_u16(value: f32) -> u16 {
        // SDL values are in m/s^2 with sign indicating direction.
        // The GBC expects a u16 value centered at 0x81D0, in units where acceleration due to
        // gravity is roughly 0x70.
        let value = (Self::ACCELEROMETER_CENTER
            + value * Self::ACCELEROMETER_GRAVITY / Self::GRAVITATIONAL_ACCELERATION_M_S2)
            .round();

        if value < 0.0 {
            0
        } else if value > f32::from(u16::MAX) {
            u16::MAX
        } else {
            value as u16
        }
    }
}

fn should_flag_interrupt(old_joyp: u8, new_joyp: u8) -> bool {
    for bit in [0x01, 0x02, 0x04, 0x08] {
        if old_joyp & bit != 0 && new_joyp & bit == 0 {
            return true;
        }
    }
    false
}

/// Update the contents of the JOYP hardware register based on the current joypad state, and request
/// a joypad interrupt if any selected buttons have been pressed.
///
/// This needs to be called after every CPU instruction because the CPU can write to the JOYP
/// register to specify whether it wants to read directions or button presses, and the same register
/// bits are used for both.
pub fn update_joyp_register(joypad_state: &JoypadState, io_registers: &mut IoRegisters) {
    let joyp = io_registers.read_register(IoRegister::JOYP);
    let actions_select = joyp & 0x20 == 0;
    let directions_select = joyp & 0x10 == 0;

    let bit_3 =
        !((actions_select && joypad_state.start) || (directions_select && joypad_state.down));
    let bit_2 =
        !((actions_select && joypad_state.select) || (directions_select && joypad_state.up));
    let bit_1 = !((actions_select && joypad_state.b) || (directions_select && joypad_state.left));
    let bit_0 = !((actions_select && joypad_state.a) || (directions_select && joypad_state.right));

    let new_joyp = (joyp & 0x30)
        | (u8::from(bit_3) << 3)
        | (u8::from(bit_2) << 2)
        | (u8::from(bit_1) << 1)
        | u8::from(bit_0);
    io_registers.privileged_set_joyp(new_joyp);

    if should_flag_interrupt(joyp, new_joyp) {
        io_registers.interrupt_flags().set(InterruptType::Joypad);
    }
}

#[must_use]
pub fn check_for_hotkey(key_down: Keycode, hotkey_map: &HotkeyMap) -> Option<Hotkey> {
    hotkey_map.0.get(&key_down).copied()
}
