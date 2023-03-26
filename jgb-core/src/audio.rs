use crate::apu;
use crate::apu::ApuState;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::AudioSubsystem;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct ApuCallback {
    sample_queue: Arc<Mutex<VecDeque<i16>>>,
}

impl AudioCallback for ApuCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [Self::Channel]) {
        let mut sample_queue = self.sample_queue.lock().unwrap();

        for value in out.iter_mut() {
            *value = sample_queue.pop_front().unwrap_or(0);
        }
    }
}

pub fn initialize_audio(
    audio_subsystem: &AudioSubsystem,
    apu_state: &ApuState,
) -> Result<AudioDevice<ApuCallback>, String> {
    let callback = ApuCallback {
        sample_queue: apu_state.get_sample_queue(),
    };
    let device = audio_subsystem.open_playback(
        None,
        &AudioSpecDesired {
            freq: Some(apu::OUTPUT_FREQUENCY as i32),
            channels: Some(2),
            samples: Some(2048),
        },
        |_spec| callback,
    )?;
    device.resume();

    Ok(device)
}
