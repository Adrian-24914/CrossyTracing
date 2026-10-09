use crate::{
    audio::Audio,
    display::{DisplayPreset, DEFAULT_DISPLAY_PRESET},
    framebuffer::Framebuffer,
    game::{DeathKind, Game, Move},
    math::Vec3,
    orbit_camera::{OrbitCamera, Projection},
    player,
    renderer::{self, RenderResources},
    scene::{self, Scene},
    skybox::Skybox,
    ui::{self, DioramaEffect, UiAssets},
    window::{Key, NativeWindow},
};
use std::{
    f32::consts::FRAC_PI_4,
    time::{Duration, Instant},
};

pub const TARGET_FPS: u32 = 30;
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

pub struct App {
    display_preset: DisplayPreset,
    framebuffer: Framebuffer,
    camera: OrbitCamera,
    game: Game,
    audio: Audio,
    skybox: Skybox,
    ui_assets: UiAssets,
    diorama_effect: DioramaEffect,
    phase: AppPhase,
    scene: Scene,
    intro_background: Option<Vec<u32>>,
    death_background: Option<Vec<u32>>,
    death_fade: Option<f32>,
    performance: PerformanceStats,
    adaptive_quality: AdaptiveQuality,
    pending_render_size: Option<(usize, usize)>,
    needs_render: bool,
}

impl App {
    pub fn initial_window_size() -> (usize, usize) {
        DEFAULT_DISPLAY_PRESET.window_size()
    }

    pub fn new() -> Self {
        let display_preset = DEFAULT_DISPLAY_PRESET;
        let (render_width, render_height) = display_preset.render_size();
        let framebuffer = Framebuffer::new(render_width, render_height);
        let camera = OrbitCamera::new(
            Vec3::new(8.5, 8.5, 11.5),
            Vec3::new(0.0, 0.0, 0.0),
            FRAC_PI_4,
        );
        let game = Game::new();
        let phase = AppPhase::Intro { elapsed: 0.0 };
        let scene = scene::build_scene_with_player(&game, player_animation(&game, &phase, &camera));

        let mut app = Self {
            display_preset,
            framebuffer,
            camera,
            game,
            audio: Audio::new(),
            skybox: Skybox::load().expect("No se pudo cargar assets/skybox/runtime"),
            ui_assets: UiAssets::load(),
            diorama_effect: DioramaEffect::new(render_width, render_height),
            phase,
            scene,
            intro_background: None,
            death_background: None,
            death_fade: None,
            performance: PerformanceStats::default(),
            adaptive_quality: AdaptiveQuality::default(),
            pending_render_size: None,
            needs_render: false,
        };
        app.render_current_scene(None);
        app.compose_overlays();
        app
    }

    pub fn framebuffer(&self) -> &Framebuffer {
        &self.framebuffer
    }

    pub fn update(&mut self, keys: &[Key], window: &NativeWindow, delta_seconds: f32) {
        self.death_fade = None;
        let reset = keys.iter().any(|key| matches!(key, Key::Reset));
        let mut changed = false;

        if let Some((render_width, render_height)) = self.pending_render_size.take() {
            self.resize_rendering(render_width, render_height);
            changed = true;
        }

        if let Some(requested_preset) = requested_display_preset(keys) {
            if requested_preset != self.display_preset {
                self.display_preset = requested_preset;
                self.adaptive_quality.reset();
                self.pending_render_size = None;
                let (window_width, window_height) = self.display_preset.window_size();
                let (render_width, render_height) = self.display_preset.render_size();
                window.set_client_size(window_width, window_height);
                self.resize_rendering(render_width, render_height);
                changed = true;
            }
        }

        if reset {
            self.game.reset();
            self.phase = AppPhase::Intro { elapsed: 0.0 };
            self.intro_background = None;
            self.death_background = None;
            changed = true;
        } else {
            if keys.iter().any(|key| matches!(key, Key::Orthographic)) {
                self.camera.toggle_projection();
                changed = true;
            }

            match &mut self.phase {
                AppPhase::Intro { elapsed } => {
                    *elapsed += delta_seconds;
                    if self.intro_background.is_none() {
                        changed = true;
                    }
                    if *elapsed >= INTRO_DURATION_SECONDS {
                        self.phase = AppPhase::Ready;
                        changed = true;
                    }
                }
                AppPhase::Ready => {
                    changed |= handle_ready_input(
                        keys,
                        window,
                        &mut self.game,
                        &mut self.camera,
                        &mut self.phase,
                    );
                }
                AppPhase::Playing => {
                    changed |= handle_playing_input(keys, window, &mut self.game, &mut self.camera);
                    changed |= self.game.update(delta_seconds);
                    if self.game.game_over {
                        self.phase = AppPhase::Death { elapsed: 0.0 };
                        self.death_background = None;
                        self.death_fade = Some(0.0);
                        changed = true;
                    }
                }
                AppPhase::Death { elapsed } => {
                    let was_animating = *elapsed < player::PLAYER_DEATH_ANIMATION_SECONDS;
                    *elapsed += delta_seconds;
                    self.death_fade = Some((*elapsed / DEATH_FADE_SECONDS).clamp(0.0, 1.0));
                    changed |= was_animating;
                }
            }
        }

        self.needs_render |= changed;
        self.audio.sync_music(
            !matches!(&self.phase, AppPhase::Death { .. }),
            self.game.paused,
        );
        if matches!(&self.phase, AppPhase::Playing) {
            for event in self.game.drain_sound_events() {
                self.audio.play(event);
            }
        }
        self.audio.update(delta_seconds);
    }

