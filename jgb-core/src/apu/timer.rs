use crate::apu;

// A timer with a clock period of {period_multiplier} * (2048 - {frequency})
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrequencyTimer {
    frequency: u16,
    timer: u16,
    period_multiplier: u16,
}

impl FrequencyTimer {
    pub(crate) fn new(period_multiplier: u16) -> Self {
        Self {
            frequency: 0,
            timer: period_multiplier * 2048,
            period_multiplier,
        }
    }

    // Reset the timer based on the current frequency.
    fn reset_timer(&mut self) {
        self.timer = self.period_multiplier * (2048 - self.frequency);
    }

    // Tick the timer for 1 M-cycle (4 APU clock cycles). Returns whether the timer clocked.
    pub(crate) fn tick_m_cycle(&mut self) -> bool {
        let mut reset = false;
        for _ in 0..apu::CLOCK_CYCLES_PER_M_CYCLE {
            reset |= self.tick();
        }
        reset
    }

    // Tick the timer for 1 APU clock cycle. Returns whether the timer clocked.
    fn tick(&mut self) -> bool {
        self.timer -= 1;
        if self.timer == 0 {
            self.reset_timer();
            true
        } else {
            false
        }
    }

    // Re-initialize the timer.
    pub(crate) fn trigger(&mut self) {
        self.reset_timer();
    }

    // Get the timer's current frequency.
    pub(crate) fn frequency(&self) -> u16 {
        self.frequency
    }

    // Update the timer's frequency. The update will take effect after the next timer clock.
    pub(crate) fn set_frequency(&mut self, frequency: u16) {
        self.frequency = frequency;
    }
}
