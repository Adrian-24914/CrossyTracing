//! Reproduccion de audio WAV mediante la API multimedia que ya incluye Windows.
//!
//! Los alias independientes permiten que la musica y los efectos se
//! reproduzcan al mismo tiempo, sin incorporar una dependencia al raytracer.

use crate::game::SoundEvent;

const AUDIO_ASSETS: [(&str, &str); 7] = [
    ("music", "assets/audio/music.wav"),
    ("grass", "assets/audio/grass.wav"),
    ("rocks", "assets/audio/rocks.wav"),
    ("log_water", "assets/audio/log_water.wav"),
    ("rail", "assets/audio/rail.wav"),
    ("train_warning", "assets/audio/train_warning.wav"),
    ("train", "assets/audio/train.wav"),
];

pub struct Audio {
    music_running: bool,
    music_paused: bool,
}

impl Audio {
    pub fn new() -> Self {
        for (alias, path) in AUDIO_ASSETS {
            command(&format!("open \"{path}\" type waveaudio alias {alias}"));
        }
        Self {
            music_running: false,
            music_paused: false,
        }
    }

    pub fn sync_music(&mut self, should_play: bool, paused: bool) {
        if !should_play {
            if self.music_running {
                command("stop music");
                command("seek music to start");
            }
            self.music_running = false;
            self.music_paused = false;
            return;
        }

        if !self.music_running {
            command("play music repeat");
            self.music_running = true;
            self.music_paused = false;
        }
        if paused && !self.music_paused {
            command("pause music");
            self.music_paused = true;
        } else if !paused && self.music_paused {
            command("resume music");
            self.music_paused = false;
        }
    }

    pub fn play(&self, event: SoundEvent) {
        let alias = match event {
            SoundEvent::GrassLanding => "grass",
            SoundEvent::RockLanding => "rocks",
            SoundEvent::LogLanding => "log_water",
            SoundEvent::RailLanding => "rail",
            SoundEvent::TrainWarning => "train_warning",
            SoundEvent::TrainCrossing => "train",
        };
        command(&format!("stop {alias}"));
        command(&format!("seek {alias} to start"));
        command(&format!("play {alias}"));
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        for (alias, _) in AUDIO_ASSETS {
            command(&format!("close {alias}"));
        }
    }
}

fn command(value: &str) {
    let wide: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
    unsafe {
        mci_send_string_w(wide.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut());
    }
}

#[link(name = "winmm")]
unsafe extern "system" {
    fn mci_send_string_w(
        command: *const u16,
        return_buffer: *mut u16,
        return_length: u32,
        callback: *mut core::ffi::c_void,
    ) -> u32;
}
