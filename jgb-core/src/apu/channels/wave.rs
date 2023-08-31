use crate::apu::channels::{Channel, FrequencyTimer, LengthTimer};
use crate::memory::ioregisters::{IoRegister, IoRegisters};
use serde::{Deserialize, Serialize};

// A custom wave channel (channel 3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WaveChannel {
    generation_on: bool,
    dac_on: bool,
    length_timer: LengthTimer,
    volume_shift: u8,
    frequency_timer: FrequencyTimer,
    sample_index: u8,
    last_sample: u8,
}

impl WaveChannel {
    pub(crate) fn new() -> Self {
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
    pub(crate) fn process_register_updates(
        &mut self,
        io_registers: &mut IoRegisters,
        divider_ticks: u64,
    ) {
        let nr30_value = io_registers.apu_read_register(IoRegister::NR30);
        let nr31_value = io_registers.apu_read_register(IoRegister::NR31);
        let nr32_value = io_registers.apu_read_register(IoRegister::NR32);
        let nr33_value = io_registers.apu_read_register(IoRegister::NR33);
        let nr34_value = io_registers.apu_read_register(IoRegister::NR34);

        // Update 8-bit length timer immediately if NR31 was written to
        if io_registers.get_dirty_bit(IoRegister::NR31) {
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
        self.frequency_timer.frequency = frequency;

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
    pub(crate) fn tick_divider(&mut self, divider_ticks: u64) {
        // Length timer ticks at a rate of 256Hz
        if divider_ticks % 2 == 0 && self.length_timer.tick() {
            // Disable channel when length timer expires
            self.generation_on = false;
        }
    }

    // Tick frequency timer for 1 M-cycle (4 APU clock cycles).
    pub(crate) fn tick_clock(&mut self, io_registers: &IoRegisters) {
        if self.frequency_timer.tick_m_cycle() {
            // Read the current 4-bit sample from custom waveform RAM and update the internal sample
            // buffer
            let samples = io_registers.read_address(0xFF30 + u16::from(self.sample_index / 2));
            let sample = if self.sample_index % 2 == 0 { samples >> 4 } else { samples & 0x0F };
            self.last_sample = sample;

            // The 32 samples loop forever (or until the length timer expires)
            self.sample_index = (self.sample_index + 1) % 32;
        }
    }
}

impl Channel for WaveChannel {
    fn channel_enabled(&self) -> bool {
        self.generation_on
    }

    fn dac_enabled(&self) -> bool {
        self.dac_on
    }

    fn sample_digital(&self) -> Option<u8> {
        if !self.dac_on {
            // Output no signal if DAC is off
            return None;
        }

        if !self.generation_on || self.volume_shift == 8 {
            // Output a constant 0 if the channel is off or channel volume is set to 0% (shift 8)
            return Some(0);
        }

        // Digital sample is whatever is in the sample buffer multiplied by the volume multiplier
        Some(self.last_sample >> self.volume_shift)
    }
}
