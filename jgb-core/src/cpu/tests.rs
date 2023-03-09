use crate::cpu::registers::{CpuRegister, CpuRegisterPair};
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

#[test]
fn load_sp_hl() {
    run_test(
        // LD HL, 0xFFBB; LD SP, HL
        "21BBFFF9",
        &ExpectedState {
            sp: Some(0xFFBB),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn load_direct_sp() {
    run_test(
        // LD HL, 0xFFEB; LD SP, HL; LD (0xD58F), SP
        "21EBFFF9088FD5",
        &ExpectedState {
            memory: hash_map! {
                0xD58F: 0xEB,
                0xD590: 0xFF,
            },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn push_stack() {
    // BC, DE, HL
    for rr_bits in [0x00, 0x10, 0x20] {
        let preload_opcode = 0x01 | rr_bits;
        // LD <rr>, 0x912C
        let preload_hex = format!("{preload_opcode:02x}2C91");

        let opcode = 0xC5 | rr_bits;
        let opcode_hex = format!("{opcode:02x}");

        run_test(
            // LD <rr>, 0x912C; PUSH <rr>; PUSH <rr>
            &format!("{preload_hex}{opcode_hex}{opcode_hex}"),
            &ExpectedState {
                sp: Some(0xFFFA),
                memory: hash_map! {
                    0xFFFA: 0x2C,
                    0xFFFB: 0x91,
                    0xFFFC: 0x2C,
                    0xFFFD: 0x91,
                },
                ..ExpectedState::empty()
            },
        );
    }

    // AF
    run_test(
        // LD A, 0x91; SCF; PUSH AF; PUSH AF
        "3E9137F5F5",
        &ExpectedState {
            sp: Some(0xFFFA),
            memory: hash_map! {
                0xFFFA: 0x10,
                0xFFFB: 0x91,
                0xFFFC: 0x10,
                0xFFFD: 0x91,
            },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn pop_stack() {
    for (rr, rr_bits) in [
        (CpuRegisterPair::BC, 0x00),
        (CpuRegisterPair::DE, 0x10),
        (CpuRegisterPair::HL, 0x20),
        (CpuRegisterPair::AF, 0x30),
    ] {
        let opcode = 0xC1 | rr_bits;
        let opcode_hex = format!("{opcode:02x}");

        let mut expected_state = ExpectedState::empty();
        let (high_ref, low_ref) = match rr {
            CpuRegisterPair::BC => (&mut expected_state.b, &mut expected_state.c),
            CpuRegisterPair::DE => (&mut expected_state.d, &mut expected_state.e),
            CpuRegisterPair::HL => (&mut expected_state.h, &mut expected_state.l),
            CpuRegisterPair::AF => (&mut expected_state.a, &mut expected_state.f),
            _ => panic!("unexpected register pair: {rr:?}"),
        };
        *high_ref = Some(0x6B);
        *low_ref = Some(0x57);
        expected_state.sp = Some(0xFFFC);

        run_test(
            // LD A, 0x57
            // LDH (0xFA), A
            // LD A, 0x6B
            // LDH (0xFB), A
            // LD SP, 0xFFFA
            // POP <rr>
            &format!("3E57E0FA3E6BE0FB31FAFF{opcode_hex}"),
            &expected_state,
        );
    }
}

#[test]
fn add_immediate() {
    run_test(
        // LD A, 0x05; ADD 0xDE
        "3E05C6DE",
        &ExpectedState {
            a: Some(0xE3),
            f: Some(0x20),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x01; ADD 0x03
        "3E01C603",
        &ExpectedState {
            a: Some(0x04),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x55; ADD 0xAB
        "3E55C6AB",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0xB0),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xFF; ADD 0x12
        "3EFFC612",
        &ExpectedState {
            a: Some(0x11),
            f: Some(0x30),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xFF; SCF; ADD 0x12
        "3EFF37C612",
        &ExpectedState {
            a: Some(0x11),
            f: Some(0x30),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn add_indirect_hl() {
    run_test(
        // LD HL, 0xCDA4; LD (HL), 0x3B; LD A, 0xA1; ADD (HL)
        "21A4CD363B3EA186",
        &ExpectedState {
            a: Some(0xDC),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn add_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let add_opcode = 0x80 | r.to_opcode_bits();
        let add_opcode_hex = format!("{add_opcode:02x}");

        let (expected_a, expected_f) = match r {
            CpuRegister::A => (0x68, 0x10),
            _ => (0xEA, 0x00),
        };

        run_test(
            // LD A, 0x36; LD <r>, 0xB4; ADD <r>
            &format!("3E36{load_opcode_hex}B4{add_opcode_hex}"),
            &ExpectedState {
                a: Some(expected_a),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );
    }
}

#[test]
fn adc_immediate() {
    run_test(
        // LD A, 0xBC; ADC 0x15
        "3EBCCE15",
        &ExpectedState {
            a: Some(0xD1),
            f: Some(0x20),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xBC; SCF; ADC 0x15
        "3EBC37CE15",
        &ExpectedState {
            a: Some(0xD2),
            f: Some(0x20),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xFD; SCF; ADC 0x02
        "3EFD37CE02",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0xB0),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn adc_indirect_hl() {
    run_test(
        // LD HL, 0xC612; LD (HL), 0xFD; LD A, 0x02; SCF; ADC (HL)
        "2112C636FD3E02378E",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0xB0),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn adc_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let adc_opcode = 0x88 | r.to_opcode_bits();
        let adc_opcode_hex = format!("{adc_opcode:02x}");

        let (expected_a, expected_f) = match r {
            CpuRegister::A => (0xA3, 0x10),
            _ => (0x19, 0x10),
        };

        run_test(
            // LD <r>, 0x47; LD A, 0xD1; SCF; ADC <r>
            &format!("{load_opcode_hex}473ED137{adc_opcode_hex}"),
            &ExpectedState {
                a: Some(expected_a),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );
    }
}

#[test]
fn sub_immediate() {
    run_test(
        // LD A, 0xF5; SUB 0x13
        "3EF5D613",
        &ExpectedState {
            a: Some(0xE2),
            f: Some(0x40),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xF5; SCF; SUB 0x13
        "3EF537D613",
        &ExpectedState {
            a: Some(0xE2),
            f: Some(0x40),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xCC; SUB 0xCC
        "3ECCD6CC",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0xC0),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x12; SUB 0x51
        "3E12D651",
        &ExpectedState {
            a: Some(0xC1),
            f: Some(0x50),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xF3; SUB 0x0A
        "3EF3D60A",
        &ExpectedState {
            a: Some(0xE9),
            f: Some(0x60),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x00; SUB 0xFF
        "3E00D6FF",
        &ExpectedState {
            a: Some(0x01),
            f: Some(0x70),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn sub_indirect_hl() {
    run_test(
        // LD HL, 0xD0BC; LD (HL), 0xDD; LD A, 0x88; SUB (HL)
        "21BCD036DD3E8896",
        &ExpectedState {
            a: Some(0xAB),
            f: Some(0x70),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn sub_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let sub_opcode = 0x90 | r.to_opcode_bits();
        let sub_opcode_hex = format!("{sub_opcode:02x}");

        let (expected_a, expected_f) = match r {
            CpuRegister::A => (0x00, 0xC0),
            _ => (0x2F, 0x60),
        };

        run_test(
            // LD <r>, 0xAF; LD A, 0xDE; SUB <r>
            &format!("{load_opcode_hex}AF3EDE{sub_opcode_hex}"),
            &ExpectedState {
                a: Some(expected_a),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );
    }
}

#[test]
fn sbc_immediate() {
    run_test(
        // LD A, 0x5E; SBC 0x23
        "3E5EDE23",
        &ExpectedState {
            a: Some(0x3B),
            f: Some(0x40),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x5E; SCF; SBC 0x23
        "3E5E37DE23",
        &ExpectedState {
            a: Some(0x3A),
            f: Some(0x40),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x01; SCF; SBC 0x01
        "3E0137DE01",
        &ExpectedState {
            a: Some(0xFF),
            f: Some(0x70),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x01; SCF; SBC 0x00
        "3E0137DE00",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0xC0),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn sbc_indirect_hl() {
    run_test(
        // LD HL, 0xD9DC; LD (HL), 0xFC; LD A, 0xF3; SCF; SBC (HL)
        "21DCD936FC3EF3379E",
        &ExpectedState {
            a: Some(0xF6),
            f: Some(0x70),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn sbc_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let sbc_opcode = 0x98 | r.to_opcode_bits();
        let sbc_opcode_hex = format!("{sbc_opcode:02x}");

        let (expected_a, expected_f) = match r {
            CpuRegister::A => (0xFF, 0x70),
            _ => (0xF6, 0x70),
        };

        run_test(
            // LD <r>, 0xFC; LD A, 0xF3; SCF; SBC <r>
            &format!("{load_opcode_hex}FC3EF337{sbc_opcode_hex}"),
            &ExpectedState {
                a: Some(expected_a),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );
    }
}

#[test]
fn cp_immediate() {
    run_test(
        // LD A, 0xF5; CP 0x13
        "3EF5FE13",
        &ExpectedState {
            a: Some(0xF5),
            f: Some(0x40),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xCC; CP 0xCC
        "3ECCFECC",
        &ExpectedState {
            a: Some(0xCC),
            f: Some(0xC0),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xCC; SCF; CP 0xCD
        "3ECC37FECD",
        &ExpectedState {
            a: Some(0xCC),
            f: Some(0x70),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x12; CP 0x51
        "3E12FE51",
        &ExpectedState {
            a: Some(0x12),
            f: Some(0x50),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xF3; CP 0x0A
        "3EF3FE0A",
        &ExpectedState {
            a: Some(0xF3),
            f: Some(0x60),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x00; CP 0xFF
        "3E00FEFF",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0x70),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn cp_indirect_hl() {
    run_test(
        // LD HL, 0xD0BC; LD (HL), 0xDD; LD A, 0x88; CP (HL)
        "21BCD036DD3E88BE",
        &ExpectedState {
            a: Some(0x88),
            f: Some(0x70),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xD0BC; LD (HL), 0xDD; LD A, 0xDD; CP (HL)
        "21BCD036DD3EDDBE",
        &ExpectedState {
            a: Some(0xDD),
            f: Some(0xC0),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn cp_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let cp_opcode = 0xB8 | r.to_opcode_bits();
        let cp_opcode_hex = format!("{cp_opcode:02x}");

        let (expected_a, expected_f) = match r {
            CpuRegister::A => (0xDE, 0xC0),
            _ => (0xDE, 0x60),
        };

        run_test(
            // LD <r>, 0xAF; LD A, 0xDE; CP <r>
            &format!("{load_opcode_hex}AF3EDE{cp_opcode_hex}"),
            &ExpectedState {
                a: Some(expected_a),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );
    }
}

#[test]
fn inc_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let inc_opcode = 0x04 | (r.to_opcode_bits() << 3);
        let inc_opcode_hex = format!("{inc_opcode:02x}");

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x39);
        expected_state.f = Some(0x00);

        run_test(
            // LD <r>, 0x38; INC <r>
            &format!("{load_opcode_hex}38{inc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x39);
        expected_state.f = Some(0x10);

        run_test(
            // LD <r>, 0x38; SCF; INC <r>
            &format!("{load_opcode_hex}3837{inc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0xA0);
        expected_state.f = Some(0x20);

        run_test(
            // LD <r>, 0x9F; INC <r>
            &format!("{load_opcode_hex}9F{inc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x00);
        expected_state.f = Some(0xA0);

        run_test(
            // LD <r>, 0xFF; INC <r>
            &format!("{load_opcode_hex}FF{inc_opcode_hex}"),
            &expected_state,
        );
    }
}

#[test]
fn inc_indirect_hl() {
    run_test(
        // LD HL, 0xDB3D; LD (HL), 0x20; INC (HL)
        "213DDB362034",
        &ExpectedState {
            f: Some(0x00),
            memory: hash_map! { 0xDB3D: 0x21 },
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xC545; LD (HL), 0xCF; INC (HL)
        "2145C536CF34",
        &ExpectedState {
            f: Some(0x20),
            memory: hash_map! { 0xC545: 0xD0 },
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xDE3A; LD (HL), 0xFF; INC (HL)
        "213ADE36FF34",
        &ExpectedState {
            f: Some(0xA0),
            memory: hash_map! { 0xDE3A: 0x00 },
            ..ExpectedState::empty()
        },
    );
}
