mod audio;
mod color;
mod cube;
mod cylinder;
mod framebuffer;
mod game;
mod leaf_cube;
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

use audio::Audio;
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
// El raytracer trabaja a la misma resolucion de la ventana. En release esta
// configuracion ronda el objetivo de 15 FPS sin reescalado ni perdida visual.
const RENDER_WIDTH: usize = WIDTH;
const RENDER_HEIGHT: usize = HEIGHT;
const TARGET_FPS: u32 = 15;
const TARGET_FRAME_DURATION: Duration = Duration::from_micros(66_667);
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

#[derive(Default)]
struct PerformanceStats {
    frame_ms: f32,
}

impl PerformanceStats {
    fn record(&mut self, elapsed: Duration) {
        let measured = elapsed.as_secs_f32() * 1_000.0;
        self.frame_ms = if self.frame_ms == 0.0 {
            measured
        } else {
            self.frame_ms * 0.82 + measured * 0.18
        };
    }

}

fn main() {
    let window = NativeWindow::new("Creative Zone - Raytracing", WIDTH, HEIGHT);
    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);
    let mut camera = OrbitCamera::new(
        Vec3::new(8.5, 8.5, 11.5),
        Vec3::new(0.0, 0.0, 0.0),
        FRAC_PI_4,
    );
    let mut game = Game::new();
    let mut audio = Audio::new();
    let skybox = Skybox::load().expect("No se pudo cargar assets/skybox/runtime");
    let render_resources = RenderResources { skybox: &skybox };
    let ui_assets = ui::UiAssets::load();
    let mut diorama_effect = ui::DioramaEffect::new(RENDER_WIDTH, RENDER_HEIGHT);
    let mut phase = AppPhase::Intro { elapsed: 0.0 };
    let mut scene = scene::build_scene_with_player(&game, player_animation(&game, &phase, &camera));
    let mut previous_frame = Instant::now();
    let mut intro_background: Option<Vec<u32>>;
    let mut death_background: Option<Vec<u32>> = None;
    let mut performance = PerformanceStats::default();

    renderer::render(
        &mut framebuffer,
        &camera,
        &scene.cubes,
        &scene.spheres,
        &scene.cylinders,
        &scene.forest_props,
        render_resources,
    );
    let render_width = framebuffer.width;
    let render_height = framebuffer.height;
    ui::blur(&mut framebuffer.color, render_width, render_height);
    intro_background = Some(framebuffer.color.clone());
    update_title(
        &window,
        &game,
        &camera,
        &phase,
        &performance,
        framebuffer.width,
        framebuffer.height,
    );

    loop {
        let frame_started = Instant::now();
        let Some(keys) = window.pump_messages() else {
            break;
        };
        let now = Instant::now();
        let delta_seconds = (now - previous_frame).as_secs_f32().min(0.10);
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
                        let render_width = framebuffer.width;
                        let render_height = framebuffer.height;
                        ui::blur(&mut framebuffer.color, render_width, render_height);
                        intro_background = Some(framebuffer.color.clone());
                    }
                    if *elapsed >= INTRO_DURATION_SECONDS {
                        phase = AppPhase::Ready;
                        changed = true;
                    } else if let Some(background) = &intro_background {
                        framebuffer.color.copy_from_slice(background);
                        ui::draw_intro(
                            &mut framebuffer.color,
                            framebuffer.width,
                            framebuffer.height,
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
            let render_started = Instant::now();
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
            if matches!(&phase, AppPhase::Ready | AppPhase::Playing) {
                diorama_effect.apply(&mut framebuffer.color);
            }
            if let AppPhase::Intro { elapsed } = &phase {
                let render_width = framebuffer.width;
                let render_height = framebuffer.height;
                ui::blur(&mut framebuffer.color, render_width, render_height);
                intro_background = Some(framebuffer.color.clone());
                ui::draw_intro(
                    &mut framebuffer.color,
                    render_width,
                    render_height,
                    &ui_assets,
                    intro_rise_progress(*elapsed),
                );
            }
            if matches!(&phase, AppPhase::Death { .. }) {
                death_background = Some(framebuffer.color.clone());
            }
            performance.record(render_started.elapsed());
        }
        if let Some(fade) = death_fade {
            if let Some(background) = &death_background {
                framebuffer.color.copy_from_slice(background);
            }
            let render_width = framebuffer.width;
            let render_height = framebuffer.height;
            ui::draw_death(
                &mut framebuffer.color,
                render_width,
                render_height,
                &ui_assets,
                fade,
            );
        }
        audio.sync_music(matches!(&phase, AppPhase::Playing), game.paused);
        if matches!(&phase, AppPhase::Playing) {
            for event in game.drain_sound_events() {
                audio.play(event);
            }
        }
        update_title(
            &window,
            &game,
            &camera,
            &phase,
            &performance,
            framebuffer.width,
            framebuffer.height,
        );
        window.present_scaled(&framebuffer.color, framebuffer.width, framebuffer.height);
        wait_for_target_frame(frame_started);
    }
}

fn wait_for_target_frame(started: Instant) {
    let elapsed = started.elapsed();
    if elapsed >= TARGET_FRAME_DURATION {
        return;
    }

    // Windows puede despertar un sleep algunos milisegundos tarde. Dormir la
    // mayor parte y ceder el último milisegundo mantiene el ritmo mucho más
    // cerca de 66.67 ms sin consumir un core completo.
    let remaining = TARGET_FRAME_DURATION - elapsed;
    let spin_margin = Duration::from_millis(1);
    if remaining > spin_margin {
        thread::sleep(remaining - spin_margin);
    }
    while started.elapsed() < TARGET_FRAME_DURATION {
        thread::yield_now();
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

fn update_title(
    window: &NativeWindow,
    game: &Game,
    camera: &OrbitCamera,
    phase: &AppPhase,
    performance: &PerformanceStats,
    render_width: usize,
    render_height: usize,
) {
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
        "Creative Zone | Puntos: {} | {} | {} | {} FPS fijos (render {:.1} ms, {}x{}) | WASD/Flechas, Espacio, O, R",
        game.score,
        state,
        projection,
        TARGET_FPS,
        performance.frame_ms,
        render_width,
        render_height,
    ));
}
