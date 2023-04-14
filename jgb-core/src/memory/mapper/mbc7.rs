use serde::{Deserialize, Serialize};

// Emulation of the MBC7 mapper's 93LC56 EEPROM chip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Mbc7Eeprom {
    #[serde(
        serialize_with = "crate::serialize::serialize_array",
        deserialize_with = "crate::serialize::deserialize_array"
    )]
    memory: [u8; 256],
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

        Self { memory }
    }

    pub(crate) fn raw_memory(&self) -> &[u8; 256] {
        &self.memory
    }
}
