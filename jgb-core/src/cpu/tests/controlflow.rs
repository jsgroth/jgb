use super::{run_test, ExpectedState};

#[test]
fn jump() {
    run_test(
        concat!(
            "3E55",   // 0x0150: LD A, 0x55
            "C35701", // 0x0152: JP 0x0157
            "3E33",   // 0x0155: LD A, 0x33
            "0677",   // 0x0157: LD B, 0x77
        ),
        &ExpectedState {
            a: Some(0x55),
            b: Some(0x77),
            ..ExpectedState::empty()
        },
    );

    run_test(
        concat!(
            "C35A01", // 0x0150: JP 0x015A
            "3E33",   // 0x0153: LD A, 0x33
            "0655",   // 0x0155: LD B, 0x55
            "C35F01", // 0x0157: JP 0x015F
            "3E77",   // 0x015A: LD A, 0x77
            "C35501", // 0x015C: JP 0x0155
            "0E88",   // 0x015F: LD C, 0x88
        ),
        &ExpectedState {
            a: Some(0x77),
            b: Some(0x55),
            c: Some(0x88),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn jump_hl() {
    run_test(
        concat!(
            "215801", // 0x0150: LD HL, 0x0158
            "3EAA",   // 0x0153: LD A, 0xAA
            "E9",     // 0x0155: JP HL
            "3ECC",   // 0x0156: LD A, 0xCC
            "06DD",   // 0x0158: LD B, 0xDD
        ),
        &ExpectedState {
            a: Some(0xAA),
            b: Some(0xDD),
            ..ExpectedState::empty()
        },
    );
}
