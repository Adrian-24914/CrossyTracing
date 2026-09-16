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
}
