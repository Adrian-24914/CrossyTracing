mod camera;
mod color;
mod framebuffer;

use camera::Camera;
use color::Color;
use framebuffer::Framebuffer;
use minifb::{Key, Window, WindowOptions};
use nalgebra_glm::Vec3;
use std::f32::consts::FRAC_PI_4;

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
    let camera = Camera::new(
        Vec3::new(8.5, 8.5, 11.5),
        Vec3::new(0.0, 0.0, 0.0),
        FRAC_PI_4,
    );

    while window.is_open() && !window.is_key_down(Key::Escape) {
        framebuffer.clear(BACKGROUND);
        draw_reference_grid(&mut framebuffer, &camera);
        window
            .update_with_buffer(&framebuffer.color, framebuffer.width, framebuffer.height)
            .expect("No se pudo actualizar la ventana");
    }
}

fn draw_reference_grid(framebuffer: &mut Framebuffer, camera: &Camera) {
    let grid_color = Color::new(92, 170, 92).to_hex();
    for z in -3..=3 {
        for x in -3..=3 {
            let point = Vec3::new(x as f32 * 1.35, 0.0, z as f32 * 1.35);
            let Some(projected) = camera.project(&point, framebuffer.width, framebuffer.height)
            else {
                continue;
            };

            let center_x = projected.x as isize;
            let center_y = projected.y as isize;
            for offset_y in -2..=2 {
                for offset_x in -2..=2 {
                    let pixel_x = center_x + offset_x;
                    let pixel_y = center_y + offset_y;
                    if pixel_x >= 0
                        && pixel_x < framebuffer.width as isize
                        && pixel_y >= 0
                        && pixel_y < framebuffer.height as isize
                    {
                        framebuffer.draw_depth_tested(
                            pixel_x as usize,
                            pixel_y as usize,
                            projected.inverse_depth,
                            grid_color,
                        );
                    }
                }
            }
        }
    }
}
