use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
struct ChipSelect(bool);

impl ChipSelect {
    fn from_register_byte(byte: u8) -> Self {
        Self(byte & 0x80 != 0)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Clock(bool);

impl Clock {
    fn from_register_byte(byte: u8) -> Self {
        Self(byte & 0x40 != 0)
    }
}

impl From<Clock> for u8 {
    fn from(value: Clock) -> Self {
        u8::from(value.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct DataIn(bool);

impl DataIn {
    fn from_register_byte(byte: u8) -> Self {
        Self(byte & 0x02 != 0)
    }
}

impl From<DataIn> for u16 {
    fn from(value: DataIn) -> Self {
        u16::from(value.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct DataOut(bool);

impl From<DataOut> for u8 {
    fn from(value: DataOut) -> Self {
        u8::from(value.0)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum WriteStatus {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy)]
enum Operation {
    Read { address: u8 },
    Write { address: u8 },
    WriteAll,
    Erase { address: u8 },
    EraseAll,
    WriteEnable,
    WriteDisable,
}

impl Operation {
    fn from_opcode(opcode: u16) -> Self {
        match opcode & 0x0300 {
            0x0000 => match opcode & 0x00C0 {
                0x0000 => Self::WriteDisable,
                0x0040 => Self::WriteAll,
                0x0080 => Self::EraseAll,
                0x00C0 => Self::WriteEnable,
                _ => panic!("{opcode} & 0x00C0 was not 0x0000/0x0040/0x0080/0x00C0"),
            },
            0x0100 => Self::Write {
                address: (opcode & 0x007F) as u8,
            },
            0x0200 => Self::Read {
                address: (opcode & 0x007F) as u8,
            },
            0x0300 => Self::Erase {
                address: (opcode & 0x007F) as u8,
            },
            _ => panic!("{opcode} & 0x0300 was not 0x0000/0x0100/0x0200/0x0300"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum WriteType {
    SingleAddress(u8),
    All,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct OpInput {
    input_bits: u16,
    bits_remaining: u8,
}

impl OpInput {
    fn new() -> Self {
        Self {
            input_bits: 0,
            bits_remaining: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct WriteInput {
    input_bits: u16,
    bits_remaining: u8,
}

impl WriteInput {
    fn new() -> Self {
        Self {
            input_bits: 0,
            bits_remaining: 16,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ReadOutput {
    value: u16,
    current_bit: u8,
}

impl ReadOutput {
    fn new(value: u16) -> Self {
        Self {
            value,
            current_bit: 16,
        }
    }

    fn current_output(self) -> DataOut {
        if self.current_bit < 16 {
            DataOut(self.value & (1 << self.current_bit) != 0)
        } else {
            DataOut(false)
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum ChipState {
    Standby(WriteStatus),
    ReadingOp(WriteStatus, OpInput),
    ReadingData(WriteType, WriteInput),
    SendingOutput(WriteStatus, ReadOutput),
    Finished(WriteStatus),
}

impl ChipState {
    #[must_use]
    fn clock(self, chip_select: ChipSelect, data_in: DataIn, memory: &mut [u8; 256]) -> Self {
        match (self, chip_select, data_in) {
            (Self::Standby(write_status), ChipSelect(true), DataIn(true)) => {
                Self::ReadingOp(write_status, OpInput::new())
            }
            (Self::ReadingOp(write_status, mut op_input), ChipSelect(true), _) => {
                op_input.bits_remaining -= 1;
                op_input.input_bits |= u16::from(data_in) << op_input.bits_remaining;

                if op_input.bits_remaining > 0 {
                    Self::ReadingOp(write_status, op_input)
                } else {
                    let operation = Operation::from_opcode(op_input.input_bits);
                    match (operation, write_status) {
                        (Operation::Read { address }, _) => {
                            let memory_address = (2 * address) as usize;
                            let value = u16::from_be_bytes([
                                memory[memory_address],
                                memory[memory_address + 1],
                            ]);
                            Self::SendingOutput(write_status, ReadOutput::new(value))
                        }
                        (Operation::WriteEnable, _) => Self::Finished(WriteStatus::Enabled),
                        (Operation::WriteDisable, _) => Self::Finished(WriteStatus::Disabled),
                        (
                            Operation::Write { .. }
                            | Operation::WriteAll
                            | Operation::Erase { .. }
                            | Operation::EraseAll,
                            WriteStatus::Disabled,
                        ) => Self::Finished(WriteStatus::Disabled),
                        (Operation::Write { address }, WriteStatus::Enabled) => {
                            Self::ReadingData(WriteType::SingleAddress(address), WriteInput::new())
                        }
                        (Operation::WriteAll, WriteStatus::Enabled) => {
                            Self::ReadingData(WriteType::All, WriteInput::new())
                        }
                        (Operation::Erase { address }, WriteStatus::Enabled) => {
                            let memory_address = (2 * address) as usize;
                            memory[memory_address] = 0;
                            memory[memory_address + 1] = 0;
                            Self::Finished(WriteStatus::Enabled)
                        }
                        (Operation::EraseAll, WriteStatus::Enabled) => {
                            *memory = [0; 256];
                            Self::Finished(WriteStatus::Enabled)
                        }
                    }
                }
            }
            (Self::ReadingData(..), ChipSelect(false), _) => Self::Standby(WriteStatus::Enabled),
            (Self::ReadingData(write_type, mut write_input), ChipSelect(true), data_in) => {
                write_input.bits_remaining -= 1;
                write_input.input_bits |= u16::from(data_in) << write_input.bits_remaining;

                if write_input.bits_remaining > 0 {
                    Self::ReadingData(write_type, write_input)
                } else {
                    let value = write_input.input_bits;
                    let [high, low] = value.to_be_bytes();

                    match write_type {
                        WriteType::SingleAddress(address) => {
                            let memory_address = (2 * address) as usize;
                            memory[memory_address] = high;
                            memory[memory_address + 1] = low;
                            Self::Finished(WriteStatus::Enabled)
                        }
                        WriteType::All => {
                            for (mem_address, byte) in
                                memory.iter_mut().zip([high, low].into_iter().cycle())
                            {
                                *mem_address = byte;
                            }
                            Self::Finished(WriteStatus::Enabled)
                        }
                    }
                }
            }
            (
                Self::Standby(write_status)
                | Self::ReadingOp(write_status, ..)
                | Self::SendingOutput(write_status, ..)
                | Self::Finished(write_status),
                ChipSelect(false),
                _,
            )
            | (Self::Standby(write_status), ChipSelect(true), DataIn(false)) => {
                Self::Standby(write_status)
            }
            (Self::SendingOutput(write_status, mut read_output), ChipSelect(true), _) => {
                if read_output.current_bit > 0 {
                    read_output.current_bit -= 1;
                    Self::SendingOutput(write_status, read_output)
                } else {
                    Self::Finished(write_status)
                }
            }
            (Self::Finished(write_status), ChipSelect(true), _) => Self::Finished(write_status),
        }
    }
}

// Emulation of the MBC7 mapper's 93LC56 EEPROM chip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Mbc7Eeprom {
    #[serde(
        serialize_with = "crate::serialize::serialize_array",
        deserialize_with = "crate::serialize::deserialize_array"
    )]
    memory: [u8; 256],
    state: ChipState,
    last_clock: Clock,
}

impl Mbc7Eeprom {
    pub(crate) fn new(loaded_ram: Option<&Vec<u8>>) -> Self {
        let mut memory = [0; 256];

        match loaded_ram {
            Some(loaded_ram) if loaded_ram.len() == memory.len() => {
                memory.copy_from_slice(loaded_ram);
            }
            _ => {}
        }

        Self {
            memory,
            state: ChipState::Standby(WriteStatus::Disabled),
            last_clock: Clock(false),
        }
    }

    pub(crate) fn handle_read(&self) -> u8 {
        let data_out = match self.state {
            ChipState::SendingOutput(_, read_output) => read_output.current_output(),
            _ => DataOut(true),
        };
        0xBE | (u8::from(self.last_clock) << 6) | u8::from(data_out)
    }

    pub(crate) fn handle_write(&mut self, value: u8) {
        let chip_select = ChipSelect::from_register_byte(value);
        let clock = Clock::from_register_byte(value);
        let data_in = DataIn::from_register_byte(value);

        if !self.last_clock.0 && clock.0 {
            log::trace!("Clocking EEPROM, current state = {:?}", self.state);
            self.state = self.state.clock(chip_select, data_in, &mut self.memory);
            log::trace!("new state = {:?}", self.state);
        } else if !chip_select.0 {
            // CS going low sets the chip to standby even if it hasn't clocked
            match self.state {
                ChipState::ReadingOp(write_status, ..)
                | ChipState::SendingOutput(write_status, ..)
                | ChipState::Finished(write_status) => {
                    self.state = ChipState::Standby(write_status);
                }
                ChipState::ReadingData(..) => {
                    self.state = ChipState::Standby(WriteStatus::Enabled);
                }
                _ => {}
            }
        }
        self.last_clock = clock;
    }

    pub(crate) fn raw_memory(&self) -> &[u8; 256] {
        &self.memory
    }
}
