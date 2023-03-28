mod timer;

use crate::apu::timer::FrequencyTimer;
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

    fn from_byte(byte: u8) -> Self {
        Self {
            volume: byte >> 4,
            sweep_direction: if byte & 0x08 != 0 {
                SweepDirection::Increasing
            } else {
                SweepDirection::Decreasing
            },
            sweep_pace: byte & 0x07,
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

        (f64::from(digital_sample) - 7.5) / 7.5
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
    frequency_timer: FrequencyTimer,
    just_triggered: bool,
    shadow_frequency: u16,
    sweep: PulseSweep,
    next_sweep: Option<PulseSweep>,
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
            length_timer: 0,
            length_timer_enabled: false,
            volume_control: VolumeControl::new(),
            frequency_timer: FrequencyTimer::new(4),
            just_triggered: false,
            shadow_frequency: 0,
            sweep: PulseSweep::DISABLED,
            next_sweep: None,
            phase_position: 0,
            nr0,
            nr1,
            nr2,
            nr3,
            nr4,
        }
    }

    fn new_channel_1() -> Self {
        Self::new(
            Some(IoRegister::NR10),
            IoRegister::NR11,
            IoRegister::NR12,
            IoRegister::NR13,
            IoRegister::NR14,
        )
    }

    fn new_channel_2() -> Self {
        Self::new(
            None,
            IoRegister::NR21,
            IoRegister::NR22,
            IoRegister::NR23,
            IoRegister::NR24,
        )
    }

    fn process_register_updates(&mut self, io_registers: &mut IoRegisters) {
        let nr1_value = io_registers.apu_read_register(self.nr1);
        let nr2_value = io_registers.apu_read_register(self.nr2);
        let nr4_value = io_registers.apu_read_register(self.nr4);

        // Check if sweep has been updated
        if let Some(nr0) = self.nr0 {
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

        // Check if duty cycle has been updated
        let duty_cycle = match nr1_value & 0xC0 {
            0x00 => DutyCycle::OneEighth,
            0x40 => DutyCycle::OneFourth,
            0x80 => DutyCycle::OneHalf,
            0xC0 => DutyCycle::ThreeFourths,
            _ => panic!("{nr1_value} & 0xC0 was not 0x00/0x40/0x80/0xC0"),
        };
        self.duty_cycle = duty_cycle;

        // Check if length timer has been reset
        if io_registers.is_register_dirty(self.nr1) {
            io_registers.clear_dirty_bit(self.nr1);
            self.length_timer = 64 - (nr1_value & 0x3F);
        }

        self.length_timer_enabled = nr4_value & 0x40 != 0;

        self.frequency_timer
            .set_frequency(read_frequency(io_registers, self.nr3, self.nr4));

        let triggered = nr4_value & 0x80 != 0;
        if triggered {
            // Clear trigger flag
            io_registers.apu_write_register(self.nr4, nr4_value & 0x7F);

            self.just_triggered = true;

            self.volume_control = VolumeControl::from_byte(nr2_value);

            if let Some(next_sweep) = self.next_sweep {
                self.sweep = next_sweep;
                self.next_sweep = None;
            }

            if self.length_timer == 0 {
                self.length_timer = 64;
            }

            self.shadow_frequency = self.frequency_timer.frequency();

            self.frequency_timer.trigger();

            self.generation_on = true;

            if self.sweep.pace > 0 {
                self.process_sweep_iteration(io_registers);
            }
        }

        self.dac_on = nr2_value & 0xF8 != 0;
        if !self.dac_on {
            self.generation_on = false;
        }
    }

    fn tick_divider(&mut self, divider_ticks: u64, io_registers: &mut IoRegisters) {
        // Pulse sweep frequency is 128/pace Hz
        if self.nr0.is_some() {
            self.frequency_timer
                .set_frequency(read_frequency(io_registers, self.nr3, self.nr4));

            if self.sweep.pace > 0
                && self.frequency_timer.frequency() > 0
                && (divider_ticks % (4 * u64::from(self.sweep.pace))) == 2
            {
                self.process_sweep_iteration(io_registers);
            }
        }

        // Length timer frequency is 256Hz
        if self.length_timer_enabled && divider_ticks % 2 == 0 {
            self.length_timer = self.length_timer.saturating_sub(1);
            if self.length_timer == 0 {
                self.generation_on = false;
            }
        }

        // Envelope frequency is 64/pace Hz
        let envelope_pace = self.volume_control.sweep_pace;
        if envelope_pace > 0 && (divider_ticks % (8 * u64::from(envelope_pace))) == 7 {
            let new_volume = match self.volume_control.sweep_direction {
                SweepDirection::Increasing => cmp::min(0x0F, self.volume_control.volume + 1),
                SweepDirection::Decreasing => self.volume_control.volume.saturating_sub(1),
            };
            self.volume_control.volume = new_volume;
        }
    }

    fn tick_clock(&mut self) {
        if self.frequency_timer.tick() {
            self.just_triggered = false;
            self.phase_position = (self.phase_position + 1) % 8;
        }
    }

    fn process_sweep_iteration(&mut self, io_registers: &mut IoRegisters) {
        let frequency = self.shadow_frequency;
        let delta = frequency >> self.sweep.slope_control;
        let new_frequency = match self.sweep.direction {
            SweepDirection::Increasing => frequency + delta,
            SweepDirection::Decreasing => frequency.saturating_sub(delta),
        };

        if new_frequency > 0x07FF {
            // Disable channel when sweep overflows frequency
            self.generation_on = false;
        } else if self.sweep.slope_control > 0 {
            self.shadow_frequency = new_frequency;
        }

        if let Some(next_sweep) = self.next_sweep {
            self.sweep = next_sweep;
            self.next_sweep = None;
        }

        let frequency = self.shadow_frequency;
        self.frequency_timer.set_frequency(frequency);

        io_registers.apu_write_register(self.nr3, (frequency & 0xFF) as u8);
        let nr4 = io_registers.apu_read_register(self.nr4);
        io_registers.apu_write_register(self.nr4, (nr4 & 0xF8) | (frequency >> 8) as u8);
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

        // Pulse channels always emit digital 0 after triggering until the next phase position shift
        if self.just_triggered {
            return Some(0);
        }

        let wave_step = self.duty_cycle.waveform()[self.phase_position as usize];
        Some(wave_step * self.volume_control.volume)
    }
}

