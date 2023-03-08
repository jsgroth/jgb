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
    // expected: Option<T>, actual: T where T: Eq
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

macro_rules! hash_map {
    ($($key:literal: $value:expr),+$(,)?) => {
        {
            let mut map = HashMap::new();
            $(
                map.insert($key, $value);
            )*
            map
        }
    }
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

#[test]
fn load_register_immediate() {
    run_test(
        // LD A, 0x45
        "3E45",
        &ExpectedState {
            a: Some(0x45),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD B, 0x45
        "0645",
        &ExpectedState {
            b: Some(0x45),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD C, 0x45
        "0E45",
        &ExpectedState {
            c: Some(0x45),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD D, 0x45
        "1645",
        &ExpectedState {
            d: Some(0x45),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD E, 0x45
        "1E45",
        &ExpectedState {
            e: Some(0x45),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD H, 0x45
        "2645",
        &ExpectedState {
            h: Some(0x45),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD L, 0x45
        "2E45",
        &ExpectedState {
            l: Some(0x45),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_register_register() {
    for r1 in ALL_REGISTERS {
        let ldri = 0x06 | (r1.to_opcode_bits() << 3);
        // LD <R>, 0x45
        let ldri = format!("{ldri:02x}45");

        for r2 in ALL_REGISTERS {
            let opcode = 0x40 | (r2.to_opcode_bits() << 3) | r1.to_opcode_bits();

            // LD <R2>, <R1>
            let ldrr = format!("{opcode:02x}");
            let program_hex = format!("{ldri}{ldrr}");

            let mut expected_state = ExpectedState::empty();
            set_in_state(&mut expected_state, r2, 0x45);

            run_test(&program_hex, &expected_state);
        }
    }
}

#[test]
fn load_register_immediate_16() {
    run_test(
        // LD BC, 0x2468
        "016824",
        &ExpectedState {
            b: Some(0x24),
            c: Some(0x68),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD DE, 0x2468
        "116824",
        &ExpectedState {
            d: Some(0x24),
            e: Some(0x68),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0x2468
        "216824",
        &ExpectedState {
            h: Some(0x24),
            l: Some(0x68),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD SP, 0x2468
        "316824",
        &ExpectedState {
            sp: Some(0x2468),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_register_indirect_hl() {
    for r in ALL_REGISTERS {
        let opcode = 0x46 | (r.to_opcode_bits() << 3);
        let opcode_hex = format!("{opcode:02x}");

        // LD HL, 0x0157; LD <R>, (HL); JP 0xFFFF; <data>
        let program_hex = format!("215701{opcode_hex}C3FFFF47");
        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x47);

        run_test(&program_hex, &expected_state);
    }
}

#[test]
fn load_indirect_hl_immediate() {
    run_test(
        // LD HL, 0xC105; LD (HL), 0x83
        "2105C13683",
        &ExpectedState {
            memory: hash_map! { 0xC105: 0x83 },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_indirect_hl_register() {
    for r in ALL_REGISTERS {
        let preload_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let preload_opcode_hex = format!("{preload_opcode:02x}");

        let opcode = 0x70 | r.to_opcode_bits();
        let opcode_hex = format!("{opcode:02x}");

        // LD <R>, E3; LD HL, 0xD075; LD (HL), <R>
        let program_hex = format!("{preload_opcode_hex}E32175D0{opcode_hex}");
        let expected_value = match r {
            CpuRegister::H => 0xD0,
            CpuRegister::L => 0x75,
            _ => 0xE3,
        };

        run_test(
            &program_hex,
            &ExpectedState {
                memory: hash_map! { 0xD075: expected_value },
                ..ExpectedState::empty()
            },
        );
    }
}
