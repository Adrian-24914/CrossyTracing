use crate::{app::TARGET_FPS, App, NativeWindow};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{closure::Closure, prelude::*, JsCast};

const TARGET_FRAME_MS: f64 = 1_000.0 / TARGET_FPS as f64;

thread_local! {
    static ANIMATION_FRAME: RefCell<Option<Closure<dyn FnMut(f64)>>> = RefCell::new(None);
}

struct WebRuntime {
    app: App,
    window: NativeWindow,
    previous_frame_ms: Option<f64>,
}

impl WebRuntime {
    fn new() -> Self {
        let (width, height) = App::initial_window_size();
        Self {
            app: App::new(),
            window: NativeWindow::new("Crossy Tracing - Raytracing", width, height),
            previous_frame_ms: None,
        }
    }

    fn frame(&mut self, timestamp_ms: f64) {
        let delta_seconds = match self.previous_frame_ms {
            Some(previous_frame_ms) => {
                let elapsed_ms = timestamp_ms - previous_frame_ms;
                if elapsed_ms < TARGET_FRAME_MS {
                    return;
                }
                (elapsed_ms / 1_000.0).min(0.10) as f32
            }
            None => 0.0,
        };
        self.previous_frame_ms = Some(timestamp_ms);

        let Some(input) = self.window.pump_messages() else {
            return;
        };
        self.app.update(&input, delta_seconds);
        if let Some((width, height)) = self.app.take_window_size_request() {
            self.window.set_client_size(width, height);
        }
        self.app.render();
        self.window.set_title(&self.app.window_title());
        let framebuffer = self.app.framebuffer();
        self.window
            .present_scaled(&framebuffer.color, framebuffer.width, framebuffer.height);
    }
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let runtime = Rc::new(RefCell::new(WebRuntime::new()));
    let animation_runtime = Rc::clone(&runtime);

    ANIMATION_FRAME.with(|animation_frame| {
        *animation_frame.borrow_mut() = Some(Closure::new(move |timestamp_ms: f64| {
            animation_runtime.borrow_mut().frame(timestamp_ms);
            schedule_animation_frame();
        }));
    });

    schedule_animation_frame();
    Ok(())
}

fn schedule_animation_frame() {
    ANIMATION_FRAME.with(|animation_frame| {
        let animation_frame = animation_frame.borrow();
        let callback = animation_frame
            .as_ref()
            .expect("animation frame callback must be initialized");
        web_sys::window()
            .expect("browser window must be available")
            .request_animation_frame(callback.as_ref().unchecked_ref())
            .expect("requestAnimationFrame must succeed");
    });
}
