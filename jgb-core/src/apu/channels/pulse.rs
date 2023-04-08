use crate::apu::channels;
use crate::apu::channels::{Channel, FrequencyTimer, LengthTimer, SlopeDirection, VolumeControl};
use crate::memory::ioregisters::{IoRegister, IoRegisters};
use serde::{Deserialize, Serialize};

// Waveform for square wave channels (12.5% / 25% / 50% / 75%). Each waveform has 8 samples which
// are each 0 or 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum DutyCycle {
    OneEighth,
    OneFourth,
    OneHalf,
    ThreeFourths,
}

impl DutyCycle {
    fn waveform(self) -> [u8; 8] {
        match self {
            Self::OneEighth => [0, 0, 0, 0, 0, 0, 0, 1],
            Self::OneFourth => [1, 0, 0, 0, 0, 0, 0, 1],
            Self::OneHalf => [1, 0, 0, 0, 0, 1, 1, 1],
            Self::ThreeFourths => [0, 1, 1, 1, 1, 1, 1, 0],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SweepResult {
    None,
    Overflowed,
    Changed(u16),
}

// Square wave channel sweep config & state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct PulseSweep {
    pace: u8,
    direction: SlopeDirection,
    shift: u8,
    timer: u8,
    enabled: bool,
    shadow_frequency: u16,
    generated_with_negate: bool,
}

impl PulseSweep {
    const DISABLED: Self = Self {
        pace: 0,
        direction: SlopeDirection::Decreasing,
        shift: 0,
        timer: 0,
        enabled: false,
        shadow_frequency: 0,
        generated_with_negate: false,
    };

    // Tick the sweep timer. This should be called at a rate of 128Hz.
    fn tick(&mut self) -> SweepResult {
        if !self.enabled {
            return SweepResult::None;
        }

        self.timer -= 1;
        if self.timer > 0 {
            return SweepResult::None;
        }

        self.reset_timer();

        if self.pace == 0 {
            // Don't perform frequency calculations if pace is 0
            return SweepResult::None;
        }

        match self.next_frequency() {
            Some(new_frequency) => {
                if self.shift > 0 {
                    self.shadow_frequency = new_frequency;
                    SweepResult::Changed(new_frequency)
                } else {
                    SweepResult::None
                }
            }
            None => SweepResult::Overflowed,
        }
    }

    // Re-init the sweep, which resets the enabled flag, the shadow frequency, and the timer.
    fn trigger(&mut self, frequency: u16) {
        self.enabled = self.pace != 0 || self.shift != 0;
        self.shadow_frequency = frequency;
        self.generated_with_negate = false;
        self.reset_timer();
    }

    fn reset_timer(&mut self) {
        // Treat pace of 0 as 8 for timer purposes
        self.timer = if self.pace > 0 { self.pace } else { 8 };
    }

    // Compute the next frequency given the current sweep. Returns None on overflow/underflow.
    fn next_frequency(&mut self) -> Option<u16> {
        if self.direction == SlopeDirection::Decreasing {
            self.generated_with_negate = true;
        }

        let frequency = self.shadow_frequency;

        let delta = frequency >> self.shift;
        let next_frequency = match self.direction {
            SlopeDirection::Increasing => frequency + delta,
            SlopeDirection::Decreasing => frequency.wrapping_sub(delta),
        };

        if next_frequency <= 0x07FF {
            Some(next_frequency)
        } else {
            None
        }
    }
}

// A square wave channel (channels 1 & 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PulseChannel {
    generation_on: bool,
    dac_on: bool,
    duty_cycle: DutyCycle,
    length_timer: LengthTimer,
    volume_control: VolumeControl,
    frequency_timer: FrequencyTimer,
    sweep: PulseSweep,
    phase_position: u64,
    nr0: Option<IoRegister>,
    nr1: IoRegister,
    nr2: IoRegister,
    nr3: IoRegister,
    nr4: IoRegister,
}

impl PulseChannel {
    pub(crate) fn new(
        nr0: Option<IoRegister>,
        nr1: IoRegister,
        nr2: IoRegister,
        nr3: IoRegister,
        nr4: IoRegister,
    ) -> Self {
        Self {
            generation_on: false,
            dac_on: false,
            duty_cycle: DutyCycle::OneEighth,
            length_timer: LengthTimer::new(),
            volume_control: VolumeControl::new(),
            frequency_timer: FrequencyTimer::new(4),
            sweep: PulseSweep::DISABLED,
            phase_position: 0,
            nr0,
            nr1,
            nr2,
            nr3,
            nr4,
        }
    }

