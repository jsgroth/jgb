use super::{hash_map, run_test, set_in_state, ExpectedState, ALL_REGISTERS};

#[test]
fn test_bit_register() {
    for r in ALL_REGISTERS {
        let ld = 0x06 | (r.to_opcode_bits() << 3);
        let ld = format!("{ld:02x}");

        for bit in 0..8 {
            let opcode = 0x40 | (bit << 3) | r.to_opcode_bits();
            let opcode = format!("CB{opcode:02x}");

            let n: u8 = rand::random();
            let n_hex = format!("{n:02x}");

            let mut expected_state = ExpectedState::empty();
            set_in_state(&mut expected_state, r, n);
            let expected_z_flag = u8::from(n & (1 << bit) == 0);
            expected_state.f = Some(0x20 | (expected_z_flag << 7));
            run_test(
                // LD <r>, <n>; BIT <b>, <r>
                &format!("{ld}{n_hex}{opcode}"),
                &expected_state,
            );
        }
    }

    run_test(
        // LD A, 0x00; SUB 0x01; LD A, 0xF7; BIT 3, A
        "3E00D6013EF7CB5F",
        &ExpectedState {
            a: Some(0xF7),
            f: Some(0xB0),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn test_bit_indirect_hl() {
    for _ in 0..10 {
        let n: u8 = rand::random();
        let n_hex = format!("{n:02x}");

        for bit in 0..8 {
            let opcode = 0x46 | (bit << 3);
            let opcode = format!("CB{opcode:02x}");

            let expected_z_flag = u8::from(n & (1 << bit) == 0);
            run_test(
                // LD HL, 0xC53E; LD (HL), <n>; BIT <b>, (HL)
                &format!("213EC536{n_hex}{opcode}"),
                &ExpectedState {
                    memory: hash_map! { 0xC53E: n },
                    f: Some(0x20 | (expected_z_flag << 7)),
                    ..ExpectedState::empty()
                },
            );
        }
    }

    run_test(
        // LD A, 0x00; SUB 0x01; LD HL, 0xC53E; LD (HL), 0xF7; BIT 3, (HL)
        "3E00D601213EC536F7CB5E",
        &ExpectedState {
            memory: hash_map! { 0xC53E: 0xF7 },
            f: Some(0xB0),
            ..ExpectedState::empty()
        },
    );
}
