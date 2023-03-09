use super::{hash_map, run_test, set_in_state, ExpectedState, ALL_REGISTERS};

use crate::cpu::registers::CpuRegister;

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
        // LD HL, 0xDB3D; LD (HL), 0x20; SCF; INC (HL)
        "213DDB36203734",
        &ExpectedState {
            f: Some(0x10),
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

#[test]
fn dec_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let dec_opcode = 0x05 | (r.to_opcode_bits() << 3);
        let dec_opcode_hex = format!("{dec_opcode:02x}");

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x3D);
        expected_state.f = Some(0x40);

        run_test(
            // LD <r>, 0x3E; DEC <r>
            &format!("{load_opcode_hex}3E{dec_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x3D);
        expected_state.f = Some(0x50);

        run_test(
            // LD <r>, 0x3E; SCF; DEC <r>
            &format!("{load_opcode_hex}3E37{dec_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x9F);
        expected_state.f = Some(0x60);

        run_test(
            // LD <r> 0xA0; DEC <r>
            &format!("{load_opcode_hex}A0{dec_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x00);
        expected_state.f = Some(0xC0);

        run_test(
            // LD <r> 0x01; DEC <r>
            &format!("{load_opcode_hex}01{dec_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0xFF);
        expected_state.f = Some(0x60);

        run_test(
            // LD <r> 0x00; DEC <r>
            &format!("{load_opcode_hex}00{dec_opcode_hex}"),
            &expected_state,
        );
    }
}

#[test]
fn dec_indirect_hl() {
    run_test(
        // LD HL, 0xDA48; LD (HL), 0x3E; DEC (HL)
        "2148DA363E35",
        &ExpectedState {
            f: Some(0x40),
            memory: hash_map! { 0xDA48: 0x3D },
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xC5EC; LD (HL), 0x3E; SCF; DEC (HL)
        "21ECC5363E3735",
        &ExpectedState {
            f: Some(0x50),
            memory: hash_map! { 0xC5EC: 0x3D },
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xDBCF; LD (HL), 0x20; DEC (HL)
        "21CFDB362035",
        &ExpectedState {
            f: Some(0x60),
            memory: hash_map! { 0xDBCF: 0x1F },
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xC1D5; LD (HL), 0x01; DEC (HL)
        "21D5C1360135",
        &ExpectedState {
            f: Some(0xC0),
            memory: hash_map! { 0xC1D5: 0x00 },
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xD31C; LD (HL), 0x00; DEC (HL)
        "211CD3360035",
        &ExpectedState {
            f: Some(0x60),
            memory: hash_map! { 0xD31C: 0xFF },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn and_immediate() {
    for n in 0x00..=0xFF {
        let nn = format!("{n:02x}");
        run_test(
            // LD A, 0x00; AND <n>
            &format!("3E00E6{nn}"),
            &ExpectedState {
                a: Some(0x00),
                f: Some(0xA0),
                ..ExpectedState::empty()
            },
        );

        run_test(
            // LD A, <n>; AND 0x00
            &format!("3E{nn}E600"),
            &ExpectedState {
                a: Some(0x00),
                f: Some(0xA0),
                ..ExpectedState::empty()
            },
        );

        let expected_f = if n & 0xA5 == 0 { 0xA0 } else { 0x20 };
        run_test(
            // LD A, <n>; AND 0xA5
            &format!("3E{nn}E6A5"),
            &ExpectedState {
                a: Some(n & 0xA5),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );

        let expected_f = if n & 0x5A == 0 { 0xA0 } else { 0x20 };
        run_test(
            // LD A, <n>; SCF; AND 0x5A
            &format!("3E{nn}37E65A"),
            &ExpectedState {
                a: Some(n & 0x5A),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );
    }
}

#[test]
fn and_indirect_hl() {
    run_test(
        // LD HL, 0xDDDE; LD (HL), 0xF0; LD A, 0x0F; AND (HL)
        "21DEDD36F03E0FA6",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0xA0),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xC83A; LD (HL), 0x3E; LD A, 0x9A; AND (HL)
        "213AC8363E3E9AA6",
        &ExpectedState {
            a: Some(0x1A),
            f: Some(0x20),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn and_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let and_opcode = 0xA0 | r.to_opcode_bits();
        let and_opcode_hex = format!("{and_opcode:02x}");

        let (expected_a, expected_f) = match r {
            CpuRegister::A => (0xE5, 0x20),
            _ => (0x25, 0x20),
        };

        run_test(
            // LD <r>, 0x37; LD A, 0xE5; AND <r>
            &format!("{load_opcode_hex}373EE5{and_opcode_hex}"),
            &ExpectedState {
                a: Some(expected_a),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );

        let (expected_a, expected_f) = match r {
            CpuRegister::A => (0xAA, 0x20),
            _ => (0x00, 0xA0),
        };

        run_test(
            // LD <r>, 0x55; LD A, 0xAA; AND <r>
            &format!("{load_opcode_hex}553EAA{and_opcode_hex}"),
            &ExpectedState {
                a: Some(expected_a),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );
    }
}

#[test]
fn or_immediate() {
    for n in 0x00..=0xFF {
        let nn = format!("{n:02x}");

        let expected_f = if n == 0x00 { 0x80 } else { 0x00 };
        run_test(
            // LD A, 0x00; OR <n>
            &format!("3E00F6{nn}"),
            &ExpectedState {
                a: Some(n),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );

        run_test(
            // LD A, 0xFF; SCF; OR <n>
            &format!("3EFF37F6{nn}"),
            &ExpectedState {
                a: Some(0xFF),
                f: Some(0x00),
                ..ExpectedState::empty()
            },
        );

        let expected_f = if n == 0x00 { 0x80 } else { 0x00 };
        run_test(
            // LD A, <n>; OR 0x00
            &format!("3E{nn}F600"),
            &ExpectedState {
                a: Some(n),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );

        run_test(
            // LD A, 0x32; OR <n>
            &format!("3E32F6{nn}"),
            &ExpectedState {
                a: Some(n | 0x32),
                f: Some(0x00),
                ..ExpectedState::empty()
            },
        );
    }
}

#[test]
fn or_indirect_hl() {
    run_test(
        // LD HL, 0xC610; LD (HL), 0x6A; LD A, 0x00; OR (HL)
        "2110C6366A3E00B6",
        &ExpectedState {
            a: Some(0x6A),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xC610; LD (HL), 0x6A; LD A, 0x33; OR (HL)
        "2110C6366A3E33B6",
        &ExpectedState {
            a: Some(0x7B),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xC610; LD (HL), 0x00; LD A, 0x00; OR (HL)
        "2110C636003E00B6",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0x80),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn or_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let or_opcode = 0xB0 | r.to_opcode_bits();
        let or_opcode_hex = format!("{or_opcode:02x}");

        let (expected_a, expected_f) = match r {
            CpuRegister::A => (0x86, 0x00),
            _ => (0xFE, 0x00),
        };
        run_test(
            // LD <r>, 0xFA; LD A, 0x86; SCF; OR <r>
            &format!("{load_opcode_hex}FA3E8637{or_opcode_hex}"),
            &ExpectedState {
                a: Some(expected_a),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );

        run_test(
            // LD <r>, 0x00; LD A, 0x00; OR <r>
            &format!("{load_opcode_hex}003E00{or_opcode_hex}"),
            &ExpectedState {
                a: Some(0x00),
                f: Some(0x80),
                ..ExpectedState::empty()
            },
        );
    }
}

#[test]
fn xor_immediate() {
    for n in 0x00..=0xFF {
        let nn = format!("{n:02x}");

        let expected_f = if n == 0x00 { 0x80 } else { 0x00 };
        run_test(
            // LD A, 0x00; XOR <n>
            &format!("3E00EE{nn}"),
            &ExpectedState {
                a: Some(n),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );

        let expected_f = if n ^ 0xFF == 0x00 { 0x80 } else { 0x00 };
        run_test(
            // LD A, 0xFF; XOR <n>
            &format!("3EFFEE{nn}"),
            &ExpectedState {
                a: Some(n ^ 0xFF),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );

        run_test(
            // LD A, <n>; XOR 0xFF
            &format!("3E{nn}EEFF"),
            &ExpectedState {
                a: Some(n ^ 0xFF),
                f: Some(expected_f),
                ..ExpectedState::empty()
            },
        );
    }

    run_test(
        // LD A, 0x00; SUB 0x01; XOR 0x33
        "3E00D601EE33",
        &ExpectedState {
            a: Some(0xCC),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn xor_indirect_hl() {}

#[test]
fn carry_flag_manipulation() {
    run_test(
        // SCF
        "37",
        &ExpectedState {
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // SCF; SCF
        "3737",
        &ExpectedState {
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // CCF
        "3F",
        &ExpectedState {
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // CCF; CCF
        "3F3F",
        &ExpectedState {
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x10; SUB 0x01; SCF
        "3E10D60137",
        &ExpectedState {
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x00; SUB 0x01; CCF
        "3E00D6013F",
        &ExpectedState {
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x10; SUB 0x01; CCF
        "3E10D6013F",
        &ExpectedState {
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn complement_accumulator() {
    for n in 0x00..=0xFF {
        let nn = format!("{n:02x}");

        run_test(
            // LD A, <n>; CPL
            &format!("3E{nn}2F"),
            &ExpectedState {
                a: Some(!n),
                f: Some(0x60),
                ..ExpectedState::empty()
            },
        );
    }

    run_test(
        // LD A, 0xC3; SCF; CPL
        "3EC3372F",
        &ExpectedState {
            a: Some(0x3C),
            f: Some(0x70),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x01; SUB 0x01; CPL
        "3E01D6012F",
        &ExpectedState {
            a: Some(0xFF),
            f: Some(0xE0),
            ..ExpectedState::empty()
        },
    );
}
