use crate::apu;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrequencyTimer {
    frequency: u16,
    timer: u64,
    period_multiplier: u16,
}

impl FrequencyTimer {
    pub(crate) fn new(period_multiplier: u16) -> Self {
        Self {
            frequency: 0,
            timer: 0,
            period_multiplier,
        }
    }

    fn reset_timer(&mut self) {
        self.timer = (self.period_multiplier * (2048 - self.frequency)).into();
    }

    pub(crate) fn tick_m_cycle(&mut self) -> bool {
        let mut reset = false;
        for _ in 0..apu::CLOCK_CYCLES_PER_M_CYCLE {
            reset |= self.tick();
        }
        reset
    }

    fn tick(&mut self) -> bool {
        if self.timer == 0 {
            self.reset_timer();
            true
        } else {
            self.timer -= 1;
            false
        }
    }

    pub(crate) fn trigger(&mut self) {
        self.reset_timer();
    }

    pub(crate) fn frequency(&self) -> u16 {
        self.frequency
    }

    pub(crate) fn set_frequency(&mut self, frequency: u16) {
        self.frequency = frequency;
    }
}
