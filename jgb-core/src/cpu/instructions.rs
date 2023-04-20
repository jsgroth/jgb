mod parse;

use crate::cpu::registers::{
    CFlag, CpuRegister, CpuRegisterPair, CpuRegisters, HFlag, NFlag, ZFlag,
};
use crate::memory::AddressSpace;
use std::num::TryFromIntError;
use thiserror::Error;

use crate::cpu::{CgbSpeedMode, ExecutionMode};
use crate::memory::ioregisters::IoRegister;
use crate::ppu::PpuState;
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
            Self::NZ => !cpu_registers.z_flag(),
            Self::Z => cpu_registers.z_flag(),
            Self::NC => !cpu_registers.c_flag(),
            Self::C => cpu_registers.c_flag(),
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
    Halt,
    // STOP
    Stop,
    // DI
    DisableInterrupts,
    // EI
    EnableInterrupts,
    // NOP
    NoOp,
}

impl Instruction {
    /// Execute the given CPU instruction, modifying CPU registers/flags and memory as needed.
    ///
    /// This method does *not* update the PC register for the given instruction. It expects that the
    /// PC register was updated before this method was called.
    pub fn execute(
        self,
        address_space: &mut AddressSpace,
        cpu_registers: &mut CpuRegisters,
        ppu_state: &PpuState,
    ) -> Result<(), ExecutionError> {
        match self {
            Self::LoadRegisterRegister(r, r_p) => {
                cpu_registers.set_register(r, cpu_registers.read_register(r_p));
            }
            Self::LoadRegisterImmediate(r, n) => {
                cpu_registers.set_register(r, n);
            }
            Self::LoadRegisterIndirectHL(r) => {
                let value = address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                cpu_registers.set_register(r, value);
            }
            Self::LoadIndirectHLRegister(r) => {
                let value = cpu_registers.read_register(r);
                address_space.write_address_u8(cpu_registers.hl(), value, ppu_state);
            }
            Self::LoadIndirectHLImmediate(n) => {
                address_space.write_address_u8(cpu_registers.hl(), n, ppu_state);
            }
            Self::LoadAccumulatorIndirectBC => {
                cpu_registers.accumulator =
                    address_space.read_address_u8(cpu_registers.bc(), ppu_state);
            }
            Self::LoadAccumulatorIndirectDE => {
                cpu_registers.accumulator =
                    address_space.read_address_u8(cpu_registers.de(), ppu_state);
            }
            Self::LoadIndirectBCAccumulator => {
                address_space.write_address_u8(
                    cpu_registers.bc(),
                    cpu_registers.accumulator,
                    ppu_state,
                );
            }
            Self::LoadIndirectDEAccumulator => {
                address_space.write_address_u8(
                    cpu_registers.de(),
                    cpu_registers.accumulator,
                    ppu_state,
                );
            }
            Self::LoadAccumulatorDirect16(nn) => {
                cpu_registers.accumulator = address_space.read_address_u8(nn, ppu_state);
            }
            Self::LoadDirect16Accumulator(nn) => {
                address_space.write_address_u8(nn, cpu_registers.accumulator, ppu_state);
            }
            Self::LoadAccumulatorIndirectC => {
                let address = u16::from_be_bytes([0xFF, cpu_registers.c]);
                cpu_registers.accumulator = address_space.read_address_u8(address, ppu_state);
            }
            Self::LoadIndirectCAccumulator => {
                let address = u16::from_be_bytes([0xFF, cpu_registers.c]);
                address_space.write_address_u8(address, cpu_registers.accumulator, ppu_state);
            }
            Self::LoadAccumulatorDirect8(n) => {
                let address = u16::from_be_bytes([0xFF, n]);
                cpu_registers.accumulator = address_space.read_address_u8(address, ppu_state);
            }
            Self::LoadDirect8Accumulator(n) => {
                let address = u16::from_be_bytes([0xFF, n]);
                address_space.write_address_u8(address, cpu_registers.accumulator, ppu_state);
            }
            Self::LoadAccumulatorIndirectHLDec => {
                let hl = cpu_registers.hl();
                cpu_registers.accumulator = address_space.read_address_u8(hl, ppu_state);
                cpu_registers.set_hl(hl.wrapping_sub(1));
            }
            Self::LoadIndirectHLDecAccumulator => {
                let hl = cpu_registers.hl();
                address_space.write_address_u8(hl, cpu_registers.accumulator, ppu_state);
                cpu_registers.set_hl(hl.wrapping_sub(1));
            }
            Self::LoadAccumulatorIndirectHLInc => {
                let hl = cpu_registers.hl();
                cpu_registers.accumulator = address_space.read_address_u8(hl, ppu_state);
                cpu_registers.set_hl(hl.wrapping_add(1));
            }
            Self::LoadIndirectHLIncAccumulator => {
                let hl = cpu_registers.hl();
                address_space.write_address_u8(hl, cpu_registers.accumulator, ppu_state);
                cpu_registers.set_hl(hl.wrapping_add(1));
            }
            Self::LoadRegisterPairImmediate(rr, nn) => {
                cpu_registers.set_register_pair(rr, nn);
            }
            Self::LoadDirectStackPointer(nn) => {
                address_space.write_address_u16(nn, cpu_registers.sp, ppu_state);
            }
            Self::LoadStackPointerHL => {
                cpu_registers.sp = cpu_registers.hl();
            }
            Self::PushStack(rr) => {
                cpu_registers.sp -= 2;
                address_space.write_address_u16(
                    cpu_registers.sp,
                    cpu_registers.read_register_pair(rr),
                    ppu_state,
                );
            }
            Self::PopStack(rr) => {
                cpu_registers.set_register_pair(
                    rr,
                    address_space.read_address_u16(cpu_registers.sp, ppu_state),
                );
                cpu_registers.sp += 2;
            }
            Self::AddRegister(r) => {
                let (sum, c_flag, h_flag) = add(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    false,
                );
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(ZFlag(sum == 0), NFlag(false), h_flag, c_flag);
            }
            Self::AddIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                let (sum, c_flag, h_flag) = add(cpu_registers.accumulator, value, false);
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(ZFlag(sum == 0), NFlag(false), h_flag, c_flag);
            }
            Self::AddImmediate(n) => {
                let (sum, c_flag, h_flag) = add(cpu_registers.accumulator, n, false);
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(ZFlag(sum == 0), NFlag(false), h_flag, c_flag);
            }
            Self::AddCarryRegister(r) => {
                let (sum, c_flag, h_flag) = add(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    cpu_registers.c_flag(),
                );
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(ZFlag(sum == 0), NFlag(false), h_flag, c_flag);
            }
            Self::AddCarryIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                let (sum, c_flag, h_flag) =
                    add(cpu_registers.accumulator, value, cpu_registers.c_flag());
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(ZFlag(sum == 0), NFlag(false), h_flag, c_flag);
            }
            Self::AddCarryImmediate(n) => {
                let (sum, c_flag, h_flag) =
                    add(cpu_registers.accumulator, n, cpu_registers.c_flag());
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(ZFlag(sum == 0), NFlag(false), h_flag, c_flag);
            }
            Self::SubtractRegister(r) => {
                let (difference, c_flag, h_flag) = sub(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    false,
                );
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::SubtractIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                let (difference, c_flag, h_flag) = sub(cpu_registers.accumulator, value, false);
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::SubtractImmediate(n) => {
                let (difference, c_flag, h_flag) = sub(cpu_registers.accumulator, n, false);
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::SubtractCarryRegister(r) => {
                let (difference, c_flag, h_flag) = sub(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    cpu_registers.c_flag(),
                );
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::SubtractCarryIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                let (difference, c_flag, h_flag) =
                    sub(cpu_registers.accumulator, value, cpu_registers.c_flag());
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::SubtractCarryImmediate(n) => {
                let (difference, c_flag, h_flag) =
                    sub(cpu_registers.accumulator, n, cpu_registers.c_flag());
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::CompareRegister(r) => {
                let (difference, c_flag, h_flag) = sub(
                    cpu_registers.accumulator,
                    cpu_registers.read_register(r),
                    false,
                );
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::CompareIndirectHL => {
                let value = address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                let (difference, c_flag, h_flag) = sub(cpu_registers.accumulator, value, false);
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::CompareImmediate(n) => {
                let (difference, c_flag, h_flag) = sub(cpu_registers.accumulator, n, false);
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::IncRegister(r) => {
                let (sum, _, h_flag) = add(cpu_registers.read_register(r), 1, false);
                cpu_registers.set_register(r, sum);
                cpu_registers.set_some_flags(
                    Some(ZFlag(sum == 0)),
                    Some(NFlag(false)),
                    Some(h_flag),
                    None,
                );
            }
            Self::IncIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address, ppu_state);
                let (sum, _, h_flag) = add(value, 1, false);
                address_space.write_address_u8(address, sum, ppu_state);
                cpu_registers.set_some_flags(
                    Some(ZFlag(sum == 0)),
                    Some(NFlag(false)),
                    Some(h_flag),
                    None,
                );
            }
            Self::DecRegister(r) => {
                let (difference, _, h_flag) = sub(cpu_registers.read_register(r), 1, false);
                cpu_registers.set_register(r, difference);
                cpu_registers.set_some_flags(
                    Some(ZFlag(difference == 0)),
                    Some(NFlag(true)),
                    Some(h_flag),
                    None,
                );
            }
            Self::DecIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address, ppu_state);
                let (difference, _, h_flag) = sub(value, 1, false);
                address_space.write_address_u8(address, difference, ppu_state);
                cpu_registers.set_some_flags(
                    Some(ZFlag(difference == 0)),
                    Some(NFlag(true)),
                    Some(h_flag),
                    None,
                );
            }
            Self::AndRegister(r) => {
                let value = cpu_registers.accumulator & cpu_registers.read_register(r);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(true), CFlag(false));
            }
            Self::AndIndirectHL => {
                let value = cpu_registers.accumulator
                    & address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(true), CFlag(false));
            }
            Self::AndImmediate(n) => {
                let value = cpu_registers.accumulator & n;
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(true), CFlag(false));
            }
            Self::OrRegister(r) => {
                let value = cpu_registers.accumulator | cpu_registers.read_register(r);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(
                    ZFlag(value == 0),
                    NFlag(false),
                    HFlag(false),
                    CFlag(false),
                );
            }
            Self::OrIndirectHL => {
                let value = cpu_registers.accumulator
                    | address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(
                    ZFlag(value == 0),
                    NFlag(false),
                    HFlag(false),
                    CFlag(false),
                );
            }
            Self::OrImmediate(n) => {
                let value = cpu_registers.accumulator | n;
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(
                    ZFlag(value == 0),
                    NFlag(false),
                    HFlag(false),
                    CFlag(false),
                );
            }
            Self::XorRegister(r) => {
                let value = cpu_registers.accumulator ^ cpu_registers.read_register(r);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(
                    ZFlag(value == 0),
                    NFlag(false),
                    HFlag(false),
                    CFlag(false),
                );
            }
            Self::XorIndirectHL => {
                let value = cpu_registers.accumulator
                    ^ address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(
                    ZFlag(value == 0),
                    NFlag(false),
                    HFlag(false),
                    CFlag(false),
                );
            }
            Self::XorImmediate(n) => {
                let value = cpu_registers.accumulator ^ n;
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(
                    ZFlag(value == 0),
                    NFlag(false),
                    HFlag(false),
                    CFlag(false),
                );
            }
            Self::AddHLRegister(rr) => {
                let (sum, c_flag, h_flag) =
                    add_u16(cpu_registers.hl(), cpu_registers.read_register_pair(rr));
                cpu_registers.set_hl(sum);
                cpu_registers.set_some_flags(None, Some(NFlag(false)), Some(h_flag), Some(c_flag));
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
                let (sp, c_flag, h_flag) = add_sp_offset(cpu_registers.sp, e);
                cpu_registers.sp = sp;
                cpu_registers.set_flags(ZFlag(false), NFlag(false), h_flag, c_flag);
            }
            Self::LoadHLStackPointerOffset(e) => {
                let (sp, c_flag, h_flag) = add_sp_offset(cpu_registers.sp, e);
                cpu_registers.set_hl(sp);
                cpu_registers.set_flags(ZFlag(false), NFlag(false), h_flag, c_flag);
            }
            Self::RotateLeftAccumulator => {
                let (value, c_flag) = rotate_left(cpu_registers.accumulator);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(ZFlag(false), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateLeftAccumulatorThruCarry => {
                let (value, c_flag) =
                    rotate_left_thru_carry(cpu_registers.accumulator, cpu_registers.c_flag());
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(ZFlag(false), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateRightAccumulator => {
                let (value, c_flag) = rotate_right(cpu_registers.accumulator);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(ZFlag(false), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateRightAccumulatorThruCarry => {
                let (value, c_flag) =
                    rotate_right_thru_carry(cpu_registers.accumulator, cpu_registers.c_flag());
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(ZFlag(false), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateLeft(r) => {
                let (value, c_flag) = rotate_left(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateLeftIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address, ppu_state);
                let (value, c_flag) = rotate_left(value);
                address_space.write_address_u8(address, value, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateLeftThruCarry(r) => {
                let (value, c_flag) =
                    rotate_left_thru_carry(cpu_registers.read_register(r), cpu_registers.c_flag());
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateLeftIndirectHLThruCarry => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address, ppu_state);
                let (value, c_flag) = rotate_left_thru_carry(value, cpu_registers.c_flag());
                address_space.write_address_u8(address, value, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateRight(r) => {
                let (value, c_flag) = rotate_right(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateRightIndirectHL => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address, ppu_state);
                let (value, c_flag) = rotate_right(value);
                address_space.write_address_u8(address, value, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateRightThruCarry(r) => {
                let (value, c_flag) =
                    rotate_right_thru_carry(cpu_registers.read_register(r), cpu_registers.c_flag());
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateRightIndirectHLThruCarry => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address, ppu_state);
                let (value, c_flag) = rotate_right_thru_carry(value, cpu_registers.c_flag());
                address_space.write_address_u8(address, value, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::ShiftLeft(r) => {
                let (value, c_flag) = shift_left(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::ShiftLeftIndirectHL => {
                let address = cpu_registers.hl();
                let (value, c_flag) = shift_left(address_space.read_address_u8(address, ppu_state));
                address_space.write_address_u8(address, value, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::Swap(r) => {
                let register = cpu_registers.get_register_mut(r);
                *register = swap_bits(*register);
                let z_flag = ZFlag(*register == 0);
                cpu_registers.set_flags(z_flag, NFlag(false), HFlag(false), CFlag(false));
            }
            Self::SwapIndirectHL => {
                let address = cpu_registers.hl();
                let value = swap_bits(address_space.read_address_u8(address, ppu_state));
                address_space.write_address_u8(address, value, ppu_state);
                cpu_registers.set_flags(
                    ZFlag(value == 0),
                    NFlag(false),
                    HFlag(false),
                    CFlag(false),
                );
            }
            Self::ShiftRight(r) => {
                let (value, c_flag) = shift_right_arithmetic(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::ShiftRightIndirectHL => {
                let address = cpu_registers.hl();
                let (value, c_flag) =
                    shift_right_arithmetic(address_space.read_address_u8(address, ppu_state));
                address_space.write_address_u8(address, value, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::ShiftRightLogical(r) => {
                let (value, c_flag) = shift_right_logical(cpu_registers.read_register(r));
                cpu_registers.set_register(r, value);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::ShiftRightLogicalIndirectHL => {
                let address = cpu_registers.hl();
                let (value, c_flag) =
                    shift_right_logical(address_space.read_address_u8(address, ppu_state));
                address_space.write_address_u8(address, value, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::TestBit(n, r) => {
                let r_value = cpu_registers.read_register(r);
                let z_flag = ZFlag(r_value & (1 << n) == 0);
                cpu_registers.set_some_flags(
                    Some(z_flag),
                    Some(NFlag(false)),
                    Some(HFlag(true)),
                    None,
                );
            }
            Self::TestBitIndirectHL(n) => {
                let value = address_space.read_address_u8(cpu_registers.hl(), ppu_state);
                let z_flag = ZFlag(value & (1 << n) == 0);
                cpu_registers.set_some_flags(
                    Some(z_flag),
                    Some(NFlag(false)),
                    Some(HFlag(true)),
                    None,
                );
            }
            Self::SetBit(n, r) => {
                let register = cpu_registers.get_register_mut(r);
                *register |= 1 << n;
            }
            Self::SetBitIndirectHL(n) => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address, ppu_state) | (1 << n);
                address_space.write_address_u8(address, value, ppu_state);
            }
            Self::ResetBit(n, r) => {
                let register = cpu_registers.get_register_mut(r);
                *register &= !(1 << n);
            }
            Self::ResetBitIndirectHL(n) => {
                let address = cpu_registers.hl();
                let value = address_space.read_address_u8(address, ppu_state) & !(1 << n);
                address_space.write_address_u8(address, value, ppu_state);
            }
            Self::ComplementCarryFlag => {
                cpu_registers.set_some_flags(
                    None,
                    Some(NFlag(false)),
                    Some(HFlag(false)),
                    Some(CFlag(!cpu_registers.c_flag())),
                );
            }
            Self::SetCarryFlag => {
                cpu_registers.set_some_flags(
                    None,
                    Some(NFlag(false)),
                    Some(HFlag(false)),
                    Some(CFlag(true)),
                );
            }
            Self::DecimalAdjustAccumulator => {
                decimal_adjust_accumulator(cpu_registers);
            }
            Self::ComplementAccumulator => {
                cpu_registers.accumulator = !cpu_registers.accumulator;
                cpu_registers.set_some_flags(None, Some(NFlag(true)), Some(HFlag(true)), None);
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
                let pc = (i32::from(cpu_registers.pc) + i32::from(e)).try_into()?;
                cpu_registers.pc = pc;
            }
            Self::RelativeJumpCond(cc, e) => {
                if cc.check(cpu_registers) {
                    let pc = (i32::from(cpu_registers.pc) + i32::from(e)).try_into()?;
                    cpu_registers.pc = pc;
                }
            }
            Self::Call(nn) => {
                cpu_registers.sp -= 2;
                address_space.write_address_u16(cpu_registers.sp, cpu_registers.pc, ppu_state);
                cpu_registers.pc = nn;
            }
            Self::CallCond(cc, nn) => {
                if cc.check(cpu_registers) {
                    cpu_registers.sp -= 2;
                    address_space.write_address_u16(cpu_registers.sp, cpu_registers.pc, ppu_state);
                    cpu_registers.pc = nn;
                }
            }
            Self::Return => {
                cpu_registers.pc = address_space.read_address_u16(cpu_registers.sp, ppu_state);
                cpu_registers.sp += 2;
            }
            Self::ReturnCond(cc) => {
                if cc.check(cpu_registers) {
                    cpu_registers.pc = address_space.read_address_u16(cpu_registers.sp, ppu_state);
                    cpu_registers.sp += 2;
                }
            }
            Self::ReturnFromInterruptHandler => {
                cpu_registers.pc = address_space.read_address_u16(cpu_registers.sp, ppu_state);
                cpu_registers.sp += 2;
                cpu_registers.ime = true;
            }
            Self::RestartCall(rst_address) => {
                cpu_registers.sp -= 2;
                address_space.write_address_u16(cpu_registers.sp, cpu_registers.pc, ppu_state);
                cpu_registers.pc = rst_address.into();
            }
            Self::Halt => {
                cpu_registers.halted = true;
            }
            Self::Stop => {
                let key1_value = address_space
                    .get_io_registers()
                    .read_register(IoRegister::KEY1);
                if matches!(cpu_registers.execution_mode, ExecutionMode::GameBoyColor)
                    && key1_value & 0x01 != 0
                {
                    // In CGB mode, STOP when KEY1 bit 0 is set means speed switch
                    toggle_cgb_speed_mode(address_space, cpu_registers, key1_value);
                } else {
                    todo!("STOP is not implemented")
                }
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

    /// Return the number of clock cycles that are required to execute this instruction.
    ///
    /// Requires CPU registers as a parameter because conditional control flow instructions can
    /// take different numbers of cycles depending on whether the condition is true or false.
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
            | Self::JumpHL
            | Self::Halt
            | Self::Stop => 4,
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
        }
    }
}

fn add(l_value: u8, r_value: u8, carry: bool) -> (u8, CFlag, HFlag) {
    let carry = u8::from(carry);
    let (sum, carry_flag) = match l_value.overflowing_add(r_value) {
        (sum, true) => (sum + carry, true),
        (sum, false) => sum.overflowing_add(carry),
    };
    let h_flag = (l_value & 0x0F) + (r_value & 0x0F) + carry >= 0x10;

    (sum, CFlag(carry_flag), HFlag(h_flag))
}

fn add_u16(l_value: u16, r_value: u16) -> (u16, CFlag, HFlag) {
    let (sum, carry_flag) = l_value.overflowing_add(r_value);
    let h_flag = (l_value & 0x0FFF) + (r_value & 0x0FFF) >= 0x1000;

    (sum, CFlag(carry_flag), HFlag(h_flag))
}

fn sub(l_value: u8, r_value: u8, carry: bool) -> (u8, CFlag, HFlag) {
    let carry = u8::from(carry);
    let (difference, carry_flag) = match l_value.overflowing_sub(r_value) {
        (difference, true) => (difference - carry, true),
        (difference, false) => difference.overflowing_sub(carry),
    };
    let h_flag = l_value & 0x0F < (r_value & 0x0F) + carry;

    (difference, CFlag(carry_flag), HFlag(h_flag))
}

fn rotate_left(value: u8) -> (u8, CFlag) {
    let leftmost_set = value & 0x80 != 0;
    let new_value = (value << 1) | u8::from(leftmost_set);

    (new_value, CFlag(leftmost_set))
}

fn rotate_left_thru_carry(value: u8, carry: bool) -> (u8, CFlag) {
    let leftmost_set = value & 0x80 != 0;
    let new_value = (value << 1) | u8::from(carry);

    (new_value, CFlag(leftmost_set))
}

fn rotate_right(value: u8) -> (u8, CFlag) {
    let rightmost_set = value & 0x01 != 0;
    let new_value = (value >> 1) | (u8::from(rightmost_set) << 7);

    (new_value, CFlag(rightmost_set))
}

fn rotate_right_thru_carry(value: u8, carry: bool) -> (u8, CFlag) {
    let rightmost_set = value & 0x01 != 0;
    let new_value = (value >> 1) | (u8::from(carry) << 7);

    (new_value, CFlag(rightmost_set))
}

fn shift_left(value: u8) -> (u8, CFlag) {
    (value << 1, CFlag(value & 0x80 != 0))
}

fn shift_right_arithmetic(value: u8) -> (u8, CFlag) {
    ((value >> 1) | (value & 0x80), CFlag(value & 0x01 != 0))
}

fn shift_right_logical(value: u8) -> (u8, CFlag) {
    (value >> 1, CFlag(value & 0x01 != 0))
}

fn swap_bits(value: u8) -> u8 {
    (value >> 4) | (value << 4)
}

fn decimal_adjust_accumulator(cpu_registers: &mut CpuRegisters) {
    if cpu_registers.n_flag() {
        // Last op was subtraction
        let mut value = cpu_registers.accumulator;
        if cpu_registers.h_flag() {
            value = value.wrapping_sub(0x06);
        }
        if cpu_registers.c_flag() {
            value = value.wrapping_sub(0x60);
        }

        cpu_registers.accumulator = value;
        cpu_registers.set_some_flags(Some(ZFlag(value == 0)), None, Some(HFlag(false)), None);
    } else {
        // Last op was addition
        let mut value = cpu_registers.accumulator;
        let mut c_flag = CFlag(false);
        if value > 0x99 || cpu_registers.c_flag() {
            value = value.wrapping_add(0x60);
            c_flag = CFlag(true);
        }
        if value & 0x0F >= 0x0A || cpu_registers.h_flag() {
            value = value.wrapping_add(0x06);
        }

        cpu_registers.accumulator = value;
        cpu_registers.set_some_flags(
            Some(ZFlag(value == 0)),
            None,
            Some(HFlag(false)),
            Some(c_flag),
        );
    }
}

fn add_sp_offset(sp: u16, offset: i8) -> (u16, CFlag, HFlag) {
    if offset >= 0 {
        let offset = offset as u16;

        let h_flag = (sp & 0x000F) + (offset & 0x000F) >= 0x0010;
        let carry_flag = (sp & 0x00FF) + (offset & 0x00FF) >= 0x0100;

        (sp.wrapping_add(offset), CFlag(carry_flag), HFlag(h_flag))
    } else {
        let offset = -i32::from(offset) as u16;

        // These flags do the opposite of what I would expect them to in this instruction...
        let h_flag = offset & 0x000F <= sp & 0x000F;
        let carry_flag = offset & 0x00FF <= sp & 0x00FF;

        (sp.wrapping_sub(offset), CFlag(carry_flag), HFlag(h_flag))
    }
}

fn toggle_cgb_speed_mode(
    address_space: &mut AddressSpace,
    cpu_registers: &mut CpuRegisters,
    key1_value: u8,
) {
    let new_speed_mode = cpu_registers.cgb_speed_mode.toggle();
    cpu_registers.cgb_speed_mode = new_speed_mode;
    cpu_registers.speed_switch_wait_cycles_remaining = Some(2050);

    // Clear bit 0, and conditionally set/clear bit 7 based on new speed mode
    let new_key1 = match new_speed_mode {
        CgbSpeedMode::Normal => key1_value & 0x7E,
        CgbSpeedMode::Double => (key1_value & 0x7E) | 0x80,
    };

    address_space
        .get_io_registers_mut()
        .privileged_set_key1(new_key1);
}
