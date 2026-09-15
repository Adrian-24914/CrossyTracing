mod color;
mod framebuffer;

use color::Color;
use framebuffer::Framebuffer;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 800;
const HEIGHT: usize = 600;
const BACKGROUND: Color = Color::new(25, 31, 43);

fn main() {
    let mut window = Window::new(
        "Creative Zone",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    )
    .expect("No se pudo abrir la ventana");
    window.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        framebuffer.clear(BACKGROUND);
        window
            .update_with_buffer(&framebuffer.color, framebuffer.width, framebuffer.height)
            .expect("No se pudo actualizar la ventana");
    }
}
