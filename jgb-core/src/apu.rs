use crate::memory::ioregisters::{IoRegister, IoRegisters};
use std::cmp;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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
            Self::OneEighth => [1, 1, 1, 1, 1, 1, 1, 0],
            Self::OneFourth => [0, 1, 1, 1, 1, 1, 1, 0],
            Self::OneHalf => [0, 1, 1, 1, 1, 0, 0, 0],
            Self::ThreeFourths => [1, 0, 0, 0, 0, 0, 0, 1],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SweepDirection {
    Increasing,
    Decreasing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VolumeControl {
    volume: u8,
    sweep_direction: SweepDirection,
    sweep_pace: u8,
}

impl VolumeControl {
    fn new() -> Self {
        Self {
            volume: 0,
            sweep_direction: SweepDirection::Decreasing,
            sweep_pace: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PulseSweep {
    pace: u8,
    direction: SweepDirection,
    slope_control: u8,
}

impl PulseSweep {
    const DISABLED: Self = Self {
        pace: 0,
        direction: SweepDirection::Decreasing,
        slope_control: 0,
    };
}

trait Channel {
    // Digital sample in the range [0, 15]
    fn sample_digital(&self) -> Option<u8>;

    // "Analog" sample in the range [-1, 1]
    fn sample_analog(&self) -> f64 {
        let Some(digital_sample) = self.sample_digital() else { return 0.0; };

        (7.5 - f64::from(digital_sample)) / 7.5
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PulseChannel {
    generation_on: bool,
    dac_on: bool,
    duty_cycle: DutyCycle,
    length_timer: u8,
    length_timer_enabled: bool,
    volume_control: VolumeControl,
    wavelength: u16,
    sweep: PulseSweep,
    next_sweep: Option<PulseSweep>,
    divider_ticks: u64,
    base_phase_position: u64,
    clock_ticks: u64,
}

impl PulseChannel {
    fn new() -> Self {
        Self {
            generation_on: false,
            dac_on: false,
            duty_cycle: DutyCycle::OneEighth,
            length_timer: 0,
            length_timer_enabled: false,
            volume_control: VolumeControl::new(),
            wavelength: 0,
            sweep: PulseSweep::DISABLED,
            next_sweep: None,
            divider_ticks: 0,
            base_phase_position: 0,
            clock_ticks: 0,
        }
    }

    fn process_register_updates(
        &mut self,
        io_registers: &mut IoRegisters,
        nr0: Option<IoRegister>,
        nr1: IoRegister,
        nr2: IoRegister,
        nr3: IoRegister,
        nr4: IoRegister,
    ) {
        let nr1_value = io_registers.apu_read_register(nr1);
        let nr2_value = io_registers.apu_read_register(nr2);
        let nr3_value = io_registers.apu_read_register(nr3);
        let nr4_value = io_registers.apu_read_register(nr4);

        if let Some(nr0) = nr0 {
            let nr0_value = io_registers.apu_read_register(nr0);

            let sweep_pace = (nr0_value & 0x70) >> 4;
            let sweep_direction = if nr0_value & 0x08 != 0 {
                SweepDirection::Decreasing
            } else {
                SweepDirection::Increasing
            };
            let sweep_slope_control = nr0_value & 0x07;

            let sweep = PulseSweep {
                pace: sweep_pace,
                direction: sweep_direction,
                slope_control: sweep_slope_control,
            };
            if sweep_pace == 0 || self.sweep.pace == 0 || !self.generation_on {
                self.sweep = sweep;
            } else {
                self.next_sweep = Some(sweep);
            }
        }

        let triggered = nr4_value & 0x80 != 0;

        let duty_cycle = match nr1_value & 0xC0 {
            0x00 => DutyCycle::OneEighth,
            0x40 => DutyCycle::OneFourth,
            0x80 => DutyCycle::OneHalf,
            0xC0 => DutyCycle::ThreeFourths,
            _ => panic!("{nr1_value} & 0xC0 was not 0x00/0x40/0x80/0xC0"),
        };
        self.duty_cycle = duty_cycle;

        if triggered || io_registers.is_register_dirty(nr1) {
            io_registers.clear_dirty_bit(nr1);
            self.length_timer = nr1_value & 0x3F;
        }

        if triggered || !self.generation_on {
            self.volume_control.volume = nr2_value >> 4;
            self.volume_control.sweep_direction = if nr2_value & 0x08 != 0 {
                SweepDirection::Increasing
            } else {
                SweepDirection::Decreasing
            };
            self.volume_control.sweep_pace = nr2_value & 0x07;
        }

        if triggered {
            // Clear trigger flag
            io_registers.apu_write_register(nr4, nr4_value & 0x7F);

            if let Some(next_sweep) = self.next_sweep {
                self.sweep = next_sweep;
                self.next_sweep = None;
            }

            self.divider_ticks = 0;

            self.base_phase_position = self.current_phase_position();
            self.clock_ticks = 0;

            self.generation_on = true;
        }

        self.dac_on = nr2_value & 0xF8 != 0;
        if !self.dac_on {
            self.generation_on = false;
        }

        self.wavelength = ((u16::from(nr4_value) & 0x07) << 8) | u16::from(nr3_value);

        self.length_timer_enabled = nr4_value & 0x40 != 0;
    }

    fn write_wavelength(&self, io_registers: &mut IoRegisters, nr3: IoRegister, nr4: IoRegister) {
        io_registers.apu_write_register(nr3, (self.wavelength & 0xFF) as u8);

        let existing_nr4 = io_registers.apu_read_register(nr4);
        io_registers.apu_write_register(nr4, (existing_nr4 & 0xF8) | (self.wavelength >> 8) as u8);
    }

    fn tick_divider(&mut self) {
        if !self.generation_on {
            return;
        }

        self.divider_ticks += 1;

        // Pulse sweep frequency is 128/pace Hz
        if self.sweep.pace > 0
            && self.wavelength > 0
            && self.divider_ticks % (4 * u64::from(self.sweep.pace)) == 0
        {
            let delta = self.wavelength / 2_u16.pow(self.sweep.slope_control.into());
            let new_wavelength = match self.sweep.direction {
                SweepDirection::Increasing => self.wavelength + delta,
                SweepDirection::Decreasing => self.wavelength.saturating_sub(delta),
            };

            if new_wavelength > 0x07FF {
                // Disable channel when sweep overflows wavelength
                self.generation_on = false;
            } else if self.sweep.slope_control > 0 {
                self.wavelength = new_wavelength;
            }

            if let Some(next_sweep) = self.next_sweep {
                self.sweep = next_sweep;
                self.next_sweep = None;
            }
        }

        // Length timer frequency is 256Hz
        if self.length_timer_enabled && self.divider_ticks % 2 == 0 {
            self.length_timer = self.length_timer.saturating_add(1);
            if self.length_timer >= 64 {
                self.generation_on = false;
            }
        }

        // Envelope frequency is 64/pace Hz
        let envelope_pace = self.volume_control.sweep_pace;
        if envelope_pace > 0 && self.divider_ticks % (8 * u64::from(envelope_pace)) == 0 {
            let new_volume = match self.volume_control.sweep_direction {
                SweepDirection::Increasing => cmp::min(0x0F, self.volume_control.volume + 1),
                SweepDirection::Decreasing => self.volume_control.volume.saturating_sub(1),
            };
            self.volume_control.volume = new_volume;
        }
    }

    fn tick_clock(&mut self) {
        self.clock_ticks += CLOCK_CYCLES_PER_M_CYCLE;
    }

    fn current_phase_position(&self) -> u64 {
        let step_frequency = 1048576.0 / (2048.0 - f64::from(self.wavelength));
        let step_interval = (APU_CLOCK_SPEED as f64) / step_frequency;
        let phase_position = ((self.clock_ticks as f64) / step_interval).round() as u64;

        (phase_position + self.base_phase_position) % 8
    }
}

impl Channel for PulseChannel {
    fn sample_digital(&self) -> Option<u8> {
        if !self.dac_on {
            return None;
        }

        if !self.generation_on {
            return Some(0);
        }

        let wave_step = self.duty_cycle.waveform()[self.current_phase_position() as usize];
        Some(wave_step * self.volume_control.volume)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WaveChannel {
    generation_on: bool,
    dac_on: bool,
    wavelength: u16,
    next_wavelength: Option<u16>,
    length_timer: u8,
    length_timer_enabled: bool,
    volume_shift: u8,
    sample_index: u8,
    last_sample: u8,
    divider_ticks: u64,
    clock_ticks: u64,
}

impl WaveChannel {
    fn new() -> Self {
        Self {
            generation_on: false,
            dac_on: false,
            wavelength: 0,
            next_wavelength: None,
            length_timer: 0,
            length_timer_enabled: false,
            volume_shift: 0,
            sample_index: 1,
            last_sample: 0,
            divider_ticks: 0,
            clock_ticks: 0,
        }
    }

    fn process_register_updates(&mut self, io_registers: &mut IoRegisters) {
        let nr30_value = io_registers.apu_read_register(IoRegister::NR30);
        let nr31_value = io_registers.apu_read_register(IoRegister::NR31);
        let nr32_value = io_registers.apu_read_register(IoRegister::NR32);
        let nr33_value = io_registers.apu_read_register(IoRegister::NR33);
        let nr34_value = io_registers.apu_read_register(IoRegister::NR34);

        self.dac_on = nr30_value & 0x80 != 0;
        if !self.dac_on {
            self.generation_on = false;
        }

        let triggered = nr34_value & 0x80 != 0;

        if io_registers.is_register_dirty(IoRegister::NR31) || triggered {
            io_registers.clear_dirty_bit(IoRegister::NR31);
            self.length_timer = nr31_value & 0x3F;
        }

        self.volume_shift = match nr32_value & 0x60 {
            0x00 => 8,
            0x20 => 0,
            0x40 => 1,
            0x60 => 2,
            _ => panic!("{nr32_value} & 0x60 was not 0x00/0x20/0x40/0x60"),
        };

        let wavelength = (u16::from(nr34_value & 0x07) << 8) | u16::from(nr33_value);
        if wavelength != self.wavelength {
            self.next_wavelength = Some(wavelength);
        }

        self.length_timer_enabled = nr34_value & 0x40 != 0;

        if triggered && self.dac_on {
            io_registers.apu_write_register(IoRegister::NR34, nr34_value & 0x7F);

            self.sample_index = 1;
            self.divider_ticks = 0;
            self.clock_ticks = 0;

            if let Some(next_wavelength) = self.next_wavelength {
                self.wavelength = next_wavelength;
                self.next_wavelength = None;
            }

            self.generation_on = true;
        }
    }

    fn tick_divider(&mut self) {
        if !self.generation_on {
            return;
        }

        self.divider_ticks += 1;

        if self.length_timer_enabled && self.divider_ticks % 2 == 0 {
            self.length_timer = self.length_timer.saturating_add(1);
            if self.length_timer >= 64 {
                self.generation_on = false;
            }
        }
    }

    fn tick_clock(&mut self, io_registers: &IoRegisters) {
        if !self.generation_on {
            return;
        }

        let prev_clock = self.clock_ticks;
        self.clock_ticks += CLOCK_CYCLES_PER_M_CYCLE;

        let step_frequency = 2097152.0 / (2048.0 - f64::from(self.wavelength));
        let step_interval = (APU_CLOCK_SPEED as f64) / step_frequency;
        if (prev_clock as f64 / step_interval).round() as u64
            != (self.clock_ticks as f64 / step_interval).round() as u64
        {
            let samples = io_registers.read_address(0xFF30 + u16::from(self.sample_index / 2));
            let sample = if self.sample_index % 2 == 0 {
                samples >> 4
            } else {
                samples & 0x0F
            };
            self.last_sample = sample;

            self.sample_index = (self.sample_index + 1) % 32;

            if let Some(next_wavelength) = self.next_wavelength {
                self.wavelength = next_wavelength;
                self.next_wavelength = None;
            }
        }
    }
}

impl Channel for WaveChannel {
    fn sample_digital(&self) -> Option<u8> {
        if !self.dac_on {
            return None;
        }

        if !self.generation_on || self.volume_shift == 8 {
            return Some(0);
        }

        Some(self.last_sample >> self.volume_shift)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NoiseChannel {
    generation_on: bool,
    dac_on: bool,
    length_timer: u8,
    length_timer_enabled: bool,
    volume_control: VolumeControl,
    clock_shift: u8,
    lfsr: u16,
    lfsr_width: u8,
    clock_divider: u8,
    divider_ticks: u64,
    clock_ticks: u64,
}

impl NoiseChannel {
    fn new() -> Self {
        Self {
            generation_on: false,
            dac_on: false,
            length_timer: 0,
            length_timer_enabled: false,
            volume_control: VolumeControl::new(),
            clock_shift: 0,
            lfsr: 0,
            lfsr_width: 7,
            clock_divider: 0,
            divider_ticks: 0,
            clock_ticks: 0,
        }
    }

    fn process_register_updates(&mut self, io_registers: &mut IoRegisters) {
        let nr41_value = io_registers.apu_read_register(IoRegister::NR41);
        let nr42_value = io_registers.apu_read_register(IoRegister::NR42);
        let nr43_value = io_registers.apu_read_register(IoRegister::NR43);
        let nr44_value = io_registers.apu_read_register(IoRegister::NR44);

        let triggered = nr44_value & 0x80 != 0;

        if io_registers.is_register_dirty(IoRegister::NR41) || triggered {
            io_registers.clear_dirty_bit(IoRegister::NR41);
            self.length_timer = nr41_value & 0x3F;
        }

        self.dac_on = nr42_value & 0xF8 != 0;
        if !self.dac_on {
            self.generation_on = false;
        }

        if triggered || !self.generation_on {
            let volume = nr42_value >> 4;
            let sweep_direction = if nr42_value & 0x08 != 0 {
                SweepDirection::Increasing
            } else {
                SweepDirection::Decreasing
            };
            let sweep_pace = nr42_value & 0x07;
            self.volume_control = VolumeControl {
                volume,
                sweep_direction,
                sweep_pace,
            };
        }

        self.clock_shift = nr43_value >> 4;
        self.lfsr_width = if nr43_value & 0x80 != 0 { 7 } else { 15 };
        self.clock_divider = nr43_value & 0x07;

        self.length_timer_enabled = nr44_value & 0x40 != 0;

        if triggered && self.dac_on {
            io_registers.apu_write_register(IoRegister::NR44, nr44_value & 0x7F);

            self.divider_ticks = 0;
            self.clock_ticks = 0;
            self.lfsr = 0;
            self.generation_on = true;
        }
    }

    fn tick_divider(&mut self) {
        if !self.generation_on {
            return;
        }

        self.divider_ticks += 1;

        if self.length_timer_enabled && self.divider_ticks % 2 == 0 {
            self.length_timer = self.length_timer.saturating_add(1);
            if self.length_timer >= 64 {
                self.generation_on = false;
            }
        }

        if self.volume_control.sweep_pace > 0
            && self.divider_ticks % (8 * u64::from(self.volume_control.sweep_pace)) == 0
        {
            let new_volume = match self.volume_control.sweep_direction {
                SweepDirection::Increasing => cmp::min(0x0F, self.volume_control.volume + 1),
                SweepDirection::Decreasing => self.volume_control.volume.saturating_sub(1),
            };
            self.volume_control.volume = new_volume;
        }
    }

    fn tick_clock(&mut self) {
        let divisor = if self.clock_divider != 0 {
            f64::from(u32::from(self.clock_divider) << self.clock_shift)
        } else {
            0.5 * 2_f64.powi(self.clock_shift.into())
        };
        let lfsr_frequency = 262144.0 / divisor;
        let lfsr_interval = (APU_CLOCK_SPEED as f64) / lfsr_frequency;

        let prev_clock = self.clock_ticks;
        self.clock_ticks += CLOCK_CYCLES_PER_M_CYCLE;

        if (prev_clock as f64 / lfsr_interval).round() as u64
            != (self.clock_ticks as f64 / lfsr_interval).round() as u64
        {
            let bit_1 = (self.lfsr & 0x02) >> 1;
            let bit_0 = self.lfsr & 0x01;
            let new_bit = !(bit_1 ^ bit_0);

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
            return None;
        }

        if !self.generation_on {
            return Some(0);
        }

        if self.lfsr & 0x0001 != 0 {
            Some(self.volume_control.volume)
        } else {
            Some(0)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApuState {
    enabled: bool,
    last_divider: u8,
    clock_ticks: u64,
    channel_1: PulseChannel,
    channel_2: PulseChannel,
    channel_3: WaveChannel,
    channel_4: NoiseChannel,
    sample_queue: Arc<Mutex<VecDeque<i16>>>,
}

impl ApuState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            last_divider: 0x00,
            clock_ticks: 0,
            channel_1: PulseChannel::new(),
            channel_2: PulseChannel::new(),
            channel_3: WaveChannel::new(),
            channel_4: NoiseChannel::new(),
            sample_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn get_sample_queue(&self) -> Arc<Mutex<VecDeque<i16>>> {
        Arc::clone(&self.sample_queue)
    }

    fn tick_divider(&mut self) {
        self.channel_1.tick_divider();
        self.channel_2.tick_divider();
        self.channel_3.tick_divider();
        self.channel_4.tick_divider();
    }

    fn tick_clock(&mut self, io_registers: &IoRegisters) {
        self.clock_ticks += CLOCK_CYCLES_PER_M_CYCLE;

        self.channel_1.tick_clock();
        self.channel_2.tick_clock();
        self.channel_3.tick_clock(io_registers);
        self.channel_4.tick_clock();
    }

    fn disable(&mut self) {
        self.enabled = false;
        self.channel_1 = PulseChannel::new();
        self.channel_2 = PulseChannel::new();
        self.channel_3 = WaveChannel::new();
        self.channel_4 = NoiseChannel::new();
    }

    fn sample(&self, nr51_value: u8) -> (f64, f64) {
        let mut sample_l = 0.0;
        let mut sample_r = 0.0;

        let ch1_sample = self.channel_1.sample_analog();
        if nr51_value & 0x10 != 0 {
            sample_l += ch1_sample;
        }
        if nr51_value & 0x01 != 0 {
            sample_r += ch1_sample;
        }

        let ch2_sample = self.channel_2.sample_analog();
        if nr51_value & 0x20 != 0 {
            sample_l += ch2_sample;
        }
        if nr51_value & 0x02 != 0 {
            sample_r += ch2_sample;
        }

        let ch3_sample = self.channel_3.sample_analog();
        if nr51_value & 0x40 != 0 {
            sample_l += ch3_sample;
        }
        if nr51_value & 0x04 != 0 {
            sample_r += ch3_sample;
        }

        let ch4_sample = self.channel_4.sample_analog();
        if nr51_value & 0x80 != 0 {
            sample_l += ch4_sample;
        }
        if nr51_value & 0x08 != 0 {
            sample_r += ch4_sample;
        }

        (sample_l, sample_r)
    }
}

pub const OUTPUT_FREQUENCY: u64 = 44100;

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

fn should_sample(apu_state: &ApuState, prev_clock_ticks: u64) -> bool {
    let prev_period = prev_clock_ticks * OUTPUT_FREQUENCY / APU_CLOCK_SPEED;
    let current_period = apu_state.clock_ticks * OUTPUT_FREQUENCY / APU_CLOCK_SPEED;

    prev_period != current_period
}

pub fn tick_m_cycle(apu_state: &mut ApuState, io_registers: &mut IoRegisters) {
    let prev_clock = apu_state.clock_ticks;
    apu_state.tick_clock(io_registers);

    let apu_enabled = io_registers.apu_read_register(IoRegister::NR52) & 0x80 != 0;

    if !apu_enabled {
        if apu_state.enabled {
            // If the APU was just disabled, clear all audio registers and reset all channels
            for audio_register in ALL_AUDIO_REGISTERS {
                io_registers.apu_write_register(audio_register, 0x00);
            }
            apu_state.disable();
        }

        if should_sample(apu_state, prev_clock) {
            let mut sample_queue = apu_state.sample_queue.lock().unwrap();
            sample_queue.push_back(0);
            sample_queue.push_back(0);
        }

        return;
    }
    apu_state.enabled = true;

    let prev_ch1_wavelength = apu_state.channel_1.wavelength;
    let divider = io_registers.read_register(IoRegister::DIV);
    if apu_state.last_divider & 0x10 != 0 && divider & 0x10 == 0 {
        apu_state.tick_divider();
    }
    apu_state.last_divider = divider;

    if prev_ch1_wavelength != apu_state.channel_1.wavelength {
        apu_state
            .channel_1
            .write_wavelength(io_registers, IoRegister::NR13, IoRegister::NR14);
    }

    apu_state.channel_1.process_register_updates(
        io_registers,
        Some(IoRegister::NR10),
        IoRegister::NR11,
        IoRegister::NR12,
        IoRegister::NR13,
        IoRegister::NR14,
    );
    apu_state.channel_2.process_register_updates(
        io_registers,
        None,
        IoRegister::NR21,
        IoRegister::NR22,
        IoRegister::NR23,
        IoRegister::NR24,
    );
    apu_state.channel_3.process_register_updates(io_registers);
    apu_state.channel_4.process_register_updates(io_registers);

    if should_sample(apu_state, prev_clock) {
        let (sample_l, sample_r) =
            apu_state.sample(io_registers.apu_read_register(IoRegister::NR51));

        let nr50_value = io_registers.apu_read_register(IoRegister::NR50);
        let l_volume = ((nr50_value & 0x70) >> 4) + 1;
        let r_volume = (nr50_value & 0x07) + 1;

        // Map [-4, 4] to [-15000, 15000] and apply L/R volume multipliers
        let sample_l = (sample_l / 4.0 * 15000.0 * f64::from(l_volume) / 8.0).round() as i16;
        let sample_r = (sample_r / 4.0 * 15000.0 * f64::from(r_volume) / 8.0).round() as i16;

        let mut sample_queue = apu_state.sample_queue.lock().unwrap();
        sample_queue.push_back(sample_l);
        sample_queue.push_back(sample_r);

        while sample_queue.len() > 8192 {
            sample_queue.pop_front();
        }
    }
}
