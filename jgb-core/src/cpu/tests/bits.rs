use super::{run_test, ExpectedState};

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
