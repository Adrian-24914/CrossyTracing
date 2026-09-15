use crate::color::Color;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub color: Vec<u32>,
    pub depth: Vec<f32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            color: vec![0; width * height],
            depth: vec![f32::NEG_INFINITY; width * height],
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.color.fill(color.to_hex());
        self.depth.fill(f32::NEG_INFINITY);
    }

    pub fn draw_depth_tested(&mut self, x: usize, y: usize, inverse_depth: f32, color: u32) {
        let index = y * self.width + x;
        if inverse_depth > self.depth[index] {
            self.depth[index] = inverse_depth;
            self.color[index] = color;
        }
    }
}
