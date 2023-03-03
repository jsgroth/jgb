mod parse;

use crate::cpu::registers::{CpuRegister, CpuRegisterPair, CpuRegisters};
use crate::memory::AddressSpace;
use lazy_static::lazy_static;
use std::num::TryFromIntError;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JumpCondition {
    NZ,
    Z,
    NC,
    C,
}

impl JumpCondition {
    fn from_opcode_bits(bits: u8) -> Self {
        match bits & 0x03 {
            0x00 => Self::NZ,
            0x01 => Self::Z,
            0x02 => Self::NC,
            0x03 => Self::C,
            _ => panic!("{bits} & 0x03 was not in range [0x00, 0x03]"),
        }
    }

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
    RegisterOverflowError {
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
    fn execute(
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
                cpu_registers.set_flags(sum == 0, false, h_flag, carry);
            }
            Self::AddIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (sum, carry, h_flag) = add(cpu_registers.accumulator, value, false);
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag, carry);
            }
            Self::AddImmediate(n) => {
                let (sum, carry, h_flag) = add(cpu_registers.accumulator, n, false);
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag, carry);
            }
            Self::AddCarryRegister(r) => {
                let (sum, carry, h_flag) = add(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    cpu_registers.carry_flag(),
                );
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag, carry);
            }
            Self::AddCarryIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (sum, carry, h_flag) =
                    add(cpu_registers.accumulator, value, cpu_registers.carry_flag());
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag, carry);
            }
            Self::AddCarryImmediate(n) => {
                let (sum, carry, h_flag) =
                    add(cpu_registers.accumulator, n, cpu_registers.carry_flag());
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(sum == 0, false, h_flag, carry);
            }
            Self::SubtractRegister(r) => {
                let (difference, carry, h_flag) = sub(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    false,
                );
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag, carry);
            }
            Self::SubtractIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (difference, carry, h_flag) = sub(cpu_registers.accumulator, value, false);
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag, carry);
            }
            Self::SubtractImmediate(n) => {
                let (difference, carry, h_flag) = sub(cpu_registers.accumulator, n, false);
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag, carry);
            }
            Self::SubtractCarryRegister(r) => {
                let (difference, carry, h_flag) = sub(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    cpu_registers.carry_flag(),
                );
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag, carry);
            }
            Self::SubtractCarryIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (difference, carry, h_flag) =
                    sub(cpu_registers.accumulator, value, cpu_registers.carry_flag());
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag, carry);
            }
            Self::SubtractCarryImmediate(n) => {
                let (difference, carry, h_flag) =
                    sub(cpu_registers.accumulator, n, cpu_registers.carry_flag());
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(difference == 0, true, h_flag, carry);
            }
            Self::CompareRegister(r) => {
                let (difference, carry, h_flag) = sub(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    false,
                );
                cpu_registers.set_flags(difference == 0, true, h_flag, carry);
            }
            Self::CompareIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let (difference, carry, h_flag) = sub(cpu_registers.accumulator, value, false);
                cpu_registers.set_flags(difference == 0, true, h_flag, carry);
            }
            Self::CompareImmediate(n) => {
                let (difference, carry, h_flag) = sub(cpu_registers.accumulator, n, false);
                cpu_registers.set_flags(difference == 0, true, h_flag, carry);
            }
            Self::IncRegister(r) => {
                let (sum, _, h_flag) = add(cpu_registers.read_register(r), 1, false);
                cpu_registers.set_register(r, sum);
                cpu_registers.set_some_flags(Some(sum == 0), Some(false), Some(h_flag), None);
            }
            Self::IncIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (sum, _, h_flag) = add(value, 1, false);
                address_space.write_address_u8(address, value);
                cpu_registers.set_some_flags(Some(sum == 0), Some(false), Some(h_flag), None);
            }
            Self::DecRegister(r) => {
                let (difference, _, h_flag) = sub(cpu_registers.read_register(r), 1, false);
                cpu_registers.set_register(r, difference);
                cpu_registers.set_some_flags(Some(difference == 0), Some(true), Some(h_flag), None);
            }
            Self::DecIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (difference, _, h_flag) = sub(value, 1, false);
                address_space.write_address_u8(address, value);
                cpu_registers.set_some_flags(Some(difference == 0), Some(true), Some(h_flag), None);
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
                cpu_registers.set_some_flags(None, Some(false), Some(h_flag), Some(carry));
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
                cpu_registers.sp = ((cpu_registers.sp as i32) + (e as i32)).try_into()?;
            }
            Self::LoadHLStackPointerOffset(e) => {
                let hl = ((cpu_registers.sp as i32) + (e as i32)).try_into()?;
                cpu_registers.set_hl(hl);
            }
            Self::RotateLeftAccumulator => {
                let (value, carry_flag) = rotate_left(cpu_registers.accumulator);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(false, false, false, carry_flag);
            }
            Self::RotateLeftAccumulatorThruCarry => {
                let (value, carry_flag) =
                    rotate_left_thru_carry(cpu_registers.accumulator, cpu_registers.carry_flag());
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(false, false, false, carry_flag);
            }
            Self::RotateRightAccumulator => {
                let (value, carry_flag) = rotate_right(cpu_registers.accumulator);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(false, false, false, carry_flag);
            }
            Self::RotateRightAccumulatorThruCarry => {
                let (value, carry_flag) =
                    rotate_right_thru_carry(cpu_registers.accumulator, cpu_registers.carry_flag());
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(false, false, false, carry_flag);
            }
            Self::RotateLeft(r) => {
                let (value, carry_flag) = rotate_left(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag);
            }
            Self::RotateLeftIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry_flag) = rotate_left(value);
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag);
            }
            Self::RotateLeftThruCarry(r) => {
                let (value, carry_flag) = rotate_left_thru_carry(
                    cpu_registers.read_register(r),
                    cpu_registers.carry_flag(),
                );
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag);
            }
            Self::RotateLeftIndirectHLThruCarry => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry_flag) = rotate_left_thru_carry(value, cpu_registers.carry_flag());
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag);
            }
            Self::RotateRight(r) => {
                let (value, carry_flag) = rotate_right(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag);
            }
            Self::RotateRightIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry_flag) = rotate_right(value);
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag);
            }
            Self::RotateRightThruCarry(r) => {
                let (value, carry_flag) = rotate_right_thru_carry(
                    cpu_registers.read_register(r),
                    cpu_registers.carry_flag(),
                );
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag);
            }
            Self::RotateRightIndirectHLThruCarry => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry_flag) =
                    rotate_right_thru_carry(value, cpu_registers.carry_flag());
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry_flag);
            }
            Self::ShiftLeft(r) => {
                let (value, carry) = cpu_registers.read_register(r).overflowing_shl(1);
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry);
            }
            Self::ShiftLeftIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry) = value.overflowing_shl(1);
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry);
            }
            Self::Swap(r) => {
                let register = cpu_registers.get_register_mut(r);
                *register = register.swap_bytes();
                let z_flag = *register == 0;
                cpu_registers.set_flags(z_flag, false, false, false);
            }
            Self::SwapIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address).swap_bytes();
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, false);
            }
            Self::ShiftRight(r) => {
                let old_value = cpu_registers.read_register(r);
                let (mut value, carry) = old_value.overflowing_shr(1);
                value |= old_value & 0x80;
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry);
            }
            Self::ShiftRightIndirectHL => {
                let address = cpu_registers.hl();
                let old_value = address_space.read_address_u8(address);
                let (mut value, carry) = old_value.overflowing_shr(1);
                value |= old_value & 0x80;
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry);
            }
            Self::ShiftRightLogical(r) => {
                let (value, carry) = cpu_registers.read_register(r).overflowing_shr(1);
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(value == 0, false, false, carry);
            }
            Self::ShiftRightLogicalIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address);
                let (value, carry) = value.overflowing_shr(1);
                address_space.write_address_u8(address, value);
                cpu_registers.set_flags(value == 0, false, false, carry);
            }
            Self::TestBit(n, r) => {
                let r_value = cpu_registers.read_register(r);
                let z_flag = r_value & (1 << n) != 0;
                cpu_registers.set_some_flags(Some(z_flag), Some(false), Some(true), None);
            }
            Self::TestBitIndirectHL(n) => {
                let value = address_space.read_address_u8(cpu_registers.hl());
                let z_flag = value & (1 << n) != 0;
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
                todo!()
            }
            Self::RestartCall(n) => {
                cpu_registers.sp -= 2;
                address_space.write_address_u16(cpu_registers.sp, cpu_registers.pc);
                cpu_registers.pc = (n << 3) as u16;
            }
            Self::HaltClock => {
                todo!()
            }
            Self::StopClocks => {
                todo!()
            }
            Self::DisableInterrupts => {
                todo!()
            }
            Self::EnableInterrupts => {
                todo!()
            }
            Self::NoOp => {}
        }

        Ok(())
    }
}

