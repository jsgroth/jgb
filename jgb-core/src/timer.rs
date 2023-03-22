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
        io_registers.privileged_set_div(old_div.wrapping_add(1));
    }

    let timer_control = io_registers.read_register(IoRegister::TAC);
    if timer_control & 0x04 == 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_timer_modulo_fn() {
        let mut io_registers = IoRegisters::new();

        io_registers.write_register(IoRegister::TMA, 0x3D);
        assert_eq!(0x3D, read_timer_modulo(&io_registers));
    }

    #[test]
    fn divider_register() {
        let mut io_registers = IoRegisters::new();
        let mut timer_counter = TimerCounter::new();

        // DIV should ignore the timer enabled bit
        io_registers.write_register(IoRegister::TAC, 0x00);

        // All DIV writes should reset the counter regardless of value
        io_registers.write_register(IoRegister::DIV, 0x46);
        assert_eq!(0x00, io_registers.read_register(IoRegister::DIV));

        update_timer_registers(&mut io_registers, &mut timer_counter, 0, 20);
        assert_eq!(0x00, io_registers.read_register(IoRegister::DIV));
        assert_eq!(20, timer_counter.0);

        update_timer_registers(&mut io_registers, &mut timer_counter, 0, 40);
        assert_eq!(0x00, io_registers.read_register(IoRegister::DIV));
        assert_eq!(60, timer_counter.0);

        update_timer_registers(&mut io_registers, &mut timer_counter, 0, 195);
        assert_eq!(0x00, io_registers.read_register(IoRegister::DIV));
        assert_eq!(255, timer_counter.0);

        update_timer_registers(&mut io_registers, &mut timer_counter, 0, 1);
        assert_eq!(0x01, io_registers.read_register(IoRegister::DIV));
        assert_eq!(256, timer_counter.0);

        for _ in 0..254 {
            update_timer_registers(&mut io_registers, &mut timer_counter, 0, 256);
        }

        assert_eq!(0xFF, io_registers.read_register(IoRegister::DIV));
        assert_eq!(256 * 255, timer_counter.0);

        update_timer_registers(&mut io_registers, &mut timer_counter, 0, 256);
        assert_eq!(0x00, io_registers.read_register(IoRegister::DIV));
        assert_eq!(256 * 256, timer_counter.0);
    }

    #[test]
    fn tima_register() {
        let mut io_registers = IoRegisters::new();
        let mut timer_counter = TimerCounter::new();

        let timer_modulo = 0x78;

        io_registers.interrupt_flags().clear(InterruptType::Timer);

        // Timer enabled, TIMA update frequency 16
        io_registers.write_register(IoRegister::TAC, 0x05);

        io_registers.write_register(IoRegister::TIMA, 0xE0);

        update_timer_registers(&mut io_registers, &mut timer_counter, timer_modulo, 15);
        assert_eq!(0xE0, io_registers.read_register(IoRegister::TIMA));

        update_timer_registers(&mut io_registers, &mut timer_counter, timer_modulo, 1);
        assert_eq!(0xE1, io_registers.read_register(IoRegister::TIMA));

        update_timer_registers(&mut io_registers, &mut timer_counter, timer_modulo, 40);
        assert_eq!(0xE3, io_registers.read_register(IoRegister::TIMA));
        assert_eq!(56, timer_counter.0);

        update_timer_registers(&mut io_registers, &mut timer_counter, timer_modulo, 40);
        assert_eq!(0xE6, io_registers.read_register(IoRegister::TIMA));
        assert_eq!(96, timer_counter.0);

        for _ in 0..(0xFF - 0xE6) {
            update_timer_registers(&mut io_registers, &mut timer_counter, timer_modulo, 16);
        }

        assert_eq!(0xFF, io_registers.read_register(IoRegister::TIMA));
        assert!(!io_registers.interrupt_flags().get(InterruptType::Timer));

        update_timer_registers(&mut io_registers, &mut timer_counter, timer_modulo, 16);
        assert_eq!(0x78, io_registers.read_register(IoRegister::TIMA));

        // Change update frequency to 64
        io_registers.write_register(IoRegister::TAC, 0x06);

        update_timer_registers(&mut io_registers, &mut timer_counter, timer_modulo, 32);
        assert_eq!(0x78, io_registers.read_register(IoRegister::TIMA));
        assert!(io_registers.interrupt_flags().get(InterruptType::Timer));

        update_timer_registers(&mut io_registers, &mut timer_counter, timer_modulo, 40);
        assert_eq!(0x79, io_registers.read_register(IoRegister::TIMA));

        // Disable timer
        io_registers.write_register(IoRegister::TAC, 0x02);

        update_timer_registers(&mut io_registers, &mut timer_counter, timer_modulo, 256);
        assert_eq!(0x79, io_registers.read_register(IoRegister::TIMA));
    }

    #[test]
    #[should_panic(expected = "cycles must be <= 256")]
    fn cycle_limit() {
        let mut io_registers = IoRegisters::new();
        let mut timer_counter = TimerCounter::new();

        update_timer_registers(&mut io_registers, &mut timer_counter, 0, 257);
    }
}
