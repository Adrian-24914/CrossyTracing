#![allow(dead_code)]

use crate::input::{InputState, Key};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{closure::Closure, Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, Document, HtmlCanvasElement, ImageData, KeyboardEvent};

#[derive(Default)]
struct KeyboardState {
    pressed: RefCell<Vec<Key>>,
    held_codes: RefCell<Vec<String>>,
}

pub struct NativeWindow {
    document: Document,
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    input: Rc<KeyboardState>,
    client_size: Cell<(usize, usize)>,
    rgba: RefCell<Vec<u8>>,
    _key_down: Closure<dyn FnMut(KeyboardEvent)>,
    _key_up: Closure<dyn FnMut(KeyboardEvent)>,
}

impl NativeWindow {
    pub fn new(title: &str, width: usize, height: usize) -> Self {
        let browser = web_sys::window().expect("No se encontró window");
        let document = browser.document().expect("No se encontró document");
        document.set_title(title);
        let canvas = document
            .get_element_by_id("game")
            .expect("No se encontró <canvas id=\"game\">")
            .dyn_into::<HtmlCanvasElement>()
            .expect("#game no es un canvas");
        let context = canvas
            .get_context("2d")
            .expect("No se pudo consultar el contexto 2D")
            .expect("Canvas 2D no está disponible")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("El contexto no es CanvasRenderingContext2d");

        let input = Rc::new(KeyboardState::default());
        let key_down_input = Rc::clone(&input);
        let key_down = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let code = event.code();
            let Some(pressed_key) = pressed_key_for_code(&code) else {
                return;
            };
            event.prevent_default();
            if !event.repeat() {
                key_down_input.pressed.borrow_mut().push(pressed_key);
            }
            let mut held_codes = key_down_input.held_codes.borrow_mut();
            if !held_codes.contains(&code) {
                held_codes.push(code);
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);
        browser
            .add_event_listener_with_callback("keydown", key_down.as_ref().unchecked_ref())
            .expect("No se pudo registrar keydown");

        let key_up_input = Rc::clone(&input);
        let key_up = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let code = event.code();
            if held_key_for_code(&code).is_none() {
                return;
            }
            event.prevent_default();
            key_up_input
                .held_codes
                .borrow_mut()
                .retain(|held| held != &code);
        }) as Box<dyn FnMut(KeyboardEvent)>);
        browser
            .add_event_listener_with_callback("keyup", key_up.as_ref().unchecked_ref())
            .expect("No se pudo registrar keyup");

        let window = Self {
            document,
            canvas,
            context,
            input,
            client_size: Cell::new((width, height)),
            rgba: RefCell::new(Vec::new()),
            _key_down: key_down,
            _key_up: key_up,
        };
        window.set_client_size(width, height);
        window
    }

    pub fn pump_messages(&self) -> Option<InputState> {
        let pressed = std::mem::take(&mut *self.input.pressed.borrow_mut());
        let held_codes = self.input.held_codes.borrow();
        Some(InputState::new(pressed, |key| {
            held_codes
                .iter()
                .any(|code| held_key_for_code(code) == Some(key))
        }))
    }

    pub fn set_title(&self, title: &str) {
        self.document.set_title(title);
    }

    pub fn set_client_size(&self, width: usize, height: usize) {
        self.client_size.set((width, height));
        let style = self.canvas.style();
        style
            .set_property(
                "width",
                &format!("min({width}px, 100vw, calc(100vh * 4 / 3))"),
            )
            .expect("No se pudo cambiar el ancho del canvas");
        style
            .set_property("height", "auto")
            .expect("No se pudo cambiar el alto del canvas");
    }

    pub fn client_size(&self) -> (usize, usize) {
        self.client_size.get()
    }

    pub fn present_scaled(&self, pixels: &[u32], source_width: usize, source_height: usize) {
        assert_eq!(pixels.len(), source_width * source_height);
        if self.canvas.width() != source_width as u32 {
            self.canvas.set_width(source_width as u32);
        }
        if self.canvas.height() != source_height as u32 {
            self.canvas.set_height(source_height as u32);
        }

        let mut rgba = self.rgba.borrow_mut();
        rgba.resize(pixels.len() * 4, 0);
        for (color, target) in pixels.iter().zip(rgba.chunks_exact_mut(4)) {
            target[0] = (color >> 16) as u8;
            target[1] = (color >> 8) as u8;
            target[2] = *color as u8;
            target[3] = 255;
        }

        let image = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(rgba.as_slice()),
            source_width as u32,
            source_height as u32,
        )
        .expect("No se pudo crear ImageData");
        self.context
            .put_image_data(&image, 0.0, 0.0)
            .expect("No se pudo presentar el framebuffer");
    }
}

fn pressed_key_for_code(code: &str) -> Option<Key> {
    match code {
        "KeyA" | "ArrowLeft" => Some(Key::Left),
        "KeyD" | "ArrowRight" => Some(Key::Right),
        "KeyW" => Some(Key::Start),
        "ArrowUp" => Some(Key::Up),
        "KeyS" | "ArrowDown" => Some(Key::Down),
        "Space" => Some(Key::Pause),
        "KeyR" => Some(Key::Reset),
        "KeyO" => Some(Key::Orthographic),
        "F1" => Some(Key::DisplayPerformance),
        "F2" => Some(Key::DisplayHigh),
        "F3" => Some(Key::DisplayUltra),
        _ => None,
    }
}

fn held_key_for_code(code: &str) -> Option<Key> {
    match code {
        "KeyW" => Some(Key::Up),
        _ => pressed_key_for_code(code),
    }
}
