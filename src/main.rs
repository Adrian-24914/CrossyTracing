mod camera;
mod color;
mod cube;
mod framebuffer;
mod math;
mod ray;
mod renderer;
mod window;

use camera::Camera;
use color::Color;
use cube::Cube;
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
    let cubes = [
        Cube::new(
            Vec3::new(0.0, -0.5, 0.0),
            Vec3::new(6.0, 1.0, 6.0),
            Color::new(92, 170, 92),
        ),
        Cube::new(
            Vec3::new(-1.5, 0.55, -0.8),
            Vec3::new(1.0, 1.1, 1.0),
            Color::new(217, 106, 67),
        ),
        Cube::new(
            Vec3::new(0.0, 0.8, 0.4),
            Vec3::new(1.0, 1.6, 1.0),
            Color::new(238, 242, 246),
        ),
        Cube::new(
            Vec3::new(1.6, 0.35, -1.2),
            Vec3::new(0.8, 0.7, 0.8),
            Color::new(107, 123, 138),
        ),
    ];

    renderer::render(&mut framebuffer, &camera, &cubes);

    while window.pump_messages() {
        window.present(&framebuffer.color);
        thread::sleep(Duration::from_millis(16));
    }
}
