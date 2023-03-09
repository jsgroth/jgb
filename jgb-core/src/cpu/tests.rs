// Don't include this test-only module in code coverage
#![cfg(not(tarpaulin_include))]

mod arithmetic;
mod bitshift;
mod load;

use crate::cpu::registers::CpuRegister;
use crate::cpu::{instructions, CpuRegisters};
use crate::memory::{AddressSpace, Cartridge};
use std::collections::HashMap;

struct ExpectedState {
    a: Option<u8>,
    f: Option<u8>,
    b: Option<u8>,
    c: Option<u8>,
    d: Option<u8>,
    e: Option<u8>,
    h: Option<u8>,
    l: Option<u8>,
    sp: Option<u16>,
    memory: HashMap<u16, u8>,
}

macro_rules! compare_bytes {
    // (expected: Option<T>, actual: T) where T: Eq
    ($([$name:literal, $expected:expr, $actual:expr]),+$(,)?) => {
        {
            let mut match_fails = Vec::new();
            $(
                if let Some(expected) = $expected {
                    let actual = $actual;
                    if expected != actual {
                        match_fails.push(format!("{} mismatch: expected 0x{:02x}, actual 0x{:02x}", $name, expected, actual));
                    }
                }
            )*
            match_fails
        }
    };
}

impl ExpectedState {
    fn empty() -> Self {
        Self {
            a: None,
            f: None,
            b: None,
            c: None,
            d: None,
            e: None,
            h: None,
            l: None,
            sp: None,
            memory: HashMap::new(),
        }
    }

    fn assert_matches(&self, cpu_registers: &CpuRegisters, address_space: &AddressSpace) {
        let mut match_fails = compare_bytes!(
            ["A", self.a, cpu_registers.accumulator],
            ["F", self.f, cpu_registers.flags],
            ["B", self.b, cpu_registers.b],
            ["C", self.c, cpu_registers.c],
            ["D", self.d, cpu_registers.d],
            ["E", self.e, cpu_registers.e],
            ["H", self.h, cpu_registers.h],
            ["L", self.l, cpu_registers.l],
            ["SP", self.sp, cpu_registers.sp],
        );

        for (&address, &expected) in &self.memory {
            let actual = address_space.read_address_u8(address);
            if expected != actual {
                match_fails.push(format!("Mismatch at memory address 0x{address:04x}: expected = {expected:02x}, actual = {actual:02x}"));
            }
        }

        if !match_fails.is_empty() {
            let error_msgs: Vec<_> = match_fails.into_iter().map(|s| format!("[{s}]")).collect();
            let error_msg = error_msgs.join(", ");
            panic!("Expected state does not match actual state: {error_msg}");
        }
    }
}

fn run_test(program_hex: &str, expected_state: &ExpectedState) {
    if program_hex.len() % 2 != 0 {
        panic!(
            "program length is {}, must be a multiple of 2",
            program_hex.len()
        );
    }

    if program_hex.chars().any(|c| {
        !('0'..='9').contains(&c) && !('a'..='f').contains(&c) && !('A'..='F').contains(&c)
    }) {
        panic!("program contains non-hexadecimal characters: '{program_hex}'");
    }

    let mut rom = vec![0x00; 0x150];
    // JP 0x0150
    rom[0x100..0x104].copy_from_slice(&[0x00, 0xC3, 0x50, 0x01]);

    for i in (0..program_hex.len()).step_by(2) {
        let byte_str = &program_hex[i..i + 2];
        let byte = u8::from_str_radix(byte_str, 16)
            .expect("program should only contain valid hexadecimal digits");
        rom.push(byte);
    }

    let rom_len = rom.len() as u16;

    let mut address_space =
        AddressSpace::new(Cartridge::new(rom).expect("synthesized test ROM should be valid"));
    let mut cpu_registers = CpuRegisters::new();

    while cpu_registers.pc < rom_len {
        let (instruction, pc) =
            instructions::parse_next_instruction(&address_space, cpu_registers.pc)
                .expect("all instructions in program should be valid");
        cpu_registers.pc = pc;

        instruction
            .execute(&mut address_space, &mut cpu_registers)
            .expect("all instructions in program should successfully execute");
    }

    expected_state.assert_matches(&cpu_registers, &address_space);
}

const ALL_REGISTERS: [CpuRegister; 7] = [
    CpuRegister::A,
    CpuRegister::B,
    CpuRegister::C,
    CpuRegister::D,
    CpuRegister::E,
    CpuRegister::H,
    CpuRegister::L,
];

fn set_in_state(state: &mut ExpectedState, register: CpuRegister, value: u8) {
    let var_ref = match register {
        CpuRegister::A => &mut state.a,
        CpuRegister::B => &mut state.b,
        CpuRegister::C => &mut state.c,
        CpuRegister::D => &mut state.d,
        CpuRegister::E => &mut state.e,
        CpuRegister::H => &mut state.h,
        CpuRegister::L => &mut state.l,
    };

    *var_ref = Some(value);
}

macro_rules! hash_map {
    ($($key:literal: $value:expr),+$(,)?) => {
        {
            let mut map = std::collections::HashMap::new();
            $(
                map.insert($key, $value);
            )*
            map
        }
    }
}

use hash_map;
