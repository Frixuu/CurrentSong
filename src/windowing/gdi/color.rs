pub struct Color {
    value: u32,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color {
            value: (r as u32) | ((g as u32) << 8) | ((b as u32) << 16),
        }
    }
}

impl From<Color> for u32 {
    fn from(color: Color) -> Self {
        color.value
    }
}
