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
    let mut camera = OrbitCamera::new(
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
        if keys.iter().any(|key| matches!(key, Key::Pause)) {
            game.toggle_pause();
            let title = if game.paused {
                "Creative Zone | PAUSA: A/D rotar, W/S zoom"
            } else {
                "Creative Zone | WASD/Flechas mover, Espacio pausar"
            };
            window.set_title(title);
        }

        let mut changed = false;
        if game.paused {
            if window.is_key_down(Key::Left) {
                camera.orbit_y(-0.035);
                changed = true;
            }
            if window.is_key_down(Key::Right) {
                camera.orbit_y(0.035);
                changed = true;
            }
            if window.is_key_down(Key::Up) {
                camera.zoom(-0.18);
                changed = true;
            }
            if window.is_key_down(Key::Down) {
                camera.zoom(0.18);
                changed = true;
            }
        } else {
            for key in keys {
                let direction = match key {
                    Key::Left => Some(Move::Left),
                    Key::Right => Some(Move::Right),
                    Key::Up => Some(Move::Forward),
                    Key::Down => Some(Move::Backward),
                    Key::Pause => None,
                };
                if let Some(direction) = direction {
                    changed |= game.try_move(direction);
                }
            }
        }

        if changed {
            scene = scene::build_scene(&game);
            renderer::render(&mut framebuffer, &camera, &scene.cubes, &scene.spheres);
        }
        window.present(&framebuffer.color);
        thread::sleep(Duration::from_millis(16));
    }
}
