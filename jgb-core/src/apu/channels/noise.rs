use crate::apu::channels::{Channel, LengthTimer, VolumeControl};
use crate::memory::ioregisters::{IoRegister, IoRegisters};

const CLOCK_CYCLES_PER_M_CYCLE: u64 = crate::apu::CLOCK_CYCLES_PER_M_CYCLE;

// A pseudo-random noise channel (channel 4). Internally uses a linear-feedback shift register.
#[derive(Debug, Clone)]
pub(crate) struct NoiseChannel {
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
    pub(crate) fn new() -> Self {
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
    pub(crate) fn process_register_updates(
        &mut self,
        io_registers: &mut IoRegisters,
        divider_ticks: u64,
    ) {
        let nr41_value = io_registers.apu_read_register(IoRegister::NR41);
        let nr42_value = io_registers.apu_read_register(IoRegister::NR42);
        let nr43_value = io_registers.apu_read_register(IoRegister::NR43);
        let nr44_value = io_registers.apu_read_register(IoRegister::NR44);

        // Update 6-bit length timer if NR41 was written to
        if io_registers.get_dirty_bit(IoRegister::NR41) {
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
    pub(crate) fn tick_divider(&mut self, divider_ticks: u64) {
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
    pub(crate) fn tick_clock(&mut self) {
        let prev_clock = self.frequency_timer;
        self.frequency_timer += CLOCK_CYCLES_PER_M_CYCLE;

        // LFSR timer clocks at a rate of (16 * divider * 2^shift) Hz, treating divider of 0 as 0.5
        let lfsr_period: u64 = if self.clock_divider != 0 {
            16 * (u32::from(self.clock_divider) << self.clock_shift)
        } else {
            8 * (1 << self.clock_shift)
        }
        .into();

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
