use super::{hash_map, run_test, set_in_state, ExpectedState, ALL_REGISTERS};

#[test]
fn rotate_left_accumulator() {
    run_test(
        // LD A, 0x00; RLCA
        "3E0007",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x0F; RLCA
        "3E0F07",
        &ExpectedState {
            a: Some(0x1E),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xC1; RLCA
        "3EC107",
        &ExpectedState {
            a: Some(0x83),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x00; SUB 0x02; RLCA
        "3E00D60207",
        &ExpectedState {
            a: Some(0xFD),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn rotate_left_indirect_hl() {
    run_test(
        // LD HL, 0xD6BE; LD (HL), 0x00; RLC (HL)
        "21BED63600CB06",
        &ExpectedState {
            memory: hash_map! { 0xD6BE: 0x00 },
            f: Some(0x80),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xCD6E; LD (HL), 0x0F; RLC (HL)
        "216ECD360FCB06",
        &ExpectedState {
            memory: hash_map! { 0xCD6E: 0x1E },
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xD78F; LD (HL), 0xC1; RLC (HL)
        "218FD736C1CB06",
        &ExpectedState {
            memory: hash_map! { 0xD78F: 0x83 },
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x00; SUB 0x02; LD HL, 0xCF42; LD (HL), 0xFE; RLC (HL)
        "3E00D6022142CF36FECB06",
        &ExpectedState {
            memory: hash_map! { 0xCF42: 0xFD },
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn rotate_left_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let rlc_opcode = r.to_opcode_bits();
        let rlc_opcode_hex = format!("CB{rlc_opcode:02x}");

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x00);
        expected_state.f = Some(0x80);
        run_test(
            // LD <r>, 0x00; RLC <r>
            &format!("{load_opcode_hex}00{rlc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x1E);
        expected_state.f = Some(0x00);
        run_test(
            // LD <r>, 0x0F; RLC <r>
            &format!("{load_opcode_hex}0F{rlc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x83);
        expected_state.f = Some(0x10);
        run_test(
            // LD <r>, 0xC1; RLC <r>
            &format!("{load_opcode_hex}C1{rlc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0xFD);
        expected_state.f = Some(0x10);
        run_test(
            // LD A, 0x00; SUB 0x02; LD <r>, 0xFE; RLC <r>
            // LD A, 0x00; SUB 0x02; RLCA
            &format!("3E00D602{load_opcode_hex}FE{rlc_opcode_hex}"),
            &expected_state,
        );
    }
}

#[test]
fn rotate_left_accumulator_thru_carry() {
    run_test(
        // LD A, 0x00; RLA
        "3E0017",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x36; RLA
        "3E3617",
        &ExpectedState {
            a: Some(0x6C),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x91; RLA
        "3E9117",
        &ExpectedState {
            a: Some(0x22),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x7D; SCF; RLA
        "3E7D3717",
        &ExpectedState {
            a: Some(0xFB),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xC8; SCF; RLA
        "3EC83717",
        &ExpectedState {
            a: Some(0x91),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x00; SUB 0x01; RLA
        "3E00D60117",
        &ExpectedState {
            a: Some(0xFF),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn rotate_right_accumulator() {
    run_test(
        // LD A, 0x00; RRCA
        "3E000F",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xFF; RRCA
        "3EFF0F",
        &ExpectedState {
            a: Some(0xFF),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x18; RRCA
        "3E180F",
        &ExpectedState {
            a: Some(0x0C),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x39; RRCA
        "3E390F",
        &ExpectedState {
            a: Some(0x9C),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x00; SUB 0x01; LD A, 0x18; RRCA
        "3E00D6013E180F",
        &ExpectedState {
            a: Some(0x0C),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn rotate_right_indirect_hl() {
    run_test(
        // LD HL, 0xCCBC; LD (HL), 0x00; RRC (HL)
        "21BCCC3600CB0E",
        &ExpectedState {
            memory: hash_map! { 0xCCBC: 0x00 },
            f: Some(0x80),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xCB48; LD (HL), 0xFF; RRC (HL)
        "2148CB36FFCB0E",
        &ExpectedState {
            memory: hash_map! { 0xCB48: 0xFF },
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xD893; LD (HL), 0x18; RRC (HL)
        "2193D83618CB0E",
        &ExpectedState {
            memory: hash_map! { 0xD893: 0x0C },
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD HL, 0xD6B4; LD (HL), 0x39; RRC (HL)
        "21B4D63639CB0E",
        &ExpectedState {
            memory: hash_map! { 0xD6B4: 0x9C },
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x00; SUB 0x01; LD HL, 0xDB24; LD (HL), 0x18; RRC (HL)
        "3E00D6012124DB3618CB0E",
        &ExpectedState {
            memory: hash_map! { 0xDB24: 0x0C },
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn rotate_right_register() {
    for r in ALL_REGISTERS {
        let load_opcode = 0x06 | (r.to_opcode_bits() << 3);
        let load_opcode_hex = format!("{load_opcode:02x}");

        let rrc_opcode = 0x08 | r.to_opcode_bits();
        let rrc_opcode_hex = format!("CB{rrc_opcode:02x}");

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x00);
        expected_state.f = Some(0x80);
        run_test(
            // LD <r>, 0x00; RRC <r>
            &format!("{load_opcode_hex}00{rrc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0xFF);
        expected_state.f = Some(0x10);
        run_test(
            // LD <r>, 0xFF; RRC <r>
            &format!("{load_opcode_hex}FF{rrc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x0C);
        expected_state.f = Some(0x00);
        run_test(
            // LD <r>, 0x18; RRC <r>
            &format!("{load_opcode_hex}18{rrc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x9C);
        expected_state.f = Some(0x10);
        run_test(
            // LD <r>, 0x39; RRC <r>
            &format!("{load_opcode_hex}39{rrc_opcode_hex}"),
            &expected_state,
        );

        let mut expected_state = ExpectedState::empty();
        set_in_state(&mut expected_state, r, 0x0C);
        expected_state.f = Some(0x00);
        run_test(
            // LD A, 0x00; SUB 0x01; LD <r>, 0x18; RRC <r>
            &format!("3E00D601{load_opcode_hex}18{rrc_opcode_hex}"),
            &expected_state,
        );
    }
}

#[test]
fn rotate_right_accumulator_thru_carry() {
    run_test(
        // LD A, 0x00; RRA
        "3E001F",
        &ExpectedState {
            a: Some(0x00),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xFF; RRA
        "3EFF1F",
        &ExpectedState {
            a: Some(0x7F),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0xFF; SCF; RRA
        "3EFF371F",
        &ExpectedState {
            a: Some(0xFF),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x34; RRA
        "3E341F",
        &ExpectedState {
            a: Some(0x1A),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x34; SCF; RRA
        "3E34371F",
        &ExpectedState {
            a: Some(0x9A),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // LD A, 0x00; SUB 0x01; LD A, 0x11; RRA
        "3E00D6013E111F",
        &ExpectedState {
            a: Some(0x88),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );
}