    // Create a square wave channel configured to read from channel 1 audio registers (w/ sweep)
    pub(crate) fn new_channel_1() -> Self {
        Self::new(
            Some(IoRegister::NR10),
            IoRegister::NR11,
            IoRegister::NR12,
            IoRegister::NR13,
            IoRegister::NR14,
        )
    }

    // Create a square wave channel configured to read from channel 2 audio registers (no sweep)
    pub(crate) fn new_channel_2() -> Self {
        Self::new(
            None,
            IoRegister::NR21,
            IoRegister::NR22,
            IoRegister::NR23,
            IoRegister::NR24,
        )
    }

    // Update the channel's internal state based on audio register contents and updates.
    pub(crate) fn process_register_updates(
        &mut self,
        io_registers: &mut IoRegisters,
        divider_ticks: u64,
    ) {
        let nr0_dirty = match self.nr0 {
            Some(nr0) => io_registers.get_dirty_bit(nr0),
            None => false,
        };
        let nr1_dirty = io_registers.get_dirty_bit(self.nr1);
        let nr2_dirty = io_registers.get_dirty_bit(self.nr2);
        let nr3_dirty = io_registers.get_dirty_bit(self.nr3);
        let nr4_dirty = io_registers.get_dirty_bit(self.nr4);

        if !nr0_dirty && !nr1_dirty && !nr2_dirty && !nr3_dirty && !nr4_dirty {
            return;
        }

        let nr2_value = io_registers.apu_read_register(self.nr2);
        let nr4_value = io_registers.apu_read_register(self.nr4);

        // Only check sweep if an NRx0 register is configured
        if let Some(nr0) = self.nr0 {
            if nr0_dirty {
                io_registers.clear_dirty_bit(nr0);

                let nr0_value = io_registers.apu_read_register(nr0);

                let sweep_pace = (nr0_value & 0x70) >> 4;
                let sweep_direction = if nr0_value & 0x08 != 0 {
                    SlopeDirection::Decreasing
                } else {
                    SlopeDirection::Increasing
                };
                let sweep_shift = nr0_value & 0x07;

                self.sweep.pace = sweep_pace;
                self.sweep.direction = sweep_direction;
                self.sweep.shift = sweep_shift;

                // If the sweep generated any frequency calculations with decreasing sweep since the
                // last trigger, switching to increasing sweep should disable the channel
                if self.sweep.generated_with_negate && sweep_direction == SlopeDirection::Increasing
                {
                    self.generation_on = false;
                }
            }
        }

        // Check if 6-bit length timer has been reset (NRx1 bits 0-5)
        if nr1_dirty {
            io_registers.clear_dirty_bit(self.nr1);

            let nr1_value = io_registers.apu_read_register(self.nr1);

            // Sync duty cycle with NRx1 register (bits 6-7), updates take effect immediately
            let duty_cycle = match nr1_value & 0xC0 {
                0x00 => DutyCycle::OneEighth,
                0x40 => DutyCycle::OneFourth,
                0x80 => DutyCycle::OneHalf,
                0xC0 => DutyCycle::ThreeFourths,
                _ => panic!("{nr1_value} & 0xC0 was not 0x00/0x40/0x80/0xC0"),
            };
            self.duty_cycle = duty_cycle;

            self.length_timer.timer = (64 - (nr1_value & 0x3F)).into();
        }

        // Zombie mode hack - increase volume by 1 when volume register is written to while
        // envelope pace is 0, wrapping around from 15 to 0
        if nr2_dirty {
            io_registers.clear_dirty_bit(self.nr2);

            let pending_volume_control = VolumeControl::from_byte(nr2_value);
            if self.volume_control.envelope_enabled
                && self.volume_control.pace == 0
                && pending_volume_control.envelope_direction == SlopeDirection::Increasing
            {
                self.volume_control.volume = (self.volume_control.volume + 1) & 0x0F;
            }
        }

        // Sync length timer enabled flag with NRx4 bit 6, updates take effect immediately
        let prev_length_timer_enabled = self.length_timer.enabled;
        self.length_timer.enabled = nr4_value & 0x40 != 0;

        // When the length timer is enabled, if this is an off-cycle divider tick, immediately
        // tick the length timer and disable the channel if it clocks
        if !prev_length_timer_enabled
            && self.length_timer.enabled
            && divider_ticks % 2 == 0
            && self.length_timer.tick()
        {
            self.generation_on = false;
        }

        // Immediately update frequency if NRx3 or NRx4 was written to
        if nr3_dirty || nr4_dirty {
            if nr3_dirty {
                io_registers.clear_dirty_bit(self.nr3);
            }
            if nr4_dirty {
                io_registers.clear_dirty_bit(self.nr4);
            }

            let new_frequency = channels::read_frequency(io_registers, self.nr3, self.nr4);
            self.frequency_timer.frequency = new_frequency;
        }

        // Re-initialize the channel if a value was written to NRx4 with bit 7 set
        let triggered = nr4_dirty && nr4_value & 0x80 != 0;
        if triggered {
            // Re-initialize sweep (if applicable)
            self.sweep.trigger(self.frequency_timer.frequency);

            // Re-initialize volume & envelope
            self.volume_control = VolumeControl::from_byte(nr2_value);

            // Reset length timer to the maximum possible if it expired
            self.length_timer.trigger(64, divider_ticks);

            // Re-initialize frequency timer
            self.frequency_timer.trigger();

            self.generation_on = true;

            // Do a sweep overflow check immediately on trigger and disable the channel if it trips
            if self.sweep.shift > 0 && self.sweep.next_frequency().is_none() {
                self.generation_on = false;
            }
        }

        // The channel's DAC is on iff any of NRx2 bits 3-7 are set
        self.dac_on = nr2_value & 0xF8 != 0;
        if !self.dac_on {
            // Disable the channel if the DAC is disabled
            self.generation_on = false;
        }
    }

