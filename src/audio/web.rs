use crate::game::SoundEvent;

pub struct Audio;

impl Audio {
    pub fn new() -> Self {
        Self
    }

    pub fn sync_music(&mut self, _should_play: bool, _paused: bool) {}

    pub fn update(&mut self, _delta_seconds: f32) {}

    pub fn play(&mut self, _event: SoundEvent) {}
}
