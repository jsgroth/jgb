pub mod addresses;

use std::path::Path;
use std::{fs, io};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CartridgeLoadError {
    #[error("header should be at least 336 bytes, was {header_len} bytes")]
    HeaderTooShort { header_len: usize },
    #[error("invalid or unsupported mapper byte in cartridge header: {mapper_byte}")]
    InvalidMapper { mapper_byte: u8 },
    #[error("invalid RAM size code, expected 0 or 2-5: {ram_size_code}")]
    InvalidRamSize { ram_size_code: u8 },
    #[error("error reading data from {file_path}: {source}")]
    FileReadError {
        file_path: String,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MapperType {
    None,
    MBC1,
}

#[derive(Debug, Clone)]
enum Mapper {
    None,
    MBC1 {
        rom_bank_bit_mask: u8,
        ram_bank_bit_mask: u8,
        ram_enable: u8,
        rom_bank_number: u8,
        ram_bank_number: u8,
        banking_mode_select: u8,
    },
}

impl Mapper {
    fn new(mapper_type: MapperType, rom_size: u32, ram_size: u32) -> Self {
        let rom_bank_bit_mask = ((rom_size >> 14) - 1) as u8;
        let ram_bank_bit_mask = if ram_size > 0 {
            ((ram_size >> 13) - 1) as u8
        } else {
            0
        };

        log::debug!("setting ROM bit bask to {rom_bank_bit_mask:02x} for size {rom_size}");
        log::debug!("setting RAM bit mask to {ram_bank_bit_mask:02x} for size {ram_size}");

        match mapper_type {
            MapperType::None => Self::None,
            MapperType::MBC1 => Self::MBC1 {
                rom_bank_bit_mask,
                ram_bank_bit_mask,
                ram_enable: 0x00,
                rom_bank_number: 0x00,
                ram_bank_number: 0x00,
                banking_mode_select: 0x00,
            },
        }
    }

    fn map_rom_address(&self, address: u16) -> u32 {
        match self {
            Self::None => u32::from(address),
            &Self::MBC1 {
                rom_bank_bit_mask,
                ram_bank_bit_mask,
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
                    _ => panic!("mapper called for address outside of cartridge address range: {address:04x}")
                }
            }
        }
    }

    fn write_rom_address(&mut self, address: u16, value: u8) {
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
                    *ram_enable = value;
                }
                _address @ 0x2000..=0x3FFF => {
                    *rom_bank_number = value & 0x1F;
                }
                _address @ 0x4000..=0x5FFF => {
                    *ram_bank_number = value & 0x03;
                }
                _address @ 0x6000..=0x7FFF => {
                    *banking_mode_select = value & 0x01;
                }
                _ => panic!("invalid ROM write address in MBC1 mapper: {address:04x}"),
            },
        }
    }

    fn map_ram_address(&self, address: u16) -> Option<u32> {
        match self {
            Self::None => Some(u32::from(address)),
            &Self::MBC1 {
                ram_bank_bit_mask,
                ram_enable,
                ram_bank_number,
                banking_mode_select,
                ..
            } => {
                if ram_enable & 0x0A == 0x0A {
                    if banking_mode_select == 0x00 {
                        Some(u32::from(address))
                    } else {
                        let bank_number = ram_bank_number & ram_bank_bit_mask;
                        Some(u32::from(address) + (u32::from(bank_number) << 13))
                    }
                } else {
                    None
                }
            }
        }
    }
}

pub struct Cartridge {
    rom: Vec<u8>,
    mapper: Mapper,
    ram: Vec<u8>,
    has_battery: bool,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>) -> Result<Self, CartridgeLoadError> {
        log::info!("Initializing cartridge using {} bytes of data", rom.len());

        if rom.len() < 0x0150 {
            return Err(CartridgeLoadError::HeaderTooShort {
                header_len: rom.len(),
            });
        }

        let mapper_byte = rom[addresses::MAPPER as usize];
        let (mapper_type, has_ram, has_battery) = match mapper_byte {
            0x00 => (MapperType::None, false, false),
            0x01 => (MapperType::MBC1, false, false),
            0x02 => (MapperType::MBC1, true, false),
            0x03 => (MapperType::MBC1, true, true),
            _ => return Err(CartridgeLoadError::InvalidMapper { mapper_byte }),
        };

        log::info!("Detected mapper type {mapper_type:?}");

        let ram = if has_ram {
            let ram_size_code = rom[addresses::RAM_SIZE as usize];
            let ram_size = match ram_size_code {
                0x00 => 0,
                0x02 => 8192,   // 8 KB
                0x03 => 32768,  // 32 KB
                0x04 => 131072, // 128 KB
                0x05 => 65536,  // 64 KB
                _ => return Err(CartridgeLoadError::InvalidRamSize { ram_size_code }),
            };
            vec![0; ram_size as usize]
        } else {
            Vec::new()
        };

        let mapper = Mapper::new(mapper_type, rom.len() as u32, ram.len() as u32);

        log::info!("Cartridge has {} bytes of external RAM", ram.len());
        log::info!("Cartridge has battery: {has_battery}");

        Ok(Self {
            rom,
            mapper,
            ram,
            has_battery,
        })
    }

    pub fn from_file(file_path: &str) -> Result<Self, CartridgeLoadError> {
        log::info!("Loading cartridge from '{file_path}'");

        let raw_data =
            fs::read(Path::new(file_path)).map_err(|err| CartridgeLoadError::FileReadError {
                file_path: file_path.into(),
                source: err,
            })?;
        Self::new(raw_data)
    }

    pub fn read_rom_address(&self, address: u16) -> u8 {
        let mapped_address = self.mapper.map_rom_address(address);
        self.rom[mapped_address as usize]
    }

    pub fn write_rom_address(&mut self, address: u16, value: u8) {
        self.mapper.write_rom_address(address, value);
    }

    pub fn read_ram_address(&self, address: u16) -> u8 {
        match self.mapper.map_ram_address(address) {
            Some(mapped_address) => self
                .ram
                .get(mapped_address as usize)
                .copied()
                .unwrap_or(0xFF),
            None => 0xFF,
        }
    }

    pub fn write_ram_address(&mut self, address: u16, value: u8) {
        if let Some(mapped_address) = self.mapper.map_ram_address(address) {
            if let Some(ram_value) = self.ram.get_mut(mapped_address as usize) {
                *ram_value = value;
            }
        }
    }
}

