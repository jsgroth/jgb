use crate::apu;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrequencyTimer {
    frequency: u16,
    clock_ticks: u64,
    period_multiplier: u16,
}

impl FrequencyTimer {
    pub(crate) fn new(period_multiplier: u16) -> Self {
        Self {
            frequency: 0,
            clock_ticks: 0,
            period_multiplier,
        }
    }

    pub(crate) fn tick(&mut self) -> bool {
        let prev_clock = self.clock_ticks;
        self.clock_ticks += apu::CLOCK_CYCLES_PER_M_CYCLE;

        let period = u64::from(self.period_multiplier * (2048 - self.frequency));
        prev_clock / period != self.clock_ticks / period
    }

    pub(crate) fn trigger(&mut self) {
        self.clock_ticks = 0;
    }

    pub(crate) fn frequency(&self) -> u16 {
        self.frequency
    }

    pub(crate) fn set_frequency(&mut self, frequency: u16) {
        self.frequency = frequency;
    }
}
