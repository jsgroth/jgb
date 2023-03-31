use crate::cpu::instructions::{Instruction, JumpCondition};
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
    pc: u16,
    ppu_state: &PpuState,
) -> Result<(Instruction, u16), ParseError> {
    let opcode = address_space.read_address_u8(pc, ppu_state);
    match opcode {
        0x00 => Ok((Instruction::NoOp, pc + 1)),
        0x01 | 0x11 | 0x21 | 0x31 => {
            let rr = register_pair_for_other_ops(opcode);
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::LoadRegisterPairImmediate(rr, nn), pc + 3))
        }
        0x02 => Ok((Instruction::LoadIndirectBCAccumulator, pc + 1)),
        0x03 | 0x13 | 0x23 | 0x33 => {
            let rr = register_pair_for_other_ops(opcode);
            Ok((Instruction::IncRegisterPair(rr), pc + 1))
        }
        0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x3C => {
            let r = CpuRegister::from_mid_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 3-5");
            Ok((Instruction::IncRegister(r), pc + 1))
        }
        0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x3D => {
            let r = CpuRegister::from_mid_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 3-5");
            Ok((Instruction::DecRegister(r), pc + 1))
        }
        0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => {
            let r = CpuRegister::from_mid_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 3-5");
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::LoadRegisterImmediate(r, n), pc + 2))
        }
        0x07 => Ok((Instruction::RotateLeftAccumulator, pc + 1)),
        0x08 => {
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::LoadDirectStackPointer(nn), pc + 3))
        }
        0x09 | 0x19 | 0x29 | 0x39 => {
            let rr = register_pair_for_other_ops(opcode);
            Ok((Instruction::AddHLRegister(rr), pc + 1))
        }
        0x0A => Ok((Instruction::LoadAccumulatorIndirectBC, pc + 1)),
        0x0B | 0x1B | 0x2B | 0x3B => {
            let rr = register_pair_for_other_ops(opcode);
            Ok((Instruction::DecRegisterPair(rr), pc + 1))
        }
        0x0F => Ok((Instruction::RotateRightAccumulator, pc + 1)),
        0x10 => Ok((Instruction::Stop, pc + 2)),
        0x12 => Ok((Instruction::LoadIndirectDEAccumulator, pc + 1)),
        0x17 => Ok((Instruction::RotateLeftAccumulatorThruCarry, pc + 1)),
        0x18 => {
            let e = address_space.read_address_u8(pc + 1, ppu_state) as i8;
            Ok((Instruction::RelativeJump(e), pc + 2))
        }
        0x1A => Ok((Instruction::LoadAccumulatorIndirectDE, pc + 1)),
        0x1F => Ok((Instruction::RotateRightAccumulatorThruCarry, pc + 1)),
        0x20 | 0x28 | 0x30 | 0x38 => {
            let cc = parse_jump_condition(opcode);
            let e = address_space.read_address_u8(pc + 1, ppu_state) as i8;
            Ok((Instruction::RelativeJumpCond(cc, e), pc + 2))
        }
        0x22 => Ok((Instruction::LoadIndirectHLIncAccumulator, pc + 1)),
        0x27 => Ok((Instruction::DecimalAdjustAccumulator, pc + 1)),
        0x2A => Ok((Instruction::LoadAccumulatorIndirectHLInc, pc + 1)),
        0x2F => Ok((Instruction::ComplementAccumulator, pc + 1)),
        0x34 => Ok((Instruction::IncIndirectHL, pc + 1)),
        0x35 => Ok((Instruction::DecIndirectHL, pc + 1)),
        0x36 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::LoadIndirectHLImmediate(n), pc + 2))
        }
        0x32 => Ok((Instruction::LoadIndirectHLDecAccumulator, pc + 1)),
        0x37 => Ok((Instruction::SetCarryFlag, pc + 1)),
        0x3A => Ok((Instruction::LoadAccumulatorIndirectHLDec, pc + 1)),
        0x3F => Ok((Instruction::ComplementCarryFlag, pc + 1)),
        0x40 | 0x41 | 0x42 | 0x43 | 0x44 | 0x45 | 0x47 | 0x48 | 0x49 | 0x4A | 0x4B | 0x4C
        | 0x4D | 0x4F | 0x50 | 0x51 | 0x52 | 0x53 | 0x54 | 0x55 | 0x57 | 0x58 | 0x59 | 0x5A
        | 0x5B | 0x5C | 0x5D | 0x5F | 0x60 | 0x61 | 0x62 | 0x63 | 0x64 | 0x65 | 0x67 | 0x68
        | 0x69 | 0x6A | 0x6B | 0x6C | 0x6D | 0x6F | 0x78 | 0x79 | 0x7A | 0x7B | 0x7C | 0x7D
        | 0x7F => {
            let r = CpuRegister::from_mid_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 3-5");
            let r_p = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            Ok((Instruction::LoadRegisterRegister(r, r_p), pc + 1))
        }
        0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x7E => {
            let r = CpuRegister::from_mid_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 3-5");
            Ok((Instruction::LoadRegisterIndirectHL(r), pc + 1))
        }
        0x70 | 0x71 | 0x72 | 0x73 | 0x74 | 0x75 | 0x77 => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            Ok((Instruction::LoadIndirectHLRegister(r), pc + 1))
        }
        0x76 => Ok((Instruction::Halt, pc + 1)),
        0x80 | 0x81 | 0x82 | 0x83 | 0x84 | 0x85 | 0x87 => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            Ok((Instruction::AddRegister(r), pc + 1))
        }
        0x86 => Ok((Instruction::AddIndirectHL, pc + 1)),
        0x88 | 0x89 | 0x8A | 0x8B | 0x8C | 0x8D | 0x8F => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            Ok((Instruction::AddCarryRegister(r), pc + 1))
        }
        0x8E => Ok((Instruction::AddCarryIndirectHL, pc + 1)),
        0x90 | 0x91 | 0x92 | 0x93 | 0x94 | 0x95 | 0x97 => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            Ok((Instruction::SubtractRegister(r), pc + 1))
        }
        0x96 => Ok((Instruction::SubtractIndirectHL, pc + 1)),
        0x98 | 0x99 | 0x9A | 0x9B | 0x9C | 0x9D | 0x9F => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            Ok((Instruction::SubtractCarryRegister(r), pc + 1))
        }
        0x9E => Ok((Instruction::SubtractCarryIndirectHL, pc + 1)),
        0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5 | 0xA7 => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            Ok((Instruction::AndRegister(r), pc + 1))
        }
        0xA6 => Ok((Instruction::AndIndirectHL, pc + 1)),
        0xA8 | 0xA9 | 0xAA | 0xAB | 0xAC | 0xAD | 0xAF => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes have a valid CPU register in bits 0-2");
            Ok((Instruction::XorRegister(r), pc + 1))
        }
        0xAE => Ok((Instruction::XorIndirectHL, pc + 1)),
        0xB0 | 0xB1 | 0xB2 | 0xB3 | 0xB4 | 0xB5 | 0xB7 => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            Ok((Instruction::OrRegister(r), pc + 1))
        }
        0xB6 => Ok((Instruction::OrIndirectHL, pc + 1)),
        0xB8 | 0xB9 | 0xBA | 0xBB | 0xBC | 0xBD | 0xBF => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            Ok((Instruction::CompareRegister(r), pc + 1))
        }
        0xBE => Ok((Instruction::CompareIndirectHL, pc + 1)),
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
            Ok((Instruction::AddImmediate(n), pc + 2))
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
            Ok((Instruction::AddCarryImmediate(n), pc + 2))
        }
        0xD6 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::SubtractImmediate(n), pc + 2))
        }
        0xD9 => Ok((Instruction::ReturnFromInterruptHandler, pc + 1)),
        0xDE => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::SubtractCarryImmediate(n), pc + 2))
        }
        0xE0 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::LoadDirect8Accumulator(n), pc + 2))
        }
        0xE2 => Ok((Instruction::LoadIndirectCAccumulator, pc + 1)),
        0xE6 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::AndImmediate(n), pc + 2))
        }
        0xE8 => {
            let e = address_space.read_address_u8(pc + 1, ppu_state) as i8;
            Ok((Instruction::AddSPImmediate(e), pc + 2))
        }
        0xE9 => Ok((Instruction::JumpHL, pc + 1)),
        0xEA => {
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::LoadDirect16Accumulator(nn), pc + 3))
        }
        0xEE => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::XorImmediate(n), pc + 2))
        }
        0xF0 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::LoadAccumulatorDirect8(n), pc + 2))
        }
        0xF2 => Ok((Instruction::LoadAccumulatorIndirectC, pc + 1)),
        0xF3 => Ok((Instruction::DisableInterrupts, pc + 1)),
        0xF6 => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::OrImmediate(n), pc + 2))
        }
        0xF8 => {
            let e = address_space.read_address_u8(pc + 1, ppu_state) as i8;
            Ok((Instruction::LoadHLStackPointerOffset(e), pc + 2))
        }
        0xF9 => Ok((Instruction::LoadStackPointerHL, pc + 1)),
        0xFA => {
            let nn = address_space.read_address_u16(pc + 1, ppu_state);
            Ok((Instruction::LoadAccumulatorDirect16(nn), pc + 3))
        }
        0xFB => Ok((Instruction::EnableInterrupts, pc + 1)),
        0xFE => {
            let n = address_space.read_address_u8(pc + 1, ppu_state);
            Ok((Instruction::CompareImmediate(n), pc + 2))
        }
        _ => Err(ParseError::InvalidOpcode {
            opcodes: vec![opcode],
        }),
    }
}

