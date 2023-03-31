use crate::config::InputConfig;
use crate::cpu::InterruptType;
use crate::memory::ioregisters::IoRegisters;
use sdl2::keyboard::Keycode;
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

fn try_parse_keycode(s: &str) -> Result<Keycode, KeyMapError> {
    Keycode::from_name(s).ok_or_else(|| KeyMapError::InvalidKeycode { keycode: s.into() })
}

macro_rules! build_key_map {
    ($($config_field:expr => $button:expr),+$(,)?) => {
        {
            let mut map = std::collections::HashMap::new();

            $(
                let keycode = try_parse_keycode(&$config_field)?;
                if let Some(_) = map.insert(keycode, $button) {
                    Err(KeyMapError::DuplicateKeycode { keycode: keycode.name() })?;
                }
            )*

            map
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyMap(HashMap<Keycode, Button>);

impl KeyMap {
    pub fn from_config(input_config: &InputConfig) -> Result<Self, KeyMapError> {
        let map = build_key_map!(
            input_config.up_keycode => Button::Up,
            input_config.down_keycode => Button::Down,
            input_config.left_keycode => Button::Left,
            input_config.right_keycode => Button::Right,
            input_config.a_keycode => Button::A,
            input_config.b_keycode => Button::B,
            input_config.start_keycode => Button::Start,
            input_config.select_keycode => Button::Select,
        );

        Ok(Self(map))
    }

    fn map(&self, keycode: Keycode) -> Option<Button> {
        self.0.get(&keycode).copied()
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
        }
    }

    fn get_field_mut(&mut self, keycode: Keycode, key_map: &KeyMap) -> Option<&mut bool> {
        match key_map.map(keycode) {
            Some(Button::Up) => Some(&mut self.up),
            Some(Button::Down) => Some(&mut self.down),
            Some(Button::Left) => Some(&mut self.left),
            Some(Button::Right) => Some(&mut self.right),
            Some(Button::A) => Some(&mut self.a),
            Some(Button::B) => Some(&mut self.b),
            Some(Button::Start) => Some(&mut self.start),
            Some(Button::Select) => Some(&mut self.select),
            _ => None,
        }
    }

    pub fn key_down(&mut self, keycode: Keycode, key_map: &KeyMap) {
        if let Some(field) = self.get_field_mut(keycode, key_map) {
            *field = true;
        }
        log::debug!("Key pressed: {keycode}, current state: {self:?}");
    }

    pub fn key_up(&mut self, keycode: Keycode, key_map: &KeyMap) {
        if let Some(field) = self.get_field_mut(keycode, key_map) {
            *field = false;
        }
        log::debug!("Key released: {keycode}, current state: {self:?}");
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
    let joyp = io_registers.privileged_read_joyp();
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
