mod color;
mod cube;
mod cylinder;
mod framebuffer;
mod game;
mod material;
mod math;
mod obstacle;
mod orbit_camera;
mod player;
mod random;
mod ray;
mod renderer;
mod scene;
mod skybox;
mod sphere;
mod train;
mod tree;
mod ui;
mod window;
mod world;

use framebuffer::Framebuffer;
use game::{DeathKind, Game, Move};
use math::Vec3;
use orbit_camera::{OrbitCamera, Projection};
use renderer::RenderResources;
use skybox::Skybox;
use std::{
    f32::consts::FRAC_PI_4,
    thread,
    time::{Duration, Instant},
};
use window::{Key, NativeWindow};

const WIDTH: usize = 800;
const HEIGHT: usize = 600;
const INTRO_HOLD_SECONDS: f32 = 1.0;
const INTRO_RISE_SECONDS: f32 = 1.45;
const INTRO_DURATION_SECONDS: f32 = INTRO_HOLD_SECONDS + INTRO_RISE_SECONDS;
const DEATH_FADE_SECONDS: f32 = 0.65;

enum AppPhase {
    Intro { elapsed: f32 },
    Ready,
    Playing,
    Death { elapsed: f32 },
}

fn main() {
    let window = NativeWindow::new("Creative Zone - Raytracing", WIDTH, HEIGHT);
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut camera = OrbitCamera::new(
        Vec3::new(8.5, 8.5, 11.5),
        Vec3::new(0.0, 0.0, 0.0),
        FRAC_PI_4,
    );
    let mut game = Game::new();
    let skybox = Skybox::load().expect("No se pudo cargar assets/skybox/runtime");
    let render_resources = RenderResources { skybox: &skybox };
    let ui_assets = ui::UiAssets::load();
    let mut phase = AppPhase::Intro { elapsed: 0.0 };
    let mut scene = scene::build_scene_with_player(&game, player_animation(&game, &phase, &camera));
    let mut previous_frame = Instant::now();
    let mut intro_background: Option<Vec<u32>>;
    let mut death_background: Option<Vec<u32>> = None;

    renderer::render(
        &mut framebuffer,
        &camera,
        &scene.cubes,
        &scene.spheres,
        &scene.cylinders,
        &scene.forest_props,
        render_resources,
    );
    ui::blur(&mut framebuffer.color, WIDTH, HEIGHT);
    intro_background = Some(framebuffer.color.clone());
    update_title(&window, &game, &camera, &phase);

    loop {
        let Some(keys) = window.pump_messages() else {
            break;
        };
        let now = Instant::now();
        let delta_seconds = (now - previous_frame).as_secs_f32().min(0.05);
        previous_frame = now;

        let reset = keys.iter().any(|key| matches!(key, Key::Reset));
        let mut changed = false;
        let mut death_fade = None;
        if reset {
            game.reset();
            phase = AppPhase::Intro { elapsed: 0.0 };
            scene = scene::build_scene_with_player(&game, player_animation(&game, &phase, &camera));
            intro_background = None;
            death_background = None;
            changed = true;
        } else {
            if keys.iter().any(|key| matches!(key, Key::Orthographic)) {
                camera.toggle_projection();
                changed = true;
            }
            match &mut phase {
                AppPhase::Intro { elapsed } => {
                    *elapsed += delta_seconds;
                    if intro_background.is_none() {
                        renderer::render(
                            &mut framebuffer,
                            &camera,
                            &scene.cubes,
                            &scene.spheres,
                            &scene.cylinders,
                            &scene.forest_props,
                            render_resources,
                        );
                        ui::blur(&mut framebuffer.color, WIDTH, HEIGHT);
                        intro_background = Some(framebuffer.color.clone());
                    }
                    if *elapsed >= INTRO_DURATION_SECONDS {
                        phase = AppPhase::Ready;
                        changed = true;
                    } else if let Some(background) = &intro_background {
                        framebuffer.color.copy_from_slice(background);
                        ui::draw_intro(
                            &mut framebuffer.color,
                            WIDTH,
                            HEIGHT,
                            &ui_assets,
                            intro_rise_progress(*elapsed),
                        );
                    }
                }
                AppPhase::Ready => {
                    changed |=
                        handle_ready_input(&keys, &window, &mut game, &mut camera, &mut phase);
                }
                AppPhase::Playing => {
                    changed |= handle_playing_input(&keys, &window, &mut game, &mut camera);
                    changed |= game.update(delta_seconds);
                    if game.game_over {
                        phase = AppPhase::Death { elapsed: 0.0 };
                        death_background = None;
                        death_fade = Some(0.0);
                        changed = true;
                    }
                }
                AppPhase::Death { elapsed } => {
                    let was_animating = *elapsed < player::PLAYER_DEATH_ANIMATION_SECONDS;
                    *elapsed += delta_seconds;
                    death_fade = Some((*elapsed / DEATH_FADE_SECONDS).clamp(0.0, 1.0));
                    changed |= was_animating;
                }
            }
        }

        if changed {
            scene = scene::build_scene_with_player(&game, player_animation(&game, &phase, &camera));
            renderer::render(
                &mut framebuffer,
                &camera,
                &scene.cubes,
                &scene.spheres,
                &scene.cylinders,
                &scene.forest_props,
                render_resources,
            );
            if let AppPhase::Intro { elapsed } = &phase {
                ui::blur(&mut framebuffer.color, WIDTH, HEIGHT);
                intro_background = Some(framebuffer.color.clone());
                ui::draw_intro(
                    &mut framebuffer.color,
                    WIDTH,
                    HEIGHT,
                    &ui_assets,
                    intro_rise_progress(*elapsed),
                );
            }
            if matches!(&phase, AppPhase::Death { .. }) {
                death_background = Some(framebuffer.color.clone());
            }
        }
        if let Some(fade) = death_fade {
            if let Some(background) = &death_background {
                framebuffer.color.copy_from_slice(background);
            }
            ui::draw_death(&mut framebuffer.color, WIDTH, HEIGHT, &ui_assets, fade);
        }
        update_title(&window, &game, &camera, &phase);
        window.present(&framebuffer.color);
        thread::sleep(Duration::from_millis(16));
    }
}