fn parse_cb_prefixed_opcode(
    address_space: &AddressSpace,
    pc: u16,
    ppu_state: &PpuState,
) -> (Instruction, u16) {
    let opcode = address_space.read_address_u8(pc + 1, ppu_state);
    match opcode {
        0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x07 => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            (Instruction::RotateLeft(r), pc + 2)
        }
        0x06 => (Instruction::RotateLeftIndirectHL, pc + 2),
        0x08 | 0x09 | 0x0A | 0x0B | 0x0C | 0x0D | 0x0F => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            (Instruction::RotateRight(r), pc + 2)
        }
        0x0E => (Instruction::RotateRightIndirectHL, pc + 2),
        0x10 | 0x11 | 0x12 | 0x13 | 0x14 | 0x15 | 0x17 => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            (Instruction::RotateLeftThruCarry(r), pc + 2)
        }
        0x16 => (Instruction::RotateLeftIndirectHLThruCarry, pc + 2),
        0x18 | 0x19 | 0x1A | 0x1B | 0x1C | 0x1D | 0x1F => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            (Instruction::RotateRightThruCarry(r), pc + 2)
        }
        0x1E => (Instruction::RotateRightIndirectHLThruCarry, pc + 2),
        0x20 | 0x21 | 0x22 | 0x23 | 0x24 | 0x25 | 0x27 => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            (Instruction::ShiftLeft(r), pc + 2)
        }
        0x26 => (Instruction::ShiftLeftIndirectHL, pc + 2),
        0x28 | 0x29 | 0x2A | 0x2B | 0x2C | 0x2D | 0x2F => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            (Instruction::ShiftRight(r), pc + 2)
        }
        0x2E => (Instruction::ShiftRightIndirectHL, pc + 2),
        0x30 | 0x31 | 0x32 | 0x33 | 0x34 | 0x35 | 0x37 => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            (Instruction::Swap(r), pc + 2)
        }
        0x36 => (Instruction::SwapIndirectHL, pc + 2),
        0x38 | 0x39 | 0x3A | 0x3B | 0x3C | 0x3D | 0x3F => {
            let r = CpuRegister::from_low_opcode_bits(opcode)
                .expect("all of these opcodes should have a valid CPU register in bits 0-2");
            (Instruction::ShiftRightLogical(r), pc + 2)
        }
        0x3E => (Instruction::ShiftRightLogicalIndirectHL, pc + 2),
        opcode @ 0x40..=0x7F => {
            let bit = (opcode & 0x38) >> 3;
            let r = CpuRegister::from_low_opcode_bits(opcode);
            match r {
                Some(r) => (Instruction::TestBit(bit, r), pc + 2),
                None => (Instruction::TestBitIndirectHL(bit), pc + 2),
            }
        }
        opcode @ 0x80..=0xBF => {
            let bit = (opcode & 0x38) >> 3;
            let r = CpuRegister::from_low_opcode_bits(opcode);
            match r {
                Some(r) => (Instruction::ResetBit(bit, r), pc + 2),
                None => (Instruction::ResetBitIndirectHL(bit), pc + 2),
            }
        }
        opcode @ 0xC0..=0xFF => {
            let bit = (opcode & 0x38) >> 3;
            let r = CpuRegister::from_low_opcode_bits(opcode);
            match r {
                Some(r) => (Instruction::SetBit(bit, r), pc + 2),
                None => (Instruction::SetBitIndirectHL(bit), pc + 2),
            }
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
