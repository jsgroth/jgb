use crate::apu::ApuState;
use crate::{apu, RunConfig};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::AudioSubsystem;
use std::thread;
use std::time::Duration;
use thiserror::Error;

const AUDIO_QUEUE_SIZE: u32 = 1024;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("error pushing audio samples to device sample queue: {msg}")]
    Playback { msg: String },
}

pub fn initialize(audio_subsystem: &AudioSubsystem) -> Result<AudioQueue<i16>, String> {
    let queue = audio_subsystem.open_queue(
        None,
        &AudioSpecDesired {
            freq: Some(apu::OUTPUT_FREQUENCY as i32),
            channels: Some(2),
            samples: Some(AUDIO_QUEUE_SIZE as u16),
        },
    )?;
    queue.resume();

    Ok(queue)
}

/// Push audio samples to the playback queue if it is not full.
///
/// If it is full and sync_to_audio is enabled, this function will block until it is not full and
/// then push samples.
pub fn push_samples(
    device_queue: &AudioQueue<i16>,
    apu_state: &mut ApuState,
    run_config: &RunConfig,
) -> Result<(), AudioError> {
    // AudioQueue::size returns size in bytes, so multiply by 4 (2 channels * 2 bytes per sample)
    while device_queue.size() >= 4 * AUDIO_QUEUE_SIZE {
        if !run_config.sync_to_audio {
            return Ok(());
        }

        thread::sleep(Duration::from_micros(250));
    }

    let sample_queue = apu_state.get_sample_queue_mut();
    let samples: Vec<_> = sample_queue.drain(..).collect();
    device_queue
        .queue_audio(&samples)
        .map_err(|msg| AudioError::Playback { msg })?;

    Ok(())
}
