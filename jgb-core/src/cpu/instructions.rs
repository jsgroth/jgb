mod parse;

use crate::cpu::registers::{
    CFlag, CpuRegister, CpuRegisterPair, CpuRegisters, HFlag, NFlag, ZFlag,
};
use crate::memory::AddressSpace;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadTarget {
    Register(CpuRegister),
    Immediate(u8),
    IndirectHL,
    IndirectHLInc,
    IndirectHLDec,
    IndirectBC,
    IndirectDE,
    Accumulator,
    FFIndirectC,
    FFDirect(u8),
    Direct(u16),
}

impl ReadTarget {
    fn read_value(
        self,
        cpu_registers: &mut CpuRegisters,
        address_space: &AddressSpace,
        ppu_state: &PpuState,
    ) -> u8 {
        match self {
            Self::Register(register) => cpu_registers.read_register(register),
            Self::Immediate(n) => n,
            Self::IndirectHL => address_space.read_address_u8(cpu_registers.hl(), ppu_state),
            Self::IndirectHLInc => {
                let hl = cpu_registers.hl();
                let value = address_space.read_address_u8(hl, ppu_state);
                cpu_registers.set_hl(hl.wrapping_add(1));
                value
            }
            Self::IndirectHLDec => {
                let hl = cpu_registers.hl();
                let value = address_space.read_address_u8(hl, ppu_state);
                cpu_registers.set_hl(hl.wrapping_sub(1));
                value
            }
            Self::IndirectBC => address_space.read_address_u8(cpu_registers.bc(), ppu_state),
            Self::IndirectDE => address_space.read_address_u8(cpu_registers.de(), ppu_state),
            Self::Accumulator => cpu_registers.accumulator,
            Self::FFIndirectC => {
                let address = u16::from_be_bytes([0xFF, cpu_registers.c]);
                address_space.read_address_u8(address, ppu_state)
            }
            Self::FFDirect(n) => {
                let address = u16::from_be_bytes([0xFF, n]);
                address_space.read_address_u8(address, ppu_state)
            }
            Self::Direct(nn) => address_space.read_address_u8(nn, ppu_state),
        }
    }

