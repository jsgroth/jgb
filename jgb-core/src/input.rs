use crate::cpu::InterruptType;
use crate::memory::ioregisters::IoRegisters;
use sdl2::keyboard::Keycode;

#[derive(Debug, Clone)]
pub struct JoypadState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,
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

    fn get_field_mut(&mut self, keycode: Keycode) -> Option<&mut bool> {
        match keycode {
            Keycode::Up => Some(&mut self.up),
            Keycode::Down => Some(&mut self.down),
            Keycode::Left => Some(&mut self.left),
            Keycode::Right => Some(&mut self.right),
            Keycode::Z => Some(&mut self.a),
            Keycode::X => Some(&mut self.b),
            Keycode::Return => Some(&mut self.start),
            Keycode::RShift => Some(&mut self.select),
            _ => None,
        }
    }

    pub fn key_down(&mut self, keycode: Keycode) {
        if let Some(field) = self.get_field_mut(keycode) {
            *field = true;
        }
        log::debug!("Key pressed: {keycode}, current state: {self:?}")
    }

    pub fn key_up(&mut self, keycode: Keycode) {
        if let Some(field) = self.get_field_mut(keycode) {
            *field = false;
        }
        log::debug!("Key released: {keycode}, current state: {self:?}")
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
