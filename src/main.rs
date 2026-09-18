mod color;
mod cube;
mod cylinder;
mod framebuffer;
mod game;
mod math;
mod orbit_camera;
mod player;
mod random;
mod ray;
mod renderer;
mod scene;
mod sphere;
mod train;
mod tree;
mod window;
mod world;

use framebuffer::Framebuffer;
use game::{Game, Move};
use math::Vec3;
use orbit_camera::OrbitCamera;
use std::{
    f32::consts::FRAC_PI_4,
    thread,
    time::{Duration, Instant},
};
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
    let mut previous_frame = Instant::now();

    renderer::render(
        &mut framebuffer,
        &camera,
        &scene.cubes,
        &scene.spheres,
        &scene.cylinders,
        &scene.trees,
    );
    update_title(&window, &game);

    loop {
        let Some(keys) = window.pump_messages() else {
            break;
        };
        let now = Instant::now();
        let delta_seconds = (now - previous_frame).as_secs_f32().min(0.05);
        previous_frame = now;

        let mut changed = false;
        if keys.iter().any(|key| matches!(key, Key::Reset)) {
            game.reset();
            changed = true;
        }
        if keys.iter().any(|key| matches!(key, Key::Pause)) {
            game.toggle_pause();
        }

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
                    Key::Pause | Key::Reset => None,
                };
                if let Some(direction) = direction {
                    changed |= game.try_move(direction);
                }
            }
        }
        changed |= game.update(delta_seconds);

        if changed {
            scene = scene::build_scene(&game);
            renderer::render(
                &mut framebuffer,
                &camera,
                &scene.cubes,
                &scene.spheres,
                &scene.cylinders,
                &scene.trees,
            );
            update_title(&window, &game);
        }
        window.present(&framebuffer.color);
        thread::sleep(Duration::from_millis(16));
    }
}

fn update_title(window: &NativeWindow, game: &Game) {
    let state = if game.game_over {
        "FIN - presiona R"
    } else if game.paused {
        "PAUSA: A/D rotar, W/S zoom"
    } else {
        "jugando"
    };
    window.set_title(&format!(
        "Creative Zone | Puntos: {} | {} | WASD/Flechas, Espacio, R",
        game.score, state
    ));
}
