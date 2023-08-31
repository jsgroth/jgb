use crate::cpu::instructions::{Instruction, JumpCondition, ModifyTarget, ReadTarget, WriteTarget};
use crate::cpu::registers::{CpuRegister, CpuRegisterPair};
use crate::memory::AddressSpace;
use crate::ppu::PpuState;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("invalid opcode sequence: {opcodes:?}")]
    InvalidOpcode { opcodes: Vec<u8> },
}

pub fn parse_next_instruction(
    address_space: &AddressSpace,
    mut pc: u16,
    ppu_state: &PpuState,
    halt_bug_triggered: bool,
) -> Result<(Instruction, u16), ParseError> {
    let opcode = address_space.read_address_u8(pc, ppu_state);

    // If HALT bug triggered, act as if the opcode read did not advance the PC
    if halt_bug_triggered {
        pc -= 1;
    }

    match opcode {
        0x00 => Ok((Instruction::NoOp, pc + 1)),
        0x01 | 0x11 | 0x21 | 0x31 => {
            let rr = register_pair_for_other_ops(opcode);
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::LoadRegisterPairImmediate(rr, nn), pc + 3))
        }
        0x02 => Ok((Instruction::Load(WriteTarget::IndirectBC, ReadTarget::Accumulator), pc + 1)),
        0x03 | 0x13 | 0x23 | 0x33 => {
            let rr = register_pair_for_other_ops(opcode);
            Ok((Instruction::IncRegisterPair(rr), pc + 1))
        }
        0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => {
            let modify_target = CpuRegister::from_mid_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            Ok((Instruction::Increment(modify_target), pc + 1))
        }
        0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
            let modify_target = CpuRegister::from_mid_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            Ok((Instruction::Decrement(modify_target), pc + 1))
        }
        0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => {
            let write_target = CpuRegister::from_mid_opcode_bits(opcode)
                .map_or(WriteTarget::IndirectHL, WriteTarget::Register);
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::Load(write_target, ReadTarget::Immediate(n)), pc + 2))
        }
        0x07 => Ok((Instruction::RotateLeft(ModifyTarget::Accumulator), pc + 1)),
        0x08 => {
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::LoadDirectStackPointer(nn), pc + 3))
        }
        0x09 | 0x19 | 0x29 | 0x39 => {
            let rr = register_pair_for_other_ops(opcode);
            Ok((Instruction::AddHLRegister(rr), pc + 1))
        }
        0x0A => Ok((Instruction::Load(WriteTarget::Accumulator, ReadTarget::IndirectBC), pc + 1)),
        0x0B | 0x1B | 0x2B | 0x3B => {
            let rr = register_pair_for_other_ops(opcode);
            Ok((Instruction::DecRegisterPair(rr), pc + 1))
        }
        0x0F => Ok((Instruction::RotateRight(ModifyTarget::Accumulator), pc + 1)),
        0x10 => Ok((Instruction::Stop, pc + 2)),
        0x12 => Ok((Instruction::Load(WriteTarget::IndirectDE, ReadTarget::Accumulator), pc + 1)),
        0x17 => Ok((Instruction::RotateLeftThruCarry(ModifyTarget::Accumulator), pc + 1)),
        0x18 => {
            let e = address_space.read_address_u8(pc + 1, ppu_state) as i8;
            Ok((Instruction::RelativeJump(e), pc + 2))
        }
        0x1A => Ok((Instruction::Load(WriteTarget::Accumulator, ReadTarget::IndirectDE), pc + 1)),
        0x1F => Ok((Instruction::RotateRightThruCarry(ModifyTarget::Accumulator), pc + 1)),
        0x20 | 0x28 | 0x30 | 0x38 => {
            let cc = parse_jump_condition(opcode);
            let e = address_space.read_address_u8(pc + 1, ppu_state) as i8;
            Ok((Instruction::RelativeJumpCond(cc, e), pc + 2))
        }
        0x22 => {
            Ok((Instruction::Load(WriteTarget::IndirectHLInc, ReadTarget::Accumulator), pc + 1))
        }
        0x27 => Ok((Instruction::DecimalAdjustAccumulator, pc + 1)),
        0x2A => {
            Ok((Instruction::Load(WriteTarget::Accumulator, ReadTarget::IndirectHLInc), pc + 1))
        }
        0x2F => Ok((Instruction::ComplementAccumulator, pc + 1)),
        0x32 => {
            Ok((Instruction::Load(WriteTarget::IndirectHLDec, ReadTarget::Accumulator), pc + 1))
        }
        0x37 => Ok((Instruction::SetCarryFlag, pc + 1)),
        0x3A => {
            Ok((Instruction::Load(WriteTarget::Accumulator, ReadTarget::IndirectHLDec), pc + 1))
        }
        0x3F => Ok((Instruction::ComplementCarryFlag, pc + 1)),
        opcode @ 0x40..=0x7F => {
            if opcode == 0x76 {
                Ok((Instruction::Halt, pc + 1))
            } else {
                let write_target = CpuRegister::from_mid_opcode_bits(opcode)
                    .map_or(WriteTarget::IndirectHL, WriteTarget::Register);
                let read_target = CpuRegister::from_low_opcode_bits(opcode)
                    .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
                Ok((Instruction::Load(write_target, read_target), pc + 1))
            }
        }
        opcode @ 0x80..=0x87 => {
            let read_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
            Ok((Instruction::Add(read_target), pc + 1))
        }
        opcode @ 0x88..=0x8F => {
            let read_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
            Ok((Instruction::AddWithCarry(read_target), pc + 1))
        }
        opcode @ 0x90..=0x97 => {
            let read_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
            Ok((Instruction::Subtract(read_target), pc + 1))
        }
        opcode @ 0x98..=0x9F => {
            let read_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
            Ok((Instruction::SubtractWithCarry(read_target), pc + 1))
        }
        opcode @ 0xA0..=0xA7 => {
            let read_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
            Ok((Instruction::And(read_target), pc + 1))
        }
        opcode @ 0xA8..=0xAF => {
            let read_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
            Ok((Instruction::Xor(read_target), pc + 1))
        }
        opcode @ 0xB0..=0xB7 => {
            let read_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
            Ok((Instruction::Or(read_target), pc + 1))
        }
        opcode @ 0xB8..=0xBF => {
            let read_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
            Ok((Instruction::Compare(read_target), pc + 1))
        }
        0xC0 | 0xC8 | 0xD0 | 0xD8 => {
            let cc = parse_jump_condition(opcode);
            Ok((Instruction::ReturnCond(cc), pc + 1))
        }
        0xC1 | 0xD1 | 0xE1 | 0xF1 => {
            let rr = register_pair_for_push_pop(opcode);
            Ok((Instruction::PopStack(rr), pc + 1))
        }
        0xC2 | 0xCA | 0xD2 | 0xDA => {
            let cc = parse_jump_condition(opcode);
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::JumpCond(cc, nn), pc + 3))
        }
        0xC3 => {
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::Jump(nn), pc + 3))
        }
        0xC4 | 0xCC | 0xD4 | 0xDC => {
            let cc = parse_jump_condition(opcode);
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::CallCond(cc, nn), pc + 3))
        }
        0xC5 | 0xD5 | 0xE5 | 0xF5 => {
            let rr = register_pair_for_push_pop(opcode);
            Ok((Instruction::PushStack(rr), pc + 1))
        }
        0xC6 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::Add(ReadTarget::Immediate(n)), pc + 2))
        }
        0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
            let rst_address = opcode & 0x38;
            Ok((Instruction::RestartCall(rst_address), pc + 1))
        }
        0xC9 => Ok((Instruction::Return, pc + 1)),
        0xCB => Ok(parse_cb_prefixed_opcode(address_space, pc, ppu_state)),
        0xCD => {
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::Call(nn), pc + 3))
        }
        0xCE => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::AddWithCarry(ReadTarget::Immediate(n)), pc + 2))
        }
        0xD6 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::Subtract(ReadTarget::Immediate(n)), pc + 2))
        }
        0xD9 => Ok((Instruction::ReturnFromInterruptHandler, pc + 1)),
        0xDE => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::SubtractWithCarry(ReadTarget::Immediate(n)), pc + 2))
        }
        0xE0 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::Load(WriteTarget::FFDirect(n), ReadTarget::Accumulator), pc + 2))
        }
        0xE2 => Ok((Instruction::Load(WriteTarget::FFIndirectC, ReadTarget::Accumulator), pc + 1)),
        0xE6 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::And(ReadTarget::Immediate(n)), pc + 2))
        }
        0xE8 => {
            let e = address_space.read_address_u8(pc + 1, ppu_state) as i8;
            Ok((Instruction::AddSPImmediate(e), pc + 2))
        }
        0xE9 => Ok((Instruction::JumpHL, pc + 1)),
        0xEA => {
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::Load(WriteTarget::Direct(nn), ReadTarget::Accumulator), pc + 3))
        }
        0xEE => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::Xor(ReadTarget::Immediate(n)), pc + 2))
        }
        0xF0 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::Load(WriteTarget::Accumulator, ReadTarget::FFDirect(n)), pc + 2))
        }
        0xF2 => Ok((Instruction::Load(WriteTarget::Accumulator, ReadTarget::FFIndirectC), pc + 1)),
        0xF3 => Ok((Instruction::DisableInterrupts, pc + 1)),
        0xF6 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::Or(ReadTarget::Immediate(n)), pc + 2))
        }
        0xF8 => {
            let e = address_space.read_address_u8(pc + 1, ppu_state) as i8;
            Ok((Instruction::LoadHLStackPointerOffset(e), pc + 2))
        }
        0xF9 => Ok((Instruction::LoadStackPointerHL, pc + 1)),
        0xFA => {
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::Load(WriteTarget::Accumulator, ReadTarget::Direct(nn)), pc + 3))
        }
        0xFB => Ok((Instruction::EnableInterrupts, pc + 1)),
        0xFE => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::Compare(ReadTarget::Immediate(n)), pc + 2))
        }
        _ => Err(ParseError::InvalidOpcode { opcodes: vec![opcode] }),
    }
}

