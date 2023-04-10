pub mod address;
pub mod ioregisters;
mod mapper;

use crate::cpu::ExecutionMode;
use crate::memory::ioregisters::IoRegisters;
use crate::memory::mapper::{Mapper, RamMapResult, RealTimeClock};
use crate::ppu::{PpuMode, PpuState};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fs, io};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CartridgeLoadError {
    #[error("header should be at least 336 bytes, was {header_len} bytes")]
    HeaderTooShort { header_len: usize },
    #[error("invalid or unsupported mapper byte in cartridge header: {mapper_byte:02X}")]
    InvalidMapper { mapper_byte: u8 },
    #[error("invalid RAM size code, expected 0 or 2-5: {ram_size_code}")]
    InvalidRamSize { ram_size_code: u8 },
    #[error("error reading data from {file_path}: {source}")]
    FileReadError {
        file_path: String,
        #[source]
        source: io::Error,
    },
    #[error("error reading dirname/filename of {file_path}")]
    PathError { file_path: String },
}

#[derive(Serialize, Deserialize)]
struct FsRamBattery {
    dirty: bool,
    sav_path: PathBuf,
}

impl FsRamBattery {
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn persist_ram(&mut self, ram: &[u8]) -> Result<(), io::Error> {
        if !self.dirty {
            return Ok(());
        }

        let tmp_file = self.sav_path.with_extension("sav.tmp");
        fs::write(&tmp_file, ram)?;
        fs::rename(&tmp_file, &self.sav_path)?;

        self.dirty = false;

        Ok(())
    }

    fn sav_path(&self) -> std::path::Display<'_> {
        self.sav_path.display()
    }
}

fn load_sav_file<P>(sav_file: P) -> Result<Option<Vec<u8>>, CartridgeLoadError>
where
    P: AsRef<Path>,
{
    let ram = if fs::metadata(sav_file.as_ref())
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        Some(
            fs::read(sav_file.as_ref()).map_err(|err| CartridgeLoadError::FileReadError {
                file_path: sav_file.as_ref().to_str().unwrap_or("").into(),
                source: err,
            })?,
        )
    } else {
        None
    };

    if ram.is_some() {
        log::info!("Loaded external RAM from {}", sav_file.as_ref().display());
    }

    Ok(ram)
}

fn load_rtc<P>(rtc_file: P) -> Result<RealTimeClock, String>
where
    P: AsRef<Path>,
{
    let rtc_bytes = match fs::read(rtc_file.as_ref()) {
        Ok(rtc_bytes) => rtc_bytes,
        Err(err) => {
            return Err(format!(
                "error reading RTC file {}: {err}",
                rtc_file.as_ref().display()
            ));
        }
    };

    let rtc = match bincode::deserialize(&rtc_bytes) {
        Ok(rtc) => rtc,
        Err(err) => {
            return Err(format!(
                "error deserializing RTC bytes from {}: {err}",
                rtc_file.as_ref().display()
            ));
        }
    };

    log::info!(
        "Successfully loaded real-time clock state from {}",
        rtc_file.as_ref().display()
    );

    Ok(rtc)
}

#[derive(Serialize, Deserialize)]
pub struct Cartridge {
    #[serde(skip)]
    rom: Vec<u8>,
    mapper: Mapper,
    ram: Vec<u8>,
    ram_battery: Option<FsRamBattery>,
}

