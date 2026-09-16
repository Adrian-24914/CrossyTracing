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
use game::{Game, Move};
use math::Vec3;
use orbit_camera::OrbitCamera;
use std::{f32::consts::FRAC_PI_4, thread, time::Duration};
use window::{Key, NativeWindow};

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
    let mut game = Game::new();
    let mut scene = scene::build_scene(&game);

    renderer::render(&mut framebuffer, &camera, &scene.cubes, &scene.spheres);

    loop {
        let Some(keys) = window.pump_messages() else {
            break;
        };
        let mut moved = false;
        for key in keys {
            let direction = match key {
                Key::Left => Move::Left,
                Key::Right => Move::Right,
                Key::Up => Move::Forward,
                Key::Down => Move::Backward,
            };
            moved |= game.try_move(direction);
        }

        if moved {
            scene = scene::build_scene(&game);
            renderer::render(&mut framebuffer, &camera, &scene.cubes, &scene.spheres);
        }
        window.present(&framebuffer.color);
        thread::sleep(Duration::from_millis(16));
    }
}