fn parse_cb_prefixed_opcode(
    address_space: &AddressSpace,
    pc: u16,
    ppu_state: &PpuState,
) -> (Instruction, u16) {
    let opcode = address_space.read_address_u8(pc + 1, ppu_state);
    match opcode {
        opcode @ 0x00..=0x07 => {
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::RotateLeft(modify_target), pc + 2)
        }
        opcode @ 0x08..=0x0F => {
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::RotateRight(modify_target), pc + 2)
        }
        opcode @ 0x10..=0x17 => {
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::RotateLeftThruCarry(modify_target), pc + 2)
        }
        opcode @ 0x18..=0x1F => {
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::RotateRightThruCarry(modify_target), pc + 2)
        }
        opcode @ 0x20..=0x27 => {
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::ShiftLeft(modify_target), pc + 2)
        }
        opcode @ 0x28..=0x2F => {
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::ArithmeticShiftRight(modify_target), pc + 2)
        }
        opcode @ 0x30..=0x37 => {
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::Swap(modify_target), pc + 2)
        }
        opcode @ 0x38..=0x3F => {
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::LogicalShiftRight(modify_target), pc + 2)
        }
        opcode @ 0x40..=0x7F => {
            let bit = (opcode & 0x38) >> 3;
            let read_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ReadTarget::IndirectHL, ReadTarget::Register);
            (Instruction::TestBit(bit, read_target), pc + 2)
        }
        opcode @ 0x80..=0xBF => {
            let bit = (opcode & 0x38) >> 3;
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::ResetBit(bit, modify_target), pc + 2)
        }
        opcode @ 0xC0..=0xFF => {
            let bit = (opcode & 0x38) >> 3;
            let modify_target = CpuRegister::from_low_opcode_bits(opcode)
                .map_or(ModifyTarget::IndirectHL, ModifyTarget::Register);
            (Instruction::SetBit(bit, modify_target), pc + 2)
        }
    }
}

fn register_pair_for_other_ops(opcode: u8) -> CpuRegisterPair {
    match opcode & 0x30 {
        0x00 => CpuRegisterPair::BC,
        0x10 => CpuRegisterPair::DE,
        0x20 => CpuRegisterPair::HL,
        0x30 => CpuRegisterPair::SP,
        _ => panic!("{opcode} & 0x30 did not produce 0x00/0x10/0x20/0x30"),
    }
}

fn register_pair_for_push_pop(opcode: u8) -> CpuRegisterPair {
    match opcode & 0x30 {
        0x00 => CpuRegisterPair::BC,
        0x10 => CpuRegisterPair::DE,
        0x20 => CpuRegisterPair::HL,
        0x30 => CpuRegisterPair::AF,
        _ => panic!("{opcode} & 0x30 did not produce 0x00/0x10/0x20/0x30"),
    }
}

fn parse_jump_condition(opcode: u8) -> JumpCondition {
    match opcode & 0x18 {
        0x00 => JumpCondition::NZ,
        0x08 => JumpCondition::Z,
        0x10 => JumpCondition::NC,
        0x18 => JumpCondition::C,
        _ => panic!("{opcode} & 0x18 did not produce 0x00/0x08/0x10/0x18"),
    }
}
