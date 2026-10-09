use crate::{app::TARGET_FPS, App, NativeWindow};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{closure::Closure, prelude::*, JsCast};
use web_sys::HtmlElement;

const TARGET_FRAME_MS: f64 = 1_000.0 / TARGET_FPS as f64;

thread_local! {
    static ANIMATION_FRAME: RefCell<Option<Closure<dyn FnMut(f64)>>> = RefCell::new(None);
}

struct WebRuntime {
    app: App,
    window: NativeWindow,
    metrics_element: HtmlElement,
    metrics: FrameMetrics,
    previous_frame_ms: Option<f64>,
}

#[derive(Default)]
struct FrameMetrics {
    sample_started_ms: Option<f64>,
    frames: u32,
    update_ms: f64,
    render_ms: f64,
    present_ms: f64,
    total_ms: f64,
}

impl FrameMetrics {
    fn record(
        &mut self,
        measured_at_ms: f64,
        update_ms: f64,
        render_ms: f64,
        present_ms: f64,
        total_ms: f64,
        raytracing_ms: f32,
        render_size: (usize, usize),
    ) -> Option<String> {
        let sample_started_ms = *self.sample_started_ms.get_or_insert(measured_at_ms);
        self.frames += 1;
        self.update_ms += update_ms;
        self.render_ms += render_ms;
        self.present_ms += present_ms;
        self.total_ms += total_ms;

        let sample_ms = measured_at_ms - sample_started_ms;
        if sample_ms < 1_000.0 {
            return None;
        }

        let frames = self.frames as f64;
        let report = format!(
            "WASM {}x{} | FPS {:.1}/{} | frame {:.1} ms | update {:.2} ms | render {:.1} ms | canvas {:.1} ms | raytracing {:.1} ms",
            render_size.0,
            render_size.1,
            frames * 1_000.0 / sample_ms,
            TARGET_FPS,
            self.total_ms / frames,
            self.update_ms / frames,
            self.render_ms / frames,
            self.present_ms / frames,
            raytracing_ms,
        );

        *self = Self {
            sample_started_ms: Some(measured_at_ms),
            ..Self::default()
        };
        Some(report)
    }
}

impl WebRuntime {
    fn new() -> Self {
        let (width, height) = App::initial_window_size();
        let metrics_element = web_sys::window()
            .expect("browser window must be available")
            .document()
            .expect("browser document must be available")
            .get_element_by_id("metrics")
            .expect("No se encontró #metrics")
            .dyn_into::<HtmlElement>()
            .expect("#metrics no es un elemento HTML");
        Self {
            app: App::new(),
            window: NativeWindow::new("Crossy Tracing - Raytracing", width, height),
            metrics_element,
            metrics: FrameMetrics::default(),
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

        let frame_started_ms = performance_now();

        let Some(input) = self.window.pump_messages() else {
            return;
        };
        self.app.update(&input, delta_seconds);
        if let Some((width, height)) = self.app.take_window_size_request() {
            self.window.set_client_size(width, height);
        }
        let update_finished_ms = performance_now();
        self.app.render();
        let render_finished_ms = performance_now();
        self.window.set_title(&self.app.window_title());
        let framebuffer = self.app.framebuffer();
        let render_size = (framebuffer.width, framebuffer.height);
        self.window
            .present_scaled(&framebuffer.color, framebuffer.width, framebuffer.height);
        let present_finished_ms = performance_now();

        if let Some(report) = self.metrics.record(
            present_finished_ms,
            update_finished_ms - frame_started_ms,
            render_finished_ms - update_finished_ms,
            present_finished_ms - render_finished_ms,
            present_finished_ms - frame_started_ms,
            self.app.render_time_ms(),
            render_size,
        ) {
            self.metrics_element.set_inner_text(&report);
        }
    }
}

fn performance_now() -> f64 {
    web_sys::window()
        .expect("browser window must be available")
        .performance()
        .expect("Performance API must be available")
        .now()
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
