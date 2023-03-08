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

        // LD <R>, 0xE3; LD HL, 0xD075; LD (HL), <R>
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

#[test]
fn load_accumulator_indirect_bc() {
    run_test(
        // LD HL, 0xC555; LD (HL), 0xC4; LD BC, 0xC555; LD A, (BC)
        "2155C536C40155C50A",
        &ExpectedState {
            a: Some(0xC4),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_accumulator_indirect_de() {
    run_test(
        // LD HL, 0xC555; LD (HL), 0x2F; LD DE, 0xC555; LD A, (DE)
        "2155C5362F1155C51A",
        &ExpectedState {
            a: Some(0x2F),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_accumulator_direct_16() {
    run_test(
        // LD HL, 0xD943; LD (HL), 0x1B; LD A, (0xD943)
        "2143D9361BFA43D9",
        &ExpectedState {
            a: Some(0x1B),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_indirect_bc_accumulator() {
    run_test(
        // LD A, 0xFA; LD BC, 0xC560; LD (BC), A
        "3EFA0160C502",
        &ExpectedState {
            memory: hash_map! { 0xC560: 0xFA },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_indirect_de_accumulator() {
    run_test(
        // LD A, 0x65; LD DE, 0xC010; LD (DE), A
        "3E651110C012",
        &ExpectedState {
            memory: hash_map! { 0xC010: 0x65 },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_direct_16_accumulator() {
    run_test(
        // LD A, 0x90; LD (0xD40E), A
        "3E90EA0ED4",
        &ExpectedState {
            memory: hash_map! { 0xD40E: 0x90 },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn ldh_accumulator_immediate() {
    run_test(
        // LD HL, 0xFF40; LD (HL), 0xC8; LDH A, (0x40)
        "2140FF36C8F040",
        &ExpectedState {
            a: Some(0xC8),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn ldh_accumulator_c() {
    // LD HL, 0xFF40; LD (HL), 0xEE; LD C, 0x40; LDH A, (C)
    run_test(
        "2140FF36EE0E40F2",
        &ExpectedState {
            a: Some(0xEE),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn ldh_immediate_accumulator() {
    run_test(
        // LD A, 0x72; LDH (0x40), A
        "3E72E040",
        &ExpectedState {
            memory: hash_map! { 0xFF40: 0x72 },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn ldh_c_accumulator() {
    run_test(
        // LD A, 0xCB; LD C, 0x40; LDH (C), A
        "3ECB0E40E2",
        &ExpectedState {
            memory: hash_map! { 0xFF40: 0xCB },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_accumulator_indirect_hl_inc() {
    run_test(
        // LD HL, 0xDC60; LD (HL), 0xD5; LD A, (HL+)
        "2160DC36D52A",
        &ExpectedState {
            a: Some(0xD5),
            h: Some(0xDC),
            l: Some(0x61),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xCFFF; LD (HL), 0x93; LD A, (HL+)
        "21FFCF36932A",
        &ExpectedState {
            a: Some(0x93),
            h: Some(0xD0),
            l: Some(0x00),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_accumulator_indirect_hl_dec() {
    run_test(
        // LD HL, 0xD49A; LD (HL), 0x92; LD A, (HL-)
        "219AD436923A",
        &ExpectedState {
            a: Some(0x92),
            h: Some(0xD4),
            l: Some(0x99),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xD000; LD (HL), 0xF9; LD A, (HL-)
        "2100D036F93A",
        &ExpectedState {
            a: Some(0xF9),
            h: Some(0xCF),
            l: Some(0xFF),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_indirect_hl_inc_accumulator() {
    run_test(
        // LD A, 0x04; LD HL, 0xD55F; LD (HL+), A
        "3E04215FD522",
        &ExpectedState {
            h: Some(0xD5),
            l: Some(0x60),
            memory: hash_map! { 0xD55F: 0x04 },
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x0D; LD HL, 0xCFFF; LD (HL+), A
        "3E0D21FFCF22",
        &ExpectedState {
            h: Some(0xD0),
            l: Some(0x00),
            memory: hash_map! { 0xCFFF: 0x0D },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_indirect_hl_dec_accumulator() {
    run_test(
        // LD A, 0x19; LD HL, 0xD37F; LD (HL-), A
        "3E19217FD332",
        &ExpectedState {
            h: Some(0xD3),
            l: Some(0x7E),
            memory: hash_map! { 0xD37F: 0x19 },
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x4A; LD HL, 0xD000; LD (HL-), A
        "3E4A2100D032",
        &ExpectedState {
            h: Some(0xCF),
            l: Some(0xFF),
            memory: hash_map! { 0xD000: 0x4A },
            ..ExpectedState::empty()
        },
    );
}
