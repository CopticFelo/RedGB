#[derive(Clone, Copy)]
pub struct GBSprite {
    pub x: i8,
    pub y: i8,
    pub tile_index: u8,
    pub priority: u8,
    pub y_flip: bool,
    pub x_flip: bool,
    pub dmg_palette: u8,
    pub cgb_palette: u8,
    pub bank: u8,
}
