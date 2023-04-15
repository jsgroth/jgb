mod mbc3;
mod mbc7;

use crate::memory::address;
use crate::memory::mapper::mbc7::Mbc7Eeprom;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt::Formatter;
use std::rc::Rc;
use std::time::SystemTime;

use crate::input::AccelerometerState;
pub(crate) use mbc3::RealTimeClock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapperType {
    None,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
    MBC7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RamMapResult {
    // Relative address into the full RAM array
    RamAddress(u32),
    // The RAM address is currently mapped to a cartridge-internal register
    MapperRegister,
    // The RAM address is invalid or RAM access is disabled
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Mbc7RamStatus {
    Enabled,
    PreEnabled,
    Disabled,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Mapper {
    None,
    MBC1 {
        rom_bank_bit_mask: u8,
        ram_bank_bit_mask: u8,
        ram_enable: u8,
        rom_bank_number: u8,
        ram_bank_number: u8,
        banking_mode_select: u8,
    },
    MBC2 {
        rom_bank_bit_mask: u8,
        ram_enable: u8,
        rom_bank_number: u8,
    },
    MBC3 {
        rom_bank_bit_mask: u8,
        ram_enable: u8,
        rom_bank_number: u8,
        ram_bank_number: u8,
        real_time_clock: Option<RealTimeClock>,
    },
    MBC5 {
        rom_bank_bit_mask: u16,
        ram_bank_bit_mask: u8,
        ram_enable: u8,
        rom_bank_number: u16,
        ram_bank_number: u8,
    },
    MBC7 {
        eeprom: Mbc7Eeprom,
        rom_bank_bit_mask: u16,
        ram_status: Mbc7RamStatus,
        rom_bank_number: u16,
        #[serde(skip)]
        live_accelerometer_state: Rc<RefCell<AccelerometerState>>,
        latched_accelerometer_state: Option<AccelerometerState>,
    },
}

impl Mapper {
    pub(crate) fn new(
        mapper_type: MapperType,
        mapper_features: MapperFeatures,
        rtc: Option<RealTimeClock>,
        rom_size: u32,
        ram_size: u32,
        loaded_ram: Option<&Vec<u8>>,
        accelerometer_state: Rc<RefCell<AccelerometerState>>,
    ) -> Self {
        let rom_bank_bit_mask = if rom_size >= 1 << 14 {
            ((rom_size >> 14) - 1) as u16
        } else {
            0
        };
        let ram_bank_bit_mask = if ram_size >= 1 << 13 {
            ((ram_size >> 13) - 1) as u8
        } else {
            0
        };

        log::debug!("setting ROM bit bask to {rom_bank_bit_mask:02X} for size {rom_size}");
        log::debug!("setting RAM bit mask to {ram_bank_bit_mask:02X} for size {ram_size}");

        match mapper_type {
            MapperType::None => Self::None,
            MapperType::MBC1 => Self::MBC1 {
                rom_bank_bit_mask: rom_bank_bit_mask as u8,
                ram_bank_bit_mask,
                ram_enable: 0x00,
                rom_bank_number: 0x00,
                ram_bank_number: 0x00,
                banking_mode_select: 0x00,
            },
            MapperType::MBC2 => Self::MBC2 {
                rom_bank_bit_mask: rom_bank_bit_mask as u8,
                ram_enable: 0x00,
                rom_bank_number: 0x00,
            },
            MapperType::MBC3 => {
                let real_time_clock = mapper_features.has_rtc.then(|| match rtc {
                    Some(mut rtc) => {
                        rtc.update(SystemTime::now());
                        rtc
                    }
                    None => RealTimeClock::new(SystemTime::now()),
                });
                Self::MBC3 {
                    rom_bank_bit_mask: rom_bank_bit_mask as u8,
                    ram_enable: 0x00,
                    rom_bank_number: 0x00,
                    ram_bank_number: 0x00,
                    real_time_clock,
                }
            }
            MapperType::MBC5 => Self::MBC5 {
                rom_bank_bit_mask,
                ram_bank_bit_mask,
                ram_enable: 0x00,
                rom_bank_number: 0x01,
                ram_bank_number: 0x00,
            },
            MapperType::MBC7 => Self::MBC7 {
                eeprom: Mbc7Eeprom::new(loaded_ram),
                rom_bank_bit_mask,
                ram_status: Mbc7RamStatus::Disabled,
                rom_bank_number: 0x01,
                live_accelerometer_state: accelerometer_state,
                latched_accelerometer_state: None,
            },
        }
    }

    pub(crate) fn map_rom_address(&self, address: u16) -> u32 {
        match self {
            Self::None => u32::from(address),
            &Self::MBC1 {
                rom_bank_bit_mask,
                rom_bank_number,
                ram_bank_number,
                banking_mode_select,
                ..
            } => {
                let rom_bank_number = if rom_bank_number == 0x00 {
                    0x01
                } else {
                    rom_bank_number
                };

                match address {
                    address @ 0x0000..=0x3FFF => {
                        if banking_mode_select == 0x00 {
                            u32::from(address)
                        } else {
                            let bank_number = (ram_bank_number << 5) & rom_bank_bit_mask;
                            u32::from(address) + (u32::from(bank_number) << 14)
                        }
                    }
                    address @ 0x4000..=0x7FFF => {
                        if banking_mode_select == 0x00 {
                            let bank_number = rom_bank_number & rom_bank_bit_mask;
                            u32::from(address - 0x4000) + (u32::from(bank_number) << 14)
                        } else {
                            let bank_number = (rom_bank_number | (ram_bank_number << 5)) & rom_bank_bit_mask;
                            u32::from(address - 0x4000) + (u32::from(bank_number) << 14)
                        }
                    }
                    _ => panic!("mapper called for address outside of cartridge address range: {address:04X}")
                }
            }
            &Self::MBC2 {
                rom_bank_bit_mask,
                rom_bank_number,
                ..
            }
            | &Self::MBC3 {
                rom_bank_bit_mask,
                rom_bank_number,
                ..
            } => {
                let rom_bank_number = if rom_bank_number == 0x00 {
                    0x01
                } else {
                    rom_bank_number
                };

                match address {
                    address @ 0x0000..=0x3FFF => u32::from(address),
                    address @ 0x4000..=0x7FFF => {
                        let bank_number = rom_bank_number & rom_bank_bit_mask;
                        u32::from(address - 0x4000) + (u32::from(bank_number) << 14)
                    }
                    _ => panic!("mapper called for address outside of cartridge address range: {address:04X}")
                }
            }
            &Self::MBC5 {
                rom_bank_bit_mask,
                rom_bank_number,
                ..
            }
            | &Self::MBC7 {
                rom_bank_bit_mask,
                rom_bank_number,
                ..
            } => {
                // ROM bank 0 is actually bank 0 in MBC5 and MBC7

                match address {
                    address @ 0x0000..=0x3FFF => u32::from(address),
                    address @ 0x4000..=0x7FFF => {
                        let bank_number = rom_bank_number & rom_bank_bit_mask;
                        u32::from(address - 0x4000) + (u32::from(bank_number) << 14)
                    }
                    _ => panic!("mapper called for address outside of cartridge address range: {address:04X}")
                }
            }
        }
    }

    // ROM writes don't actually modify the ROM (it is read-only after all) but they do modify
    // cartridge registers
    pub(crate) fn write_rom_address(&mut self, address: u16, value: u8) {
        match self {
            Self::None => {}
            Self::MBC1 {
                ram_enable,
                rom_bank_number,
                ram_bank_number,
                banking_mode_select,
                ..
            } => match address {
                _address @ 0x0000..=0x1FFF => {
                    log::trace!("ram_enable changed to {value:02X}");
                    *ram_enable = value;
                }
                _address @ 0x2000..=0x3FFF => {
                    log::trace!("rom_bank_number changed to {value:02X}");
                    *rom_bank_number = value & 0x1F;
                }
                _address @ 0x4000..=0x5FFF => {
                    log::trace!("ram_bank_number changed to {value:02X}");
                    *ram_bank_number = value & 0x03;
                }
                _address @ 0x6000..=0x7FFF => {
                    log::trace!("banking_mode_select changed to {value:02X}");
                    *banking_mode_select = value & 0x01;
                }
                _ => panic!("invalid ROM write address in MBC1 mapper: {address:04X}"),
            },
            Self::MBC2 {
                ram_enable,
                rom_bank_number,
                ..
            } => match address {
                address @ 0x0000..=0x3FFF => {
                    if address & 0x0100 != 0 {
                        *rom_bank_number = value & 0x0F;
                    } else {
                        *ram_enable = value;
                    }
                }
                _address @ 0x4000..=0x7FFF => {}
                _ => panic!("invalid ROM write address in MBC2 mapper: {address:04X}"),
            },
            Self::MBC3 {
                ram_enable,
                rom_bank_number,
                ram_bank_number,
                real_time_clock,
                ..
            } => match address {
                _address @ 0x0000..=0x1FFF => {
                    *ram_enable = value;
                }
                _address @ 0x2000..=0x3FFF => {
                    *rom_bank_number = value & 0x7F;
                }
                _address @ 0x4000..=0x5FFF => {
                    *ram_bank_number = value;
                }
                _address @ 0x6000..=0x7FFF => {
                    if let Some(real_time_clock) = real_time_clock {
                        real_time_clock.process_register_write(value);
                    }
                }
                _ => panic!("invalid ROM write address in MBC3 mapper: {address:04X}"),
            },
            Self::MBC5 {
                ram_enable,
                rom_bank_number,
                ram_bank_number,
                ..
            } => match address {
                _address @ 0x0000..=0x1FFF => {
                    *ram_enable = value;
                }
                _address @ 0x2000..=0x2FFF => {
                    *rom_bank_number = (*rom_bank_number & 0xFF00) | u16::from(value);
                }
                _address @ 0x3000..=0x3FFF => {
                    *rom_bank_number = (u16::from(value) << 8) | (*rom_bank_number & 0x00FF);
                }
                _address @ 0x4000..=0x5FFF => {
                    *ram_bank_number = value;
                }
                _address @ 0x6000..=0x7FFF => {}
                _ => panic!("invalid ROM write address in MBC5 mapper: {address:04X}"),
            },
            Self::MBC7 {
                rom_bank_number,
                ram_status,
                ..
            } => match address {
                _address @ 0x0000..=0x1FFF => {
                    if value == 0x0A {
                        *ram_status = Mbc7RamStatus::PreEnabled;
                    } else {
                        *ram_status = Mbc7RamStatus::Disabled;
                    }
                }
                _address @ 0x2000..=0x3FFF => {
                    *rom_bank_number = value.into();
                }
                _address @ 0x4000..=0x5FFF => {
                    if *ram_status == Mbc7RamStatus::PreEnabled && value == 0x40 {
                        *ram_status = Mbc7RamStatus::Enabled;
                    }
                }
                _address @ 0x6000..=0x7FFF => {}
                _ => panic!("invalid ROM write address in MBC7 mapper: {address:04X}"),
            },
        }
    }

    pub(crate) fn map_ram_address(&self, address: u16) -> RamMapResult {
        let relative_address = address - address::EXTERNAL_RAM_START;

        match self {
            Self::None => RamMapResult::RamAddress(u32::from(relative_address)),
            &Self::MBC1 {
                ram_bank_bit_mask,
                ram_enable,
                ram_bank_number,
                banking_mode_select,
                ..
            } => {
                if ram_enable & 0x0A == 0x0A {
                    if banking_mode_select == 0x00 {
                        RamMapResult::RamAddress(u32::from(relative_address))
                    } else {
                        let bank_number = ram_bank_number & ram_bank_bit_mask;
                        RamMapResult::RamAddress(
                            u32::from(relative_address) + (u32::from(bank_number) << 13),
                        )
                    }
                } else {
                    RamMapResult::None
                }
            }
            &Self::MBC2 { ram_enable, .. } => {
                if ram_enable == 0x0A {
                    RamMapResult::RamAddress(u32::from(relative_address))
                } else {
                    RamMapResult::None
                }
            }
            &Self::MBC3 {
                ram_enable,
                ram_bank_number,
                ..
            } => {
                if ram_enable & 0x0A == 0x0A {
                    match ram_bank_number {
                        ram_bank_number @ 0x00..=0x03 => RamMapResult::RamAddress(
                            u32::from(relative_address) + (u32::from(ram_bank_number) << 13),
                        ),
                        // 0x08-0x0C are RTC registers
                        _ram_bank_number @ 0x08..=0x0C => RamMapResult::MapperRegister,
                        _ => RamMapResult::None,
                    }
                } else {
                    RamMapResult::None
                }
            }
            &Self::MBC5 {
                ram_bank_bit_mask,
                ram_enable,
                ram_bank_number,
                ..
            } => {
                if ram_enable & 0x0A == 0x0A {
                    let bank_number = ram_bank_number & ram_bank_bit_mask;
                    match bank_number {
                        bank_number @ 0x00..=0x03 => RamMapResult::RamAddress(
                            u32::from(relative_address) + (u32::from(bank_number) << 13),
                        ),
                        // 0x08-0x0C are RTC registers
                        _bank_number @ 0x08..=0x0C => RamMapResult::MapperRegister,
                        _ => RamMapResult::None,
                    }
                } else {
                    RamMapResult::None
                }
            }
            &Self::MBC7 { ram_status, .. } => match ram_status {
                Mbc7RamStatus::Enabled => RamMapResult::MapperRegister,
                _ => RamMapResult::None,
            },
        }
    }

    pub(crate) fn read_ram_addressed_register(&self, address: u16) -> Option<u8> {
        match self {
            Self::MBC3 {
                ram_bank_number,
                real_time_clock: Some(real_time_clock),
                ..
            } => real_time_clock.handle_ram_read(*ram_bank_number),
            Self::MBC7 {
                eeprom,
                latched_accelerometer_state,
                ..
            } if (0xA000..=0xAFFF).contains(&address) => {
                match address & 0x00F0 {
                    0x0020 => {
                        let x = latched_accelerometer_state
                            .unwrap_or(AccelerometerState::default())
                            .x;
                        Some((x & 0x00FF) as u8)
                    }
                    0x0030 => {
                        let x = latched_accelerometer_state
                            .unwrap_or(AccelerometerState::default())
                            .x;
                        Some((x >> 8) as u8)
                    }
                    0x0040 => {
                        let y = latched_accelerometer_state
                            .unwrap_or(AccelerometerState::default())
                            .y;
                        Some((y & 0x00FF) as u8)
                    }
                    0x0050 => {
                        let y = latched_accelerometer_state
                            .unwrap_or(AccelerometerState::default())
                            .y;
                        Some((y >> 8) as u8)
                    }
                    // Unused register that always reads out 0, possibly meant for accelerometer Z
                    0x0060 => Some(0x00),
                    0x0080 => {
                        log::trace!("Reading from EEPROM, address={address:04X}");
                        let value = eeprom.handle_read();
                        log::trace!("Read {value:02X}");
                        Some(value)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(crate) fn write_ram_addressed_register(&mut self, address: u16, value: u8) {
        match self {
            Self::MBC3 {
                ram_bank_number,
                real_time_clock: Some(real_time_clock),
                ..
            } => {
                real_time_clock.handle_ram_write(*ram_bank_number, value);
            }
            Self::MBC7 {
                eeprom,
                live_accelerometer_state,
                latched_accelerometer_state,
                ..
            } if (0xA000..=0xAFFF).contains(&address) => match address & 0x00F0 {
                0x0000 => {
                    if value == 0x55 {
                        *latched_accelerometer_state = None;
                    }
                }
                0x0010 => {
                    if latched_accelerometer_state.is_none() && value == 0xAA {
                        *latched_accelerometer_state = Some(*live_accelerometer_state.borrow());
                    }
                }
                0x0080 => {
                    log::trace!("Writing to EEPROM, value={value:02X}");
                    eeprom.handle_write(value);
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub(crate) fn update_rtc(&mut self) {
        let Self::MBC3 { real_time_clock: Some(real_time_clock), .. } = self else { return };
        real_time_clock.update(SystemTime::now());
    }

    pub(crate) fn get_clock(&self) -> Option<&RealTimeClock> {
        match self {
            Self::MBC3 {
                real_time_clock, ..
            } => real_time_clock.as_ref(),
            _ => None,
        }
    }

    /// Get a reference to the mapper chip's raw EEPROM, if any (only MBC7 mapper has such a chip)
    pub(crate) fn get_eeprom_memory(&self) -> Option<&[u8]> {
        match self {
            Self::MBC7 { eeprom, .. } => Some(eeprom.raw_memory()),
            _ => None,
        }
    }

    pub(crate) fn move_unserializable_fields_from(&mut self, other: Self) {
        if let (
            Self::MBC7 {
                live_accelerometer_state,
                ..
            },
            Self::MBC7 {
                live_accelerometer_state: other_live_accelerometer_state,
                ..
            },
        ) = (self, other)
        {
            *live_accelerometer_state = other_live_accelerometer_state;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MapperFeatures {
    pub(crate) has_ram: bool,
    pub(crate) has_battery: bool,
    pub(crate) has_rtc: bool,
}

impl std::fmt::Display for MapperFeatures {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "has_ram={}, has_battery={}, has_rtc={}",
            self.has_ram, self.has_battery, self.has_rtc
        )
    }
}

pub(crate) fn parse_byte(mapper_byte: u8) -> Option<(MapperType, MapperFeatures)> {
    let (mapper_type, has_ram, has_battery) = match mapper_byte {
        0x00 => (MapperType::None, false, false),
        0x01 => (MapperType::MBC1, false, false),
        0x02 => (MapperType::MBC1, true, false),
        0x03 => (MapperType::MBC1, true, true),
        0x05 => (MapperType::MBC2, true, false),
        0x06 => (MapperType::MBC2, true, true),
        0x0F => (MapperType::MBC3, false, true),
        // 0x10 is w/ RTC, 0x13 is w/o RTC
        0x10 | 0x13 => (MapperType::MBC3, true, true),
        0x11 => (MapperType::MBC3, false, false),
        0x12 => (MapperType::MBC3, true, false),
        // 0x19 is w/o rumble, 0x1C is w/ rumble
        0x19 | 0x1C => (MapperType::MBC5, false, false),
        // 0x1A is w/o rumble, 0x1D is w/ rumble
        0x1A | 0x1D => (MapperType::MBC5, true, false),
        // 0x1B is w/o rumble, 0x1E is w/ rumble
        0x1B | 0x1E => (MapperType::MBC5, true, true),
        0x22 => (MapperType::MBC7, true, true),
        _ => return None,
    };

    let has_rtc = mapper_byte == 0x0F || mapper_byte == 0x10;

    let features = MapperFeatures {
        has_ram,
        has_battery,
        has_rtc,
    };
    Some((mapper_type, features))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapper_features() -> MapperFeatures {
        MapperFeatures {
            has_ram: false,
            has_battery: false,
            has_rtc: false,
        }
    }

    #[test]
    fn mbc1_mapper_rom_small() {
        // 256KB ROM
        let mut mapper = Mapper::new(
            MapperType::MBC1,
            mapper_features(),
            None,
            1 << 18,
            0,
            None,
            Rc::default(),
        );

        assert_eq!(0x0000, mapper.map_rom_address(0x0000));
        assert_eq!(0x3FFF, mapper.map_rom_address(0x3FFF));
        assert_eq!(0x4000, mapper.map_rom_address(0x4000));
        assert_eq!(0x7FFF, mapper.map_rom_address(0x7FFF));

        // Set ROM bank number
        mapper.write_rom_address(0x2000, 0x05);

        assert_eq!(0x0000, mapper.map_rom_address(0x0000));
        assert_eq!(0x3FFF, mapper.map_rom_address(0x3FFF));
        assert_eq!(0x14000, mapper.map_rom_address(0x4000));
        assert_eq!(0x15324, mapper.map_rom_address(0x5324));
        assert_eq!(0x17FFF, mapper.map_rom_address(0x7FFF));

        // Set ROM bank number higher than the highest bank number, should get masked to 0x05
        mapper.write_rom_address(0x2000, 0x15);

        assert_eq!(0x0000, mapper.map_rom_address(0x0000));
        assert_eq!(0x3FFF, mapper.map_rom_address(0x3FFF));
        assert_eq!(0x14000, mapper.map_rom_address(0x4000));
        assert_eq!(0x15324, mapper.map_rom_address(0x5324));
        assert_eq!(0x17FFF, mapper.map_rom_address(0x7FFF));

        // Test that banking select mode + RAM bank number is ignored
        mapper.write_rom_address(0x6000, 0x01);
        mapper.write_rom_address(0x4000, 0x01);

        assert_eq!(0x0000, mapper.map_rom_address(0x0000));
        assert_eq!(0x3FFF, mapper.map_rom_address(0x3FFF));
        assert_eq!(0x14000, mapper.map_rom_address(0x4000));
        assert_eq!(0x15324, mapper.map_rom_address(0x5324));
        assert_eq!(0x17FFF, mapper.map_rom_address(0x7FFF));
    }

    #[test]
    fn mbc1_mapper_rom_large() {
        // 2MB ROM
        let mut mapper = Mapper::new(
            MapperType::MBC1,
            mapper_features(),
            None,
            1 << 21,
            0,
            None,
            Rc::default(),
        );

        assert_eq!(0x0000, mapper.map_rom_address(0x0000));
        assert_eq!(0x3FFF, mapper.map_rom_address(0x3FFF));
        assert_eq!(0x4000, mapper.map_rom_address(0x4000));
        assert_eq!(0x7FFF, mapper.map_rom_address(0x7FFF));

        // Set banking select mode, ROM bank number, RAM bank number
        mapper.write_rom_address(0x6000, 0x01);
        mapper.write_rom_address(0x2000, 0x05);
        mapper.write_rom_address(0x4000, 0x02);

        assert_eq!(0x100000, mapper.map_rom_address(0x0000));
        assert_eq!(0x103FFF, mapper.map_rom_address(0x3FFF));
        assert_eq!(0x114000, mapper.map_rom_address(0x4000));
        assert_eq!(0x115234, mapper.map_rom_address(0x5234));
        assert_eq!(0x117FFF, mapper.map_rom_address(0x7FFF));

        // Set ROM bank number to 00, should be treated as 01
        mapper.write_rom_address(0x2000, 0x00);

        assert_eq!(0x100000, mapper.map_rom_address(0x0000));
        assert_eq!(0x103FFF, mapper.map_rom_address(0x3FFF));
        assert_eq!(0x104000, mapper.map_rom_address(0x4000));
        assert_eq!(0x105234, mapper.map_rom_address(0x5234));
        assert_eq!(0x107FFF, mapper.map_rom_address(0x7FFF));
    }

    #[test]
    fn mbc1_mapper_ram() {
        // 256KB ROM, 8KB RAM
        let mut mapper = Mapper::new(
            MapperType::MBC1,
            mapper_features(),
            None,
            1 << 18,
            8192,
            None,
            Rc::default(),
        );

        // Enable RAM
        mapper.write_rom_address(0x0000, 0x0A);

        assert_eq!(
            RamMapResult::RamAddress(0x0000),
            mapper.map_ram_address(0xA000)
        );
        assert_eq!(
            RamMapResult::RamAddress(0x1000),
            mapper.map_ram_address(0xB000)
        );
        assert_eq!(
            RamMapResult::RamAddress(0x1234),
            mapper.map_ram_address(0xB234)
        );
    }
}
