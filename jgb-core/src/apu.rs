mod timer;

use crate::apu::timer::FrequencyTimer;
use crate::memory::ioregisters::{IoRegister, IoRegisters};
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

// Waveform for square wave channels (12.5% / 25% / 50% / 75%). Each waveform has 8 samples which
// are each 0 or 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
enum SweepDirection {
    Increasing,
    Decreasing,
}

// Volume state & envelope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VolumeControl {
    volume: u8,
    sweep_direction: SweepDirection,
    pace: u8,
    timer: u8,
    envelope_enabled: bool,
}

impl VolumeControl {
    fn new() -> Self {
        Self {
            volume: 0,
            sweep_direction: SweepDirection::Decreasing,
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
            sweep_direction: if byte & 0x08 != 0 {
                SweepDirection::Increasing
            } else {
                SweepDirection::Decreasing
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

                let overflowed = match self.sweep_direction {
                    SweepDirection::Increasing => self.volume == 0x0F,
                    SweepDirection::Decreasing => self.volume == 0x00,
                };
                if overflowed {
                    self.envelope_enabled = false;
                } else {
                    let new_volume = match self.sweep_direction {
                        SweepDirection::Increasing => self.volume + 1,
                        SweepDirection::Decreasing => self.volume - 1,
                    };
                    self.volume = new_volume;
                }
            }
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
#[derive(Debug, Clone, Copy)]
struct PulseSweep {
    pace: u8,
    direction: SweepDirection,
    shift: u8,
    timer: u8,
    enabled: bool,
    shadow_frequency: u16,
    generated_with_negate: bool,
}

impl PulseSweep {
    const DISABLED: Self = Self {
        pace: 0,
        direction: SweepDirection::Decreasing,
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
        if self.direction == SweepDirection::Decreasing {
            self.generated_with_negate = true;
        }

        let frequency = self.shadow_frequency;

        let delta = frequency >> self.shift;
        let next_frequency = match self.direction {
            SweepDirection::Increasing => frequency + delta,
            SweepDirection::Decreasing => frequency.wrapping_sub(delta),
        };

        if next_frequency <= 0x07FF {
            Some(next_frequency)
        } else {
            None
        }
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

trait Channel {
    // Digital sample in the range [0, 15]
    fn sample_digital(&self) -> Option<u8>;

    // "Analog" sample in the range [-1, 1]
    fn sample_analog(&self) -> f64 {
        let Some(digital_sample) = self.sample_digital() else { return 0.0; };

        (f64::from(digital_sample) - 7.5) / 7.5
    }
}

// A square wave channel (channels 1 & 2).
#[derive(Debug, Clone)]
struct PulseChannel {
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
    fn new(
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
    fn new_channel_1() -> Self {
        Self::new(
            Some(IoRegister::NR10),
            IoRegister::NR11,
            IoRegister::NR12,
            IoRegister::NR13,
            IoRegister::NR14,
        )
    }

    // Create a square wave channel configured to read from channel 2 audio registers (no sweep)
    fn new_channel_2() -> Self {
        Self::new(
            None,
            IoRegister::NR21,
            IoRegister::NR22,
            IoRegister::NR23,
            IoRegister::NR24,
        )
    }

    // Update the channel's internal state based on audio register contents and updates.
    fn process_register_updates(&mut self, io_registers: &mut IoRegisters, divider_ticks: u64) {
        let nr1_value = io_registers.apu_read_register(self.nr1);
        let nr2_value = io_registers.apu_read_register(self.nr2);
        let nr4_value = io_registers.apu_read_register(self.nr4);

        // Only check sweep if an NRx0 register is configured
        if let Some(nr0) = self.nr0 {
            let nr0_value = io_registers.apu_read_register(nr0);

            let sweep_pace = (nr0_value & 0x70) >> 4;
            let sweep_direction = if nr0_value & 0x08 != 0 {
                SweepDirection::Decreasing
            } else {
                SweepDirection::Increasing
            };
            let sweep_shift = nr0_value & 0x07;

            self.sweep.pace = sweep_pace;
            self.sweep.direction = sweep_direction;
            self.sweep.shift = sweep_shift;

            // If the sweep generated any frequency calculations with decreasing sweep since the
            // last trigger, switching to increasing sweep should disable the channel
            if self.sweep.generated_with_negate && sweep_direction == SweepDirection::Increasing {
                self.generation_on = false;
            }
        }

        // Sync duty cycle with NRx1 register (bits 6-7), updates take effect immediately
        let duty_cycle = match nr1_value & 0xC0 {
            0x00 => DutyCycle::OneEighth,
            0x40 => DutyCycle::OneFourth,
            0x80 => DutyCycle::OneHalf,
            0xC0 => DutyCycle::ThreeFourths,
            _ => panic!("{nr1_value} & 0xC0 was not 0x00/0x40/0x80/0xC0"),
        };
        self.duty_cycle = duty_cycle;

        // Check if 6-bit length timer has been reset (NRx1 bits 0-5)
        if io_registers.is_register_dirty(self.nr1) {
            io_registers.clear_dirty_bit(self.nr1);
            self.length_timer.timer = (64 - (nr1_value & 0x3F)).into();
        }

        // Zombie mode hack - increase volume by 1 when volume register is written to while
        // envelope pace is 0, wrapping around from 15 to 0
        if io_registers.is_register_dirty(self.nr2) {
            io_registers.clear_dirty_bit(self.nr2);

            let pending_volume_control = VolumeControl::from_byte(nr2_value);
            if self.volume_control.envelope_enabled
                && self.volume_control.pace == 0
                && pending_volume_control.sweep_direction == SweepDirection::Increasing
            {
                self.volume_control.volume = (self.volume_control.volume + 1) % 16;
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
        if io_registers.is_register_dirty(self.nr3) || io_registers.is_register_dirty(self.nr4) {
            io_registers.clear_dirty_bit(self.nr3);
            io_registers.clear_dirty_bit(self.nr4);

            let new_frequency = read_frequency(io_registers, self.nr3, self.nr4);
            self.frequency_timer.set_frequency(new_frequency);
        }

        // Re-initialize the channel if a value was written to NRx4 with bit 7 set
        let triggered = nr4_value & 0x80 != 0;
        if triggered {
            // Clear trigger flag
            io_registers.apu_write_register(self.nr4, nr4_value & 0x7F);

            // Re-initialize sweep (if applicable)
            self.sweep.trigger(self.frequency_timer.frequency());

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
    fn tick_divider(&mut self, divider_ticks: u64, io_registers: &mut IoRegisters) {
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
    fn tick_clock(&mut self) {
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
                    self.frequency_timer.set_frequency(frequency);

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
    fn sample_digital(&self) -> Option<u8> {
        if !self.dac_on {
            // Return no signal if the DAC is disabled
            return None;
        }

        if !self.generation_on {
            // Return a constant 0 if the channel is disabled but the DAC is on
            return Some(0);
        }

        // TODO this is a hack, remove once better audio downsampling is implemented
        if 131072.0 / f64::from(2048 - self.frequency_timer.frequency())
            > OUTPUT_FREQUENCY as f64 / 2.0
        {
            return Some(0);
        }

        // Digital output is 0 if waveform sample is 0, {volume} otherwise
        let wave_step = self.duty_cycle.waveform()[self.phase_position as usize];
        Some(wave_step * self.volume_control.volume)
    }
}

// Read an 11-bit wave frequency out of the specified NRx3 and NRx4 registers.
// The lower 8 bits come from NRx3 and the higher 3 bits come from NRx4 (bits 0-2).
fn read_frequency(io_registers: &IoRegisters, nr3: IoRegister, nr4: IoRegister) -> u16 {
    let nr3_value = io_registers.apu_read_register(nr3);
    let nr4_value = io_registers.apu_read_register(nr4);

    ((u16::from(nr4_value) & 0x07) << 8) | u16::from(nr3_value)
}

// A custom wave channel (channel 3).
#[derive(Debug, Clone)]
struct WaveChannel {
    generation_on: bool,
    dac_on: bool,
    length_timer: LengthTimer,
    volume_shift: u8,
    frequency_timer: FrequencyTimer,
    sample_index: u8,
    last_sample: u8,
}

impl WaveChannel {
    fn new() -> Self {
        Self {
            generation_on: false,
            dac_on: false,
            length_timer: LengthTimer::new(),
            volume_shift: 0,
            frequency_timer: FrequencyTimer::new(2),
            sample_index: 1,
            last_sample: 0,
        }
    }

    // Update the channel's internal state based on audio register contents and updates.
    fn process_register_updates(&mut self, io_registers: &mut IoRegisters, divider_ticks: u64) {
        let nr30_value = io_registers.apu_read_register(IoRegister::NR30);
        let nr31_value = io_registers.apu_read_register(IoRegister::NR31);
        let nr32_value = io_registers.apu_read_register(IoRegister::NR32);
        let nr33_value = io_registers.apu_read_register(IoRegister::NR33);
        let nr34_value = io_registers.apu_read_register(IoRegister::NR34);

        // Update 8-bit length timer immediately if NR31 was written to
        if io_registers.is_register_dirty(IoRegister::NR31) {
            io_registers.clear_dirty_bit(IoRegister::NR31);
            self.length_timer.timer = 256 - u16::from(nr31_value);
        }

        // Sync volume shift from NR32, updates take effect immediately
        self.volume_shift = match nr32_value & 0x60 {
            0x00 => 8, // Disabled
            0x20 => 0, // 100%
            0x40 => 1, // 50%
            0x60 => 2, // 25%
            _ => panic!("{nr32_value} & 0x60 was not 0x00/0x20/0x40/0x60"),
        };

        // Sync frequency with NR33 and NR34 registers, updates take effect "immediately" (the next
        // time the frequency timer clocks)
        let frequency = (u16::from(nr34_value & 0x07) << 8) | u16::from(nr33_value);
        self.frequency_timer.set_frequency(frequency);

        // Sync length timer enabled flag with NRx4 bit 6, updates take effect immediately
        let prev_length_timer_enabled = self.length_timer.enabled;
        self.length_timer.enabled = nr34_value & 0x40 != 0;

        // When the length timer is enabled, if this is an off-cycle divider tick, immediately
        // tick the length timer and disable the channel if it clocks
        if !prev_length_timer_enabled
            && self.length_timer.enabled
            && divider_ticks % 2 == 0
            && self.length_timer.tick()
        {
            self.generation_on = false;
        }

        // Re-initialize channel if NR34 bit 7 was set
        let triggered = nr34_value & 0x80 != 0;
        if triggered {
            // Clear trigger flag
            io_registers.apu_write_register(IoRegister::NR34, nr34_value & 0x7F);

            // Reset frequency timer and wave sample index
            self.frequency_timer.trigger();
            self.sample_index = 1;

            // Reset length timer to the maximum possible if it expired
            self.length_timer.trigger(256, divider_ticks);

            self.generation_on = true;
        }

        // DAC is on iff NR30 bit 7 is set
        self.dac_on = nr30_value & 0x80 != 0;
        if !self.dac_on {
            // Disable channel if DAC is off
            self.generation_on = false;
        }
    }

    // Tick internal sequencer timers. This should be called at a rate of 512Hz
    fn tick_divider(&mut self, divider_ticks: u64) {
        // Length timer ticks at a rate of 256Hz
        if divider_ticks % 2 == 0 && self.length_timer.tick() {
            // Disable channel when length timer expires
            self.generation_on = false;
        }
    }

    // Tick frequency timer for 1 M-cycle (4 APU clock cycles).
    fn tick_clock(&mut self, io_registers: &IoRegisters) {
        if self.frequency_timer.tick_m_cycle() {
            // Read the current 4-bit sample from custom waveform RAM and update the internal sample
            // buffer
            let samples = io_registers.read_address(0xFF30 + u16::from(self.sample_index / 2));
            let sample = if self.sample_index % 2 == 0 {
                samples >> 4
            } else {
                samples & 0x0F
            };
            self.last_sample = sample;

            // The 32 samples loop forever (or until the length timer expires)
            self.sample_index = (self.sample_index + 1) % 32;
        }
    }
}

impl Channel for WaveChannel {
    fn sample_digital(&self) -> Option<u8> {
        if !self.dac_on {
            // Output no signal if DAC is off
            return None;
        }

        if !self.generation_on || self.volume_shift == 8 {
            // Output a constant 0 if the channel is off or channel volume is set to 0% (shift 8)
            return Some(0);
        }

        // TODO this is a hack, remove once better audio downsampling is implemented
        if 65536.0 / f64::from(2048 - self.frequency_timer.frequency())
            > OUTPUT_FREQUENCY as f64 / 2.0
        {
            return Some(0);
        }

        // Digital sample is whatever is in the sample buffer multiplied by the volume multiplier
        Some(self.last_sample >> self.volume_shift)
    }
}

// A pseudo-random noise channel (channel 4). Internally uses a linear-feedback shift register.
#[derive(Debug, Clone)]
struct NoiseChannel {
    generation_on: bool,
    dac_on: bool,
    length_timer: LengthTimer,
    volume_control: VolumeControl,
    clock_shift: u8,
    lfsr: u16,
    lfsr_width: u8,
    clock_divider: u8,
    frequency_timer: u64,
}

impl NoiseChannel {
    fn new() -> Self {
        Self {
            generation_on: false,
            dac_on: false,
            length_timer: LengthTimer::new(),
            volume_control: VolumeControl::new(),
            clock_shift: 0,
            lfsr: 0,
            lfsr_width: 7,
            clock_divider: 0,
            frequency_timer: 0,
        }
    }

    // Update the channel's internal state based on audio register contents and updates.
    fn process_register_updates(&mut self, io_registers: &mut IoRegisters, divider_ticks: u64) {
        let nr41_value = io_registers.apu_read_register(IoRegister::NR41);
        let nr42_value = io_registers.apu_read_register(IoRegister::NR42);
        let nr43_value = io_registers.apu_read_register(IoRegister::NR43);
        let nr44_value = io_registers.apu_read_register(IoRegister::NR44);

        // Update 6-bit length timer if NR41 was written to
        if io_registers.is_register_dirty(IoRegister::NR41) {
            io_registers.clear_dirty_bit(IoRegister::NR41);
            self.length_timer.timer = (64 - (nr41_value & 0x3F)).into();
        }

        // Update LFSR parameters from NR43, updates take effect the next frequency timer clock
        self.clock_shift = nr43_value >> 4;
        self.lfsr_width = if nr43_value & 0x80 != 0 { 7 } else { 15 };
        self.clock_divider = nr43_value & 0x07;

        // Sync length timer enabled flag with NRx4 bit 6, updates take effect immediately
        let prev_length_timer_enabled = self.length_timer.enabled;
        self.length_timer.enabled = nr44_value & 0x40 != 0;

        // When the length timer is enabled, if this is an off-cycle divider tick, immediately
        // tick the length timer and disable the channel if it clocks
        if !prev_length_timer_enabled
            && self.length_timer.enabled
            && divider_ticks % 2 == 0
            && self.length_timer.tick()
        {
            self.generation_on = false;
        }

        // Re-initialize channel if NR44 bit 7 was set
        let triggered = nr44_value & 0x80 != 0;
        if triggered {
            // Clear trigger flag
            io_registers.apu_write_register(IoRegister::NR44, nr44_value & 0x7F);

            // Reset frequency timer
            self.frequency_timer = 0;

            // Re-initialize volume & envelope from NR42
            self.volume_control = VolumeControl::from_byte(nr42_value);

            // Reset length timer to the maximum possible if it expired
            self.length_timer.trigger(64, divider_ticks);

            // Fully clear the LFSR
            self.lfsr = 0;

            self.generation_on = true;
        }

        // DAC is on iff any of NR42 bits 3-7 are set
        self.dac_on = nr42_value & 0xF8 != 0;
        if !self.dac_on {
            // Disable channel if DAC is off
            self.generation_on = false;
        }
    }

    // Tick internal sequencer timers (512Hz)
    fn tick_divider(&mut self, divider_ticks: u64) {
        // Length timer ticks at a rate of 256Hz
        if divider_ticks % 2 == 0 && self.length_timer.tick() {
            // Disable channel if length timer expired
            self.generation_on = false;
        }

        // Volume envelope timer ticks at a rate of 64Hz
        if divider_ticks % 8 == 7 {
            self.volume_control.tick();
        }
    }

    // Tick internal frequency timer for 1 M-cycle (4 APU clock cycles).
    fn tick_clock(&mut self) {
        let prev_clock = self.frequency_timer;
        self.frequency_timer += CLOCK_CYCLES_PER_M_CYCLE;

        // LFSR timer clocks at a rate of (16 * divider * 2^shift) Hz, treating divider of 0 as 0.5
        let divisor = if self.clock_divider != 0 {
            f64::from(u32::from(self.clock_divider) << self.clock_shift)
        } else {
            0.5 * 2_f64.powi(self.clock_shift.into())
        };
        let lfsr_period = (16.0 * divisor).round() as u64;

        if prev_clock / lfsr_period != self.frequency_timer / lfsr_period {
            // Update and shift LFSR
            let bit_1 = (self.lfsr & 0x02) >> 1;
            let bit_0 = self.lfsr & 0x01;
            let new_bit = !(bit_1 ^ bit_0);

            // Feedback always applies to bit 15, and if LFSR width is 7 it also applies to bit 7
            let new_lfsr = if self.lfsr_width == 15 {
                (new_bit << 15) | (self.lfsr & 0x7FFF)
            } else {
                (new_bit << 15) | (new_bit << 7) | (self.lfsr & 0x7F7F)
            };
            self.lfsr = new_lfsr >> 1;
        }
    }
}

impl Channel for NoiseChannel {
    fn sample_digital(&self) -> Option<u8> {
        if !self.dac_on {
            // Output no signal if DAC is off
            return None;
        }

        if !self.generation_on {
            // Output a constant 0 if channel is off but DAC is on
            return Some(0);
        }

        // Output 0 or <volume> based on the current LFSR bit 0
        if self.lfsr & 0x0001 != 0 {
            Some(self.volume_control.volume)
        } else {
            Some(0)
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ApuDebugOutput {
    pub ch1_l: f64,
    pub ch1_r: f64,
    pub ch2_l: f64,
    pub ch2_r: f64,
    pub ch3_l: f64,
    pub ch3_r: f64,
    pub ch4_l: f64,
    pub ch4_r: f64,
    pub master_l: i16,
    pub master_r: i16,
}

pub trait DebugSink {
    fn collect_samples(&self, samples: &ApuDebugOutput);
}
// High-pass filter capacitor charge factor
static HPF_CHARGE_FACTOR: Lazy<f64> =
    Lazy::new(|| 0.999958_f64.powf((4 * 1024 * 1024) as f64 / OUTPUT_FREQUENCY as f64));
static HPF_CHARGE_FACTOR_60HZ: Lazy<f64> = Lazy::new(|| {
    0.999958_f64.powf((4 * 1024 * 1024) as f64 / OUTPUT_FREQUENCY as f64 * 60.0 / 59.7)
});

pub struct ApuState {
    enabled: bool,
    last_divider: u8,
    divider_ticks: u64,
    clock_ticks: u64,
    channel_1: PulseChannel,
    channel_2: PulseChannel,
    channel_3: WaveChannel,
    channel_4: NoiseChannel,
    hpf_capacitor_l: f64,
    hpf_capacitor_r: f64,
    sample_queue: Arc<Mutex<VecDeque<i16>>>,
    debug_sink: Option<Box<dyn DebugSink>>,
}

impl ApuState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            last_divider: 0x00,
            divider_ticks: 0,
            clock_ticks: 0,
            channel_1: PulseChannel::new_channel_1(),
            channel_2: PulseChannel::new_channel_2(),
            channel_3: WaveChannel::new(),
            channel_4: NoiseChannel::new(),
            hpf_capacitor_l: 0.0,
            hpf_capacitor_r: 0.0,
            sample_queue: Arc::new(Mutex::new(VecDeque::new())),
            debug_sink: None,
        }
    }

    pub fn new_with_debug_sink(debug_sink: Box<dyn DebugSink>) -> Self {
        Self {
            debug_sink: Some(debug_sink),
            ..Self::new()
        }
    }

    pub fn get_sample_queue(&self) -> &Arc<Mutex<VecDeque<i16>>> {
        &self.sample_queue
    }

    fn process_register_updates(&mut self, io_registers: &mut IoRegisters) {
        if self.enabled {
            self.channel_1
                .process_register_updates(io_registers, self.divider_ticks);
            self.channel_2
                .process_register_updates(io_registers, self.divider_ticks);
            self.channel_3
                .process_register_updates(io_registers, self.divider_ticks);
            self.channel_4
                .process_register_updates(io_registers, self.divider_ticks);
        }
    }

    // Tick 512Hz timers by 1 tick
    fn tick_divider(&mut self, io_registers: &mut IoRegisters) {
        self.divider_ticks += 1;

        if self.enabled {
            self.channel_1
                .tick_divider(self.divider_ticks, io_registers);
            self.channel_2
                .tick_divider(self.divider_ticks, io_registers);
            self.channel_3.tick_divider(self.divider_ticks);
            self.channel_4.tick_divider(self.divider_ticks);
        }
    }

    // Tick M-cycle / APU clock cycle timers by 1 M-cycle (4 APU clock cycles)
    fn tick_clock(&mut self, io_registers: &IoRegisters) {
        self.clock_ticks += CLOCK_CYCLES_PER_M_CYCLE;

        if self.enabled {
            self.channel_1.tick_clock();
            self.channel_2.tick_clock();
            self.channel_3.tick_clock(io_registers);
            self.channel_4.tick_clock();
        }
    }

    // Turn off the APU and disable all channels and DACs
    fn disable(&mut self) {
        self.enabled = false;
        self.channel_1 = PulseChannel::new_channel_1();
        self.channel_2 = PulseChannel::new_channel_2();
        self.channel_3 = WaveChannel::new();
        self.channel_4 = NoiseChannel::new();
    }

    // Retrieve left and right audio samples based on the current channel states.
    //
    // Note that calling this method modifies the high-pass filter capacitor values. It should be
    // called at a rate of 4.194304MHz / output frequency.
    //
    // Downsampling from the raw 1.048576MHz signal to the output frequency is done using dumb
    // nearest-neighbor sampling.
    fn sample(&mut self, nr50_value: u8, nr51_value: u8, audio_60hz: bool) -> (i16, i16) {
        let mut sample_l = 0.0;
        let mut sample_r = 0.0;

        // Sample channel 1
        let ch1_sample = self.channel_1.sample_analog();
        let ch1_l = ch1_sample * f64::from(nr51_value & 0x10 != 0);
        let ch1_r = ch1_sample * f64::from(nr51_value & 0x01 != 0);
        sample_l += ch1_l;
        sample_r += ch1_r;

        // Sample channel 2
        let ch2_sample = self.channel_2.sample_analog();
        let ch2_l = ch2_sample * f64::from(nr51_value & 0x20 != 0);
        let ch2_r = ch2_sample * f64::from(nr51_value & 0x02 != 0);
        sample_l += ch2_l;
        sample_r += ch2_r;

        // Sample channel 3
        let ch3_sample = self.channel_3.sample_analog();
        let ch3_l = ch3_sample * f64::from(nr51_value & 0x40 != 0);
        let ch3_r = ch3_sample * f64::from(nr51_value & 0x04 != 0);
        sample_l += ch3_l;
        sample_r += ch3_r;

        // Sample channel 4
        let ch4_sample = self.channel_4.sample_analog();
        let ch4_l = ch4_sample * f64::from(nr51_value & 0x80 != 0);
        let ch4_r = ch4_sample * f64::from(nr51_value & 0x08 != 0);
        sample_l += ch4_l;
        sample_r += ch4_r;

        // Master volume multiplers range from [1, 8]
        let l_volume = ((nr50_value & 0x70) >> 4) + 1;
        let r_volume = (nr50_value & 0x07) + 1;

        // Map [-4, 4] to [-1, 1] before applying high-pass filter
        let mut sample_l = sample_l / 4.0;
        let mut sample_r = sample_r / 4.0;

        // Apply high-pass filter if any of the four DACs are on
        if self.channel_1.dac_on
            || self.channel_2.dac_on
            || self.channel_3.dac_on
            || self.channel_4.dac_on
        {
            sample_l = high_pass_filter(sample_l, &mut self.hpf_capacitor_l, audio_60hz);
            sample_r = high_pass_filter(sample_r, &mut self.hpf_capacitor_r, audio_60hz);
        }

        // Map [-1, 1] to [-30000, 30000] and apply L/R volume multipliers
        let sample_l = (sample_l * 30000.0 * f64::from(l_volume) / 8.0).round() as i16;
        let sample_r = (sample_r * 30000.0 * f64::from(r_volume) / 8.0).round() as i16;

        if let Some(debug_sink) = &self.debug_sink {
            debug_sink.collect_samples(&ApuDebugOutput {
                ch1_l,
                ch1_r,
                ch2_l,
                ch2_r,
                ch3_l,
                ch3_r,
                ch4_l,
                ch4_r,
                master_l: sample_l,
                master_r: sample_r,
            });
        }

        (sample_l, sample_r)
    }
}

// Apply a simple high-pass filter to the given sample. This mimics what the actual hardware does.
fn high_pass_filter(sample: f64, capacitor: &mut f64, audio_60hz: bool) -> f64 {
    let filtered_sample = sample - *capacitor;

    let charge_factor = if audio_60hz {
        *HPF_CHARGE_FACTOR_60HZ
    } else {
        *HPF_CHARGE_FACTOR
    };

    *capacitor = sample - charge_factor * filtered_sample;

    filtered_sample
}

// Output sample frequency in Hz
pub const OUTPUT_FREQUENCY: u64 = 48000;

const CLOCK_CYCLES_PER_M_CYCLE: u64 = 4;
const APU_CLOCK_SPEED: u64 = 4 * 1024 * 1024;

const ALL_AUDIO_REGISTERS: [IoRegister; 21] = [
    IoRegister::NR10,
    IoRegister::NR11,
    IoRegister::NR12,
    IoRegister::NR13,
    IoRegister::NR14,
    IoRegister::NR21,
    IoRegister::NR22,
    IoRegister::NR23,
    IoRegister::NR24,
    IoRegister::NR30,
    IoRegister::NR31,
    IoRegister::NR32,
    IoRegister::NR33,
    IoRegister::NR34,
    IoRegister::NR41,
    IoRegister::NR42,
    IoRegister::NR43,
    IoRegister::NR44,
    IoRegister::NR50,
    IoRegister::NR51,
    IoRegister::NR52,
];

// Return whether the APU emulator should output audio samples during the current M-cycle tick.
// This is currently just a naive "output every 4.194304 MHz / <output_frequency> clock cycles"
fn should_sample(apu_state: &ApuState, prev_clock_ticks: u64, audio_60hz: bool) -> bool {
    let prev_period = prev_clock_ticks as f64 * OUTPUT_FREQUENCY as f64 / APU_CLOCK_SPEED as f64;
    let current_period =
        apu_state.clock_ticks as f64 * OUTPUT_FREQUENCY as f64 / APU_CLOCK_SPEED as f64;

    let (prev_period, current_period) = if audio_60hz {
        (
            (prev_period * 59.7 / 60.0).round() as u64,
            (current_period * 59.7 / 60.0).round() as u64,
        )
    } else {
        (prev_period.round() as u64, current_period.round() as u64)
    };

    prev_period != current_period
}

// Progress the APU by 1 M-cycle (4 APU clock cycles). Audio samples will be written to the APU
// state's sample queue if appropriate.
pub fn tick_m_cycle(apu_state: &mut ApuState, io_registers: &mut IoRegisters, audio_60hz: bool) {
    // Tick M-cycle / APU clock cycle timers
    let prev_clock = apu_state.clock_ticks;
    apu_state.tick_clock(io_registers);

    let nr52_value = io_registers.apu_read_register(IoRegister::NR52);
    let apu_enabled = nr52_value & 0x80 != 0;

    if !apu_enabled {
        if apu_state.enabled {
            // If the APU was just disabled, clear all audio registers and reset all channels
            for audio_register in ALL_AUDIO_REGISTERS {
                io_registers.apu_write_register(audio_register, 0x00);
            }
            apu_state.disable();
        }

        if should_sample(apu_state, prev_clock, audio_60hz) {
            // Output constant 0s if the APU is disabled
            let mut sample_queue = apu_state.sample_queue.lock().unwrap();
            sample_queue.push_back(0);
            sample_queue.push_back(0);
        }

        return;
    }
    apu_state.enabled = true;

    // Tick 512Hz timers every time DIV bit 4 flips from 1 to 0
    let divider = io_registers.read_register(IoRegister::DIV);
    if apu_state.last_divider & 0x10 != 0 && divider & 0x10 == 0 {
        apu_state.tick_divider(io_registers);
    }
    apu_state.last_divider = divider;

    // Update channel states based on audio register contents and updates
    apu_state.process_register_updates(io_registers);

    // Write out the read-only NR52 bits that specify which channels are enabled
    let new_nr52_value = (nr52_value & 0x80)
        | (u8::from(apu_state.channel_4.generation_on) << 3)
        | (u8::from(apu_state.channel_3.generation_on) << 2)
        | (u8::from(apu_state.channel_2.generation_on) << 1)
        | u8::from(apu_state.channel_1.generation_on);
    io_registers.apu_write_register(IoRegister::NR52, new_nr52_value);

    if should_sample(apu_state, prev_clock, audio_60hz) {
        let (sample_l, sample_r) = apu_state.sample(
            io_registers.apu_read_register(IoRegister::NR50),
            io_registers.apu_read_register(IoRegister::NR51),
            audio_60hz,
        );

        let mut sample_queue = apu_state.sample_queue.lock().unwrap();
        sample_queue.push_back(sample_l);
        sample_queue.push_back(sample_r);

        // Ensure that the sample queue doesn't get too large. This should only ever trip if
        // audio sync is disabled.
        while sample_queue.len() > 8192 {
            sample_queue.pop_front();
        }
    }
}
