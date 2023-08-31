use crate::cpu::instructions::{JumpCondition, ModifyTarget, ReadTarget, WriteTarget};
use crate::cpu::registers::{CpuRegister, CpuRegisterPair};
use crate::cpu::{CpuRegisters, ExecutionMode};

#[test]
fn validate_cycles_required() {
    use crate::cpu::instructions::Instruction as I;

    let cr = CpuRegisters::new(ExecutionMode::GameBoy);

    // 8-bit load instructions
    assert_eq!(
        4,
        I::Load(WriteTarget::Register(CpuRegister::A), ReadTarget::Register(CpuRegister::B))
            .cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::Load(WriteTarget::Register(CpuRegister::A), ReadTarget::Immediate(0))
            .cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::Load(WriteTarget::Register(CpuRegister::A), ReadTarget::IndirectHL).cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::Load(WriteTarget::IndirectHL, ReadTarget::Register(CpuRegister::A)).cycles_required(&cr)
    );
    assert_eq!(12, I::Load(WriteTarget::IndirectHL, ReadTarget::Immediate(0)).cycles_required(&cr));
    assert_eq!(8, I::Load(WriteTarget::Accumulator, ReadTarget::IndirectBC).cycles_required(&cr));
    assert_eq!(8, I::Load(WriteTarget::Accumulator, ReadTarget::IndirectDE).cycles_required(&cr));
    assert_eq!(8, I::Load(WriteTarget::IndirectBC, ReadTarget::Accumulator).cycles_required(&cr));
    assert_eq!(8, I::Load(WriteTarget::IndirectDE, ReadTarget::Accumulator).cycles_required(&cr));
    assert_eq!(16, I::Load(WriteTarget::Accumulator, ReadTarget::Direct(0)).cycles_required(&cr));
    assert_eq!(16, I::Load(WriteTarget::Direct(0), ReadTarget::Accumulator).cycles_required(&cr));
    assert_eq!(8, I::Load(WriteTarget::Accumulator, ReadTarget::FFIndirectC).cycles_required(&cr));
    assert_eq!(8, I::Load(WriteTarget::FFIndirectC, ReadTarget::Accumulator).cycles_required(&cr));
    assert_eq!(12, I::Load(WriteTarget::Accumulator, ReadTarget::FFDirect(0)).cycles_required(&cr));
    assert_eq!(12, I::Load(WriteTarget::FFDirect(0), ReadTarget::Accumulator).cycles_required(&cr));
    assert_eq!(
        8,
        I::Load(WriteTarget::Accumulator, ReadTarget::IndirectHLDec).cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::Load(WriteTarget::IndirectHLDec, ReadTarget::Accumulator).cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::Load(WriteTarget::Accumulator, ReadTarget::IndirectHLInc).cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::Load(WriteTarget::IndirectHLInc, ReadTarget::Accumulator).cycles_required(&cr)
    );

    // 16-bit load instructions
    assert_eq!(12, I::LoadRegisterPairImmediate(CpuRegisterPair::BC, 0).cycles_required(&cr));
    assert_eq!(20, I::LoadDirectStackPointer(0).cycles_required(&cr));
    assert_eq!(8, I::LoadStackPointerHL.cycles_required(&cr));
    assert_eq!(16, I::PushStack(CpuRegisterPair::BC).cycles_required(&cr));
    assert_eq!(12, I::PopStack(CpuRegisterPair::BC).cycles_required(&cr));

    // 8-bit arithmetic/logical instructions
    assert_eq!(4, I::Add(ReadTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(8, I::Add(ReadTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::Add(ReadTarget::Immediate(0)).cycles_required(&cr));
    assert_eq!(4, I::AddWithCarry(ReadTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(8, I::AddWithCarry(ReadTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::AddWithCarry(ReadTarget::Immediate(0)).cycles_required(&cr));
    assert_eq!(4, I::Subtract(ReadTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(8, I::Subtract(ReadTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::Subtract(ReadTarget::Immediate(0)).cycles_required(&cr));
    assert_eq!(4, I::SubtractWithCarry(ReadTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(8, I::SubtractWithCarry(ReadTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::SubtractWithCarry(ReadTarget::Immediate(0)).cycles_required(&cr));
    assert_eq!(4, I::Compare(ReadTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(8, I::Compare(ReadTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::Compare(ReadTarget::Immediate(0)).cycles_required(&cr));
    assert_eq!(4, I::Increment(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(12, I::Increment(ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(4, I::Decrement(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(12, I::Decrement(ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(4, I::And(ReadTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(8, I::And(ReadTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::And(ReadTarget::Immediate(0)).cycles_required(&cr));
    assert_eq!(4, I::Or(ReadTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(8, I::Or(ReadTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::Or(ReadTarget::Immediate(0)).cycles_required(&cr));
    assert_eq!(4, I::Xor(ReadTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(8, I::Xor(ReadTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::Xor(ReadTarget::Immediate(0)).cycles_required(&cr));
    assert_eq!(4, I::ComplementCarryFlag.cycles_required(&cr));
    assert_eq!(4, I::SetCarryFlag.cycles_required(&cr));
    assert_eq!(4, I::DecimalAdjustAccumulator.cycles_required(&cr));
    assert_eq!(4, I::ComplementAccumulator.cycles_required(&cr));

    // 16-bit arithmetic instructions
    assert_eq!(8, I::AddHLRegister(CpuRegisterPair::BC).cycles_required(&cr));
    assert_eq!(8, I::IncRegisterPair(CpuRegisterPair::BC).cycles_required(&cr));
    assert_eq!(8, I::DecRegisterPair(CpuRegisterPair::BC).cycles_required(&cr));
    assert_eq!(16, I::AddSPImmediate(0).cycles_required(&cr));
    assert_eq!(12, I::LoadHLStackPointerOffset(0).cycles_required(&cr));

    // Bit rotate/shift instructions
    assert_eq!(4, I::RotateLeft(ModifyTarget::Accumulator).cycles_required(&cr));
    assert_eq!(4, I::RotateLeftThruCarry(ModifyTarget::Accumulator).cycles_required(&cr));
    assert_eq!(4, I::RotateRight(ModifyTarget::Accumulator).cycles_required(&cr));
    assert_eq!(4, I::RotateRightThruCarry(ModifyTarget::Accumulator).cycles_required(&cr));
    assert_eq!(8, I::RotateLeft(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(16, I::RotateLeft(ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(
        8,
        I::RotateLeftThruCarry(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr)
    );
    assert_eq!(16, I::RotateLeftThruCarry(ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::RotateRight(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(16, I::RotateRight(ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(
        8,
        I::RotateRightThruCarry(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr)
    );
    assert_eq!(16, I::RotateRightThruCarry(ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::ShiftLeft(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(16, I::ShiftLeft(ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(
        8,
        I::ArithmeticShiftRight(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr)
    );
    assert_eq!(16, I::ArithmeticShiftRight(ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(
        8,
        I::LogicalShiftRight(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr)
    );
    assert_eq!(16, I::LogicalShiftRight(ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::Swap(ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(16, I::Swap(ModifyTarget::IndirectHL).cycles_required(&cr));

    // Single bit instructions
    assert_eq!(8, I::TestBit(0, ReadTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(12, I::TestBit(0, ReadTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::SetBit(0, ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(16, I::SetBit(0, ModifyTarget::IndirectHL).cycles_required(&cr));
    assert_eq!(8, I::ResetBit(0, ModifyTarget::Register(CpuRegister::B)).cycles_required(&cr));
    assert_eq!(16, I::ResetBit(0, ModifyTarget::IndirectHL).cycles_required(&cr));

    // Unconditional control flow instructions
    assert_eq!(16, I::Jump(0).cycles_required(&cr));
    assert_eq!(4, I::JumpHL.cycles_required(&cr));
    assert_eq!(12, I::RelativeJump(0).cycles_required(&cr));
    assert_eq!(24, I::Call(0).cycles_required(&cr));
    assert_eq!(16, I::Return.cycles_required(&cr));
    assert_eq!(16, I::ReturnFromInterruptHandler.cycles_required(&cr));
    assert_eq!(16, I::RestartCall(0).cycles_required(&cr));
    assert_eq!(4, I::DisableInterrupts.cycles_required(&cr));
    assert_eq!(4, I::EnableInterrupts.cycles_required(&cr));
    assert_eq!(4, I::NoOp.cycles_required(&cr));
    assert_eq!(4, I::Halt.cycles_required(&cr));

    // Conditional control flow instructions
    let all_flags_false = CpuRegisters { flags: 0x00, ..CpuRegisters::new(ExecutionMode::GameBoy) };

    assert_eq!(12, I::JumpCond(JumpCondition::Z, 0).cycles_required(&all_flags_false));
    assert_eq!(16, I::JumpCond(JumpCondition::NZ, 0).cycles_required(&all_flags_false));

    assert_eq!(8, I::RelativeJumpCond(JumpCondition::Z, 0).cycles_required(&all_flags_false));
    assert_eq!(12, I::RelativeJumpCond(JumpCondition::NZ, 0).cycles_required(&all_flags_false));

    assert_eq!(12, I::CallCond(JumpCondition::Z, 0).cycles_required(&all_flags_false));
    assert_eq!(24, I::CallCond(JumpCondition::NZ, 0).cycles_required(&all_flags_false));

    assert_eq!(8, I::ReturnCond(JumpCondition::Z).cycles_required(&all_flags_false));
    assert_eq!(20, I::ReturnCond(JumpCondition::NZ).cycles_required(&all_flags_false));
}
