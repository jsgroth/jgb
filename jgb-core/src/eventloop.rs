use crate::audio::AudioError;
use crate::cpu::instructions;
use crate::cpu::instructions::{ExecutionError, ParseError};
use crate::graphics::GraphicsError;
use crate::input::{
    ControllerMap, Hotkey, HotkeyMap, JoypadState, JoystickError, Joysticks, KeyMap, KeyMapError,
};
use crate::memory::ioregisters::IoRegister;
use crate::ppu::Mode;
use crate::serialize::SaveStateError;
use crate::startup::{EmulationState, SdlState};
use crate::timer::TimerCounter;
use crate::{apu, audio, cpu, graphics, input, ppu, serialize, timer, RunConfig};
use sdl2::event::Event;
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
    #[error("audio playback error: {source}")]
    AudioPlayback {
        #[from]
        source: AudioError,
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
    #[error("error saving/loading save state: {source}")]
    SaveState {
        #[from]
        source: SaveStateError,
    },
    #[error("error opening controller device: {source}")]
    Controller {
        #[from]
        source: JoystickError,
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
        mut execution_mode,
    } = emulation_state;

    // Don't need explicit handles to subsystems because they won't be dropped until the function
    // returns
    let SdlState {
        audio_playback_queue,
        joystick_subsystem,
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
    let hotkey_map = HotkeyMap::from_config(&run_config.hotkey_config)?;
    let mut joysticks = Joysticks::new(&joystick_subsystem);
    let controller_map = ControllerMap::from_config(&run_config.controller_config)?;

    let save_state_path = serialize::determine_save_state_path(&run_config.gb_file_path);

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

            if let Some(audio_device_queue) = &audio_playback_queue {
                audio::push_samples(audio_device_queue, &mut apu_state, run_config)?;
            }

            // TODO better handle the unlikely scenario where a key is pressed *and released* between frames
            for event in event_pump.poll_iter() {
                if matches!(event, Event::JoyAxisMotion { .. }) {
                    log::trace!("Received SDL event: {event:?}");
                } else {
                    log::debug!("Received SDL event: {event:?}");
                }
                match event {
                    Event::Quit { .. } => {
                        break 'running;
                    }
                    Event::KeyDown {
                        keycode: Some(keycode),
                        ..
                    } => {
                        joypad_state.key_down(keycode, &key_map);

                        match input::check_for_hotkey(keycode, &hotkey_map) {
                            Some(Hotkey::Exit) => {
                                break 'running;
                            }
                            Some(Hotkey::ToggleFullscreen) => {
                                graphics::toggle_fullscreen(&mut canvas, run_config)?;
                            }
                            Some(Hotkey::SaveState) => {
                                let state = EmulationState {
                                    address_space,
                                    cpu_registers,
                                    ppu_state,
                                    apu_state,
                                    execution_mode,
                                };

                                serialize::save_state(&state, &save_state_path)?;

                                address_space = state.address_space;
                                cpu_registers = state.cpu_registers;
                                ppu_state = state.ppu_state;
                                apu_state = state.apu_state;
                                execution_mode = state.execution_mode;
                            }
                            Some(Hotkey::LoadState) => {
                                match serialize::load_state(
                                    &save_state_path,
                                    apu_state,
                                    &address_space,
                                ) {
                                    Ok(state) => {
                                        address_space = state.address_space;
                                        cpu_registers = state.cpu_registers;
                                        ppu_state = state.ppu_state;
                                        apu_state = state.apu_state;
                                    }
                                    Err((err, old_apu_state)) => {
                                        log::error!("error loading save state: {err}");

                                        apu_state = *old_apu_state;
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                    Event::KeyUp {
                        keycode: Some(keycode),
                        ..
                    } => {
                        joypad_state.key_up(keycode, &key_map);
                    }
                    Event::JoyDeviceAdded { which, .. } => {
                        joysticks.device_added(which)?;
                    }
                    Event::JoyDeviceRemoved { which, .. } => {
                        joysticks.device_removed(which);
                    }
                    Event::JoyButtonDown { button_idx, .. } => {
                        joypad_state.joy_button_down(button_idx, &controller_map);
                    }
                    Event::JoyButtonUp { button_idx, .. } => {
                        joypad_state.joy_button_up(button_idx, &controller_map);
                    }
                    Event::JoyAxisMotion {
                        axis_idx, value, ..
                    } => {
                        joypad_state.joy_axis_motion(axis_idx, value, &controller_map);
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