impl Cartridge {
    /// Create a new Cartridge value from the given ROM.
    ///
    /// # `CartridgeLoadError`
    ///
    /// This function will return an error in the following scenarios:
    /// * The ROM is too short (must be at least 0x150 bytes)
    /// * The mapper byte in the cartridge header is invalid (or not implemented yet)
    /// * The RAM size byte in the cartridge header is invalid
    pub fn new(rom: Vec<u8>, sav_path: Option<PathBuf>) -> Result<Self, CartridgeLoadError> {
        log::info!("Initializing cartridge using {} bytes of data", rom.len());

        if rom.len() < 0x0150 {
            return Err(CartridgeLoadError::HeaderTooShort {
                header_len: rom.len(),
            });
        }

        let mapper_byte = rom[address::MAPPER as usize];
        let Some((mapper_type, mapper_features)) = mapper::parse_byte(mapper_byte)
        else {
            return Err(CartridgeLoadError::InvalidMapper { mapper_byte });
        };

        log::info!("Detected mapper type {mapper_type:?} (byte: {mapper_byte:02X})");
        log::info!("Mapper features: {mapper_features}");

        let ram = if let Some(sav_path) = &sav_path {
            load_sav_file(sav_path)?
        } else {
            None
        };

        let rtc = match (mapper_features.has_rtc, &sav_path) {
            (true, Some(sav_path)) => {
                let rtc_path = sav_path.with_extension("rtc");
                match load_rtc(rtc_path) {
                    Ok(rtc) => Some(rtc),
                    Err(err) => {
                        log::warn!("error attempting to load previous RTC, resetting: {err}");
                        None
                    }
                }
            }
            _ => None,
        };

        let ram = match (mapper_features.has_ram, mapper_features.has_battery, ram) {
            (true, true, Some(ram)) => ram,
            (true, _, _) => {
                let ram_size_code = rom[address::RAM_SIZE as usize];
                let ram_size: usize = match ram_size_code {
                    0x00 => 0,
                    0x02 => 8192,   // 8 KB
                    0x03 => 32768,  // 32 KB
                    0x04 => 131072, // 128 KB
                    0x05 => 65536,  // 64 KB
                    _ => return Err(CartridgeLoadError::InvalidRamSize { ram_size_code }),
                };
                vec![0; ram_size]
            }
            _ => Vec::new(),
        };

        let ram_battery = match (mapper_features.has_battery, sav_path) {
            (true, Some(sav_path)) => Some(FsRamBattery {
                dirty: false,
                sav_path,
            }),
            _ => None,
        };

        if let Some(ram_battery) = &ram_battery {
            log::info!("Persisting external RAM to {}", ram_battery.sav_path());
        }

        let mapper = Mapper::new(
            mapper_type,
            mapper_features,
            rtc,
            rom.len() as u32,
            ram.len() as u32,
        );

        log::info!("Cartridge has {} bytes of external RAM", ram.len());
        log::info!("Cartridge has battery: {}", mapper_features.has_battery);

        Ok(Self {
            rom,
            mapper,
            ram,
            ram_battery,
        })
    }

    #[cfg(test)]
    pub fn new_cgb_test() -> Self {
        let mut rom = vec![0; 0x0150];
        rom[address::CGB_SUPPORT as usize] = 0x80;
        Self::new(rom, None).unwrap()
    }

    pub fn from_file(file_path: &str) -> Result<Self, CartridgeLoadError> {
        log::info!("Loading cartridge from '{file_path}'");

        let rom =
            fs::read(Path::new(file_path)).map_err(|err| CartridgeLoadError::FileReadError {
                file_path: file_path.into(),
                source: err,
            })?;

        let sav_file = Path::new(file_path).with_extension("sav");

        Self::new(rom, Some(sav_file))
    }

    /// Read a value from the given ROM address.
    ///
    /// # Panics
    ///
    /// This method will panic if the ROM address is invalid. ROM addresses must be in the range
    /// \[0x0000, 0x7FFF\].
    pub fn read_rom_address(&self, address: u16) -> u8 {
        let mapped_address = self.mapper.map_rom_address(address);
        self.rom[mapped_address as usize]
    }

    /// Write a value to the given ROM address (or in reality, set a cartridge register).
    ///
    /// # Panics
    ///
    /// This method will panic if the ROM address is invalid. ROM addresses must be in the range
    /// \[0x0000, 0x7FFF\].
    pub fn write_rom_address(&mut self, address: u16, value: u8) {
        self.mapper.write_rom_address(address, value);
    }

    /// Read a value from the given cartridge RAM address. Returns 0xFF if the address is not valid.
    pub fn read_ram_address(&self, address: u16) -> u8 {
        match self.mapper.map_ram_address(address) {
            RamMapResult::RamAddress(mapped_address) => self
                .ram
                .get(mapped_address as usize)
                .copied()
                .unwrap_or(0xFF),
            RamMapResult::MapperRegister => {
                self.mapper.read_ram_addressed_register().unwrap_or(0xFF)
            }
            RamMapResult::None => 0xFF,
        }
    }

