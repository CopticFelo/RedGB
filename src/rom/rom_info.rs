#[derive(Debug)]
pub enum CGBMode {
    Monochrome,
    Color { exclusive: bool },
}

impl Default for CGBMode {
    fn default() -> Self {
        Self::Color { exclusive: false }
    }
}

#[derive(Debug)]
pub struct ROMInfo {
    pub title: String,
    pub cgb: CGBMode,
    pub sgb: bool,
    pub cartridge_type: u8,
    pub rom_banks: u16,
    pub mem_banks: u16,
    pub header_checksum: u8,
    pub rom_checksum: u16,
}

impl Default for ROMInfo {
    fn default() -> Self {
        Self {
            title: String::default(),
            cgb: CGBMode::default(),
            sgb: true,
            cartridge_type: 0x10,
            rom_banks: 1,
            mem_banks: 0x3,
            header_checksum: u8::default(),
            rom_checksum: u16::default(),
        }
    }
}
