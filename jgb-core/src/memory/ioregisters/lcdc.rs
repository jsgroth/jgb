#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressRange {
    pub start: u16,
    pub end_inclusive: u16,
}

const BG_TILE_MAP_AREA_0: AddressRange = AddressRange {
    start: 0x9800,
    end_inclusive: 0x9BFF,
};

const BG_TILE_MAP_AREA_1: AddressRange = AddressRange {
    start: 0x9C00,
    end_inclusive: 0x9FFF,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileDataRange {
    Block0,
    Block1,
}

impl TileDataRange {
    pub fn start_address(self) -> u16 {
        match self {
            Self::Block0 => 0x9000,
            Self::Block1 => 0x8000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteMode {
    Stacked,
    Single,
}

impl SpriteMode {
    pub fn height(self) -> u8 {
        match self {
            Self::Stacked => 16,
            Self::Single => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lcdc<'a>(pub(super) &'a u8);

impl<'a> Lcdc<'a> {
    pub fn lcd_enabled(self) -> bool {
        *self.0 & 0x80 != 0
    }

    pub fn window_tile_map_area(self) -> AddressRange {
        if *self.0 & 0x40 != 0 {
            BG_TILE_MAP_AREA_1
        } else {
            BG_TILE_MAP_AREA_0
        }
    }

    pub fn window_enabled(self) -> bool {
        *self.0 & 0x20 != 0
    }

    pub fn bg_tile_data_area(self) -> TileDataRange {
        if *self.0 & 0x10 != 0 {
            TileDataRange::Block1
        } else {
            TileDataRange::Block0
        }
    }

    pub fn bg_tile_map_area(self) -> AddressRange {
        if *self.0 & 0x08 != 0 {
            BG_TILE_MAP_AREA_1
        } else {
            BG_TILE_MAP_AREA_0
        }
    }

    pub fn sprite_mode(self) -> SpriteMode {
        if *self.0 & 0x04 != 0 {
            SpriteMode::Stacked
        } else {
            SpriteMode::Single
        }
    }

    pub fn sprites_enabled(self) -> bool {
        *self.0 & 0x02 != 0
    }

    pub fn bg_enabled(self) -> bool {
        *self.0 & 0x01 != 0
    }
}
