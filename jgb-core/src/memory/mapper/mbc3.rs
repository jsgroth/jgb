use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct RtcTime {
    nanos: u32,
    seconds: u8,
    minutes: u8,
    hours: u8,
    days: u16,
    day_overflow_flag: bool,
}

impl RtcTime {
    fn new() -> Self {
        Self { nanos: 0, seconds: 0, minutes: 0, hours: 0, days: 0, day_overflow_flag: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RealTimeClock {
    last_update: SystemTime,
    current_time: RtcTime,
    latched_time: Option<RtcTime>,
    pre_latched: bool,
    halted: bool,
}

impl RealTimeClock {
    pub(crate) fn new(now: SystemTime) -> Self {
        Self {
            last_update: now,
            current_time: RtcTime::new(),
            latched_time: None,
            pre_latched: false,
            halted: false,
        }
    }

    pub(crate) fn update(&mut self, now: SystemTime) {
        let since = now.duration_since(self.last_update).unwrap_or_else(|err| {
            log::error!(
                "Time has gone backwards: last_update={:?}, now={now:?}: {err}",
                self.last_update
            );
            Duration::from_secs(0)
        });

        self.last_update = now;

        if self.halted {
            return;
        }

        let nanos = u128::from(self.current_time.nanos) + since.as_nanos();
        self.current_time.nanos = (nanos % 1_000_000_000) as u32;
        if nanos < 1_000_000_000 {
            return;
        }

        let seconds = u64::from(self.current_time.seconds) + (nanos / 1_000_000_000) as u64;
        self.current_time.seconds = (seconds % 60) as u8;
        if seconds < 60 {
            return;
        }

        let minutes = u64::from(self.current_time.minutes) + (seconds / 60);
        self.current_time.minutes = (minutes % 60) as u8;
        if minutes < 60 {
            return;
        }

        let hours = u64::from(self.current_time.hours) + (minutes / 60);
        self.current_time.hours = (hours % 24) as u8;
        if hours < 24 {
            return;
        }

        let days = u64::from(self.current_time.days) + (hours / 24);
        self.current_time.days = (days % 512) as u16;
        if days < 512 {
            return;
        }

        self.current_time.day_overflow_flag = true;
    }

    pub(crate) fn process_register_write(&mut self, value: u8) {
        if value == 0x01 && self.pre_latched {
            self.pre_latched = false;
            self.latched_time = Some(self.current_time);
        } else if value == 0x00 {
            self.pre_latched = true;
            self.latched_time = None;
        } else {
            self.pre_latched = false;
            self.latched_time = None;
        }
    }

    pub(crate) fn handle_ram_read(&self, ram_bank_number: u8) -> Option<u8> {
        let time = self.latched_time.unwrap_or(self.current_time);

        match ram_bank_number {
            0x08 => Some(time.seconds),
            0x09 => Some(time.minutes),
            0x0A => Some(time.hours),
            0x0B => Some((time.days & 0xFF) as u8),
            0x0C => Some(
                (u8::from(time.day_overflow_flag) << 7)
                    | (u8::from(self.halted) << 6)
                    | (time.days >> 8) as u8,
            ),
            _ => None,
        }
    }

    pub(crate) fn handle_ram_write(&mut self, ram_bank_number: u8, value: u8) {
        match ram_bank_number {
            0x08 => {
                self.current_time.seconds = value;
                self.current_time.nanos = 0;
            }
            0x09 => {
                self.current_time.minutes = value;
            }
            0x0A => {
                self.current_time.hours = value;
            }
            0x0B => {
                self.current_time.days = (self.current_time.days & 0xFF00) | u16::from(value);
            }
            0x0C => {
                self.current_time.days =
                    (self.current_time.days & 0x00FF) | (u16::from(value & 0x01) << 8);
                self.halted = value & 0x40 != 0;
                self.current_time.day_overflow_flag = value & 0x80 != 0;
            }
            _ => {}
        }
    }
}
