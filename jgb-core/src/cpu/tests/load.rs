use super::{ALL_REGISTERS, ExpectedState, hash_map, run_test, set_in_state};

use crate::cpu::registers::{CpuRegister, CpuRegisterPair};

#[test]
fn load_register_immediate() {
    run_test(
        // LD A, 0x45
        "3E45",
        &ExpectedState { a: Some(0x45), ..ExpectedState::empty() },
    );

    run_test(
        // LD B, 0x45
        "0645",
        &ExpectedState { b: Some(0x45), ..ExpectedState::empty() },
    );

    run_test(
        // LD C, 0x45
        "0E45",
        &ExpectedState { c: Some(0x45), ..ExpectedState::empty() },
    );

    run_test(
        // LD D, 0x45
        "1645",
        &ExpectedState { d: Some(0x45), ..ExpectedState::empty() },
    );

    run_test(
        // LD E, 0x45
        "1E45",
        &ExpectedState { e: Some(0x45), ..ExpectedState::empty() },
    );

    run_test(
        // LD H, 0x45
        "2645",
        &ExpectedState { h: Some(0x45), ..ExpectedState::empty() },
    );

    run_test(
        // LD L, 0x45
        "2E45",
        &ExpectedState { l: Some(0x45), ..ExpectedState::empty() },
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
        &ExpectedState { b: Some(0x24), c: Some(0x68), ..ExpectedState::empty() },
    );

    run_test(
        // LD DE, 0x2468
        "116824",
        &ExpectedState { d: Some(0x24), e: Some(0x68), ..ExpectedState::empty() },
    );

    run_test(
        // LD HL, 0x2468
        "216824",
        &ExpectedState { h: Some(0x24), l: Some(0x68), ..ExpectedState::empty() },
    );

    run_test(
        // LD SP, 0x2468
        "316824",
        &ExpectedState { sp: Some(0x2468), ..ExpectedState::empty() },
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
        &ExpectedState { memory: hash_map! { 0xC105: 0x83 }, ..ExpectedState::empty() },
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

        run_test(&program_hex, &ExpectedState {
            memory: hash_map! { 0xD075: expected_value },
            ..ExpectedState::empty()
        });
    }
}

#[test]
fn load_accumulator_indirect_bc() {
    run_test(
        // LD HL, 0xC555; LD (HL), 0xC4; LD BC, 0xC555; LD A, (BC)
        "2155C536C40155C50A",
        &ExpectedState { a: Some(0xC4), ..ExpectedState::empty() },
    );
}

#[test]
fn load_accumulator_indirect_de() {
    run_test(
        // LD HL, 0xC555; LD (HL), 0x2F; LD DE, 0xC555; LD A, (DE)
        "2155C5362F1155C51A",
        &ExpectedState { a: Some(0x2F), ..ExpectedState::empty() },
    );
}

#[test]
fn load_accumulator_direct_16() {
    run_test(
        // LD HL, 0xD943; LD (HL), 0x1B; LD A, (0xD943)
        "2143D9361BFA43D9",
        &ExpectedState { a: Some(0x1B), ..ExpectedState::empty() },
    );
}

#[test]
fn load_indirect_bc_accumulator() {
    run_test(
        // LD A, 0xFA; LD BC, 0xC560; LD (BC), A
        "3EFA0160C502",
        &ExpectedState { memory: hash_map! { 0xC560: 0xFA }, ..ExpectedState::empty() },
    );
}

#[test]
fn load_indirect_de_accumulator() {
    run_test(
        // LD A, 0x65; LD DE, 0xC010; LD (DE), A
        "3E651110C012",
        &ExpectedState { memory: hash_map! { 0xC010: 0x65 }, ..ExpectedState::empty() },
    );
}

#[test]
fn load_direct_16_accumulator() {
    run_test(
        // LD A, 0x90; LD (0xD40E), A
        "3E90EA0ED4",
        &ExpectedState { memory: hash_map! { 0xD40E: 0x90 }, ..ExpectedState::empty() },
    );
}

#[test]
fn ldh_accumulator_immediate() {
    run_test(
        // LD HL, 0xFF40; LD (HL), 0xC8; LDH A, (0x40)
        "2140FF36C8F040",
        &ExpectedState { a: Some(0xC8), ..ExpectedState::empty() },
    );
}

#[test]
fn ldh_accumulator_c() {
    // LD HL, 0xFF40; LD (HL), 0xEE; LD C, 0x40; LDH A, (C)
    run_test("2140FF36EE0E40F2", &ExpectedState { a: Some(0xEE), ..ExpectedState::empty() });
}

#[test]
fn ldh_immediate_accumulator() {
    run_test(
        // LD A, 0x72; LDH (0x40), A
        "3E72E040",
        &ExpectedState { memory: hash_map! { 0xFF40: 0x72 }, ..ExpectedState::empty() },
    );
}

#[test]
fn ldh_c_accumulator() {
    run_test(
        // LD A, 0xCB; LD C, 0x40; LDH (C), A
        "3ECB0E40E2",
        &ExpectedState { memory: hash_map! { 0xFF40: 0xCB }, ..ExpectedState::empty() },
    );
}

#[test]
fn load_accumulator_indirect_hl_inc() {
    run_test(
        // LD HL, 0xDC60; LD (HL), 0xD5; LD A, (HL+)
        "2160DC36D52A",
        &ExpectedState { a: Some(0xD5), h: Some(0xDC), l: Some(0x61), ..ExpectedState::empty() },
    );

    run_test(
        // LD HL, 0xCFFF; LD (HL), 0x93; LD A, (HL+)
        "21FFCF36932A",
        &ExpectedState { a: Some(0x93), h: Some(0xD0), l: Some(0x00), ..ExpectedState::empty() },
    );
}

#[test]
fn load_accumulator_indirect_hl_dec() {
    run_test(
        // LD HL, 0xD49A; LD (HL), 0x92; LD A, (HL-)
        "219AD436923A",
        &ExpectedState { a: Some(0x92), h: Some(0xD4), l: Some(0x99), ..ExpectedState::empty() },
    );

    run_test(
        // LD HL, 0xD000; LD (HL), 0xF9; LD A, (HL-)
        "2100D036F93A",
        &ExpectedState { a: Some(0xF9), h: Some(0xCF), l: Some(0xFF), ..ExpectedState::empty() },
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
        &ExpectedState { sp: Some(0xFFBB), ..ExpectedState::empty() },
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
        *low_ref = Some(match rr {
            CpuRegisterPair::AF => 0x50,
            _ => 0x57,
        });
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
