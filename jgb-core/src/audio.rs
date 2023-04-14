use crate::apu::ApuState;
use crate::{apu, RunConfig};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::AudioSubsystem;
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;
use thiserror::Error;

const AUDIO_QUEUE_SIZE: u32 = 1024;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("error pushing audio samples to device sample queue: {msg}")]
    Playback { msg: String },
}

pub fn initialize(audio_subsystem: &AudioSubsystem) -> Result<AudioQueue<f32>, String> {
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
/// If it is full and `sync_to_audio` is enabled, this function will block until it is not full and
/// then push samples.
pub fn push_samples(
    device_queue: &AudioQueue<f32>,
    apu_state: &mut ApuState,
    run_config: &RunConfig,
    fast_forwarding: bool,
) -> Result<(), AudioError> {
    // AudioQueue::size returns size in bytes, so multiply by 8 (2 channels * 4 bytes per sample)
    while device_queue.size() >= 8 * AUDIO_QUEUE_SIZE {
        if !run_config.sync_to_audio {
            return Ok(());
        }

        thread::sleep(Duration::from_micros(250));
    }

    let samples = drain_sample_queue(apu_state.get_sample_queue_mut(), fast_forwarding);
    device_queue
        .queue_audio(&samples)
        .map_err(|msg| AudioError::Playback { msg })?;

    Ok(())
}

fn drain_sample_queue<T>(sample_queue: &mut VecDeque<T>, fast_forwarding: bool) -> Vec<T> {
    if !fast_forwarding {
        return sample_queue.drain(..).collect();
    }

    // Skip every other sample when fast-forwarding
    let drain_len = sample_queue.len() - (sample_queue.len() % 2);
    sample_queue
        .drain(..drain_len)
        .enumerate()
        .filter_map(|(i, sample)| (i % 2 == 0).then_some(sample))
        .collect()
}