    // Tick the channel's sequencer timers for one divider cycle (512Hz)
    pub(crate) fn tick_divider(&mut self, divider_ticks: u64, io_registers: &mut IoRegisters) {
        // Pulse sweep timer ticks at a rate of 128 Hz
        if self.nr0.is_some() && divider_ticks % 4 == 2 {
            self.process_sweep_tick(io_registers);
        }

        // Length timer ticks at a rate of 256Hz
        if divider_ticks % 2 == 0 && self.length_timer.tick() {
            // Disable channel when length timer expires
            self.generation_on = false;
        }

        // Volume envelope timer ticks at a rate of 64Hz
        if divider_ticks % 8 == 7 {
            self.volume_control.tick();
        }
    }

    // Tick the channel's frequency timer for 1 M-cycle (4 APU clock cycles)
    pub(crate) fn tick_clock(&mut self) {
        if self.frequency_timer.tick_m_cycle() {
            // The 8 phase positions loop forever
            self.phase_position = (self.phase_position + 1) % 8;
        }
    }

    // Should be called when the sweep timer clocks.
    fn process_sweep_tick(&mut self, io_registers: &mut IoRegisters) {
        let sweep_result = self.sweep.tick();

        match sweep_result {
            SweepResult::None => {}
            SweepResult::Overflowed => {
                // Disable channel when sweep overflows/underflows frequency
                self.generation_on = false;
            }
            SweepResult::Changed(frequency) => {
                // Only update frequency timer if shift is non-zero
                if self.sweep.shift > 0 {
                    self.frequency_timer.frequency = frequency;

                    // Write out updated frequency to NRx3 and NRx4
                    io_registers.apu_write_register(self.nr3, (frequency & 0xFF) as u8);
                    let nr4 = io_registers.apu_read_register(self.nr4);
                    io_registers
                        .apu_write_register(self.nr4, (nr4 & 0xF8) | (frequency >> 8) as u8);

                    // Immediately run an overflow check and disable the channel if it trips
                    if self.sweep.next_frequency().is_none() {
                        self.generation_on = false;
                    }
                }
            }
        }
    }
}

impl Channel for PulseChannel {
    fn channel_enabled(&self) -> bool {
        self.generation_on
    }

    fn dac_enabled(&self) -> bool {
        self.dac_on
    }

    fn sample_digital(&self) -> Option<u8> {
        if !self.dac_on {
            // Return no signal if the DAC is disabled
            return None;
        }

        if !self.generation_on {
            // Return a constant 0 if the channel is disabled but the DAC is on
            return Some(0);
        }

        // Digital output is 0 if waveform sample is 0, {volume} otherwise
        let wave_step = self.duty_cycle.waveform()[self.phase_position as usize];
        Some(wave_step * self.volume_control.volume)
    }
}