    fn cycles_required(self) -> u32 {
        match self {
            Self::Register(..) | Self::Accumulator => 0,
            Self::Immediate(..)
            | Self::IndirectHL
            | Self::IndirectHLInc
            | Self::IndirectHLDec
            | Self::IndirectBC
            | Self::IndirectDE
            | Self::FFIndirectC => 4,
            Self::FFDirect(..) => 8,
            Self::Direct(..) => 12,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteTarget {
    Register(CpuRegister),
    IndirectHL,
    IndirectHLInc,
    IndirectHLDec,
    IndirectBC,
    IndirectDE,
    Accumulator,
    FFIndirectC,
    FFDirect(u8),
    Direct(u16),
}

impl WriteTarget {
    fn write_value(
        self,
        value: u8,
        cpu_registers: &mut CpuRegisters,
        address_space: &mut AddressSpace,
        ppu_state: &PpuState,
    ) {
        match self {
            Self::Register(register) => {
                cpu_registers.set_register(register, value);
            }
            Self::IndirectHL => {
                address_space.write_address_u8(cpu_registers.hl(), value, ppu_state);
            }
            Self::IndirectHLInc => {
                let hl = cpu_registers.hl();
                address_space.write_address_u8(hl, value, ppu_state);
                cpu_registers.set_hl(hl.wrapping_add(1));
            }
            Self::IndirectHLDec => {
                let hl = cpu_registers.hl();
                address_space.write_address_u8(hl, value, ppu_state);
                cpu_registers.set_hl(hl.wrapping_sub(1));
            }
            Self::IndirectBC => {
                address_space.write_address_u8(cpu_registers.bc(), value, ppu_state);
            }
            Self::IndirectDE => {
                address_space.write_address_u8(cpu_registers.de(), value, ppu_state);
            }
            Self::Accumulator => {
                cpu_registers.accumulator = value;
            }
            Self::FFIndirectC => {
                let address = u16::from_be_bytes([0xFF, cpu_registers.c]);
                address_space.write_address_u8(address, value, ppu_state);
            }
            Self::FFDirect(n) => {
                let address = u16::from_be_bytes([0xFF, n]);
                address_space.write_address_u8(address, value, ppu_state);
            }
            Self::Direct(nn) => {
                address_space.write_address_u8(nn, value, ppu_state);
            }
        }
    }

    fn cycles_required(self) -> u32 {
        match self {
            Self::Register(..) | Self::Accumulator => 0,
            Self::IndirectHL
            | Self::IndirectHLInc
            | Self::IndirectHLDec
            | Self::IndirectBC
            | Self::IndirectDE
            | Self::FFIndirectC => 4,
            Self::FFDirect(..) => 8,
            Self::Direct(..) => 12,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifyTarget {
    Register(CpuRegister),
    IndirectHL,
    Accumulator,
}

impl ModifyTarget {
    fn read_value(
        self,
        cpu_registers: &CpuRegisters,
        address_space: &AddressSpace,
        ppu_state: &PpuState,
    ) -> u8 {
        match self {
            Self::Register(register) => cpu_registers.read_register(register),
            Self::IndirectHL => address_space.read_address_u8(cpu_registers.hl(), ppu_state),
            Self::Accumulator => cpu_registers.accumulator,
        }
    }

    fn write_value(
        self,
        value: u8,
        cpu_registers: &mut CpuRegisters,
        address_space: &mut AddressSpace,
        ppu_state: &PpuState,
    ) {
        match self {
            Self::Register(register) => {
                cpu_registers.set_register(register, value);
            }
            Self::IndirectHL => {
                address_space.write_address_u8(cpu_registers.hl(), value, ppu_state);
            }
            Self::Accumulator => {
                cpu_registers.accumulator = value;
            }
        }
    }

    fn cycles_required(self) -> u32 {
        match self {
            Self::Register(..) | Self::Accumulator => 0,
            Self::IndirectHL => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // All 8-bit LD/LDH instructions
    Load(WriteTarget, ReadTarget),
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
    // ADD r / (HL) / n
    Add(ReadTarget),
    // ADC r / (HL) / n
    AddWithCarry(ReadTarget),
    // SUB r / (HL) / n
    Subtract(ReadTarget),
    // SBC r / (HL) / n
    SubtractWithCarry(ReadTarget),
    // CP r / (HL) / n
    Compare(ReadTarget),
    // INC r / (HL)
    Increment(ModifyTarget),
    // DEC r / (HL)
    Decrement(ModifyTarget),
    // AND r / (HL) / n
    And(ReadTarget),
    // OR r / (HL) / n
    Or(ReadTarget),
    // XOR r / (HL) / n
    Xor(ReadTarget),
    // ADD HL, rr
    AddHLRegister(CpuRegisterPair),
    // INC rr,
    IncRegisterPair(CpuRegisterPair),
    // DEC rr,
    DecRegisterPair(CpuRegisterPair),
    // ADD SP, e
    AddSPImmediate(i8),
    // RLCA / RLC r / RLC (HL)
    RotateLeft(ModifyTarget),
    // RLA / RL r / RL (HL)
    RotateLeftThruCarry(ModifyTarget),
    // RRCA / RRC r / RRC (HL)
    RotateRight(ModifyTarget),
    // RRA / RR r / RR (HL)
    RotateRightThruCarry(ModifyTarget),
    // SLA r / (HL)
    ShiftLeft(ModifyTarget),
    // SWAP r / (HL)
    Swap(ModifyTarget),
    // SRA r / (HL)
    ArithmeticShiftRight(ModifyTarget),
    // SRL r / (HL)
    LogicalShiftRight(ModifyTarget),
    // BIT n, r / (HL)
    TestBit(u8, ReadTarget),
    // RES n, r / (HL)
    ResetBit(u8, ModifyTarget),
    // SET n, r / (HL)
    SetBit(u8, ModifyTarget),
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
    ) {
        match self {
            Self::Load(write_target, read_target) => {
                let value = read_target.read_value(cpu_registers, address_space, ppu_state);
                write_target.write_value(value, cpu_registers, address_space, ppu_state);
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
            Self::Add(read_target) => {
                let (sum, c_flag, h_flag) = add(
                    cpu_registers.accumulator,
                    read_target.read_value(cpu_registers, address_space, ppu_state),
                    false,
                );
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(ZFlag(sum == 0), NFlag(false), h_flag, c_flag);
            }
            Self::AddWithCarry(read_target) => {
                let (sum, c_flag, h_flag) = add(
                    cpu_registers.accumulator,
                    read_target.read_value(cpu_registers, address_space, ppu_state),
                    cpu_registers.c_flag(),
                );
                cpu_registers.accumulator = sum;
                cpu_registers.set_flags(ZFlag(sum == 0), NFlag(false), h_flag, c_flag);
            }
            Self::Subtract(read_target) => {
                let (difference, c_flag, h_flag) = sub(
                    cpu_registers.accumulator,
                    read_target.read_value(cpu_registers, address_space, ppu_state),
                    false,
                );
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::SubtractWithCarry(read_target) => {
                let (difference, c_flag, h_flag) = sub(
                    cpu_registers.accumulator,
                    read_target.read_value(cpu_registers, address_space, ppu_state),
                    cpu_registers.c_flag(),
                );
                cpu_registers.accumulator = difference;
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::Compare(read_target) => {
                let (difference, c_flag, h_flag) = sub(
                    cpu_registers.accumulator,
                    read_target.read_value(cpu_registers, address_space, ppu_state),
                    false,
                );
                cpu_registers.set_flags(ZFlag(difference == 0), NFlag(true), h_flag, c_flag);
            }
            Self::Increment(modify_target) => {
                let value = modify_target.read_value(cpu_registers, address_space, ppu_state);
                let (sum, _, h_flag) = add(value, 1, false);
                modify_target.write_value(sum, cpu_registers, address_space, ppu_state);
                cpu_registers.set_some_flags(
                    Some(ZFlag(sum == 0)),
                    Some(NFlag(false)),
                    Some(h_flag),
                    None,
                );
            }
            Self::Decrement(modify_target) => {
                let value = modify_target.read_value(cpu_registers, address_space, ppu_state);
                let (difference, _, h_flag) = sub(value, 1, false);
                modify_target.write_value(difference, cpu_registers, address_space, ppu_state);
                cpu_registers.set_some_flags(
                    Some(ZFlag(difference == 0)),
                    Some(NFlag(true)),
                    Some(h_flag),
                    None,
                );
            }
            Self::And(read_target) => {
                let value = cpu_registers.accumulator
                    & read_target.read_value(cpu_registers, address_space, ppu_state);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(true), CFlag(false));
            }
            Self::Or(read_target) => {
                let value = cpu_registers.accumulator
                    | read_target.read_value(cpu_registers, address_space, ppu_state);
                cpu_registers.accumulator = value;
                cpu_registers.set_flags(
                    ZFlag(value == 0),
                    NFlag(false),
                    HFlag(false),
                    CFlag(false),
                );
            }
            Self::Xor(read_target) => {
                let value = cpu_registers.accumulator
                    ^ read_target.read_value(cpu_registers, address_space, ppu_state);
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
            Self::RotateLeft(modify_target) => {
                let (value, c_flag) =
                    rotate_left(modify_target.read_value(cpu_registers, address_space, ppu_state));
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
                let z_flag = ZFlag(modify_target != ModifyTarget::Accumulator && value == 0);
                cpu_registers.set_flags(z_flag, NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateLeftThruCarry(modify_target) => {
                let (value, c_flag) = rotate_left_thru_carry(
                    modify_target.read_value(cpu_registers, address_space, ppu_state),
                    cpu_registers.c_flag(),
                );
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
                let z_flag = ZFlag(modify_target != ModifyTarget::Accumulator && value == 0);
                cpu_registers.set_flags(z_flag, NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateRight(modify_target) => {
                let (value, c_flag) =
                    rotate_right(modify_target.read_value(cpu_registers, address_space, ppu_state));
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
                let z_flag = ZFlag(modify_target != ModifyTarget::Accumulator && value == 0);
                cpu_registers.set_flags(z_flag, NFlag(false), HFlag(false), c_flag);
            }
            Self::RotateRightThruCarry(modify_target) => {
                let (value, c_flag) = rotate_right_thru_carry(
                    modify_target.read_value(cpu_registers, address_space, ppu_state),
                    cpu_registers.c_flag(),
                );
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
                let z_flag = ZFlag(modify_target != ModifyTarget::Accumulator && value == 0);
                cpu_registers.set_flags(z_flag, NFlag(false), HFlag(false), c_flag);
            }
            Self::ShiftLeft(modify_target) => {
                let (value, c_flag) =
                    shift_left(modify_target.read_value(cpu_registers, address_space, ppu_state));
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::Swap(modify_target) => {
                let value =
                    swap_bits(modify_target.read_value(cpu_registers, address_space, ppu_state));
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
                cpu_registers.set_flags(
                    ZFlag(value == 0),
                    NFlag(false),
                    HFlag(false),
                    CFlag(false),
                );
            }
            Self::ArithmeticShiftRight(modify_target) => {
                let (value, c_flag) = shift_right_arithmetic(modify_target.read_value(
                    cpu_registers,
                    address_space,
                    ppu_state,
                ));
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::LogicalShiftRight(modify_target) => {
                let (value, c_flag) = shift_right_logical(modify_target.read_value(
                    cpu_registers,
                    address_space,
                    ppu_state,
                ));
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
                cpu_registers.set_flags(ZFlag(value == 0), NFlag(false), HFlag(false), c_flag);
            }
            Self::TestBit(n, read_target) => {
                let r_value = read_target.read_value(cpu_registers, address_space, ppu_state);
                let z_flag = ZFlag(r_value & (1 << n) == 0);
                cpu_registers.set_some_flags(
                    Some(z_flag),
                    Some(NFlag(false)),
                    Some(HFlag(true)),
                    None,
                );
            }
            Self::SetBit(n, modify_target) => {
                let value =
                    (1 << n) | modify_target.read_value(cpu_registers, address_space, ppu_state);
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
            }
            Self::ResetBit(n, modify_target) => {
                let value =
                    !(1 << n) & modify_target.read_value(cpu_registers, address_space, ppu_state);
                modify_target.write_value(value, cpu_registers, address_space, ppu_state);
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
                let pc = (i32::from(cpu_registers.pc) + i32::from(e)) as u16;
                cpu_registers.pc = pc;
            }
            Self::RelativeJumpCond(cc, e) => {
                if cc.check(cpu_registers) {
                    let pc = (i32::from(cpu_registers.pc) + i32::from(e)) as u16;
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
                return;
            }
            Self::NoOp => {}
        }

        cpu_registers.interrupt_delay = false;
    }

    /// Return the number of clock cycles that are required to execute this instruction.
    ///
    /// Requires CPU registers as a parameter because conditional control flow instructions can
    /// take different numbers of cycles depending on whether the condition is true or false.
    pub fn cycles_required(self, cpu_registers: &CpuRegisters) -> u32 {
        match self {
            Self::DecimalAdjustAccumulator
            | Self::ComplementAccumulator
            | Self::RotateLeft(ModifyTarget::Accumulator)
            | Self::RotateLeftThruCarry(ModifyTarget::Accumulator)
            | Self::RotateRight(ModifyTarget::Accumulator)
            | Self::RotateRightThruCarry(ModifyTarget::Accumulator)
            | Self::SetCarryFlag
            | Self::ComplementCarryFlag
            | Self::NoOp
            | Self::DisableInterrupts
            | Self::EnableInterrupts
            | Self::JumpHL
            | Self::Halt
            | Self::Stop => 4,

            Self::LoadStackPointerHL
            | Self::AddHLRegister(..)
            | Self::IncRegisterPair(..)
            | Self::DecRegisterPair(..) => 8,
            Self::LoadRegisterPairImmediate(..)
            | Self::PopStack(..)
            | Self::LoadHLStackPointerOffset(..)
            | Self::RelativeJump(..) => 12,
            Self::PushStack(..)
            | Self::AddSPImmediate(..)
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
            Self::Load(write_target, read_target) => {
                4 + read_target.cycles_required() + write_target.cycles_required()
            }
            Self::Add(read_target)
            | Self::AddWithCarry(read_target)
            | Self::Subtract(read_target)
            | Self::SubtractWithCarry(read_target)
            | Self::And(read_target)
            | Self::Or(read_target)
            | Self::Xor(read_target)
            | Self::Compare(read_target) => 4 + read_target.cycles_required(),
            Self::TestBit(_, read_target) => 8 + read_target.cycles_required(),
            Self::Increment(modify_target) | Self::Decrement(modify_target) => {
                4 + 2 * modify_target.cycles_required()
            }
            Self::RotateLeft(modify_target)
            | Self::RotateRight(modify_target)
            | Self::RotateLeftThruCarry(modify_target)
            | Self::RotateRightThruCarry(modify_target)
            | Self::ShiftLeft(modify_target)
            | Self::ArithmeticShiftRight(modify_target)
            | Self::LogicalShiftRight(modify_target)
            | Self::Swap(modify_target)
            | Self::ResetBit(_, modify_target)
            | Self::SetBit(_, modify_target) => 8 + 2 * modify_target.cycles_required(),
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