fn read_frequency(io_registers: &IoRegisters, nr3: IoRegister, nr4: IoRegister) -> u16 {
    let nr3_value = io_registers.apu_read_register(nr3);
    let nr4_value = io_registers.apu_read_register(nr4);

    ((u16::from(nr4_value) & 0x07) << 8) | u16::from(nr3_value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WaveChannel {
    generation_on: bool,
    dac_on: bool,
    next_frequency: Option<u16>,
    length_timer: u16,
    length_timer_enabled: bool,
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
            next_frequency: None,
            length_timer: 0,
            length_timer_enabled: false,
            volume_shift: 0,
            frequency_timer: FrequencyTimer::new(2),
            sample_index: 1,
            last_sample: 0,
        }
    }

    fn process_register_updates(&mut self, io_registers: &mut IoRegisters) {
        let nr30_value = io_registers.apu_read_register(IoRegister::NR30);
        let nr31_value = io_registers.apu_read_register(IoRegister::NR31);
        let nr32_value = io_registers.apu_read_register(IoRegister::NR32);
        let nr33_value = io_registers.apu_read_register(IoRegister::NR33);
        let nr34_value = io_registers.apu_read_register(IoRegister::NR34);

        if io_registers.is_register_dirty(IoRegister::NR31) {
            io_registers.clear_dirty_bit(IoRegister::NR31);
            self.length_timer = 256 - u16::from(nr31_value);
        }

        self.volume_shift = match nr32_value & 0x60 {
            0x00 => 8,
            0x20 => 0,
            0x40 => 1,
            0x60 => 2,
            _ => panic!("{nr32_value} & 0x60 was not 0x00/0x20/0x40/0x60"),
        };

        let frequency = (u16::from(nr34_value & 0x07) << 8) | u16::from(nr33_value);
        if frequency != self.frequency_timer.frequency() {
            self.next_frequency = Some(frequency);
        }

        self.length_timer_enabled = nr34_value & 0x40 != 0;

        let triggered = nr34_value & 0x80 != 0;
        if triggered {
            io_registers.apu_write_register(IoRegister::NR34, nr34_value & 0x7F);

            self.frequency_timer.trigger();
            self.sample_index = 1;

            if self.length_timer == 0 {
                self.length_timer = 256;
            }

            if let Some(next_frequency) = self.next_frequency {
                self.frequency_timer.set_frequency(next_frequency);
                self.next_frequency = None;
            }

            self.generation_on = true;
        }

        self.dac_on = nr30_value & 0x80 != 0;
        if !self.dac_on {
            self.generation_on = false;
        }
    }

    fn tick_divider(&mut self, divider_ticks: u64) {
        if self.length_timer_enabled && divider_ticks % 2 == 0 {
            self.length_timer = self.length_timer.saturating_sub(1);
            if self.length_timer == 0 {
                self.generation_on = false;
            }
        }
    }

    fn tick_clock(&mut self, io_registers: &IoRegisters) {
        if self.frequency_timer.tick() {
            let samples = io_registers.read_address(0xFF30 + u16::from(self.sample_index / 2));
            let sample = if self.sample_index % 2 == 0 {
                samples >> 4
            } else {
                samples & 0x0F
            };
            self.last_sample = sample;

            self.sample_index = (self.sample_index + 1) % 32;

            if let Some(next_frequency) = self.next_frequency {
                self.frequency_timer.set_frequency(next_frequency);
                self.next_frequency = None;
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
    frequency_timer: u64,
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
            frequency_timer: 0,
        }
    }

    fn process_register_updates(&mut self, io_registers: &mut IoRegisters) {
        let nr41_value = io_registers.apu_read_register(IoRegister::NR41);
        let nr42_value = io_registers.apu_read_register(IoRegister::NR42);
        let nr43_value = io_registers.apu_read_register(IoRegister::NR43);
        let nr44_value = io_registers.apu_read_register(IoRegister::NR44);

        if io_registers.is_register_dirty(IoRegister::NR41) {
            io_registers.clear_dirty_bit(IoRegister::NR41);
            self.length_timer = 64 - (nr41_value & 0x3F);
        }

        self.clock_shift = nr43_value >> 4;
        self.lfsr_width = if nr43_value & 0x80 != 0 { 7 } else { 15 };
        self.clock_divider = nr43_value & 0x07;

        self.length_timer_enabled = nr44_value & 0x40 != 0;

        let triggered = nr44_value & 0x80 != 0;
        if triggered {
            io_registers.apu_write_register(IoRegister::NR44, nr44_value & 0x7F);

            self.frequency_timer = 0;

            self.volume_control = VolumeControl::from_byte(nr42_value);

            if self.length_timer == 0 {
                self.length_timer = 64;
            }

            self.lfsr = 0;

            self.generation_on = true;
        }

        self.dac_on = nr42_value & 0xF8 != 0;
        if !self.dac_on {
            self.generation_on = false;
        }
    }

    fn tick_divider(&mut self, divider_ticks: u64) {
        if self.length_timer_enabled && divider_ticks % 2 == 0 {
            self.length_timer = self.length_timer.saturating_sub(1);
            if self.length_timer == 0 {
                self.generation_on = false;
            }
        }

        if self.volume_control.sweep_pace > 0
            && divider_ticks % (8 * u64::from(self.volume_control.sweep_pace)) == 7
        {
            let new_volume = match self.volume_control.sweep_direction {
                SweepDirection::Increasing => cmp::min(0x0F, self.volume_control.volume + 1),
                SweepDirection::Decreasing => self.volume_control.volume.saturating_sub(1),
            };
            self.volume_control.volume = new_volume;
        }
    }

    fn tick_clock(&mut self) {
        let prev_clock = self.frequency_timer;
        self.frequency_timer += CLOCK_CYCLES_PER_M_CYCLE;

        let divisor = if self.clock_divider != 0 {
            f64::from(u32::from(self.clock_divider) << self.clock_shift)
        } else {
            0.5 * 2_f64.powi(self.clock_shift.into())
        };
        let lfsr_period = (16.0 * divisor).round() as u64;

        if prev_clock / lfsr_period != self.frequency_timer / lfsr_period {
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
// 0.999958.powf(4194304 / 44100)
const HPF_CHARGE_FACTOR: f64 = 0.996013;

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

    pub fn get_sample_queue(&self) -> Arc<Mutex<VecDeque<i16>>> {
        Arc::clone(&self.sample_queue)
    }

    fn process_register_updates(&mut self, io_registers: &mut IoRegisters) {
        if self.enabled {
            self.channel_1.process_register_updates(io_registers);
            self.channel_2.process_register_updates(io_registers);
            self.channel_3.process_register_updates(io_registers);
            self.channel_4.process_register_updates(io_registers);
        }
    }

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

    fn tick_clock(&mut self, io_registers: &IoRegisters) {
        self.clock_ticks += CLOCK_CYCLES_PER_M_CYCLE;

        if self.enabled {
            self.channel_1.tick_clock();
            self.channel_2.tick_clock();
            self.channel_3.tick_clock(io_registers);
            self.channel_4.tick_clock();
        }
    }

    fn disable(&mut self) {
        self.enabled = false;
        self.channel_1 = PulseChannel::new_channel_1();
        self.channel_2 = PulseChannel::new_channel_2();
        self.channel_3 = WaveChannel::new();
        self.channel_4 = NoiseChannel::new();
    }

    fn sample(&mut self, nr50_value: u8, nr51_value: u8) -> (i16, i16) {
        let mut sample_l = 0.0;
        let mut sample_r = 0.0;

        let ch1_sample = self.channel_1.sample_analog();
        let ch1_l = ch1_sample * f64::from(nr51_value & 0x10 != 0);
        let ch1_r = ch1_sample * f64::from(nr51_value & 0x01 != 0);
        sample_l += ch1_l;
        sample_r += ch1_r;

        let ch2_sample = self.channel_2.sample_analog();
        let ch2_l = ch2_sample * f64::from(nr51_value & 0x20 != 0);
        let ch2_r = ch2_sample * f64::from(nr51_value & 0x02 != 0);
        sample_l += ch2_l;
        sample_r += ch2_r;

        let ch3_sample = self.channel_3.sample_analog();
        let ch3_l = ch3_sample * f64::from(nr51_value & 0x40 != 0);
        let ch3_r = ch3_sample * f64::from(nr51_value & 0x04 != 0);
        sample_l += ch3_l;
        sample_r += ch3_r;

        let ch4_sample = self.channel_4.sample_analog();
        let ch4_l = ch4_sample * f64::from(nr51_value & 0x80 != 0);
        let ch4_r = ch4_sample * f64::from(nr51_value & 0x08 != 0);
        sample_l += ch4_l;
        sample_r += ch4_r;

        let l_volume = ((nr50_value & 0x70) >> 4) + 1;
        let r_volume = (nr50_value & 0x07) + 1;

        // Map [-4, 4] to [-1, 1] before applying HPF
        let mut sample_l = sample_l / 4.0;
        let mut sample_r = sample_r / 4.0;

        if self.channel_1.dac_on
            || self.channel_2.dac_on
            || self.channel_3.dac_on
            || self.channel_4.dac_on
        {
            sample_l = high_pass_filter(sample_l, &mut self.hpf_capacitor_l);
            sample_r = high_pass_filter(sample_r, &mut self.hpf_capacitor_r);
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
            })
        }

        (sample_l, sample_r)
    }
}

fn high_pass_filter(sample: f64, capacitor: &mut f64) -> f64 {
    let filtered_sample = sample - *capacitor;

    *capacitor = sample - HPF_CHARGE_FACTOR * filtered_sample;

    filtered_sample
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

        if should_sample(apu_state, prev_clock) {
            let mut sample_queue = apu_state.sample_queue.lock().unwrap();
            sample_queue.push_back(0);
            sample_queue.push_back(0);
        }

        return;
    }
    apu_state.enabled = true;

    let divider = io_registers.read_register(IoRegister::DIV);
    if apu_state.last_divider & 0x10 != 0 && divider & 0x10 == 0 {
        apu_state.tick_divider(io_registers);
    }
    apu_state.last_divider = divider;

    apu_state.process_register_updates(io_registers);

    let new_nr52_value = (nr52_value & 0x80)
        | (u8::from(apu_state.channel_4.generation_on) << 3)
        | (u8::from(apu_state.channel_3.generation_on) << 2)
        | (u8::from(apu_state.channel_2.generation_on) << 1)
        | u8::from(apu_state.channel_1.generation_on);
    io_registers.apu_write_register(IoRegister::NR52, new_nr52_value);

    if should_sample(apu_state, prev_clock) {
        let (sample_l, sample_r) = apu_state.sample(
            io_registers.apu_read_register(IoRegister::NR50),
            io_registers.apu_read_register(IoRegister::NR51),
        );

        let mut sample_queue = apu_state.sample_queue.lock().unwrap();
        sample_queue.push_back(sample_l);
        sample_queue.push_back(sample_r);

        while sample_queue.len() > 8192 {
            sample_queue.pop_front();
        }
    }
}