    /// Write a value to the given cartridge RAM address. Does nothing if the address is not valid.
    pub fn write_ram_address(&mut self, address: u16, value: u8) {
        match self.mapper.map_ram_address(address) {
            RamMapResult::RamAddress(mapped_address) => {
                if let Some(ram_value) = self.ram.get_mut(mapped_address as usize) {
                    *ram_value = value;
                    if let Some(ram_battery) = &mut self.ram_battery {
                        ram_battery.mark_dirty();
                    }
                }
            }
            RamMapResult::MapperRegister => {
                self.mapper.write_ram_addressed_register(value);
            }
            RamMapResult::None => {}
        }
    }

    /// If this cartridge has external RAM, save it to disk if it has been modified since the last
    /// time this method was called.
    pub fn persist_external_ram(&mut self) -> Result<(), io::Error> {
        if let Some(ram_battery) = &mut self.ram_battery {
            ram_battery.persist_ram(&self.ram)
        } else {
            Ok(())
        }
    }

    /// Save the current state of the real-time clock, if this cartridge has one.
    pub fn persist_rtc(&self) -> Result<(), io::Error> {
        if let (Some(rtc), Some(battery)) = (self.mapper.get_clock(), self.ram_battery.as_ref()) {
            let rtc_bytes = bincode::serialize(rtc)
                .expect("RTC value-to-bytes serialization should never fail");

            let rtc_path = battery.sav_path.with_extension("rtc");
            fs::write(rtc_path, rtc_bytes)?;
        }

        Ok(())
    }

    /// Update the current state of the real-time clock based on the current time, if this
    /// cartridge has one.
    pub fn update_rtc(&mut self) {
        self.mapper.update_rtc();
    }

    /// Whether or not this cartridge supports CGB enhancements (or requires CGB)
    pub fn supports_cgb_mode(&self) -> bool {
        self.rom[address::CGB_SUPPORT as usize] & 0x80 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VramBank {
    Bank0,
    Bank1,
}

#[derive(Serialize, Deserialize)]
pub struct AddressSpace {
    execution_mode: ExecutionMode,
    cartridge: Cartridge,
    #[serde(
        serialize_with = "crate::serialize::serialize_array",
        deserialize_with = "crate::serialize::deserialize_array"
    )]
    vram: [u8; 16384],
    #[serde(
        serialize_with = "crate::serialize::serialize_array",
        deserialize_with = "crate::serialize::deserialize_array"
    )]
    working_ram: [u8; 32768],
    #[serde(
        serialize_with = "crate::serialize::serialize_array",
        deserialize_with = "crate::serialize::deserialize_array"
    )]
    oam: [u8; 160],
    io_registers: IoRegisters,
    #[serde(
        serialize_with = "crate::serialize::serialize_array",
        deserialize_with = "crate::serialize::deserialize_array"
    )]
    hram: [u8; 127],
    ie_register: u8,
}

impl AddressSpace {
    pub fn new(cartridge: Cartridge, execution_mode: ExecutionMode) -> Self {
        Self {
            execution_mode,
            cartridge,
            vram: [0; 16384],
            working_ram: [0; 32768],
            oam: [0; 160],
            io_registers: IoRegisters::new(execution_mode),
            hram: [0; 127],
            ie_register: 0,
        }
    }

    fn is_cpu_access_allowed(address: u16, ppu_state: &PpuState) -> bool {
        // Non-HRAM access not allowed while an OAM DMA transfer is active, even if the PPU is disabled
        if ppu_state.oam_dma_status().is_some()
            && !(address::HRAM_START..=address::HRAM_END).contains(&address)
        {
            return false;
        }

        // OAM access not allowed while PPU is scanning OAM or rendering a scanline
        if ppu_state.enabled()
            && matches!(
                ppu_state.mode(),
                PpuMode::ScanningOAM | PpuMode::RenderingScanline
            )
            && (address::OAM_START..=address::OAM_END).contains(&address)
        {
            return false;
        }

        // VRAM access not allowed while PPU is rendering a scanline
        !(ppu_state.enabled()
            && matches!(ppu_state.mode(), PpuMode::RenderingScanline)
            && (address::VRAM_START..=address::VRAM_END).contains(&address))
    }

