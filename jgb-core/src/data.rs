pub struct Cartridge {}

pub struct VRam {}

pub struct AddressSpace {
    cartridge: Cartridge,
    system_ram: [u8; 8192],
    vram: VRam,
}

impl AddressSpace {
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
