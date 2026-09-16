#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn to_hex(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    pub fn lit(self, intensity: f32) -> Self {
        let intensity = intensity.clamp(0.0, 1.0);
        Self::new(
            (self.r as f32 * intensity) as u8,
            (self.g as f32 * intensity) as u8,
            (self.b as f32 * intensity) as u8,
        )
    }
}
