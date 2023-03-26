use crate::apu::ApuState;
use crate::cpu::instructions;
use crate::cpu::instructions::{ExecutionError, ParseError};
use crate::graphics::RenderError;
use crate::input::JoypadState;
use crate::memory::ioregisters::IoRegister;
use crate::ppu::Mode;
use crate::startup::SdlState;
use crate::timer::TimerCounter;
use crate::{apu, audio, cpu, graphics, input, ppu, timer, EmulationState, RunConfig};
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
    #[error("rendering error: {source}")]
    Rendering {
        #[from]
        source: RenderError,
    },
}

/// Start and run the emulator until it terminates, either by closing it or due to an error.
pub fn run(
    emulation_state: EmulationState,
    sdl_state: SdlState,
    run_config: &RunConfig,
) -> Result<(), RunError> {
    log::info!("Running with config:\n{run_config}");

    let EmulationState {
        mut address_space,
        mut cpu_registers,
        mut ppu_state,
    } = emulation_state;

    let SdlState {
        audio,
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

    let mut apu_state = ApuState::new();
    let mut joypad_state = JoypadState::new();
    let mut timer_counter = TimerCounter::new();

    let _audio_device = if run_config.audio_enabled {
        Some(audio::initialize_audio(&audio, &apu_state))
    } else {
        None
    };

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
        } else if !cpu_registers.halted {
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

            apu::tick_m_cycle(&mut apu_state, address_space.get_io_registers_mut());
            if run_config.audio_enabled && run_config.sync_to_audio {
                audio::sync(&apu_state);
            }
        }

        // Check if the PPU just entered VBlank mode, which indicates that the next frame is ready
        // to render
        if prev_mode != Mode::VBlank && ppu_state.mode() == Mode::VBlank {
            graphics::render_frame(&ppu_state, &mut canvas, &mut texture)?;

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
        }
    }

    Ok(())
}
