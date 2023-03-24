// TODO remove this once this project is closer to working
#![allow(unused_variables, dead_code)]

mod config;
mod cpu;
mod eventloop;
mod graphics;
mod input;
mod memory;
mod ppu;
mod startup;
mod timer;

use crate::cpu::CpuRegisters;
use crate::memory::AddressSpace;
use std::error::Error;
use thiserror::Error;

use crate::ppu::PpuState;
pub use config::{PersistentConfig, RunConfig};

#[derive(Error, Debug)]
pub enum RunError {}

pub struct EmulationState {
    address_space: AddressSpace,
    cpu_registers: CpuRegisters,
    ppu_state: PpuState,
}

pub fn run(
    persistent_config: PersistentConfig,
    run_config: RunConfig,
) -> Result<(), Box<dyn Error>> {
    let emulation_state = startup::init_emulation_state(&persistent_config, &run_config)?;

    let sdl_state = startup::init_sdl_state(&persistent_config, &run_config)?;

    eventloop::run(emulation_state, sdl_state)?;

    //
    // let texture_creator = sdl_state.canvas.texture_creator();
    // let mut window_texture =
    //     texture_creator.create_texture_streaming(PixelFormatEnum::RGB24, 160, 144)?;
    //
    // 'running: loop {
    //     for event in sdl_state.event_pump.poll_iter() {
    //         match event {
    //             Event::Quit { .. }
    //             | Event::KeyDown {
    //                 keycode: Some(Keycode::Escape),
    //                 ..
    //             } => {
    //                 break 'running;
    //             }
    //             _ => {}
    //         }
    //     }
    //
    //     sdl_state.canvas.clear();
    //     window_texture.with_lock(None, |pixels, pitch| {
    //         for i in 0..144 {
    //             for j in 0..160 {
    //                 pixels[i * pitch + j * 3..i * pitch + (j + 1) * 3].copy_from_slice(&[
    //                     rand::random(),
    //                     rand::random(),
    //                     rand::random(),
    //                 ]);
    //             }
    //         }
    //     })?;
    //     sdl_state.canvas.copy(&window_texture, None, None)?;
    //     sdl_state.canvas.present();
    //
    //     thread::sleep(Duration::from_millis(10));
    // }

    // cpu::run(emulation_state)?;

    Ok(())
}
