use super::{hash_map, run_test, ExpectedState};

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

#[test]
fn conditional_jump_nz() {
    run_test(
        concat!(
            "06AA",   // 0x0150: LD B, 0xAA
            "3E00",   // 0x0152: LD A, 0x00
            "FE00",   // 0x0154: CP 0x00,
            "C25B01", // 0x0156: JP NZ, 0x015B
            "06BB",   // 0x0159: LD B, 0xBB
            "0ECC",   // 0x015B: LD C, 0xCC
        ),
        &ExpectedState {
            a: Some(0x00),
            b: Some(0xBB),
            c: Some(0xCC),
            f: Some(0xC0),
            ..ExpectedState::empty()
        },
    );

    run_test(
        concat!(
            "06AA",   // 0x0150: LD B, 0xAA
            "3E01",   // 0x0152: LD A, 0x01,
            "FE00",   // 0x0154: CP 0x00,
            "C25B01", // 0x0156: JP NZ, 0x015B
            "06BB",   // 0x0159: LD B, 0xBB
            "0ECC",   // 0x015B: LD C, 0xCC
        ),
        &ExpectedState {
            a: Some(0x01),
            b: Some(0xAA),
            c: Some(0xCC),
            f: Some(0x40),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn conditional_jump_z() {
    run_test(
        concat!(
            "06AA",   // 0x0150: LD B, 0xAA
            "3E00",   // 0x0152: LD A, 0x00
            "FE00",   // 0x0154: CP 0x00,
            "CA5B01", // 0x0156: JP Z, 0x015B
            "06BB",   // 0x0159: LD B, 0xBB
            "0ECC",   // 0x015B: LD C, 0xCC
        ),
        &ExpectedState {
            a: Some(0x00),
            b: Some(0xAA),
            c: Some(0xCC),
            f: Some(0xC0),
            ..ExpectedState::empty()
        },
    );

    run_test(
        concat!(
            "06AA",   // 0x0150: LD B, 0xAA
            "3E01",   // 0x0152: LD A, 0x01,
            "FE00",   // 0x0154: CP 0x00,
            "CA5B01", // 0x0156: JP Z, 0x015B
            "06BB",   // 0x0159: LD B, 0xBB
            "0ECC",   // 0x015B: LD C, 0xCC
        ),
        &ExpectedState {
            a: Some(0x01),
            b: Some(0xBB),
            c: Some(0xCC),
            f: Some(0x40),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn conditional_jump_nc() {
    run_test(
        concat!(
            "06AA",   // 0x0150: LD B, 0xAA
            "37",     // 0x0152: SCF
            "3F",     // 0x0153: CCF
            "D25901", // 0x0154: JP NC, 0x0159
            "06BB",   // 0x0157: LD B, 0xBB
            "0ECC",   // 0x0159: LD C, 0xCC
        ),
        &ExpectedState {
            b: Some(0xAA),
            c: Some(0xCC),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        concat!(
            "06AA",   // 0x0150: LD B, 0xAA
            "37",     // 0x0152: SCF
            "D25801", // 0x0153: JP NC, 0x0158
            "06BB",   // 0x0156: LD B, 0xBB
            "0ECC",   // 0x0158: LD C, 0xCC
        ),
        &ExpectedState {
            b: Some(0xBB),
            c: Some(0xCC),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn conditional_jump_c() {
    run_test(
        concat!(
            "06AA",   // 0x0150: LD B, 0xAA
            "37",     // 0x0152: SCF
            "3F",     // 0x0153: CCF
            "DA5901", // 0x0154: JP C, 0x0159
            "06BB",   // 0x0157: LD B, 0xBB
            "0ECC",   // 0x0159: LD C, 0xCC
        ),
        &ExpectedState {
            b: Some(0xBB),
            c: Some(0xCC),
            f: Some(0x00),
            ..ExpectedState::empty()
        },
    );

    run_test(
        concat!(
            "06AA",   // 0x0150: LD B, 0xAA
            "37",     // 0x0152: SCF
            "DA5801", // 0x0153: JP C, 0x0158
            "06BB",   // 0x0156: LD B, 0xBB
            "0ECC",   // 0x0158: LD C, 0xCC
        ),
        &ExpectedState {
            b: Some(0xAA),
            c: Some(0xCC),
            f: Some(0x10),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn relative_jump() {
    run_test(
        concat!(
            "06AA", // 0x0150: LD B, 0xAA
            "1802", // 0x0152: JR 2
            "06BB", // 0x0154: LD B, 0xBB
            "0ECC", // 0x0156: LD C, 0xCC
        ),
        &ExpectedState {
            b: Some(0xAA),
            c: Some(0xCC),
            ..ExpectedState::empty()
        },
    );

    run_test(
        concat!(
            "06AA", // 0x0150: LD B, 0xAA
            "1806", // 0x0152: JR 6
            "0688", // 0x0154: LD B, 0x88
            "3E99", // 0x0156: LD A, 0x99
            "1802", // 0x0158: JR 2
            "18FA", // 0x015A: JR -6
            "0ECC", // 0x015C: LD C, 0xCC
        ),
        &ExpectedState {
            a: Some(0x99),
            b: Some(0xAA),
            c: Some(0xCC),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn relative_jump_nz_z() {
    run_test(
        concat!(
            "06AA", // 0x0150: LD B, 0xAA
            "0ECC", // 0x0152: LD C, 0xCC
            "3E01", // 0x0154: LD A, 0x01
            "FE00", // 0x0156: CP 0x00
            "2006", // 0x0158: JR NZ 6
            "06BB", // 0x015A: LD B, 0xBB
            "16DD", // 0x015C: LD D, 0xDD
            "1804", // 0x015E: JR 4
            "28FA", // 0x0160: JR Z, -6
            "20F8", // 0x0162: JR NZ, -8
            "1EEE", // 0x0164: LD E, 0xEE
        ),
        &ExpectedState {
            a: Some(0x01),
            b: Some(0xAA),
            c: Some(0xCC),
            d: Some(0xDD),
            e: Some(0xEE),
            f: Some(0x40),
            ..ExpectedState::empty()
        },
    );

    run_test(
        concat!(
            "06AA", // 0x0150: LD B, 0xAA
            "0ECC", // 0x0152: LD C, 0xCC
            "3E00", // 0x0154: LD A, 0x00
            "FE00", // 0x0156: CP 0x00
            "2806", // 0x0158: JR Z 6
            "06BB", // 0x015A: LD B, 0xBB
            "16DD", // 0x015C: LD D, 0xDD
            "1804", // 0x015E: JR 4
            "20FA", // 0x0160: JR NZ, -6
            "28F8", // 0x0162: JR Z, -8
            "1EEE", // 0x0164: LD E, 0xEE
        ),
        &ExpectedState {
            a: Some(0x00),
            b: Some(0xAA),
            c: Some(0xCC),
            d: Some(0xDD),
            e: Some(0xEE),
            f: Some(0xC0),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn call_return() {
    run_test(
        concat!(
            "06AA",   // 0x0150: LD B, 0xAA
            "1807",   // 0x0152: JR 7
            "06BB",   // 0x0154: LD B, 0xBB
            "0ECC",   // 0x0156: LD C, 0xCC
            "C9",     // 0x0158: RET
            "06FF",   // 0x0159: LD B, 0xFF
            "16DD",   // 0x015B: LD D, 0xDD
            "CD5601", // 0x015D: CALL 0x0156
            "1EEE",   // 0x0160: LD E, 0xEE
        ),
        &ExpectedState {
            b: Some(0xAA),
            c: Some(0xCC),
            d: Some(0xDD),
            e: Some(0xEE),
            sp: Some(0xFFFE),
            memory: hash_map! {
                0xFFFC: 0x60,
                0xFFFD: 0x01,
            },
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn conditional_call_nz_z() {
    // C4: CALL NZ, nn
    // CC: CALL Z, nn
}

#[test]
fn conditional_call_nc_c() {
    // D4: CALL NC, nn
    // DC: CALL C, nn
}

#[test]
fn conditional_return_nz_z() {
    // C0: RET NZ
    // C8: RET Z
}

#[test]
fn conditional_return_nc_c() {
    // D0: RET NC
    // D8: RET C
}

#[test]
fn rst_call() {
    // C7 | 00 xxx 000: RST xxx
}

#[test]
fn enable_interrupts() {
    run_test(
        // EI
        "FB",
        &ExpectedState {
            ime: Some(true.into()),
            interrupt_delay: Some(true.into()),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // EI; EI
        "FBFB",
        &ExpectedState {
            ime: Some(true.into()),
            interrupt_delay: Some(true.into()),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // EI; NOP
        "FB00",
        &ExpectedState {
            ime: Some(true.into()),
            interrupt_delay: Some(false.into()),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn disable_interrupts() {
    run_test(
        // DI
        "F3",
        &ExpectedState {
            ime: Some(false.into()),
            ..ExpectedState::empty()
        },
    );

    run_test(
        // EI; DI
        "FBF3",
        &ExpectedState {
            ime: Some(false.into()),
            ..ExpectedState::empty()
        },
    );
}

#[test]
fn return_from_interrupt_handler() {
    // D9: RETI
}
