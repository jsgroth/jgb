use crate::cpu::instructions::JumpCondition;
use crate::cpu::registers::{CpuRegister, CpuRegisterPair};
use crate::cpu::{CpuRegisters, ExecutionMode};

#[test]
fn validate_cycles_required() {
    use crate::cpu::instructions::Instruction as I;

    let cr = CpuRegisters::new(ExecutionMode::GameBoy);

    // 8-bit load instructions
    assert_eq!(
        4,
        I::LoadRegisterRegister(CpuRegister::A, CpuRegister::B).cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::LoadRegisterImmediate(CpuRegister::A, 0).cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::LoadRegisterIndirectHL(CpuRegister::A).cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::LoadIndirectHLRegister(CpuRegister::A).cycles_required(&cr)
    );
    assert_eq!(12, I::LoadIndirectHLImmediate(0).cycles_required(&cr));
    assert_eq!(8, I::LoadAccumulatorIndirectBC.cycles_required(&cr));
    assert_eq!(8, I::LoadAccumulatorIndirectDE.cycles_required(&cr));
    assert_eq!(8, I::LoadIndirectBCAccumulator.cycles_required(&cr));
    assert_eq!(8, I::LoadIndirectDEAccumulator.cycles_required(&cr));
    assert_eq!(16, I::LoadAccumulatorDirect16(0).cycles_required(&cr));
    assert_eq!(16, I::LoadDirect16Accumulator(0).cycles_required(&cr));
    assert_eq!(8, I::LoadAccumulatorIndirectC.cycles_required(&cr));
    assert_eq!(8, I::LoadIndirectCAccumulator.cycles_required(&cr));
    assert_eq!(12, I::LoadAccumulatorDirect8(0).cycles_required(&cr));
    assert_eq!(12, I::LoadDirect8Accumulator(0).cycles_required(&cr));
    assert_eq!(8, I::LoadAccumulatorIndirectHLDec.cycles_required(&cr));
    assert_eq!(8, I::LoadIndirectHLDecAccumulator.cycles_required(&cr));
    assert_eq!(8, I::LoadAccumulatorIndirectHLInc.cycles_required(&cr));
    assert_eq!(8, I::LoadIndirectHLIncAccumulator.cycles_required(&cr));

    // 16-bit load instructions
    assert_eq!(
        12,
        I::LoadRegisterPairImmediate(CpuRegisterPair::BC, 0).cycles_required(&cr)
    );
    assert_eq!(20, I::LoadDirectStackPointer(0).cycles_required(&cr));
    assert_eq!(8, I::LoadStackPointerHL.cycles_required(&cr));
    assert_eq!(16, I::PushStack(CpuRegisterPair::BC).cycles_required(&cr));
    assert_eq!(12, I::PopStack(CpuRegisterPair::BC).cycles_required(&cr));

    // 8-bit arithmetic/logical instructions
    assert_eq!(4, I::AddRegister(CpuRegister::B).cycles_required(&cr));
    assert_eq!(8, I::AddIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::AddImmediate(0).cycles_required(&cr));
    assert_eq!(4, I::AddCarryRegister(CpuRegister::B).cycles_required(&cr));
    assert_eq!(8, I::AddCarryIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::AddCarryImmediate(0).cycles_required(&cr));
    assert_eq!(4, I::SubtractRegister(CpuRegister::B).cycles_required(&cr));
    assert_eq!(8, I::SubtractIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::SubtractImmediate(0).cycles_required(&cr));
    assert_eq!(
        4,
        I::SubtractCarryRegister(CpuRegister::B).cycles_required(&cr)
    );
    assert_eq!(8, I::SubtractCarryIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::SubtractCarryImmediate(0).cycles_required(&cr));
    assert_eq!(4, I::CompareRegister(CpuRegister::B).cycles_required(&cr));
    assert_eq!(8, I::CompareIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::CompareImmediate(0).cycles_required(&cr));
    assert_eq!(4, I::IncRegister(CpuRegister::B).cycles_required(&cr));
    assert_eq!(12, I::IncIndirectHL.cycles_required(&cr));
    assert_eq!(4, I::DecRegister(CpuRegister::B).cycles_required(&cr));
    assert_eq!(12, I::DecIndirectHL.cycles_required(&cr));
    assert_eq!(4, I::AndRegister(CpuRegister::B).cycles_required(&cr));
    assert_eq!(8, I::AndIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::AndImmediate(0).cycles_required(&cr));
    assert_eq!(4, I::OrRegister(CpuRegister::B).cycles_required(&cr));
    assert_eq!(8, I::OrIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::OrImmediate(0).cycles_required(&cr));
    assert_eq!(4, I::XorRegister(CpuRegister::B).cycles_required(&cr));
    assert_eq!(8, I::XorIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::XorImmediate(0).cycles_required(&cr));
    assert_eq!(4, I::ComplementCarryFlag.cycles_required(&cr));
    assert_eq!(4, I::SetCarryFlag.cycles_required(&cr));
    assert_eq!(4, I::DecimalAdjustAccumulator.cycles_required(&cr));
    assert_eq!(4, I::ComplementAccumulator.cycles_required(&cr));

    // 16-bit arithmetic instructions
    assert_eq!(
        8,
        I::AddHLRegister(CpuRegisterPair::BC).cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::IncRegisterPair(CpuRegisterPair::BC).cycles_required(&cr)
    );
    assert_eq!(
        8,
        I::DecRegisterPair(CpuRegisterPair::BC).cycles_required(&cr)
    );
    assert_eq!(16, I::AddSPImmediate(0).cycles_required(&cr));
    assert_eq!(12, I::LoadHLStackPointerOffset(0).cycles_required(&cr));

    // Bit rotate/shift instructions
    assert_eq!(4, I::RotateLeftAccumulator.cycles_required(&cr));
    assert_eq!(4, I::RotateLeftAccumulatorThruCarry.cycles_required(&cr));
    assert_eq!(4, I::RotateRightAccumulator.cycles_required(&cr));
    assert_eq!(4, I::RotateRightAccumulatorThruCarry.cycles_required(&cr));
    assert_eq!(8, I::RotateLeft(CpuRegister::B).cycles_required(&cr));
    assert_eq!(16, I::RotateLeftIndirectHL.cycles_required(&cr));
    assert_eq!(
        8,
        I::RotateLeftThruCarry(CpuRegister::B).cycles_required(&cr)
    );
    assert_eq!(16, I::RotateLeftIndirectHLThruCarry.cycles_required(&cr));
    assert_eq!(8, I::RotateRight(CpuRegister::B).cycles_required(&cr));
    assert_eq!(16, I::RotateRightIndirectHL.cycles_required(&cr));
    assert_eq!(
        8,
        I::RotateRightThruCarry(CpuRegister::B).cycles_required(&cr)
    );
    assert_eq!(16, I::RotateRightIndirectHLThruCarry.cycles_required(&cr));
    assert_eq!(8, I::ShiftLeft(CpuRegister::B).cycles_required(&cr));
    assert_eq!(16, I::ShiftLeftIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::ShiftRight(CpuRegister::B).cycles_required(&cr));
    assert_eq!(16, I::ShiftRightIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::ShiftRightLogical(CpuRegister::B).cycles_required(&cr));
    assert_eq!(16, I::ShiftRightLogicalIndirectHL.cycles_required(&cr));
    assert_eq!(8, I::Swap(CpuRegister::B).cycles_required(&cr));
    assert_eq!(16, I::SwapIndirectHL.cycles_required(&cr));

    // Single bit instructions
    assert_eq!(8, I::TestBit(0, CpuRegister::B).cycles_required(&cr));
    assert_eq!(12, I::TestBitIndirectHL(0).cycles_required(&cr));
    assert_eq!(8, I::SetBit(0, CpuRegister::B).cycles_required(&cr));
    assert_eq!(16, I::SetBitIndirectHL(0).cycles_required(&cr));
    assert_eq!(8, I::ResetBit(0, CpuRegister::B).cycles_required(&cr));
    assert_eq!(16, I::ResetBitIndirectHL(0).cycles_required(&cr));

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
    let all_flags_false = CpuRegisters {
        flags: 0x00,
        ..CpuRegisters::new(ExecutionMode::GameBoy)
    };

    assert_eq!(
        12,
        I::JumpCond(JumpCondition::Z, 0).cycles_required(&all_flags_false)
    );
    assert_eq!(
        16,
        I::JumpCond(JumpCondition::NZ, 0).cycles_required(&all_flags_false)
    );

    assert_eq!(
        8,
        I::RelativeJumpCond(JumpCondition::Z, 0).cycles_required(&all_flags_false)
    );
    assert_eq!(
        12,
        I::RelativeJumpCond(JumpCondition::NZ, 0).cycles_required(&all_flags_false)
    );

    assert_eq!(
        12,
        I::CallCond(JumpCondition::Z, 0).cycles_required(&all_flags_false)
    );
    assert_eq!(
        24,
        I::CallCond(JumpCondition::NZ, 0).cycles_required(&all_flags_false)
    );

    assert_eq!(
        8,
        I::ReturnCond(JumpCondition::Z).cycles_required(&all_flags_false)
    );
    assert_eq!(
        20,
        I::ReturnCond(JumpCondition::NZ).cycles_required(&all_flags_false)
    );
}