fn add(l_value: u8, r_value: u8, carry: bool) -> (u8, bool, bool) {
    let carry = u8::from(carry);
    let (sum, carry_flag) = match l_value.overflowing_add(r_value) {
        (sum, true) => (sum + carry, true),
        (sum, false) => sum.overflowing_add(carry),
    };
    let h_flag = (l_value & 0x0F) + (r_value & 0x0F) >= 0x10;

    (sum, carry_flag, h_flag)
}

fn add_u16(l_value: u16, r_value: u16) -> (u16, bool, bool) {
    let (sum, carry_flag) = l_value.overflowing_add(r_value);
    let h_flag = (l_value & 0x0FFF) + (r_value & 0x0FFF) >= 0x1000;

    (sum, carry_flag, h_flag)
}

fn sub(l_value: u8, r_value: u8, carry: bool) -> (u8, bool, bool) {
    let carry = u8::from(carry);
    let (difference, carry_flag) = match l_value.overflowing_sub(r_value) {
        (difference, true) => (difference - carry, true),
        (difference, false) => difference.overflowing_sub(carry),
    };
    let h_flag = l_value & 0x0F < r_value & 0x0F;

    (difference, carry_flag, h_flag)
}

fn rotate_left(value: u8) -> (u8, bool) {
    let leftmost_set = value & 0x80 != 0;
    let new_value = (value << 1) | u8::from(leftmost_set);

    (new_value, leftmost_set)
}

fn rotate_left_thru_carry(value: u8, carry: bool) -> (u8, bool) {
    let leftmost_set = value & 0x80 != 0;
    let new_value = (value << 1) | u8::from(carry);

    (new_value, leftmost_set)
}

fn rotate_right(value: u8) -> (u8, bool) {
    let rightmost_set = value & 0x01 != 0;
    let new_value = (value >> 1) | (u8::from(rightmost_set) << 7);

    (new_value, rightmost_set)
}

fn rotate_right_thru_carry(value: u8, carry: bool) -> (u8, bool) {
    let rightmost_set = value & 0x01 != 0;
    let new_value = (value >> 1) | (u8::from(carry) << 7);

    (new_value, rightmost_set)
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
