use crate::audio::AudioError;
use crate::cpu::instructions::{ExecutionError, ParseError};
use crate::cpu::{instructions, CgbSpeedMode, CpuRegisters};
use crate::graphics::{GbFrameTexture, GraphicsError, Modal, RenderFrameArgs};
use crate::input::{
    ControllerMap, Hotkey, HotkeyMap, JoypadState, JoystickError, Joysticks, KeyMap, KeyMapError,
};
use crate::memory::ioregisters::IoRegister;
use crate::memory::AddressSpace;
use crate::ppu::{PpuMode, PpuState};
use crate::serialize::SaveStateError;
use crate::startup::{EmulationState, SdlState};
use crate::timer::TimerCounter;
use crate::{apu, audio, cpu, font, graphics, input, ppu, serialize, timer, RunConfig};
use sdl2::event::Event;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    #[error("rendering error: {source}")]
    Rendering {
        #[from]
        source: GraphicsError,
    },
    #[error("font load error: {msg}")]
    FontLoad { msg: String },
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
    #[error("error writing real-time clock to rtc file: {source}")]
    RtcPersist {
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
        texture_creator,
        mut event_pump,
        ttf_ctx,
        ..
    } = sdl_state;

    let mut texture = GbFrameTexture::create(&texture_creator)?;

    let font =
        font::load_font(&ttf_ctx, graphics::FONT_SIZE).map_err(|msg| RunError::FontLoad { msg })?;

    let mut joypad_state = JoypadState::new();
    let mut timer_counter = TimerCounter::new();

    let key_map = KeyMap::from_config(&run_config.input_config)?;
    let hotkey_map = HotkeyMap::from_config(&run_config.hotkey_config)?;
    let mut joysticks = Joysticks::new(&joystick_subsystem);
    let controller_map = ControllerMap::from_config(&run_config.controller_config)?;

    let save_state_path = serialize::determine_save_state_path(&run_config.gb_file_path);
    let save_state_file_name = save_state_path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .unwrap_or("<Unknown>");

    let mut modals = Vec::new();

    let mut total_cycles = 0;
    let mut total_frames = 0;

    // Track how many 4MHz clock cycles are "left over" when running in double speed mode
    let mut leftover_cpu_cycles = 0;
    'running: loop {
        input::update_joyp_register(&joypad_state, address_space.get_io_registers_mut());

        // Read TMA register before executing anything in case the instruction updates the register
        let timer_modulo = timer::read_timer_modulo(address_space.get_io_registers());

        // The number of 4MHz clock cycles
        // (CPU M-cycles * 4 in normal speed, CPU M-cycles * 2 in double speed)
        let mut cycles_required = leftover_cpu_cycles;
        while cycles_required < 4 {
            let tick_cycles = tick_cpu(&mut address_space, &mut cpu_registers, &ppu_state)?;

            if matches!(cpu_registers.cgb_speed_mode, CgbSpeedMode::Double) {
                cycles_required += tick_cycles / 2;
            } else {
                cycles_required += tick_cycles;
            }
        }
        leftover_cpu_cycles = cycles_required & 0x00000003;
        cycles_required &= 0xFFFFFFFC;

        let double_speed = matches!(cpu_registers.cgb_speed_mode, CgbSpeedMode::Double);

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
                                modals.push(Modal::new(
                                    format!("Saved state to {save_state_file_name}"),
                                    Duration::from_secs(3),
                                ));

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

                                        modals.push(Modal::new(
                                            format!("Loaded state from {save_state_file_name}",),
                                            Duration::from_secs(3),
                                        ));
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
                    Event::JoyHatMotion { hat_idx, state, .. } => {
                        joypad_state.hat_motion(hat_idx, state, &controller_map);
                    }
                    Event::JoyAxisMotion {
                        axis_idx, value, ..
                    } => {
                        joypad_state.joy_axis_motion(axis_idx, value, &controller_map);
                    }
                    _ => {}
                }
            }

            address_space.update_rtc();

            // Write out cartridge state roughly once per second at most
            total_frames += 1;
            if total_frames % 60 == 0 {
                address_space
                    .persist_cartridge_state()
                    .map_err(|err| RunError::RamPersist { source: err })?;
            }

            modals.retain(|modal| !modal.is_finished());
        }
        total_cycles += u64::from(cycles_required);

        // Timer updates pause while a VRAM DMA transfer is in progress
        if !ppu_state.is_vram_dma_in_progress() {
            let timer_cycles = if double_speed {
                // Timer and divider registers update twice as fast in double speed mode
                2 * u64::from(cycles_required)
            } else {
                cycles_required.into()
            };
            timer::update_timer_registers(
                address_space.get_io_registers_mut(),
                &mut timer_counter,
                timer_modulo,
                timer_cycles,
            );
        }

        let prev_mode = ppu_state.mode();
        let prev_enabled = ppu_state.enabled();
        for _ in (0..cycles_required).step_by(4) {
            ppu::progress_oam_dma_transfer(&mut ppu_state, &mut address_space);
            if double_speed {
                // OAM DMA transfers progress at double speed in double speed mode so call twice
                ppu::progress_oam_dma_transfer(&mut ppu_state, &mut address_space);
            }

            // Shadow prev_mode so that it correctly updates when doing VRAM DMA transfers in double
            // speed mode
            let prev_mode = ppu_state.mode();
            ppu::tick_m_cycle(&mut ppu_state, &mut address_space);

            // Progress VRAM DMA transfer by 2 bytes per PPU M-cycle
            let current_mode = ppu_state.mode();
            ppu::progress_vram_dma_transfer(&mut ppu_state, &mut address_space, prev_mode);
            ppu::progress_vram_dma_transfer(&mut ppu_state, &mut address_space, current_mode);

            apu::tick_m_cycle(
                &mut apu_state,
                address_space.get_io_registers_mut(),
                cpu_registers.cgb_speed_mode,
                run_config.audio_60hz,
            );
        }

        // Check if the PPU just entered VBlank mode, which indicates that the next frame is ready
        // to render. Also render a (blank) frame if the PPU was just disabled.
        if ppu_state.should_render_current_frame()
            && ((prev_mode != PpuMode::VBlank && ppu_state.mode() == PpuMode::VBlank)
                || (prev_enabled && !ppu_state.enabled()))
        {
            graphics::render_frame(RenderFrameArgs {
                execution_mode,
                ppu_state: &ppu_state,
                canvas: &mut canvas,
                texture_creator: &texture_creator,
                texture: &mut texture,
                font: &font,
                modals: &modals,
                run_config,
            })?;
        }
    }

    Ok(())
}