    /// Read the value at the given address from the perspective of the CPU. Returns 0xFF if the
    /// CPU is not able to access the given address because of PPU state.
    pub fn read_address_u8(&self, address: u16, ppu_state: &PpuState) -> u8 {
        if !Self::is_cpu_access_allowed(address, ppu_state) {
            return 0xFF;
        }

        self.read_address_u8_no_access_check(address)
    }

    fn map_vram_address(&self, address: u16) -> usize {
        match self.execution_mode {
            ExecutionMode::GameBoy => (address - address::VRAM_START) as usize,
            ExecutionMode::GameBoyColor => {
                (self.io_registers.get_cgb_vram_bank() << 13)
                    + (address - address::VRAM_START) as usize
            }
        }
    }

    fn map_working_ram_address(&self, address: u16) -> usize {
        match self.execution_mode {
            ExecutionMode::GameBoy => (address - address::WORKING_RAM_START) as usize,
            ExecutionMode::GameBoyColor => match address {
                address @ address::WORKING_RAM_START..=address::CGB_BANK_0_WORKING_RAM_END => {
                    (address - address::WORKING_RAM_START) as usize
                }
                _ => {
                    let ram_bank_number = self.io_registers.get_cgb_working_ram_bank();
                    (ram_bank_number << 12)
                        + (address - address::CGB_BANKED_WORKING_RAM_START) as usize
                }
            },
        }
    }

    fn read_address_u8_no_access_check(&self, address: u16) -> u8 {
        match address {
            address @ address::ROM_START..=address::ROM_END => {
                self.cartridge.read_rom_address(address)
            }
            address @ address::VRAM_START..=address::VRAM_END => {
                self.vram[self.map_vram_address(address)]
            }
            address @ address::EXTERNAL_RAM_START..=address::EXTERNAL_RAM_END => {
                self.cartridge.read_ram_address(address)
            }
            address @ address::WORKING_RAM_START..=address::WORKING_RAM_END => {
                self.working_ram[self.map_working_ram_address(address)]
            }
            address @ address::ECHO_RAM_START..=address::ECHO_RAM_END => {
                self.working_ram[self.map_working_ram_address(
                    address - address::ECHO_RAM_START + address::WORKING_RAM_START,
                )]
            }
            address @ address::OAM_START..=address::OAM_END => {
                self.oam[(address - address::OAM_START) as usize]
            }
            _address @ address::UNUSABLE_START..=address::UNUSABLE_END => 0xFF,
            address @ address::IO_REGISTERS_START..=address::IO_REGISTERS_END => {
                self.io_registers.read_address(address)
            }
            address @ address::HRAM_START..=address::HRAM_END => {
                self.hram[(address - address::HRAM_START) as usize]
            }
            address::IE_REGISTER => self.ie_register,
        }
    }

    /// Read the OAM/VRAM value at the given address from the perspective of the PPU, bypassing the
    /// CPU access check.
    ///
    /// # Panics
    ///
    /// This method will panic if the address is not an OAM or VRAM address.
    pub fn ppu_read_address_u8(&self, address: u16) -> u8 {
        match address {
            address @ address::OAM_START..=address::OAM_END => {
                self.oam[(address - address::OAM_START) as usize]
            }
            address @ address::VRAM_START..=address::VRAM_END => {
                self.vram[(address - address::VRAM_START) as usize]
            }
            _ => panic!("PPU read method is only allowed to read OAM and VRAM"),
        }
    }

    /// Read the value at the given address and the following address as a little-endian 16-bit
    /// value.
    pub fn read_address_u16(&self, address: u16, ppu_state: &PpuState) -> u16 {
        let lsb = self.read_address_u8(address, ppu_state);
        let msb = self.read_address_u8(address + 1, ppu_state);
        u16::from_le_bytes([lsb, msb])
    }

    /// Assign a value to the given address from the perspective of the CPU. The write is ignored
    /// if the CPU is not allowed to access the given address due to PPU state.
    pub fn write_address_u8(&mut self, address: u16, value: u8, ppu_state: &PpuState) {
        if !Self::is_cpu_access_allowed(address, ppu_state) {
            return;
        }

        self.write_address_u8_no_access_check(address, value);
    }

