use crate::memory::address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapperType {
    None,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
}

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
        ram_bank_bit_mask: u8,
        ram_enable: u8,
        rom_bank_number: u8,
        ram_bank_number: u8,
    },
    MBC5 {
        rom_bank_bit_mask: u16,
        ram_bank_bit_mask: u8,
        ram_enable: u8,
        rom_bank_number: u16,
        ram_bank_number: u8,
    },
}

impl Mapper {
    pub(crate) fn new(mapper_type: MapperType, rom_size: u32, ram_size: u32) -> Self {
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
            MapperType::MBC3 => Self::MBC3 {
                rom_bank_bit_mask: rom_bank_bit_mask as u8,
                ram_bank_bit_mask,
                ram_enable: 0x00,
                rom_bank_number: 0x00,
                ram_bank_number: 0x00,
            },
            MapperType::MBC5 => Self::MBC5 {
                rom_bank_bit_mask,
                ram_bank_bit_mask,
                ram_enable: 0x00,
                rom_bank_number: 0x01,
                ram_bank_number: 0x00,
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
            } => {
                // ROM bank 0 is actually bank 0 in MBC5

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
                ..
            } => match address {
                _address @ 0x0000..=0x1FFF => {
                    *ram_enable = value;
                }
                _address @ 0x2000..=0x3FFF => {
                    *rom_bank_number = value & 0x7F;
                }
                _address @ 0x4000..=0x5FFF => {
                    *ram_bank_number = value & 0x03;
                }
                _address @ 0x6000..=0x7FFF => {
                    // Real-time clock not implemented
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
        }
    }

    pub(crate) fn map_ram_address(&self, address: u16) -> Option<u32> {
        let relative_address = address - address::EXTERNAL_RAM_START;

        match self {
            Self::None => Some(u32::from(relative_address)),
            &Self::MBC1 {
                ram_bank_bit_mask,
                ram_enable,
                ram_bank_number,
                banking_mode_select,
                ..
            } => {
                if ram_enable & 0x0A == 0x0A {
                    if banking_mode_select == 0x00 {
                        Some(u32::from(relative_address))
                    } else {
                        let bank_number = ram_bank_number & ram_bank_bit_mask;
                        Some(u32::from(relative_address) + (u32::from(bank_number) << 13))
                    }
                } else {
                    None
                }
            }
            &Self::MBC2 { ram_enable, .. } => {
                if ram_enable == 0x0A {
                    Some(u32::from(relative_address))
                } else {
                    None
                }
            }
            &Self::MBC3 {
                ram_bank_bit_mask,
                ram_enable,
                ram_bank_number,
                ..
            }
            | &Self::MBC5 {
                ram_bank_bit_mask,
                ram_enable,
                ram_bank_number,
                ..
            } => {
                if ram_enable & 0x0A == 0x0A {
                    let bank_number = ram_bank_number & ram_bank_bit_mask;
                    Some(u32::from(relative_address) + (u32::from(bank_number) << 13))
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MapperFeatures {
    has_ram: bool,
    has_battery: bool,
}

impl MapperFeatures {
    pub(crate) fn has_ram(self) -> bool {
        self.has_ram
    }

    pub(crate) fn has_battery(self) -> bool {
        self.has_battery
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
        // 0x0F-0x10 are MBC3 w/ RTC, RTC not supported yet
        0x0F => (MapperType::MBC3, false, true),
        0x10 => (MapperType::MBC3, true, true),
        0x11 => (MapperType::MBC3, false, false),
        0x12 => (MapperType::MBC3, true, false),
        0x13 => (MapperType::MBC3, true, true),
        0x19 => (MapperType::MBC5, false, false),
        0x1A => (MapperType::MBC5, true, false),
        0x1B => (MapperType::MBC5, true, true),
        // MBC5 w/ rumble, rumble not supported
        0x1C => (MapperType::MBC5, false, false),
        0x1D => (MapperType::MBC5, true, false),
        0x1E => (MapperType::MBC5, true, true),
        _ => return None,
    };

    let features = MapperFeatures {
        has_ram,
        has_battery,
    };
    Some((mapper_type, features))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mbc1_mapper_rom_small() {
        // 256KB ROM
        let mut mapper = Mapper::new(MapperType::MBC1, 1 << 18, 0);

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
        let mut mapper = Mapper::new(MapperType::MBC1, 1 << 21, 0);

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
        let mut mapper = Mapper::new(MapperType::MBC1, 1 << 18, 8192);

        // Enable RAM
        mapper.write_rom_address(0x0000, 0x0A);

        assert_eq!(Some(0x0000), mapper.map_ram_address(0xA000));
        assert_eq!(Some(0x1000), mapper.map_ram_address(0xB000));
        assert_eq!(Some(0x1234), mapper.map_ram_address(0xB234));
    }
}