fn intro_rise_progress(elapsed: f32) -> f32 {
    ((elapsed - INTRO_HOLD_SECONDS) / INTRO_RISE_SECONDS).clamp(0.0, 1.0)
}

fn player_animation(
    game: &Game,
    phase: &AppPhase,
    camera: &OrbitCamera,
) -> player::PlayerAnimation {
    match phase {
        AppPhase::Death { elapsed } => player::PlayerAnimation::Dying {
            elapsed: *elapsed,
            camera_position: camera.eye,
            kind: match game.death_kind() {
                Some(DeathKind::FellOffWorld) => game
                    .cliff_fall_position()
                    .map(|(x, y, z)| player::DeathAnimationKind::CliffFall {
                        origin: Vec3::new(x, y, z),
                    })
                    .unwrap_or(player::DeathAnimationKind::Standard),
                Some(DeathKind::Hazard) | None => player::DeathAnimationKind::Standard,
            },
        },
        AppPhase::Intro { .. } | AppPhase::Ready | AppPhase::Playing => {
            player::PlayerAnimation::Alive
        }
    }
}

fn handle_ready_input(
    keys: &[Key],
    window: &NativeWindow,
    game: &mut Game,
    camera: &mut OrbitCamera,
    phase: &mut AppPhase,
) -> bool {
    if keys.iter().any(|key| matches!(key, Key::Pause)) {
        game.toggle_pause();
    }
    if game.paused {
        return handle_orbit_input(window, camera);
    }
    if keys
        .iter()
        .any(|key| matches!(key, Key::Start | Key::Left | Key::Right | Key::Down))
    {
        *phase = AppPhase::Playing;
        return true;
    }
    false
}

fn handle_playing_input(
    keys: &[Key],
    window: &NativeWindow,
    game: &mut Game,
    camera: &mut OrbitCamera,
) -> bool {
    if keys.iter().any(|key| matches!(key, Key::Pause)) {
        game.toggle_pause();
    }
    if game.paused {
        return handle_orbit_input(window, camera);
    }
    let mut changed = false;
    for key in keys {
        let direction = match key {
            Key::Left => Some(Move::Left),
            Key::Right => Some(Move::Right),
            Key::Start | Key::Up => Some(Move::Forward),
            Key::Down => Some(Move::Backward),
            Key::Pause | Key::Reset | Key::Orthographic => None,
        };
        if let Some(direction) = direction {
            changed |= game.try_move(direction);
        }
    }
    changed
}

fn handle_orbit_input(window: &NativeWindow, camera: &mut OrbitCamera) -> bool {
    let mut changed = false;
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
    changed
}

fn update_title(window: &NativeWindow, game: &Game, camera: &OrbitCamera, phase: &AppPhase) {
    let state = match phase {
        AppPhase::Intro { .. } => "INTRO: R reinicia",
        AppPhase::Ready if game.paused => "LISTO: pausa para orbitar",
        AppPhase::Ready => "LISTO: WASD inicia",
        AppPhase::Playing if game.paused => "PAUSA: A/D rotar, W/S zoom",
        AppPhase::Playing => "jugando",
        AppPhase::Death { .. } => "YOU DIED: R reinicia",
    };
    let projection = match camera.projection() {
        Projection::Perspective => "perspectiva",
        Projection::Orthographic => "ortográfica",
    };
    window.set_title(&format!(
        "Creative Zone | Puntos: {} | {} | {} | WASD/Flechas, Espacio, O, R",
        game.score, state, projection
    ));
}