    fn write_address_u8_no_access_check(&mut self, address: u16, value: u8) {
        match address {
            address @ address::ROM_START..=address::ROM_END => {
                self.cartridge.write_rom_address(address, value);
            }
            address @ address::VRAM_START..=address::VRAM_END => {
                self.vram[self.map_vram_address(address)] = value;
            }
            address @ address::EXTERNAL_RAM_START..=address::EXTERNAL_RAM_END => {
                self.cartridge.write_ram_address(address, value);
            }
            address @ address::WORKING_RAM_START..=address::WORKING_RAM_END => {
                self.working_ram[self.map_working_ram_address(address)] = value;
            }
            address @ address::ECHO_RAM_START..=address::ECHO_RAM_END => {
                self.working_ram[self.map_working_ram_address(
                    address - address::ECHO_RAM_START + address::WORKING_RAM_START,
                )] = value;
            }
            address @ address::OAM_START..=address::OAM_END => {
                self.oam[(address - address::OAM_START) as usize] = value;
            }
            _address @ address::UNUSABLE_START..=address::UNUSABLE_END => {}
            address @ address::IO_REGISTERS_START..=address::IO_REGISTERS_END => {
                self.io_registers.write_address(address, value);
            }
            address @ address::HRAM_START..=address::HRAM_END => {
                self.hram[(address - address::HRAM_START) as usize] = value;
            }
            address::IE_REGISTER => {
                self.ie_register = value;
            }
        }
    }

    /// Assign a 16-bit value to the given address and the following address, using
    /// little-endian.
    pub fn write_address_u16(&mut self, address: u16, value: u16, ppu_state: &PpuState) {
        let [lsb, msb] = value.to_le_bytes();
        self.write_address_u8(address, lsb, ppu_state);
        self.write_address_u8(address + 1, msb, ppu_state);
    }

    pub fn get_io_registers(&self) -> &IoRegisters {
        &self.io_registers
    }

    pub fn get_io_registers_mut(&mut self) -> &mut IoRegisters {
        &mut self.io_registers
    }

    /// Read a byte directly from VRAM using the given address+bank. This should only be called
    /// by the PPU.
    pub fn read_vram_direct(&self, address: u16, vram_bank: VramBank) -> u8 {
        if !(address::VRAM_START..=address::VRAM_END).contains(&address) {
            panic!("read_vram_direct called with a non-VRAM address: {address}");
        }

        match vram_bank {
            VramBank::Bank0 => self.vram[(address - address::VRAM_START) as usize],
            VramBank::Bank1 => self.vram[8192 + (address - address::VRAM_START) as usize],
        }
    }

    /// Retrieve the current value of the IE register (interrupts enabled).
    pub fn get_ie_register(&self) -> u8 {
        self.ie_register
    }

    /// Copy a byte from the given source address to the given destination address, bypassing
    /// access checks related to PPU state. Intended for use in OAM DMA transfer.
    pub fn copy_byte(&mut self, src_address: u16, dst_address: u16) {
        let byte = self.read_address_u8_no_access_check(src_address);
        self.write_address_u8_no_access_check(dst_address, byte);
    }

    pub fn persist_cartridge_ram(&mut self) -> Result<(), io::Error> {
        self.cartridge.persist_external_ram()
    }

    pub fn persist_rtc(&mut self) -> Result<(), io::Error> {
        self.cartridge.persist_rtc()
    }

    pub fn update_rtc(&mut self) {
        self.cartridge.update_rtc();
    }

