use crate::cpu::instructions;
use crate::cpu::instructions::{ExecutionError, ParseError};
use crate::memory::ioregisters::IoRegisters;
use crate::startup::SdlState;
use crate::EmulationState;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunError {
    #[error("error parsing CPU instruction: {source}")]
    InstructionParse {
        #[from]
        source: ParseError,
    },
    #[error("error executing CPU instruction: {source}")]
    InstructionExecute {
        #[from]
        source: ExecutionError,
    },
}

#[derive(Debug, Clone)]
struct JoypadState {
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
    fn new() -> Self {
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

    fn key_down(&mut self, keycode: Keycode) {
        if let Some(field) = self.get_field_mut(keycode) {
            *field = true;
        }
        log::debug!("Key pressed: {keycode}, current state: {self:?}")
    }

    fn key_up(&mut self, keycode: Keycode) {
        if let Some(field) = self.get_field_mut(keycode) {
            *field = false;
        }
        log::debug!("Key released: {keycode}, current state: {self:?}")
    }
}

pub fn run(emulation_state: EmulationState, sdl_state: SdlState) -> Result<(), RunError> {
    let EmulationState {
        mut address_space,
        mut cpu_registers,
        ppu_state,
    } = emulation_state;

    let SdlState {
        sdl,
        video,
        canvas,
        game_controller,
        mut event_pump,
    } = sdl_state;

    let mut joypad_state = JoypadState::new();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    joypad_state.key_down(keycode);
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    joypad_state.key_up(keycode);
                }
                _ => {}
            }
        }

        update_joyp_register(&joypad_state, address_space.get_io_registers_mut());

        // TODO check interrupts here

        let (instruction, pc) =
            instructions::parse_next_instruction(&address_space, cpu_registers.pc, &ppu_state)?;

        log::trace!("Updating PC from 0x{:04X} to {:04X}", cpu_registers.pc, pc);
        cpu_registers.pc = pc;

        let cycles_required = instruction.cycles_required(&cpu_registers);

        log::trace!("Executing instruction {instruction:04X?}, will take {cycles_required} cycles");
        instruction.execute(&mut address_space, &mut cpu_registers, &ppu_state)?;

        // TODO execute PPU here

        // TODO update timer registers here

        // TODO if frame completed, sleep here until next frametime

        // *CPU/PPU main loop (continue until complete frame is rendered, then sleep until next frametime)*
        //
        // update JOYP register based on user inputs and JOYP request bits
        //
        // if OAM DMA is executing:
        //   ???
        // else if IME && !interrupt_delay && (IE & IF != 0):
        //   execute interrupt service routine
        // else:
        //   fetch next instruction
        //   update PC
        //   execute next instruction
        //
        // update timer registers based on # of cycles required to execute last instruction
        //
        // execute PPU based on # of cycles required to execute last instruction
    }

    Ok(())
}

fn update_joyp_register(joypad_state: &JoypadState, io_registers: &mut IoRegisters) {
    let joyp = io_registers.privileged_read_joyp();
    let actions_select = joyp & 0x20 == 0;
    let directions_select = joyp & 0x10 == 0;

    let bit_3 = !(actions_select && joypad_state.start || directions_select && joypad_state.down);
    let bit_2 = !(actions_select && joypad_state.select || directions_select && joypad_state.up);
    let bit_1 = !(actions_select && joypad_state.b || directions_select && joypad_state.left);
    let bit_0 = !(actions_select && joypad_state.a || directions_select && joypad_state.right);

    let new_joyp = (joyp & 0x30)
        | (u8::from(bit_3) << 3)
        | (u8::from(bit_2) << 2)
        | (u8::from(bit_1) << 1)
        | u8::from(bit_0);
    io_registers.privileged_set_joyp(new_joyp);
}
