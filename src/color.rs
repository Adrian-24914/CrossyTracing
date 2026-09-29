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

    pub fn lit_with_exposure(self, intensity: f32, exposure: f32) -> Self {
        let scale = (intensity.max(0.0) * exposure.max(0.0)).min(1.1);
        Self::new(
            (self.r as f32 * scale).min(255.0) as u8,
            (self.g as f32 * scale).min(255.0) as u8,
            (self.b as f32 * scale).min(255.0) as u8,
        )
    }
}
