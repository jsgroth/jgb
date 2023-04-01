mod channels;

use crate::apu::channels::{Channel, NoiseChannel, PulseChannel, WaveChannel};
use crate::memory::ioregisters::{IoRegister, IoRegisters};
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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

#[derive(Debug, Clone)]
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
        self.divider_ticks = 0;

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
        log::trace!("ch1: {:?}", self.channel_1);
        let ch1_sample = self.channel_1.sample_analog();
        let ch1_l = ch1_sample * f64::from(nr51_value & 0x10 != 0);
        let ch1_r = ch1_sample * f64::from(nr51_value & 0x01 != 0);
        sample_l += ch1_l;
        sample_r += ch1_r;

        // Sample channel 2
        log::trace!("ch2: {:?}", self.channel_2);
        let ch2_sample = self.channel_2.sample_analog();
        let ch2_l = ch2_sample * f64::from(nr51_value & 0x20 != 0);
        let ch2_r = ch2_sample * f64::from(nr51_value & 0x02 != 0);
        sample_l += ch2_l;
        sample_r += ch2_r;

        // Sample channel 3
        log::trace!("ch3: {:?}", self.channel_3);
        let ch3_sample = self.channel_3.sample_analog();
        let ch3_l = ch3_sample * f64::from(nr51_value & 0x40 != 0);
        let ch3_r = ch3_sample * f64::from(nr51_value & 0x04 != 0);
        sample_l += ch3_l;
        sample_r += ch3_r;

        // Sample channel 4
        log::trace!("ch4: {:?}", self.channel_4);
        let ch4_sample = self.channel_4.sample_analog();
        let ch4_l = ch4_sample * f64::from(nr51_value & 0x80 != 0);
        let ch4_r = ch4_sample * f64::from(nr51_value & 0x08 != 0);
        sample_l += ch4_l;
        sample_r += ch4_r;

        // Master volume multipliers range from [1, 8]
        let l_volume = ((nr50_value & 0x70) >> 4) + 1;
        let r_volume = (nr50_value & 0x07) + 1;

        // Map [-4, 4] to [-1, 1] before applying high-pass filter
        let mut sample_l = sample_l / 4.0;
        let mut sample_r = sample_r / 4.0;

        // Apply high-pass filter if any of the four DACs are on
        if self.channel_1.dac_enabled()
            || self.channel_2.dac_enabled()
            || self.channel_3.dac_enabled()
            || self.channel_4.dac_enabled()
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

// Progress the APU by 1 M-cycle (4 APU clock cycles). Audio samples will be written to the APU
// state's sample queue if appropriate.
pub fn tick_m_cycle(apu_state: &mut ApuState, io_registers: &mut IoRegisters, audio_60hz: bool) {
    let nr52_value = io_registers.apu_read_register(IoRegister::NR52);
    let apu_enabled = nr52_value & 0x80 != 0;

    // Tick M-cycle / APU clock cycle timers
    let prev_clock = apu_state.clock_ticks;
    apu_state.tick_clock(io_registers);

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
        | (u8::from(apu_state.channel_4.channel_enabled()) << 3)
        | (u8::from(apu_state.channel_3.channel_enabled()) << 2)
        | (u8::from(apu_state.channel_2.channel_enabled()) << 1)
        | u8::from(apu_state.channel_1.channel_enabled());
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

// High-pass filter capacitor charge factor
static HPF_CHARGE_FACTOR: Lazy<f64> =
    Lazy::new(|| 0.999958_f64.powf((4 * 1024 * 1024) as f64 / OUTPUT_FREQUENCY as f64));
static HPF_CHARGE_FACTOR_60HZ: Lazy<f64> = Lazy::new(|| {
    0.999958_f64.powf((4 * 1024 * 1024) as f64 / OUTPUT_FREQUENCY as f64 * 60.0 / 59.7)
});

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

static SAMPLE_RATE: Lazy<f64> = Lazy::new(|| OUTPUT_FREQUENCY as f64 / APU_CLOCK_SPEED as f64);
static SAMPLE_RATE_60HZ: Lazy<f64> =
    Lazy::new(|| OUTPUT_FREQUENCY as f64 / APU_CLOCK_SPEED as f64 * 59.7 / 60.0);

// Return whether the APU emulator should output audio samples during the current M-cycle tick.
// This is currently just a naive "output every 4.194304 MHz / <output_frequency> clock cycles"
fn should_sample(apu_state: &ApuState, prev_clock_ticks: u64, audio_60hz: bool) -> bool {
    let sample_rate = if audio_60hz {
        *SAMPLE_RATE_60HZ
    } else {
        *SAMPLE_RATE
    };

    let prev_period = (prev_clock_ticks as f64 * sample_rate).round() as u64;
    let current_period = (apu_state.clock_ticks as f64 * sample_rate).round() as u64;

    prev_period != current_period
}
