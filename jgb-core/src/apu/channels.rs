mod noise;
mod pulse;
mod wave;

use crate::memory::ioregisters::{IoRegister, IoRegisters};
pub(crate) use noise::NoiseChannel;
pub(crate) use pulse::PulseChannel;
pub(crate) use wave::WaveChannel;

pub(crate) trait Channel {
    fn channel_enabled(&self) -> bool;

    fn dac_enabled(&self) -> bool;

    // Digital sample in the range [0, 15]
    fn sample_digital(&self) -> Option<u8>;

    // "Analog" sample in the range [-1, 1]
    fn sample_analog(&self) -> f64 {
        let Some(digital_sample) = self.sample_digital() else { return 0.0; };

        (f64::from(digital_sample) - 7.5) / 7.5
    }
}

#[derive(Debug, Clone)]
struct LengthTimer {
    enabled: bool,
    timer: u16,
}

impl LengthTimer {
    fn new() -> Self {
        Self {
            enabled: false,
            timer: 0,
        }
    }

    fn tick(&mut self) -> bool {
        if !self.enabled {
            return false;
        }

        self.timer = self.timer.saturating_sub(1);
        self.timer == 0
    }

    fn trigger(&mut self, max_value: u16, divider_ticks: u64) {
        if self.timer == 0 {
            self.timer = max_value;

            // Immediately tick if the length timer is enabled and this is an off-cycle
            if self.enabled && divider_ticks % 2 == 0 {
                self.timer -= 1;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlopeDirection {
    Increasing,
    Decreasing,
}

// Volume state & envelope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VolumeControl {
    volume: u8,
    envelope_direction: SlopeDirection,
    pace: u8,
    timer: u8,
    envelope_enabled: bool,
}

impl VolumeControl {
    fn new() -> Self {
        Self {
            volume: 0,
            envelope_direction: SlopeDirection::Decreasing,
            pace: 0,
            timer: 0,
            envelope_enabled: false,
        }
    }

    // Create a newly initialized VolumeControl from the given NRx2 value
    fn from_byte(byte: u8) -> Self {
        let pace = byte & 0x07;
        Self {
            volume: byte >> 4,
            envelope_direction: if byte & 0x08 != 0 {
                SlopeDirection::Increasing
            } else {
                SlopeDirection::Decreasing
            },
            pace,
            timer: pace,
            envelope_enabled: true,
        }
    }

    // Tick the envelope timer. This should be called at a rate of 64Hz.
    // If the timer clocks then volume will be increased or decreased by 1, down to a min of 0
    // or a max of 15.
    fn tick(&mut self) {
        if self.pace != 0 && self.envelope_enabled {
            self.timer -= 1;
            if self.timer == 0 {
                self.timer = self.pace;

                let overflowed = match self.envelope_direction {
                    SlopeDirection::Increasing => self.volume == 0x0F,
                    SlopeDirection::Decreasing => self.volume == 0x00,
                };
                if overflowed {
                    self.envelope_enabled = false;
                } else {
                    let new_volume = match self.envelope_direction {
                        SlopeDirection::Increasing => self.volume + 1,
                        SlopeDirection::Decreasing => self.volume - 1,
                    };
                    self.volume = new_volume;
                }
            }
        }
    }
}

// Read an 11-bit wave frequency out of the specified NRx3 and NRx4 registers.
// The lower 8 bits come from NRx3 and the higher 3 bits come from NRx4 (bits 0-2).
fn read_frequency(io_registers: &IoRegisters, nr3: IoRegister, nr4: IoRegister) -> u16 {
    let nr3_value = io_registers.apu_read_register(nr3);
    let nr4_value = io_registers.apu_read_register(nr4);

    ((u16::from(nr4_value) & 0x07) << 8) | u16::from(nr3_value)
}
