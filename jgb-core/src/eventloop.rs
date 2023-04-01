use crate::cpu::instructions;
use crate::cpu::instructions::{ExecutionError, ParseError};
use crate::graphics::GraphicsError;
use crate::input::{JoypadState, KeyMap, KeyMapError};
use crate::memory::ioregisters::IoRegister;
use crate::ppu::Mode;
use crate::startup::SdlState;
use crate::timer::TimerCounter;
use crate::{apu, audio, cpu, graphics, input, ppu, timer, EmulationState, RunConfig};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::TextureValueError;
use std::io;
use std::sync::{Arc, Mutex};
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
    #[error("rendering error: {source}")]
    Rendering {
        #[from]
        source: GraphicsError,
    },
    #[error("debug setup error: {source}")]
    DebugSetup {
        #[from]
        source: io::Error,
    },
    #[error("error writing cartridge RAM to sav file: {source}")]
    RamPersist {
        #[source]
        source: io::Error,
    },
    #[error("error processing input config: {source}")]
    InputConfig {
        #[from]
        source: KeyMapError,
    },
}

const CYCLES_PER_FRAME: u64 = 4 * 1024 * 1024 / 60;

/// Start and run the emulator until it terminates, either by closing it or due to an error.
pub fn run(
    emulation_state: EmulationState,
    sdl_state: SdlState,
    run_config: &RunConfig,
    quit_signal: Arc<Mutex<bool>>,
) -> Result<(), RunError> {
    log::info!("Running with config:\n{run_config}");

    let EmulationState {
        mut address_space,
        mut cpu_registers,
        mut ppu_state,
        mut apu_state,
    } = emulation_state;

    // Don't need explicit handles to subsystems or audio device because they won't be dropped until
    // the function returns
    let SdlState {
        mut canvas,
        mut event_pump,
        ..
    } = sdl_state;

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_streaming(
        PixelFormatEnum::RGB24,
        ppu::SCREEN_WIDTH.into(),
        ppu::SCREEN_HEIGHT.into(),
    )?;

    let mut joypad_state = JoypadState::new();
    let mut timer_counter = TimerCounter::new();

    let key_map = KeyMap::from_config(&run_config.input_config)?;

    let mut return_down = false;
    let mut lalt_down = false;
    let mut ralt_down = false;

    let mut total_cycles = 0;
    'running: loop {
        input::update_joyp_register(&joypad_state, address_space.get_io_registers_mut());

        // Read TMA register before executing anything in case the instruction updates the register
        let timer_modulo = timer::read_timer_modulo(address_space.get_io_registers());

        let cycles_required = if cpu::interrupt_triggered(&cpu_registers, &address_space) {
            cpu::execute_interrupt_service_routine(
                &mut cpu_registers,
                &mut address_space,
                &ppu_state,
            );

            cpu::ISR_CYCLES_REQUIRED
        } else if !cpu_registers.halted || cpu::interrupt_triggered_no_ime_check(&address_space) {
            cpu_registers.halted = false;

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
        } else {
            // Do nothing, let PPU and timer execute for 1 M-cycle
            4
        };

        assert!(cycles_required > 0 && cycles_required % 4 == 0);

        // Process SDL events and write save file roughly once per frametime
        if total_cycles / CYCLES_PER_FRAME
            != (total_cycles + u64::from(cycles_required)) / CYCLES_PER_FRAME
        {
            if *quit_signal.lock().unwrap() {
                log::info!("Quit signal received, exiting main loop");
                break;
            }

            if run_config.audio_enabled && run_config.sync_to_audio {
                audio::sync(&apu_state);
            }

            // TODO better handle the unlikely scenario where a key is pressed *and released* between frames
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
                        joypad_state.key_down(keycode, &key_map);

                        // TODO this code should really go somewhere else
                        match keycode {
                            Keycode::Return => return_down = true,
                            Keycode::LAlt => lalt_down = true,
                            Keycode::RAlt => ralt_down = true,
                            _ => {}
                        }

                        if return_down && (lalt_down || ralt_down) {
                            graphics::toggle_fullscreen(&mut canvas, run_config)?;
                        }
                    }
                    Event::KeyUp {
                        keycode: Some(keycode),
                        ..
                    } => {
                        joypad_state.key_up(keycode, &key_map);

                        // TODO this code should really go somewhere else
                        match keycode {
                            Keycode::Return => return_down = false,
                            Keycode::LAlt => lalt_down = false,
                            Keycode::RAlt => ralt_down = false,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            address_space
                .persist_cartridge_ram()
                .map_err(|err| RunError::RamPersist { source: err })?;
        }
        total_cycles += u64::from(cycles_required);

        timer::update_timer_registers(
            address_space.get_io_registers_mut(),
            &mut timer_counter,
            timer_modulo,
            cycles_required.into(),
        );

        let prev_mode = ppu_state.mode();
        for _ in (0..cycles_required).step_by(4) {
            ppu::progress_oam_dma_transfer(&mut ppu_state, &mut address_space);
            ppu::tick_m_cycle(&mut ppu_state, &mut address_space);

            apu::tick_m_cycle(
                &mut apu_state,
                address_space.get_io_registers_mut(),
                run_config.audio_60hz,
            );
        }

        // Check if the PPU just entered VBlank mode, which indicates that the next frame is ready
        // to render
        if prev_mode != Mode::VBlank && ppu_state.mode() == Mode::VBlank {
            graphics::render_frame(&ppu_state, &mut canvas, &mut texture, run_config)?;
        }
    }

    Ok(())
}
