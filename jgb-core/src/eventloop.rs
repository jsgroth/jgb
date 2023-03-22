use crate::cpu::instructions;
use crate::cpu::instructions::{ExecutionError, ParseError};
use crate::input::JoypadState;
use crate::startup::SdlState;
use crate::timer::TimerCounter;
use crate::{input, timer, EmulationState};
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
    let mut timer_counter = TimerCounter::new();

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

        input::update_joyp_register(&joypad_state, address_space.get_io_registers_mut());

        // Read TMA register before executing anything in case the instruction updates the register
        let timer_modulo = timer::read_timer_modulo(address_space.get_io_registers());

        // TODO check interrupts here

        let (instruction, pc) =
            instructions::parse_next_instruction(&address_space, cpu_registers.pc, &ppu_state)?;

        log::trace!("Updating PC from 0x{:04X} to {:04X}", cpu_registers.pc, pc);
        cpu_registers.pc = pc;

        let cycles_required = instruction.cycles_required(&cpu_registers);

        log::trace!("Executing instruction {instruction:04X?}, will take {cycles_required} cycles");
        instruction.execute(&mut address_space, &mut cpu_registers, &ppu_state)?;

        // TODO execute PPU here
        // TODO OAM DMA transfer - here or in PPU code?

        timer::update_timer_registers(
            address_space.get_io_registers_mut(),
            &mut timer_counter,
            timer_modulo,
            cycles_required.into(),
        );

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
