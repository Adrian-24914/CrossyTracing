mod app;
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

use app::App;
use std::{
    thread,
    time::{Duration, Instant},
};
use window::NativeWindow;

const TARGET_FRAME_DURATION: Duration = Duration::from_micros(33_333);

fn main() {
    let (window_width, window_height) = App::initial_window_size();
    let window = NativeWindow::new("Crossy Tracing - Raytracing", window_width, window_height);
    let mut app = App::new();
    let mut previous_frame = Instant::now();

    loop {
        let frame_started = Instant::now();
        let Some(keys) = window.pump_messages() else {
            break;
        };
        let now = Instant::now();
        let delta_seconds = (now - previous_frame).as_secs_f32().min(0.10);
        previous_frame = now;

        app.update(&keys, &window, delta_seconds);
        app.render();
        window.set_title(&app.window_title());
        let framebuffer = app.framebuffer();
        window.present_scaled(
            &framebuffer.color,
            framebuffer.width,
            framebuffer.height,
        );
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
