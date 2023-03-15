mod parse;

use crate::cpu::registers::{CpuRegister, CpuRegisterPair, CpuRegisters};
use crate::memory::AddressSpace;
use std::num::TryFromIntError;
use thiserror::Error;

pub use parse::{parse_next_instruction, ParseError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpCondition {
    NZ,
    Z,
    NC,
    C,
}

impl JumpCondition {
    fn check(self, cpu_registers: &CpuRegisters) -> bool {
        match self {
            Self::NZ => !cpu_registers.zero_flag(),
            Self::Z => cpu_registers.zero_flag(),
            Self::NC => !cpu_registers.carry_flag(),
            Self::C => cpu_registers.carry_flag(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("error adding relative offset to SP or PC register: {source}")]
    RegisterOverflow {
        #[from]
        source: TryFromIntError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // LD r, r'
    LoadRegisterRegister(CpuRegister, CpuRegister),
    // LD r, n
    LoadRegisterImmediate(CpuRegister, u8),
    // LD r, (HL)
    LoadRegisterIndirectHL(CpuRegister),
    // LD (HL), r
    LoadIndirectHLRegister(CpuRegister),
    // LD (HL), n
    LoadIndirectHLImmediate(u8),
    // LD A, (BC)
    LoadAccumulatorIndirectBC,
    // LD A, (DE)
    LoadAccumulatorIndirectDE,
    // LD (BC), A
    LoadIndirectBCAccumulator,
    // LD (DE), A
    LoadIndirectDEAccumulator,
    // LD A, (nn)
    LoadAccumulatorDirect16(u16),
    // LD (nn), A
    LoadDirect16Accumulator(u16),
    // LDH A, (C)
    LoadAccumulatorIndirectC,
    // LDH (C), A
    LoadIndirectCAccumulator,
    // LDH A, (n)
    LoadAccumulatorDirect8(u8),
    // LDH (n), A
    LoadDirect8Accumulator(u8),
    // LD A, (HL-)
    LoadAccumulatorIndirectHLDec,
    // LD (HL-), A
    LoadIndirectHLDecAccumulator,
    // LD A, (HL+)
    LoadAccumulatorIndirectHLInc,
    // LD (HL+), A
    LoadIndirectHLIncAccumulator,
    // LD rr, nn
    LoadRegisterPairImmediate(CpuRegisterPair, u16),
    // LD (nn), SP
    LoadDirectStackPointer(u16),
    // LD SP, HL
    LoadStackPointerHL,
    // LDHL SP, e
    LoadHLStackPointerOffset(i8),
    // PUSH rr
    PushStack(CpuRegisterPair),
    // POP rr
    PopStack(CpuRegisterPair),
    // ADD r
    AddRegister(CpuRegister),
    // ADD (HL)
    AddIndirectHL,
    // ADD n
    AddImmediate(u8),
    // ADC r
    AddCarryRegister(CpuRegister),
    // ADC (HL)
    AddCarryIndirectHL,
    // ADC n
    AddCarryImmediate(u8),
    // SUB r
    SubtractRegister(CpuRegister),
    // SUB (HL)
    SubtractIndirectHL,
    // SUB n
    SubtractImmediate(u8),
    // SBC r
    SubtractCarryRegister(CpuRegister),
    // SBC (HL)
    SubtractCarryIndirectHL,
    // SBC n
    SubtractCarryImmediate(u8),
    // CP r
    CompareRegister(CpuRegister),
    // CP (HL)
    CompareIndirectHL,
    // CP n
    CompareImmediate(u8),
    // INC r
    IncRegister(CpuRegister),
    // INC (HL)
    IncIndirectHL,
    // DEC r
    DecRegister(CpuRegister),
    // DEC (HL)
    DecIndirectHL,
    // AND r
    AndRegister(CpuRegister),
    // AND (HL)
    AndIndirectHL,
    // AND n
    AndImmediate(u8),
    // OR r
    OrRegister(CpuRegister),
    // OR (HL)
    OrIndirectHL,
    // OR n
    OrImmediate(u8),
    // XOR r
    XorRegister(CpuRegister),
    // XOR (HL)
    XorIndirectHL,
    // XOR n
    XorImmediate(u8),
    // ADD HL, rr
    AddHLRegister(CpuRegisterPair),
    // INC rr,
    IncRegisterPair(CpuRegisterPair),
    // DEC rr,
    DecRegisterPair(CpuRegisterPair),
    // ADD SP, e
    AddSPImmediate(i8),
    // RLCA
    RotateLeftAccumulator,
    // RLA
    RotateLeftAccumulatorThruCarry,
    // RRCA
    RotateRightAccumulator,
    // RRA
    RotateRightAccumulatorThruCarry,
    // RLC r
    RotateLeft(CpuRegister),
    // RLC (HL)
    RotateLeftIndirectHL,
    // RL r
    RotateLeftThruCarry(CpuRegister),
    // RL (HL)
    RotateLeftIndirectHLThruCarry,
    // RRC r
    RotateRight(CpuRegister),
    // RRC (HL)
    RotateRightIndirectHL,
    // RR r
    RotateRightThruCarry(CpuRegister),
    // RR (HL)
    RotateRightIndirectHLThruCarry,
    // SLA r
    ShiftLeft(CpuRegister),
    // SLA (HL)
    ShiftLeftIndirectHL,
    // SWAP r
    Swap(CpuRegister),
    // SWAP (HL)
    SwapIndirectHL,
    // SRA r
    ShiftRight(CpuRegister),
    // SRA (HL)
    ShiftRightIndirectHL,
    // SRL r
    ShiftRightLogical(CpuRegister),
    // SRL (HL)
    ShiftRightLogicalIndirectHL,
    // BIT n, r
    TestBit(u8, CpuRegister),
    // BIT n, (HL)
    TestBitIndirectHL(u8),
    // SET n, r
    SetBit(u8, CpuRegister),
    // SET n, (HL)
    SetBitIndirectHL(u8),
    // RES n, r
    ResetBit(u8, CpuRegister),
    // RES n, (HL)
    ResetBitIndirectHL(u8),
    // CCF
    ComplementCarryFlag,
    // SCF
    SetCarryFlag,
    // DAA
    DecimalAdjustAccumulator,
    // CPL
    ComplementAccumulator,
    // JP nn
    Jump(u16),
    // JP HL
    JumpHL,
    // JP cc, nn
    JumpCond(JumpCondition, u16),
    // JR e
    RelativeJump(i8),
    // JR cc, e
    RelativeJumpCond(JumpCondition, i8),
    // CALL nn,
    Call(u16),
    // CALL cc, nn
    CallCond(JumpCondition, u16),
    // RET
    Return,
    // RET cc
    ReturnCond(JumpCondition),
    // RETI
    ReturnFromInterruptHandler,
    // RST n
    RestartCall(u8),
    // HALT
    HaltClock,
    // STOP
    StopClocks,
    // DI
    DisableInterrupts,
    // EI
    EnableInterrupts,
    // NOP
    NoOp,
}

impl Instruction {
    pub fn execute(
        self,
        address_space: &mut AddressSpace,
        cpu_registers: &mut CpuRegisters,
    ) -> Result<(), ExecutionError> {
        match self {
            Self::LoadRegisterRegister(r, r_p) => {
                cpu_registers.set_register(r, cpu_registers.read_register(r_p));
            }
            Self::LoadRegisterImmediate(r, n) => {
                cpu_registers.set_register(r, n);
            }
            Self::LoadRegisterIndirectHL(r) => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                cpu_registers.set_register(r, value);
            }
            Self::LoadIndirectHLRegister(r) => {
                let value = cpu_registers.read_register(r);
                address_space.write_address_u8(cpu_registers.hl(), value);
            }
            Self::LoadIndirectHLImmediate(n) => {
                address_space.write_address_u8(cpu_registers.hl(), n);
            }
            Self::LoadAccumulatorIndirectBC => {
                cpu_registers.accumulator = address_space.read_address_u8(cpu_registers.bc());
            }
            Self::LoadAccumulatorIndirectDE => {
                cpu_registers.accumulator = address_space.read_address_u8(cpu_registers.de());
            }
            Self::LoadIndirectBCAccumulator => {
                address_space.write_address_u8(cpu_registers.bc(), cpu_registers.accumulator);
            }
            Self::LoadIndirectDEAccumulator => {
                address_space.write_address_u8(cpu_registers.de(), cpu_registers.accumulator);
            }
            Self::LoadAccumulatorDirect16(nn) => {
                cpu_registers.accumulator = address_space.read_address_u8(nn);
            }
            Self::LoadDirect16Accumulator(nn) => {
                address_space.write_address_u8(nn, cpu_registers.accumulator);
            }
            Self::LoadAccumulatorIndirectC => {
                let address = u16::from_be_bytes([0xFF, cpu_registers.c]);
                cpu_registers.accumulator = address_space.read_address_u8(address);
            }
            Self::LoadIndirectCAccumulator => {
                let address = u16::from_be_bytes([0xFF, cpu_registers.c]);
                address_space.write_address_u8(address, cpu_registers.accumulator);
            }
            Self::LoadAccumulatorDirect8(n) => {
                let address = u16::from_be_bytes([0xFF, n]);
                cpu_registers.accumulator = address_space.read_address_u8(address);
            }
            Self::LoadDirect8Accumulator(n) => {
                let address = u16::from_be_bytes([0xFF, n]);
                address_space.write_address_u8(address, cpu_registers.accumulator);
            }
            Self::LoadAccumulatorIndirectHLDec => {
                let hl = cpu_registers.hl();
                cpu_registers.accumulator = address_space.read_address_u8(hl);
                cpu_registers.set_hl(hl.wrapping_sub(1));
            }
            Self::LoadIndirectHLDecAccumulator => {
                let hl = cpu_registers.hl();
                address_space.write_address_u8(hl, cpu_registers.accumulator);
                cpu_registers.set_hl(hl.wrapping_sub(1));
            }
            Self::LoadAccumulatorIndirectHLInc => {
                let hl = cpu_registers.hl();
                cpu_registers.accumulator = address_space.read_address_u8(hl);
                cpu_registers.set_hl(hl.wrapping_add(1));
            }
            Self::LoadIndirectHLIncAccumulator => {
                let hl = cpu_registers.hl();
                address_space.write_address_u8(hl, cpu_registers.accumulator);
                cpu_registers.set_hl(hl.wrapping_add(1));
            }
            Self::LoadRegisterPairImmediate(rr, nn) => {
                cpu_registers.set_register_pair(rr, nn);
            }
            Self::LoadDirectStackPointer(nn) => {
                address_space.write_address_u16(nn, cpu_registers.sp);
            }
            Self::LoadStackPointerHL => {
                cpu_registers.sp = cpu_registers.hl();
            }
            Self::PushStack(rr) => {
                cpu_registers.sp -= 2;
                address_space
                    .write_address_u16(cpu_registers.sp, cpu_registers.read_register_pair(rr));
            }
            Self::PopStack(rr) => {
                cpu_registers
                    .set_register_pair(rr, address_space.read_address_u16(cpu_registers.sp));
                cpu_registers.sp += 2;
            }
            Self::AddRegister(r) => {
                let (sum, carry, h_flag) = add(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    false,
                );
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag.into(), carry.into());
            }
            Self::AddIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (sum, carry, h_flag) = add(cpu_registers.accumulator, value, false);
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag.into(), carry.into());
            }
            Self::AddImmediate(n) => {
                let (sum, carry, h_flag) = add(cpu_registers.accumulator, n, false);
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag.into(), carry.into());
            }
            Self::AddCarryRegister(r) => {
                let (sum, carry, h_flag) = add(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    cpu_registers.carry_flag(),
                );
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag.into(), carry.into());
            }
            Self::AddCarryIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (sum, carry, h_flag) =
                    add(cpu_registers.accumulator, value, cpu_registers.carry_flag());
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag.into(), carry.into());
            }
            Self::AddCarryImmediate(n) => {
                let (sum, carry, h_flag) =
                    add(cpu_registers.accumulator, n, cpu_registers.carry_flag());
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag.into(), carry.into());
            }
            Self::SubtractRegister(r) => {
                let (difference, carry, h_flag) = sub(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    false,
                );
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag.into(), carry.into());
            }
            Self::SubtractIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (difference, carry, h_flag) = sub(cpu_registers.accumulator, value, false);
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag.into(), carry.into());
            }
            Self::SubtractImmediate(n) => {
                let (difference, carry, h_flag) = sub(cpu_registers.accumulator, n, false);
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag.into(), carry.into());
            }
            Self::SubtractCarryRegister(r) => {
                let (difference, carry, h_flag) = sub(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    cpu_registers.carry_flag(),
                );
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag.into(), carry.into());
            }
            Self::SubtractCarryIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (difference, carry, h_flag) =
                    sub(cpu_registers.accumulator, value, cpu_registers.carry_flag());
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag.into(), carry.into());
            }
            Self::SubtractCarryImmediate(n) => {
                let (difference, carry, h_flag) =
                    sub(cpu_registers.accumulator, n, cpu_registers.carry_flag());
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag.into(), carry.into());
            }
            Self::CompareRegister(r) => {
                let (difference, carry, h_flag) = sub(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    false,
                );
                cpu_registers.set_flags(difference == 0, true, h_flag.into(), carry.into());
            }
            Self::CompareIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (difference, carry, h_flag) = sub(cpu_registers.accumulator, value, false);
                cpu_registers.set_flags(difference == 0, true, h_flag.into(), carry.into());
            }
            Self::CompareImmediate(n) => {
                let (difference, carry, h_flag) = sub(cpu_registers.accumulator, n, false);
                cpu_registers.set_flags(difference == 0, true, h_flag.into(), carry.into());
            }
            Self::IncRegister(r) => {
                let (sum, _, h_flag) = add(cpu_registers.read_register(r), 1, false);
                cpu_registers.set_register(r, sum);
                cpu_registers.set_some_flags(
                    Some(sum == 0),
                    Some(false),
                    Some(h_flag.into()),
                    None,
                );
            }
            Self::IncIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (sum, _, h_flag) = add(value, 1, false);
                address_space.write_address_u8(address, sum);
                cpu_registers.set_some_flags(
                    Some(sum == 0),
                    Some(false),
                    Some(h_flag.into()),
                    None,
                );
            }
            Self::DecRegister(r) => {
                let (difference, _, h_flag) = sub(cpu_registers.read_register(r), 1, false);
                cpu_registers.set_register(r, difference);
                cpu_registers.set_some_flags(
                    Some(difference == 0),
                    Some(true),
                    Some(h_flag.into()),
                    None,
                );
            }
            Self::DecIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (difference, _, h_flag) = sub(value, 1, false);
                address_space.write_address_u8(address, difference);
                cpu_registers.set_some_flags(
                    Some(difference == 0),
                    Some(true),
                    Some(h_flag.into()),
                    None,
                );
            }
            Self::AndRegister(r) => {
                let value = cpu_registers.accumulator & cpu_registers.read_register(r);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(value == 0, false, true, false);
            }
            Self::AndIndirectHL => {
                let value =
                    cpu_registers.accumulator & address_space.read_address_u8(cpu_registers.hl());
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(value == 0, false, true, false);
            }
            Self::AndImmediate(n) => {
                let value = cpu_registers.accumulator & n;
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(value == 0, false, true, false);
            }
            Self::OrRegister(r) => {
                let value = cpu_registers.accumulator | cpu_registers.read_register(r);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(value == 0, false, false, false);
            }
            Self::OrIndirectHL => {
                let value =
                    cpu_registers.accumulator | address_space.read_address_u8(cpu_registers.hl());
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(value == 0, false, false, false);
            }
            Self::OrImmediate(n) => {
                let value = cpu_registers.accumulator | n;
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(value == 0, false, false, false);
            }
            Self::XorRegister(r) => {
                let value = cpu_registers.accumulator ^ cpu_registers.read_register(r);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(value == 0, false, false, false);
            }
            Self::XorIndirectHL => {
                let value =
                    cpu_registers.accumulator ^ address_space.read_address_u8(cpu_registers.hl());
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(value == 0, false, false, false);
            }
            Self::XorImmediate(n) => {
                let value = cpu_registers.accumulator ^ n;
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(value == 0, false, false, false);
            }
            Self::AddHLRegister(rr) => {
                let (sum, carry, h_flag) =
                    add_u16(cpu_registers.hl(), cpu_registers.read_register_pair(rr));
                cpu_registers.set_hl(sum);
                cpu_registers.set_some_flags(
                    None,
                    Some(false),
                    Some(h_flag.into()),
                    Some(carry.into()),
                );
            }
            Self::IncRegisterPair(rr) => {
                cpu_registers
                    .set_register_pair(rr, cpu_registers.read_register_pair(rr).wrapping_add(1));
            }
            Self::DecRegisterPair(rr) => {
                cpu_registers
                    .set_register_pair(rr, cpu_registers.read_register_pair(rr).wrapping_sub(1));
            }
            Self::AddSPImmediate(e) => {
                let (sp, carry_flag, h_flag) = add_sp_offset(cpu_registers.sp, e);
                cpu_registers.sp = sp;
                cpu_registers.set_flags(false, false, h_flag.into(), carry_flag.into());
            }
            Self::LoadHLStackPointerOffset(e) => {
                let (sp, carry_flag, h_flag) = add_sp_offset(cpu_registers.sp, e);
                cpu_registers.set_hl(sp);
                cpu_registers.set_flags(false, false, h_flag.into(), carry_flag.into());
            }
            Self::RotateLeftAccumulator => {
                let (value, carry_flag) = rotate_left(cpu_registers.accumulator);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(false, false, false, carry_flag.into());
            }
            Self::RotateLeftAccumulatorThruCarry => {
                let (value, carry_flag) =
                    rotate_left_thru_carry(cpu_registers.accumulator, cpu_registers.carry_flag());
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(false, false, false, carry_flag.into());
            }
            Self::RotateRightAccumulator => {
                let (value, carry_flag) = rotate_right(cpu_registers.accumulator);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(false, false, false, carry_flag.into());
            }
            Self::RotateRightAccumulatorThruCarry => {
                let (value, carry_flag) =
                    rotate_right_thru_carry(cpu_registers.accumulator, cpu_registers.carry_flag());
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(false, false, false, carry_flag.into());
            }
            Self::RotateLeft(r) => {
                let (value, carry_flag) = rotate_left(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag.into());
            }
            Self::RotateLeftIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry_flag) = rotate_left(value);
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag.into());
            }
            Self::RotateLeftThruCarry(r) => {
                let (value, carry_flag) = rotate_left_thru_carry(
                    cpu_registers.read_register(r),
                    cpu_registers.carry_flag(),
                );
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag.into());
            }
            Self::RotateLeftIndirectHLThruCarry => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry_flag) = rotate_left_thru_carry(value, cpu_registers.carry_flag());
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag.into());
            }
            Self::RotateRight(r) => {
                let (value, carry_flag) = rotate_right(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag.into());
            }
            Self::RotateRightIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry_flag) = rotate_right(value);
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag.into());
            }
            Self::RotateRightThruCarry(r) => {
                let (value, carry_flag) = rotate_right_thru_carry(
                    cpu_registers.read_register(r),
                    cpu_registers.carry_flag(),
                );
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag.into());
            }
            Self::RotateRightIndirectHLThruCarry => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry_flag) =
                    rotate_right_thru_carry(value, cpu_registers.carry_flag());
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag.into());
            }
            Self::ShiftLeft(r) => {
                let (value, carry) = shift_left(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry.into());
            }
            Self::ShiftLeftIndirectHL => {
                let address = cpu_registers.hl();
                let (value, carry) = shift_left(address_space.read_address_u8(address));
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry.into());
            }
            Self::Swap(r) => {
                let register = cpu_registers.get_register_mut(r);
                *register = swap_bits(*register);
                let z_flag = *register == 0;
                cpu_registers.set_flags(z_flag, false, false, false);
            }
            Self::SwapIndirectHL => {
                let address = cpu_registers.hl();
                let value = swap_bits(address_space.read_address_u8(address));
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, false);
            }
            Self::ShiftRight(r) => {
                let (value, carry) = shift_right_arithmetic(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry.into());
            }
            Self::ShiftRightIndirectHL => {
                let address = cpu_registers.hl();
                let (value, carry) = shift_right_arithmetic(address_space.read_address_u8(address));
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry.into());
            }
            Self::ShiftRightLogical(r) => {
                let (value, carry) = shift_right_logical(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry.into());
            }
            Self::ShiftRightLogicalIndirectHL => {
                let address = cpu_registers.hl();
                let (value, carry) = shift_right_logical(address_space.read_address_u8(address));
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry.into());
            }
            Self::TestBit(n, r) => {
                let r_value = cpu_registers.read_register(r);
                let z_flag = r_value & (1 << n) == 0;
                cpu_registers.set_some_flags(Some(z_flag), Some(false), Some(true), None);
            }
            Self::TestBitIndirectHL(n) => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let z_flag = value & (1 << n) == 0;
                cpu_registers.set_some_flags(Some(z_flag), Some(false), Some(true), None);
            }
            Self::SetBit(n, r) => {
                let register = cpu_registers.get_register_mut(r);
                *register |= 1 << n;
            }
            Self::SetBitIndirectHL(n) => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address) | (1 << n);
                address_space.write_address_u8(address, value);
            }
            Self::ResetBit(n, r) => {
                let register = cpu_registers.get_register_mut(r);
                *register &= !(1 << n);
            }
            Self::ResetBitIndirectHL(n) => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address) & !(1 << n);
                address_space.write_address_u8(address, value);
            }
            Self::ComplementCarryFlag => {
                cpu_registers.set_some_flags(
                    None,
                    Some(false),
                    Some(false),
                    Some(!cpu_registers.carry_flag()),
                );
            }
            Self::SetCarryFlag => {
                cpu_registers.set_some_flags(None, Some(false), Some(false), Some(true));
            }
            Self::DecimalAdjustAccumulator => {
                decimal_adjust_accumulator(cpu_registers);
            }
            Self::ComplementAccumulator => {
                cpu_registers.accumulator = !cpu_registers.accumulator;
                cpu_registers.set_some_flags(None, Some(true), Some(true), None);
            }
            Self::Jump(nn) => {
                cpu_registers.pc = nn;
            }
            Self::JumpHL => {
                cpu_registers.pc = cpu_registers.hl();
            }
            Self::JumpCond(cc, nn) => {
                if cc.check(cpu_registers) {
                    cpu_registers.pc = nn;
                }
            }
            Self::RelativeJump(e) => {
                let pc = ((cpu_registers.pc as i32) + (e as i32)).try_into()?;
                cpu_registers.pc = pc;
            }
            Self::RelativeJumpCond(cc, e) => {
                if cc.check(cpu_registers) {
                    let pc = ((cpu_registers.pc as i32) + (e as i32)).try_into()?;
                    cpu_registers.pc = pc;
                }
            }
            Self::Call(nn) => {
                cpu_registers.sp -= 2;
                address_space.write_address_u16(cpu_registers.sp, cpu_registers.pc);
                cpu_registers.pc = nn;
            }
            Self::CallCond(cc, nn) => {
                if cc.check(cpu_registers) {
                    cpu_registers.sp -= 2;
                    address_space.write_address_u16(cpu_registers.sp, cpu_registers.pc);
                    cpu_registers.pc = nn;
                }
            }
            Self::Return => {
                cpu_registers.pc = address_space.read_address_u16(cpu_registers.sp);
                cpu_registers.sp += 2;
            }
            Self::ReturnCond(cc) => {
                if cc.check(cpu_registers) {
                    cpu_registers.pc = address_space.read_address_u16(cpu_registers.sp);
                    cpu_registers.sp += 2;
                }
            }
            Self::ReturnFromInterruptHandler => {
                cpu_registers.pc = address_space.read_address_u16(cpu_registers.sp);
                cpu_registers.sp += 2;
                cpu_registers.ime = true;
            }
            Self::RestartCall(rst_address) => {
                cpu_registers.sp -= 2;
                address_space.write_address_u16(cpu_registers.sp, cpu_registers.pc);
                cpu_registers.pc = rst_address.into();
            }
            Self::HaltClock => {
                todo!("HALT is not implemented")
            }
            Self::StopClocks => {
                todo!("STOP is not implemented")
            }
            Self::DisableInterrupts => {
                cpu_registers.ime = false;
            }
            Self::EnableInterrupts => {
                cpu_registers.ime = true;

                cpu_registers.interrupt_delay = true;
                // Return early because this is the only instruction that should not unset interrupt
                // delay
                return Ok(());
            }
            Self::NoOp => {}
        }

        cpu_registers.interrupt_delay = false;

        Ok(())
    }

    pub fn cycles_required(self, cpu_registers: &CpuRegisters) -> u32 {
        match self {
            Self::LoadRegisterRegister(..)
            | Self::AddRegister(..)
            | Self::AddCarryRegister(..)
            | Self::SubtractRegister(..)
            | Self::SubtractCarryRegister(..)
            | Self::AndRegister(..)
            | Self::OrRegister(..)
            | Self::XorRegister(..)
            | Self::CompareRegister(..)
            | Self::IncRegister(..)
            | Self::DecRegister(..)
            | Self::DecimalAdjustAccumulator
            | Self::ComplementAccumulator
            | Self::RotateLeftAccumulator
            | Self::RotateLeftAccumulatorThruCarry
            | Self::RotateRightAccumulator
            | Self::RotateRightAccumulatorThruCarry
            | Self::SetCarryFlag
            | Self::ComplementCarryFlag
            | Self::NoOp
            | Self::DisableInterrupts
            | Self::EnableInterrupts
            | Self::JumpHL => 4,
            Self::LoadRegisterImmediate(..)
            | Self::LoadRegisterIndirectHL(..)
            | Self::LoadIndirectHLRegister(..)
            | Self::LoadAccumulatorIndirectBC
            | Self::LoadAccumulatorIndirectDE
            | Self::LoadIndirectBCAccumulator
            | Self::LoadIndirectDEAccumulator
            | Self::LoadAccumulatorIndirectC
            | Self::LoadIndirectCAccumulator
            | Self::LoadAccumulatorIndirectHLInc
            | Self::LoadAccumulatorIndirectHLDec
            | Self::LoadIndirectHLIncAccumulator
            | Self::LoadIndirectHLDecAccumulator
            | Self::LoadStackPointerHL
            | Self::AddImmediate(..)
            | Self::AddIndirectHL
            | Self::AddCarryImmediate(..)
            | Self::AddCarryIndirectHL
            | Self::SubtractImmediate(..)
            | Self::SubtractIndirectHL
            | Self::SubtractCarryImmediate(..)
            | Self::SubtractCarryIndirectHL
            | Self::AndImmediate(..)
            | Self::AndIndirectHL
            | Self::OrImmediate(..)
            | Self::OrIndirectHL
            | Self::XorImmediate(..)
            | Self::XorIndirectHL
            | Self::CompareImmediate(..)
            | Self::CompareIndirectHL
            | Self::AddHLRegister(..)
            | Self::IncRegisterPair(..)
            | Self::DecRegisterPair(..)
            | Self::RotateLeft(..)
            | Self::RotateLeftThruCarry(..)
            | Self::RotateRight(..)
            | Self::RotateRightThruCarry(..)
            | Self::ShiftLeft(..)
            | Self::ShiftRight(..)
            | Self::ShiftRightLogical(..)
            | Self::Swap(..)
            | Self::TestBit(..)
            | Self::SetBit(..)
            | Self::ResetBit(..) => 8,
            Self::LoadIndirectHLImmediate(..)
            | Self::LoadAccumulatorDirect8(..)
            | Self::LoadDirect8Accumulator(..)
            | Self::LoadRegisterPairImmediate(..)
            | Self::PopStack(..)
            | Self::IncIndirectHL
            | Self::DecIndirectHL
            | Self::LoadHLStackPointerOffset(..)
            | Self::TestBitIndirectHL(..)
            | Self::RelativeJump(..) => 12,
            Self::LoadAccumulatorDirect16(..)
            | Self::LoadDirect16Accumulator(..)
            | Self::PushStack(..)
            | Self::AddSPImmediate(..)
            | Self::RotateLeftIndirectHL
            | Self::RotateLeftIndirectHLThruCarry
            | Self::RotateRightIndirectHL
            | Self::RotateRightIndirectHLThruCarry
            | Self::ShiftLeftIndirectHL
            | Self::ShiftRightIndirectHL
            | Self::ShiftRightLogicalIndirectHL
            | Self::SwapIndirectHL
            | Self::SetBitIndirectHL(..)
            | Self::ResetBitIndirectHL(..)
            | Self::Jump(..)
            | Self::Return
            | Self::ReturnFromInterruptHandler
            | Self::RestartCall(..) => 16,
            Self::LoadDirectStackPointer(..) => 20,
            Self::Call(..) => 24,
            Self::JumpCond(cc, ..) => {
                if cc.check(cpu_registers) {
                    16
                } else {
                    12
                }
            }
            Self::RelativeJumpCond(cc, ..) => {
                if cc.check(cpu_registers) {
                    12
                } else {
                    8
                }
            }
            Self::CallCond(cc, ..) => {
                if cc.check(cpu_registers) {
                    24
                } else {
                    12
                }
            }
            Self::ReturnCond(cc) => {
                if cc.check(cpu_registers) {
                    20
                } else {
                    8
                }
            }
            Self::HaltClock => todo!("HALT is not implemented"),
            Self::StopClocks => todo!("STOP is not implemented"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CarryFlag(bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HalfCarryFlag(bool);

impl From<CarryFlag> for bool {
    fn from(value: CarryFlag) -> Self {
        value.0
    }
}

impl From<HalfCarryFlag> for bool {
    fn from(value: HalfCarryFlag) -> Self {
        value.0
    }
}

fn add(l_value: u8, r_value: u8, carry: bool) -> (u8, CarryFlag, HalfCarryFlag) {
    let carry = u8::from(carry);
    let (sum, carry_flag) = match l_value.overflowing_add(r_value) {
        (sum, true) => (sum + carry, true),
        (sum, false) => sum.overflowing_add(carry),
    };
    let h_flag = (l_value & 0x0F) + (r_value & 0x0F) + carry >= 0x10;

    (sum, CarryFlag(carry_flag), HalfCarryFlag(h_flag))
}

fn add_u16(l_value: u16, r_value: u16) -> (u16, CarryFlag, HalfCarryFlag) {
    let (sum, carry_flag) = l_value.overflowing_add(r_value);
    let h_flag = (l_value & 0x0FFF) + (r_value & 0x0FFF) >= 0x1000;

    (sum, CarryFlag(carry_flag), HalfCarryFlag(h_flag))
}

fn sub(l_value: u8, r_value: u8, carry: bool) -> (u8, CarryFlag, HalfCarryFlag) {
    let carry = u8::from(carry);
    let (difference, carry_flag) = match l_value.overflowing_sub(r_value) {
        (difference, true) => (difference - carry, true),
        (difference, false) => difference.overflowing_sub(carry),
    };
    let h_flag = l_value & 0x0F < (r_value & 0x0F) + carry;

    (difference, CarryFlag(carry_flag), HalfCarryFlag(h_flag))
}

fn rotate_left(value: u8) -> (u8, CarryFlag) {
    let leftmost_set = value & 0x80 != 0;
    let new_value = (value << 1) | u8::from(leftmost_set);

    (new_value, CarryFlag(leftmost_set))
}

fn rotate_left_thru_carry(value: u8, carry: bool) -> (u8, CarryFlag) {
    let leftmost_set = value & 0x80 != 0;
    let new_value = (value << 1) | u8::from(carry);

    (new_value, CarryFlag(leftmost_set))
}

fn rotate_right(value: u8) -> (u8, CarryFlag) {
    let rightmost_set = value & 0x01 != 0;
    let new_value = (value >> 1) | (u8::from(rightmost_set) << 7);

    (new_value, CarryFlag(rightmost_set))
}

fn rotate_right_thru_carry(value: u8, carry: bool) -> (u8, CarryFlag) {
    let rightmost_set = value & 0x01 != 0;
    let new_value = (value >> 1) | (u8::from(carry) << 7);

    (new_value, CarryFlag(rightmost_set))
}

fn shift_left(value: u8) -> (u8, CarryFlag) {
    (value << 1, CarryFlag(value & 0x80 != 0))
}

fn shift_right_arithmetic(value: u8) -> (u8, CarryFlag) {
    ((value >> 1) | (value & 0x80), CarryFlag(value & 0x01 != 0))
}

fn shift_right_logical(value: u8) -> (u8, CarryFlag) {
    (value >> 1, CarryFlag(value & 0x01 != 0))
}

fn swap_bits(value: u8) -> u8 {
    (value >> 4) | (value << 4)
}

fn decimal_adjust_accumulator(cpu_registers: &mut CpuRegisters) {
    if cpu_registers.n_flag() {
        // Last op was subtraction
        let mut value = cpu_registers.accumulator;
        if cpu_registers.half_carry_flag() {
            value = value.wrapping_sub(0x06);
        }
        if cpu_registers.carry_flag() {
            value = value.wrapping_sub(0x60);
        }

        cpu_registers.accumulator = value;
        cpu_registers.set_some_flags(Some(value == 0), None, Some(false), None);
    } else {
        // Last op was addition
        let mut value = u16::from(cpu_registers.accumulator);
        if value & 0x0F >= 0x0A || cpu_registers.half_carry_flag() {
            value += 0x06;
        }
        if value & 0xF0 >= 0xA0 || cpu_registers.carry_flag() {
            value += 0x60;
        }

        let carry_flag = value > 0xFF;
        let value = value as u8;
        cpu_registers.accumulator = value;
        cpu_registers.set_some_flags(Some(value == 0), None, Some(false), Some(carry_flag));
    }
}

fn add_sp_offset(sp: u16, offset: i8) -> (u16, CarryFlag, HalfCarryFlag) {
    if offset >= 0 {
        add_u16(sp, offset as u16)
    } else {
        let sp = sp as i32;
        let offset = -(offset as i32);

        let h_flag = offset > sp & 0x0FFF;

        let new_sp = sp - offset;
        if new_sp >= 0x0000 {
            (new_sp as u16, CarryFlag(false), HalfCarryFlag(h_flag))
        } else {
            (
                (new_sp + 0x10000) as u16,
                CarryFlag(true),
                HalfCarryFlag(h_flag),
            )
        }
    }
}
