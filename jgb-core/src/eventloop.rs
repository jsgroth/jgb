use crate::cpu::instructions;
use crate::cpu::instructions::{ExecutionError, ParseError};
use crate::input::JoypadState;
use crate::memory::ioregisters::IoRegister;
use crate::ppu::Mode;
use crate::startup::SdlState;
use crate::timer::TimerCounter;
use crate::{cpu, input, ppu, timer, EmulationState};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::TextureValueError;
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
    #[error("error creating SDL2 texture: {source}")]
    TextureCreation {
        #[from]
        source: TextureValueError,
    },
    #[error("SDL2 rendering error: {msg}")]
    Rendering { msg: String },
}

pub fn run(emulation_state: EmulationState, sdl_state: SdlState) -> Result<(), RunError> {
    let EmulationState {
        mut address_space,
        mut cpu_registers,
        mut ppu_state,
    } = emulation_state;

    let SdlState {
        sdl,
        video,
        mut canvas,
        game_controller,
        mut event_pump,
    } = sdl_state;

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_streaming(PixelFormatEnum::RGB24, 160, 144)?;

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

        let cycles_required = if cpu::interrupt_triggered(&cpu_registers, &address_space) {
            cpu::execute_interrupt_service_routine(
                &mut cpu_registers,
                &mut address_space,
                &ppu_state,
            );

            cpu::ISR_CYCLES_REQUIRED
        } else {
            let (instruction, pc) =
                instructions::parse_next_instruction(&address_space, cpu_registers.pc, &ppu_state)?;

            log::trace!("Updating PC from 0x{:04X} to {:04X}", cpu_registers.pc, pc);
            cpu_registers.pc = pc;

            let cycles_required = instruction.cycles_required(&cpu_registers);

            log::trace!(
                "Executing instruction {instruction:04X?}, will take {cycles_required} cycles"
            );
            log::trace!("CPU registers before instruction execution: {cpu_registers:04X?}");
            log::trace!(
                "IE register before instruction execution: {:02X}",
                address_space.get_ie_register()
            );
            log::trace!(
                "IF register before instruction execution: {:02X}",
                address_space
                    .get_io_registers()
                    .read_register(IoRegister::IF)
            );
            instruction.execute(&mut address_space, &mut cpu_registers, &ppu_state)?;

            cycles_required
        };

        assert!(cycles_required > 0 && cycles_required % 4 == 0);

        let prev_mode = ppu_state.mode();
        for _ in (0..cycles_required).step_by(4) {
            ppu::progress_oam_dma_transfer(&mut ppu_state, &mut address_space);
            ppu::tick_m_cycle(&mut ppu_state, &mut address_space);
        }

        timer::update_timer_registers(
            address_space.get_io_registers_mut(),
            &mut timer_counter,
            timer_modulo,
            cycles_required.into(),
        );

        if prev_mode != Mode::VBlank && ppu_state.mode() == Mode::VBlank {
            let frame_buffer = ppu_state.frame_buffer();
            canvas.clear();
            texture
                .with_lock(None, |pixels, pitch| {
                    for i in 0..144 {
                        for j in 0..160 {
                            let gb_color = frame_buffer[i][j];
                            let color = (f64::from(gb_color) / 3.0 * 255.0).round() as u8;

                            log::trace!("Setting pixel at ({i}, {j}) to {color} from {gb_color}");

                            pixels[i * pitch + j * 3] = color;
                            pixels[i * pitch + j * 3 + 1] = color;
                            pixels[i * pitch + j * 3 + 2] = color;
                        }
                    }
                })
                .map_err(|msg| RunError::Rendering { msg })?;
            canvas
                .copy(&texture, None, None)
                .map_err(|msg| RunError::Rendering { msg })?;
            canvas.present();
        }

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
