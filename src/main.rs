mod camera;
mod color;
mod framebuffer;
mod math;
mod window;

use camera::Camera;
use color::Color;
use framebuffer::Framebuffer;
use math::Vec3;
use std::{f32::consts::FRAC_PI_4, thread, time::Duration};
use window::NativeWindow;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;

fn main() {
    let window = NativeWindow::new("Creative Zone - Raytracing", WIDTH, HEIGHT);
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let camera = Camera::new(
        Vec3::new(8.5, 8.5, 11.5),
        Vec3::new(0.0, 0.0, 0.0),
        FRAC_PI_4,
    );

    render_sky(&mut framebuffer, &camera);

    while window.pump_messages() {
        window.present(&framebuffer.color);
        thread::sleep(Duration::from_millis(16));
    }
}

fn render_sky(framebuffer: &mut Framebuffer, camera: &Camera) {
    framebuffer.clear(Color::new(25, 31, 43));
    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let ray = camera.ray_direction(x, y, framebuffer.width, framebuffer.height);
            let blend = (ray.y * 0.5 + 0.5).clamp(0.0, 1.0);
            let color = Color::new(
                (35.0 + 65.0 * blend) as u8,
                (45.0 + 90.0 * blend) as u8,
                (65.0 + 125.0 * blend) as u8,
            );
            framebuffer.set_pixel(x, y, color.to_hex());
        }
    }
}
