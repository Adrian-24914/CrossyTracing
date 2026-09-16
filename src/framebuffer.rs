pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub color: Vec<u32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            color: vec![0; width * height],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        let index = y * self.width + x;
        self.color[index] = color;
    }
}