fn tick_cpu(
    address_space: &mut AddressSpace,
    cpu_registers: &mut CpuRegisters,
    ppu_state: &PpuState,
) -> Result<u32, RunError> {
    if ppu_state.is_vram_dma_in_progress() {
        // CPU is halted while a VRAM DMA transfer is actively copying bytes
        return Ok(4);
    }

    let result = if let Some(wait_cycles_remaining) =
        cpu_registers.speed_switch_wait_cycles_remaining
    {
        if wait_cycles_remaining == 1 {
            cpu_registers.speed_switch_wait_cycles_remaining = None;
        } else {
            cpu_registers.speed_switch_wait_cycles_remaining = Some(wait_cycles_remaining - 1);
        }

        4
    } else if cpu::interrupt_triggered(cpu_registers, address_space) {
        cpu::execute_interrupt_service_routine(cpu_registers, address_space, ppu_state);

        cpu::ISR_CYCLES_REQUIRED
    } else if !cpu_registers.halted || cpu::interrupt_triggered_no_ime_check(address_space) {
        cpu_registers.halted = false;

        let (instruction, pc) =
            instructions::parse_next_instruction(address_space, cpu_registers.pc, ppu_state)?;

        log::trace!("Updating PC from 0x{:04X} to {:04X}", cpu_registers.pc, pc);
        cpu_registers.pc = pc;

        let cycles_required = instruction.cycles_required(cpu_registers);

        log::trace!("Executing instruction {instruction:04X?}, will take {cycles_required} cycles");
        log::trace!("CPU registers before instruction execution: {cpu_registers:04X?}");
        log::trace!(
            "Other registers before execution: IE={:02X}, IF={:02X}, LCDC={:02X}, LY={:02X}, LYC={:02X}, STAT={:02X}",
            address_space.get_ie_register(),
            address_space
                .get_io_registers()
                .read_register(IoRegister::IF),
            address_space
                .get_io_registers()
                .read_register(IoRegister::LCDC),
            address_space
                .get_io_registers()
                .read_register(IoRegister::LY),
            address_space.get_io_registers().read_register(IoRegister::LYC),
            address_space.get_io_registers().read_register(IoRegister::STAT),
        );
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
        instruction.execute(address_space, cpu_registers, ppu_state)?;

        cycles_required
    } else {
        // Do nothing, let other processors execute for 1 M-cycle
        4
    };

    Ok(result)
}
