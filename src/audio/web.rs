use crate::game::SoundEvent;
use js_sys::ArrayBuffer;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioScheduledSourceNode, Event, GainNode,
    Response,
};

const MUSIC_URL: &str = "/assets/audio/music.wav";
const GRASS_URL: &str = "/assets/audio/grass.wav";
const ROCKS_URL: &str = "/assets/audio/rocks.wav";
const LOG_WATER_URL: &str = "/assets/audio/log_water.wav";
const RAIL_URL: &str = "/assets/audio/rail.wav";
const TRAIN_WARNING_URL: &str = "/assets/audio/train_warning.wav";
const TRAIN_URL: &str = "/assets/audio/train.wav";

#[derive(Clone, Copy)]
enum AudioAsset {
    Music,
    Effect(SoundEvent),
}

#[derive(Default)]
struct AudioBuffers {
    music: Option<AudioBuffer>,
    grass: Option<AudioBuffer>,
    rocks: Option<AudioBuffer>,
    log_water: Option<AudioBuffer>,
    rail: Option<AudioBuffer>,
    train_warning: Option<AudioBuffer>,
    train: Option<AudioBuffer>,
}

impl AudioBuffers {
    fn set(&mut self, asset: AudioAsset, buffer: AudioBuffer) {
        match asset {
            AudioAsset::Music => self.music = Some(buffer),
            AudioAsset::Effect(SoundEvent::GrassLanding) => self.grass = Some(buffer),
            AudioAsset::Effect(SoundEvent::RockLanding) => self.rocks = Some(buffer),
            AudioAsset::Effect(SoundEvent::LogLanding) => self.log_water = Some(buffer),
            AudioAsset::Effect(SoundEvent::RailLanding) => self.rail = Some(buffer),
            AudioAsset::Effect(SoundEvent::TrainWarning) => self.train_warning = Some(buffer),
            AudioAsset::Effect(SoundEvent::TrainCrossing) => self.train = Some(buffer),
        }
    }

    fn effect(&self, event: SoundEvent) -> Option<&AudioBuffer> {
        match event {
            SoundEvent::GrassLanding => self.grass.as_ref(),
            SoundEvent::RockLanding => self.rocks.as_ref(),
            SoundEvent::LogLanding => self.log_water.as_ref(),
            SoundEvent::RailLanding => self.rail.as_ref(),
            SoundEvent::TrainWarning => self.train_warning.as_ref(),
            SoundEvent::TrainCrossing => self.train.as_ref(),
        }
    }
}

struct ActiveEffect {
    event: SoundEvent,
    source: AudioBufferSourceNode,
    gain: GainNode,
    elapsed: f32,
    duration: f32,
    fade_in: f32,
    fade_out: f32,
    maximum_volume: f32,
}

struct AudioState {
    context: AudioContext,
    buffers: AudioBuffers,
    music_source: Option<AudioBufferSourceNode>,
    active_effects: Vec<ActiveEffect>,
    unlocked: bool,
    suspended: bool,
    should_play_music: bool,
    paused: bool,
}

impl AudioState {
    fn unlock(&mut self) {
        if self.unlocked {
            return;
        }
        self.unlocked = true;
        self.suspended = false;
        let _ = self.context.resume();
        self.apply_music_state();
    }

    fn sync_music(&mut self, should_play: bool, paused: bool) {
        self.should_play_music = should_play;
        self.paused = paused;
        if !self.unlocked {
            return;
        }

        if paused && !self.suspended {
            let _ = self.context.suspend();
            self.suspended = true;
        } else if !paused && self.suspended {
            let _ = self.context.resume();
            self.suspended = false;
        }
        self.apply_music_state();
    }

    fn apply_music_state(&mut self) {
        if !self.unlocked || self.paused {
            return;
        }
        if !self.should_play_music {
            self.stop_music();
            return;
        }
        if self.music_source.is_some() {
            return;
        }
        let Some(buffer) = self.buffers.music.as_ref() else {
            return;
        };
        let Ok(source) = self.context.create_buffer_source() else {
            return;
        };
        source.set_buffer(Some(buffer));
        source.set_loop(true);
        if source
            .connect_with_audio_node(&self.context.destination())
            .is_ok()
            && source.start().is_ok()
        {
            self.music_source = Some(source);
        }
    }

    fn stop_music(&mut self) {
        if let Some(source) = self.music_source.take() {
            stop_source(&source);
            let _ = source.disconnect();
        }
    }

    fn play(&mut self, event: SoundEvent) {
        if !self.unlocked || self.paused {
            return;
        }
        let Some(buffer) = self.buffers.effect(event) else {
            return;
        };
        let Ok(source) = self.context.create_buffer_source() else {
            return;
        };
        let Ok(gain) = self.context.create_gain() else {
            return;
        };
        source.set_buffer(Some(buffer));
        gain.gain().set_value(0.0);
        if source.connect_with_audio_node(&gain).is_err()
            || gain
                .connect_with_audio_node(&self.context.destination())
                .is_err()
            || source.start().is_err()
        {
            return;
        }

        for active in &self.active_effects {
            if active.event == event {
                stop_source(&active.source);
            }
        }
        self.active_effects.retain(|active| active.event != event);
        let (duration, fade_in, fade_out, maximum_volume) = effect_settings(event);
        self.active_effects.push(ActiveEffect {
            event,
            source,
            gain,
            elapsed: 0.0,
            duration,
            fade_in,
            fade_out,
            maximum_volume,
        });
    }

