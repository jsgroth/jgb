use std::path::Path;
use std::{fs, io};

pub struct Cartridge {
    raw_data: Vec<u8>,
}

impl Cartridge {
    pub fn new(raw_data: Vec<u8>) -> Self {
        Self { raw_data }
    }

    pub fn from_file(file_path: &str) -> Result<Self, io::Error> {
        let raw_data = fs::read(Path::new(file_path))?;
        Ok(Self { raw_data })
    }
}

pub struct VRam {}

pub struct AddressSpace {
    cartridge: Cartridge,
    system_ram: [u8; 8192],
    vram: VRam,
}

impl AddressSpace {
    pub fn new(cartridge: Cartridge, vram: VRam) -> Self {
        Self {
            cartridge,
            system_ram: [0; 8192],
            vram,
        }
    }

    pub fn read_address_u8(&self, address: u16) -> u8 {
        todo!()
    }

    pub fn read_address_u16(&self, address: u16) -> u16 {
        todo!()
    }

    pub fn write_address_u8(&mut self, address: u16, value: u8) {
        todo!()
    }

    pub fn write_address_u16(&mut self, address: u16, value: u16) {
        todo!()
    }

    pub fn get_address_u8_mut(&mut self, address: u16) -> &mut u8 {
        todo!()
    }

    pub fn get_address_u16_mut(&mut self, address: u16) -> &mut u16 {
        todo!()
    }
}