    pub fn copy_cartridge_rom_from(&mut self, other: &Self) {
        self.cartridge.rom = other.cartridge.rom.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::ioregisters::IoRegister;

    #[test]
    fn cgb_vram_banks() {
        let mut address_space =
            AddressSpace::new(Cartridge::new_cgb_test(), ExecutionMode::GameBoyColor);
        let ppu_state = PpuState::new(ExecutionMode::GameBoyColor);

        address_space
            .get_io_registers_mut()
            .write_register(IoRegister::VBK, 0x00);

        assert_eq!(0x00, address_space.read_address_u8(0x8500, &ppu_state));
        address_space.write_address_u8(0x8500, 0xCD, &ppu_state);
        assert_eq!(0xCD, address_space.read_address_u8(0x8500, &ppu_state));

        assert_eq!(0x00, address_space.read_address_u8(0x9CDE, &ppu_state));
        address_space.write_address_u8(0x9CDE, 0x35, &ppu_state);
        assert_eq!(0x35, address_space.read_address_u8(0x9CDE, &ppu_state));

        address_space
            .get_io_registers_mut()
            .write_register(IoRegister::VBK, 0x01);

        assert_eq!(0x00, address_space.read_address_u8(0x8500, &ppu_state));
        assert_eq!(0x00, address_space.read_address_u8(0x9CDE, &ppu_state));

        address_space.write_address_u8(0x8500, 0xEF, &ppu_state);
        assert_eq!(0xEF, address_space.read_address_u8(0x8500, &ppu_state));

        address_space.write_address_u8(0x9CDE, 0x46, &ppu_state);
        assert_eq!(0x46, address_space.read_address_u8(0x9CDE, &ppu_state));

        // Check that bits other than 0 are ignored
        address_space
            .get_io_registers_mut()
            .write_register(IoRegister::VBK, 0xFE);
        assert_eq!(0xCD, address_space.read_address_u8(0x8500, &ppu_state));
        assert_eq!(0x35, address_space.read_address_u8(0x9CDE, &ppu_state));
    }

    #[test]
    fn cgb_working_ram_banks() {
        let mut address_space =
            AddressSpace::new(Cartridge::new_cgb_test(), ExecutionMode::GameBoyColor);
        let ppu_state = PpuState::new(ExecutionMode::GameBoyColor);

        address_space
            .get_io_registers_mut()
            .write_register(IoRegister::SVBK, 0x00);

        assert_eq!(0x00, address_space.read_address_u8(0xC500, &ppu_state));
        address_space.write_address_u8(0xC500, 0xDE, &ppu_state);
        assert_eq!(0xDE, address_space.read_address_u8(0xC500, &ppu_state));

        assert_eq!(0x00, address_space.read_address_u8(0xD500, &ppu_state));
        address_space.write_address_u8(0xD500, 0xCF, &ppu_state);
        assert_eq!(0xCF, address_space.read_address_u8(0xD500, &ppu_state));
        assert_eq!(0xDE, address_space.read_address_u8(0xC500, &ppu_state));

        // Bank 1 should behave the same as 0
        address_space
            .get_io_registers_mut()
            .write_register(IoRegister::SVBK, 0x01);
        assert_eq!(0xCF, address_space.read_address_u8(0xD500, &ppu_state));
        assert_eq!(0xCF, address_space.working_ram[0x1500]);
        assert_eq!(0xDE, address_space.read_address_u8(0xC500, &ppu_state));

        address_space
            .get_io_registers_mut()
            .write_register(IoRegister::SVBK, 0x04);
        assert_eq!(0x00, address_space.read_address_u8(0xD500, &ppu_state));
        assert_eq!(0xDE, address_space.read_address_u8(0xC500, &ppu_state));

        address_space.write_address_u8(0xD500, 0x57, &ppu_state);
        assert_eq!(0x57, address_space.read_address_u8(0xD500, &ppu_state));
        assert_eq!(0x57, address_space.working_ram[0x4500]);
        assert_eq!(0xDE, address_space.read_address_u8(0xC500, &ppu_state));

        // Check that only the lower 3 bits of SVBK are read
        address_space
            .get_io_registers_mut()
            .write_register(IoRegister::SVBK, 0xF8);
        assert_eq!(0xCF, address_space.read_address_u8(0xD500, &ppu_state));
        assert_eq!(0xDE, address_space.read_address_u8(0xC500, &ppu_state));

        // Test the highest bank number
        address_space
            .get_io_registers_mut()
            .write_register(IoRegister::SVBK, 0x07);
        assert_eq!(0x00, address_space.read_address_u8(0xD500, &ppu_state));
        assert_eq!(0xDE, address_space.read_address_u8(0xC500, &ppu_state));

        address_space.write_address_u8(0xD500, 0x21, &ppu_state);
        assert_eq!(0x21, address_space.read_address_u8(0xD500, &ppu_state));
        assert_eq!(0x21, address_space.working_ram[0x7500]);
        assert_eq!(0xDE, address_space.read_address_u8(0xC500, &ppu_state));
    }
}