    fn update(&mut self, delta_seconds: f32) {
        for effect in &mut self.active_effects {
            effect.elapsed += delta_seconds;
            let entrance = (effect.elapsed / effect.fade_in).clamp(0.0, 1.0);
            let exit = ((effect.duration - effect.elapsed) / effect.fade_out).clamp(0.0, 1.0);
            effect
                .gain
                .gain()
                .set_value(effect.maximum_volume * entrance.min(exit));
        }
        for effect in &self.active_effects {
            if effect.elapsed >= effect.duration {
                stop_source(&effect.source);
            }
        }
        self.active_effects
            .retain(|effect| effect.elapsed < effect.duration);
    }
}

pub struct Audio {
    state: Rc<RefCell<AudioState>>,
    _unlock: Closure<dyn FnMut(Event)>,
}

impl Audio {
    pub fn new() -> Self {
        let context = AudioContext::new().expect("Web Audio no está disponible");
        let state = Rc::new(RefCell::new(AudioState {
            context,
            buffers: AudioBuffers::default(),
            music_source: None,
            active_effects: Vec::new(),
            unlocked: false,
            suspended: true,
            should_play_music: true,
            paused: false,
        }));

        for (asset, url) in [
            (AudioAsset::Music, MUSIC_URL),
            (AudioAsset::Effect(SoundEvent::GrassLanding), GRASS_URL),
            (AudioAsset::Effect(SoundEvent::RockLanding), ROCKS_URL),
            (AudioAsset::Effect(SoundEvent::LogLanding), LOG_WATER_URL),
            (AudioAsset::Effect(SoundEvent::RailLanding), RAIL_URL),
            (
                AudioAsset::Effect(SoundEvent::TrainWarning),
                TRAIN_WARNING_URL,
            ),
            (AudioAsset::Effect(SoundEvent::TrainCrossing), TRAIN_URL),
        ] {
            load_asset(Rc::clone(&state), asset, url);
        }

        let unlock_state = Rc::clone(&state);
        let unlock = Closure::wrap(Box::new(move |_event: Event| {
            unlock_state.borrow_mut().unlock();
        }) as Box<dyn FnMut(Event)>);
        let browser = web_sys::window().expect("No se encontró window");
        browser
            .add_event_listener_with_callback("keydown", unlock.as_ref().unchecked_ref())
            .expect("No se pudo registrar el desbloqueo de audio");
        browser
            .add_event_listener_with_callback("pointerdown", unlock.as_ref().unchecked_ref())
            .expect("No se pudo registrar el desbloqueo de audio");

        Self {
            state,
            _unlock: unlock,
        }
    }

    pub fn sync_music(&mut self, should_play: bool, paused: bool) {
        self.state.borrow_mut().sync_music(should_play, paused);
    }

    pub fn update(&mut self, delta_seconds: f32) {
        self.state.borrow_mut().update(delta_seconds);
    }

    pub fn play(&mut self, event: SoundEvent) {
        self.state.borrow_mut().play(event);
    }
}

fn load_asset(state: Rc<RefCell<AudioState>>, asset: AudioAsset, url: &'static str) {
    let context = state.borrow().context.clone();
    spawn_local(async move {
        match fetch_audio_buffer(&context, url).await {
            Ok(buffer) => {
                let mut state = state.borrow_mut();
                state.buffers.set(asset, buffer);
                if matches!(asset, AudioAsset::Music) {
                    state.apply_music_state();
                }
            }
            Err(error) => web_sys::console::error_2(
                &JsValue::from_str("No se pudo cargar un recurso de audio:"),
                &error,
            ),
        }
    });
}

async fn fetch_audio_buffer(context: &AudioContext, url: &str) -> Result<AudioBuffer, JsValue> {
    let response = JsFuture::from(
        web_sys::window()
            .ok_or_else(|| JsValue::from_str("No se encontró window"))?
            .fetch_with_str(url),
    )
    .await?
    .dyn_into::<Response>()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!(
            "{} respondió con HTTP {}",
            url,
            response.status()
        )));
    }
    let bytes = JsFuture::from(response.array_buffer()?)
        .await?
        .dyn_into::<ArrayBuffer>()?;
    JsFuture::from(context.decode_audio_data(&bytes)?)
        .await?
        .dyn_into::<AudioBuffer>()
}

fn effect_settings(event: SoundEvent) -> (f32, f32, f32, f32) {
    match event {
        SoundEvent::GrassLanding => (0.50, 0.035, 0.07, 0.08),
        SoundEvent::RockLanding => (0.50, 0.035, 0.07, 0.12),
        SoundEvent::LogLanding => (0.55, 0.040, 0.08, 0.20),
        SoundEvent::RailLanding => (0.50, 0.035, 0.07, 0.12),
        SoundEvent::TrainWarning => (2.00, 0.12, 0.16, 0.12),
        SoundEvent::TrainCrossing => (2.50, 0.12, 0.20, 0.12),
    }
}

fn stop_source(source: &AudioBufferSourceNode) {
    let scheduled: &AudioScheduledSourceNode = source.unchecked_ref();
    let _ = scheduled.stop();
}