pub struct AddressSpace {
    cartridge: Cartridge,
    vram: [u8; 8192],
    working_ram: [u8; 8192],
    oam: [u8; 160],
    io_registers: [u8; 128],
    hram: [u8; 127],
    ie_register: u8,
}

impl AddressSpace {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge,
            vram: [0; 8192],
            working_ram: [0; 8192],
            oam: [0; 160],
            io_registers: [0; 128],
            hram: [0; 127],
            ie_register: 0,
        }
    }

    pub fn read_address_u8(&self, address: u16) -> u8 {
        match address {
            address @ addresses::ROM_START..=addresses::ROM_END => {
                self.cartridge.read_rom_address(address)
            }
            address @ addresses::VRAM_START..=addresses::VRAM_END => {
                self.vram[(address - addresses::VRAM_START) as usize]
            }
            address @ addresses::EXTERNAL_RAM_START..=addresses::EXTERNAL_RAM_END => {
                self.cartridge.read_ram_address(address)
            }
            address @ addresses::WORKING_RAM_START..=addresses::WORKING_RAM_END => {
                self.working_ram[(address - addresses::WORKING_RAM_START) as usize]
            }
            address @ addresses::ECHO_RAM_START..=addresses::ECHO_RAM_END => {
                self.working_ram[(address - addresses::ECHO_RAM_START) as usize]
            }
            address @ addresses::OAM_START..=addresses::OAM_END => {
                self.oam[(address - addresses::OAM_START) as usize]
            }
            _address @ addresses::UNUSABLE_START..=addresses::UNUSABLE_END => {
                todo!("should return 0xFF if OAM is blocked, 0x00 otherwise")
            }
            address @ addresses::IO_REGISTERS_START..=addresses::IO_REGISTERS_END => {
                self.io_registers[(address - addresses::IO_REGISTERS_START) as usize]
            }
            address @ addresses::HRAM_START..=addresses::HRAM_END => {
                self.hram[(address - addresses::HRAM_START) as usize]
            }
            addresses::IE_REGISTER => self.ie_register,
        }
    }

    pub fn read_address_u16(&self, address: u16) -> u16 {
        let lsb = self.read_address_u8(address);
        let msb = self.read_address_u8(address + 1);
        u16::from_le_bytes([lsb, msb])
    }

    pub fn write_address_u8(&mut self, address: u16, value: u8) {
        match address {
            address @ addresses::ROM_START..=addresses::ROM_END => {
                self.cartridge.write_rom_address(address, value);
            }
            address @ addresses::VRAM_START..=addresses::VRAM_END => {
                self.vram[(address - addresses::VRAM_START) as usize] = value;
            }
            address @ addresses::EXTERNAL_RAM_START..=addresses::EXTERNAL_RAM_END => {
                self.cartridge.write_ram_address(address, value);
            }
            address @ addresses::WORKING_RAM_START..=addresses::WORKING_RAM_END => {
                self.working_ram[(address - addresses::WORKING_RAM_START) as usize] = value;
            }
            address @ addresses::ECHO_RAM_START..=addresses::ECHO_RAM_END => {
                self.working_ram[(address - addresses::ECHO_RAM_START) as usize] = value;
            }
            address @ addresses::OAM_START..=addresses::OAM_END => {
                self.oam[(address - addresses::OAM_START) as usize] = value;
            }
            _address @ addresses::UNUSABLE_START..=addresses::UNUSABLE_END => {
                todo!("should return 0xFF if OAM is blocked, 0x00 otherwise")
            }
            address @ addresses::IO_REGISTERS_START..=addresses::IO_REGISTERS_END => {
                self.io_registers[(address - addresses::IO_REGISTERS_START) as usize] = value;
            }
            address @ addresses::HRAM_START..=addresses::HRAM_END => {
                self.hram[(address - addresses::HRAM_START) as usize] = value;
            }
            addresses::IE_REGISTER => {
                self.ie_register = value;
            }
        }
    }

    pub fn write_address_u16(&mut self, address: u16, value: u16) {
        let [lsb, msb] = value.to_le_bytes();
        self.write_address_u8(address, lsb);
        self.write_address_u8(address + 1, msb);
    }
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
}