#![allow(unused)]

mod config;
mod cpu;
mod memory;
mod ppu;
mod startup;

use crate::cpu::CpuRegisters;
use crate::memory::AddressSpace;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use std::error::Error;
use std::path::Path;
use std::thread;
use std::time::Duration;
use thiserror::Error;

pub use config::{PersistentConfig, RunConfig};

#[derive(Error, Debug)]
pub enum RunError {}

pub struct EmulationState {
    address_space: AddressSpace,
    cpu_registers: CpuRegisters,
}

pub fn run(
    persistent_config: PersistentConfig,
    run_config: RunConfig,
) -> Result<(), Box<dyn Error>> {
    let emulation_state = startup::init_emulation_state(&persistent_config, &run_config)?;

    // let mut sdl_state = startup::init_sdl_state(&persistent_config, &run_config)?;
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

    cpu::run(emulation_state)?;

    Ok(())
}
