mod color;
mod cube;
mod framebuffer;
mod game;
mod math;
mod orbit_camera;
mod ray;
mod renderer;
mod scene;
mod sphere;
mod window;

use framebuffer::Framebuffer;
use game::Game;
use math::Vec3;
use orbit_camera::OrbitCamera;
use std::{f32::consts::FRAC_PI_4, thread, time::Duration};
use window::NativeWindow;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;

fn main() {
    let window = NativeWindow::new("Creative Zone - Raytracing", WIDTH, HEIGHT);
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let camera = OrbitCamera::new(
        Vec3::new(8.5, 8.5, 11.5),
        Vec3::new(0.0, 0.0, 0.0),
        FRAC_PI_4,
    );
    let game = Game::new();
    let scene = scene::build_scene(&game);

    renderer::render(&mut framebuffer, &camera, &scene.cubes, &scene.spheres);

    while window.pump_messages() {
        window.present(&framebuffer.color);
        thread::sleep(Duration::from_millis(16));
    }
}
