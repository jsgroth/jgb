use crate::cpu::InterruptType;
use crate::memory::ioregisters::{IoRegister, IoRegisters};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerCounter(u64);

impl TimerCounter {
    pub fn new() -> Self {
        Self(0)
    }
}

const DIV_UPDATE_FREQUENCY: u64 = 256;

pub fn read_timer_modulo(io_registers: &IoRegisters) -> u8 {
    io_registers.read_register(IoRegister::TMA)
}

pub fn update_timer_registers(
    io_registers: &mut IoRegisters,
    counter: &mut TimerCounter,
    timer_modulo: u8,
    cycles: u64,
) {
    if cycles > DIV_UPDATE_FREQUENCY {
        panic!("cycles must be <= {DIV_UPDATE_FREQUENCY}, was {cycles}");
    }

    let old_cycles = counter.0;
    let new_cycles = old_cycles + cycles;
    counter.0 = new_cycles;

    if old_cycles / DIV_UPDATE_FREQUENCY != new_cycles / DIV_UPDATE_FREQUENCY {
        let old_div = io_registers.read_register(IoRegister::DIV);
        io_registers.write_register(IoRegister::DIV, old_div.wrapping_add(1));
    }

    let timer_control = io_registers.read_register(IoRegister::TAC);
    if timer_control & 0x40 != 0 {
        // TIMA updates are disabled
        return;
    }

    let tima_update_frequency_bits = match timer_control & 0x03 {
        0x00 => 10, // 1024
        0x01 => 4,  // 16
        0x02 => 6,  // 64
        0x03 => 8,  // 256
        _ => panic!("{timer_control} & 0x03 produced a number that was not 0x00/0x01/0x02/0x03"),
    };

    let tima_diff =
        (new_cycles >> tima_update_frequency_bits) - (old_cycles >> tima_update_frequency_bits);
    let tima_diff: u8 = tima_diff.try_into().expect("TIMA diff should always be less than 256 due to this function not accepting large cycle values");

    // This is not the most efficient but generally this loop will only execute 0 or 1 times
    for _ in 0..tima_diff {
        let old_tima = io_registers.read_register(IoRegister::TIMA);
        match old_tima.overflowing_add(1) {
            (new_tima, false) => {
                io_registers.write_register(IoRegister::TIMA, new_tima);
            }
            (_, true) => {
                io_registers.write_register(IoRegister::TIMA, timer_modulo);

                io_registers.interrupt_flags().set(InterruptType::Timer);
            }
        }
    }
}