    pub fn render(&mut self) {
        if self.needs_render {
            let render_started = Instant::now();
            self.scene = scene::build_scene_with_player(
                &self.game,
                player_animation(&self.game, &self.phase, &self.camera),
            );
            self.render_current_scene(Some(render_started));
            self.needs_render = false;
        }
        self.compose_overlays();
    }

    pub fn window_title(&self) -> String {
        let state = match &self.phase {
            AppPhase::Intro { .. } => "INTRO: R reinicia",
            AppPhase::Ready if self.game.paused => "LISTO: pausa para orbitar",
            AppPhase::Ready => "LISTO: WASD inicia",
            AppPhase::Playing if self.game.paused => "PAUSA: A/D rotar, W/S zoom",
            AppPhase::Playing => "jugando",
            AppPhase::Death { .. } => "YOU DIED: R reinicia",
        };
        let projection = match self.camera.projection() {
            Projection::Perspective => "perspectiva",
            Projection::Orthographic => "ortográfica",
        };
        format!(
            "Crossy Tracing | Puntos: {} | {} | {} | Calidad {} ({}x{}, render {:.1} ms, objetivo {} FPS) | F1/F2/F3 calidad | WASD/Flechas, Espacio, O, R",
            self.game.score,
            state,
            projection,
            self.display_preset.label(),
            self.framebuffer.width,
            self.framebuffer.height,
            self.performance.frame_ms,
            TARGET_FPS,
        )
    }

    fn resize_rendering(&mut self, width: usize, height: usize) {
        self.framebuffer = Framebuffer::new(width, height);
        self.diorama_effect = DioramaEffect::new(width, height);
        self.intro_background = None;
        self.death_background = None;
    }

    fn render_current_scene(&mut self, render_started: Option<Instant>) {
        renderer::render(
            &mut self.framebuffer,
            &self.camera,
            &self.scene.cubes,
            &self.scene.spheres,
            &self.scene.cylinders,
            &self.scene.forest_props,
            RenderResources {
                skybox: &self.skybox,
            },
        );

        if matches!(&self.phase, AppPhase::Ready | AppPhase::Playing) {
            self.diorama_effect.apply(&mut self.framebuffer.color);
        }
        if matches!(&self.phase, AppPhase::Intro { .. }) {
            let width = self.framebuffer.width;
            let height = self.framebuffer.height;
            ui::blur(&mut self.framebuffer.color, width, height);
            self.intro_background = Some(self.framebuffer.color.clone());
        }
        if matches!(&self.phase, AppPhase::Death { .. }) {
            self.death_background = Some(self.framebuffer.color.clone());
        }

        if let Some(render_started) = render_started {
            let render_elapsed = render_started.elapsed();
            self.performance.record(render_elapsed);
            self.pending_render_size = self.adaptive_quality.observe(
                self.display_preset,
                (self.framebuffer.width, self.framebuffer.height),
                render_elapsed,
            );
        }
    }

    fn compose_overlays(&mut self) {
        if let AppPhase::Intro { elapsed } = &self.phase {
            if let Some(background) = &self.intro_background {
                self.framebuffer.color.copy_from_slice(background);
                ui::draw_intro(
                    &mut self.framebuffer.color,
                    self.framebuffer.width,
                    self.framebuffer.height,
                    &self.ui_assets,
                    intro_rise_progress(*elapsed),
                );
            }
        }

        if let Some(fade) = self.death_fade {
            if let Some(background) = &self.death_background {
                self.framebuffer.color.copy_from_slice(background);
            }
            ui::draw_death(
                &mut self.framebuffer.color,
                self.framebuffer.width,
                self.framebuffer.height,
                &self.ui_assets,
                fade,
            );
        }
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

    let target_pixels = current.0 as f32 * current.1 as f32 * (29.0 / frame_ms);
    sizes[..current_index]
        .iter()
        .rev()
        .find(|&&(width, height)| width as f32 * height as f32 <= target_pixels)
        .copied()
        .or_else(|| sizes.first().copied())
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
