mod audio;
mod color;
mod cube;
mod cylinder;
mod display;
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
use display::{DisplayPreset, DEFAULT_DISPLAY_PRESET};
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

const TARGET_FPS: u32 = 30;
const TARGET_FRAME_DURATION: Duration = Duration::from_micros(33_333);
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

#[derive(Default)]
struct AdaptiveQuality {
    slow_frames: u32,
    fast_frames: u32,
}

impl AdaptiveQuality {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn observe(
        &mut self,
        preset: DisplayPreset,
        render_size: (usize, usize),
        render_time: Duration,
    ) -> Option<(usize, usize)> {
        let frame_ms = render_time.as_secs_f32() * 1_000.0;
        if frame_ms > 30.5 {
            self.slow_frames += 1;
            self.fast_frames = 0;
            if self.slow_frames >= 2 {
                self.slow_frames = 0;
                return lower_render_size_for_budget(preset, render_size, frame_ms);
            }
        } else if frame_ms < 24.0 {
            self.fast_frames += 1;
            self.slow_frames = 0;
            if self.fast_frames >= 120 {
                self.fast_frames = 0;
                return adjacent_render_size(preset, render_size, 1);
            }
        } else {
            self.slow_frames = 0;
            self.fast_frames = 0;
        }
        None
    }
}

fn adjacent_render_size(
    preset: DisplayPreset,
    current: (usize, usize),
    direction: i32,
) -> Option<(usize, usize)> {
    let sizes = preset.adaptive_render_sizes();
    let index = sizes.iter().position(|&candidate| candidate == current)? as i32;
    sizes.get((index + direction) as usize).copied()
}

fn lower_render_size_for_budget(
    preset: DisplayPreset,
    current: (usize, usize),
    frame_ms: f32,
) -> Option<(usize, usize)> {
    let sizes = preset.adaptive_render_sizes();
    let current_index = sizes.iter().position(|&candidate| candidate == current)?;
    if current_index == 0 {
        return None;
    }

    // El costo crece aproximadamente con la cantidad de píxeles. Estimamos
    // el área que cabe en 29 ms y elegimos el escalón más alto seguro.
    let target_pixels = current.0 as f32 * current.1 as f32 * (29.0 / frame_ms);
    sizes[..current_index]
        .iter()
        .rev()
        .find(|&&(width, height)| width as f32 * height as f32 <= target_pixels)
        .copied()
        .or_else(|| sizes.first().copied())
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
    let mut display_preset = DEFAULT_DISPLAY_PRESET;
    let (window_width, window_height) = display_preset.window_size();
    let (render_width, render_height) = display_preset.render_size();
    let window = NativeWindow::new("Crossy Tracing - Raytracing", window_width, window_height);
    let mut framebuffer = Framebuffer::new(render_width, render_height);
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
    let mut diorama_effect = ui::DioramaEffect::new(render_width, render_height);
    let mut phase = AppPhase::Intro { elapsed: 0.0 };
    let mut scene = scene::build_scene_with_player(&game, player_animation(&game, &phase, &camera));
    let mut previous_frame = Instant::now();
    let mut intro_background: Option<Vec<u32>>;
    let mut death_background: Option<Vec<u32>> = None;
    let mut performance = PerformanceStats::default();
    let mut adaptive_quality = AdaptiveQuality::default();
    let mut pending_render_size = None;

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
        &framebuffer,
        display_preset,
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
        if let Some((render_width, render_height)) = pending_render_size.take() {
            framebuffer = Framebuffer::new(render_width, render_height);
            diorama_effect = ui::DioramaEffect::new(render_width, render_height);
            intro_background = None;
            death_background = None;
            changed = true;
        }
        if let Some(requested_preset) = requested_display_preset(&keys) {
            if requested_preset != display_preset {
                display_preset = requested_preset;
                adaptive_quality.reset();
                pending_render_size = None;
                let (window_width, window_height) = display_preset.window_size();
                let (render_width, render_height) = display_preset.render_size();
                window.set_client_size(window_width, window_height);
                framebuffer = Framebuffer::new(render_width, render_height);
                diorama_effect = ui::DioramaEffect::new(render_width, render_height);
                intro_background = None;
                death_background = None;
                changed = true;
            }
        }
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
            let render_elapsed = render_started.elapsed();
            performance.record(render_elapsed);
            pending_render_size = adaptive_quality.observe(
                display_preset,
                (framebuffer.width, framebuffer.height),
                render_elapsed,
            );
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
        audio.sync_music(!matches!(&phase, AppPhase::Death { .. }), game.paused);
        if matches!(&phase, AppPhase::Playing) {
            for event in game.drain_sound_events() {
                audio.play(event);
            }
        }
        audio.update(delta_seconds);
        update_title(
            &window,
            &game,
            &camera,
            &phase,
            &performance,
            &framebuffer,
            display_preset,
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
    // mayor parte y reservar un margen corto de espera activa evita saltos
    // visibles alrededor del objetivo de 33.33 ms.
    let remaining = TARGET_FRAME_DURATION - elapsed;
    let spin_margin = Duration::from_millis(2);
    if remaining > spin_margin {
        thread::sleep(remaining - spin_margin);
    }
    while started.elapsed() < TARGET_FRAME_DURATION {
        std::hint::spin_loop();
    }
}

fn requested_display_preset(keys: &[Key]) -> Option<DisplayPreset> {
    keys.iter().find_map(|key| match key {
        Key::DisplayPerformance => DisplayPreset::from_shortcut_index(0),
        Key::DisplayHigh => DisplayPreset::from_shortcut_index(1),
        Key::DisplayUltra => DisplayPreset::from_shortcut_index(2),
        _ => None,
    })
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
            Key::Pause
            | Key::Reset
            | Key::Orthographic
            | Key::DisplayPerformance
            | Key::DisplayHigh
            | Key::DisplayUltra => None,
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
    framebuffer: &Framebuffer,
    display_preset: DisplayPreset,
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
        "Crossy Tracing | Puntos: {} | {} | {} | Calidad {} ({}x{}, render {:.1} ms, objetivo {} FPS) | F1/F2/F3 calidad | WASD/Flechas, Espacio, O, R",
        game.score,
        state,
        projection,
        display_preset.label(),
        framebuffer.width,
        framebuffer.height,
        performance.frame_ms,
        TARGET_FPS,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_quality_reduces_resolution_after_repeated_slow_frames() {
        let mut quality = AdaptiveQuality::default();
        let mut next = None;
        for _ in 0..2 {
            next = quality.observe(DisplayPreset::High, (816, 612), Duration::from_millis(35));
        }

        assert_eq!(next, Some((720, 540)));
    }

    #[test]
    fn adaptive_quality_recovers_detail_only_after_sustained_fast_frames() {
        let mut quality = AdaptiveQuality::default();
        let mut next = None;
        for _ in 0..120 {
            next = quality.observe(DisplayPreset::High, (720, 540), Duration::from_millis(20));
        }

        assert_eq!(next, Some((768, 576)));
    }

    #[test]
    fn ultra_quality_also_reduces_resolution_to_protect_thirty_fps() {
        let mut quality = AdaptiveQuality::default();
        let mut next = None;
        for _ in 0..2 {
            next = quality.observe(DisplayPreset::Ultra, (960, 720), Duration::from_millis(50));
        }

        assert_eq!(next, Some((720, 540)));
    }
}
