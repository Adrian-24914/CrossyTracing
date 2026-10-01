//! Reproduccion de audio WAV mediante la API multimedia de Windows.
//!
//! La musica se reproduce permanentemente en bucle.
//! Puede pausarse y reanudarse mediante el estado `paused`.
//! Los efectos de sonido se reproducen de manera independiente.

use crate::game::SoundEvent;

// ============================================================
// CONFIGURACION DE AUDIO
// ============================================================
//
// Cambia los valores de volumen desde este unico lugar.
//
// MCI utiliza normalmente valores de volumen entre:
//      0    = silencio
//      1000 = volumen maximo
//
// Ejemplos:
//      50   = muy bajo
//      100  = bajo
//      250  = medio-bajo
//      500  = medio
//      750  = alto
//      1000 = maximo
//
// IMPORTANTE:
// El volumen de MUSIC no se modifica actualmente mediante
// `setaudio`, porque este comando dio problemas con music.wav.
// La constante se deja aqui preparada para una futura mejora.


// --------------------------
// EFECTOS
// --------------------------

const GRASS_VOLUME: u32 = 80;
const ROCKS_VOLUME: u32 = 120;
const LOG_WATER_VOLUME: u32 = 120;
const RAIL_VOLUME: u32 = 120;
const TRAIN_WARNING_VOLUME: u32 = 120;
const TRAIN_VOLUME: u32 = 120;

// ============================================================
// CONFIGURACION DE ARCHIVOS
// ============================================================
//
// Si quieres cambiar algun WAV, puedes hacerlo desde aqui.

const AUDIO_ASSETS: [(&str, &str); 7] = [
    ("music", "assets/audio/music.wav"),
    ("grass", "assets/audio/grass.wav"),
    ("rocks", "assets/audio/rocks.wav"),
    ("log_water", "assets/audio/log_water.wav"),
    ("rail", "assets/audio/rail.wav"),
    ("train_warning", "assets/audio/train_warning.wav"),
    ("train", "assets/audio/train.wav"),
];

// ============================================================
// EFECTOS ACTIVOS
// ============================================================

struct ActiveEffect {
    alias: &'static str,
    elapsed: f32,
    duration: f32,
    fade_in: f32,
    fade_out: f32,
    volume: u32,
}

// ============================================================
// AUDIO
// ============================================================

pub struct Audio {
    music_running: bool,
    music_paused: bool,
    active_effects: Vec<ActiveEffect>,
}

impl Audio {
    pub fn new() -> Self {
        // Abrir todos los archivos WAV.
        for (alias, relative_path) in AUDIO_ASSETS {
            let path = std::env::current_dir()
                .map(|directory| directory.join(relative_path))
                .unwrap_or_else(|_| relative_path.into());

            command(&format!(
                "open \"{}\" type waveaudio alias {alias}",
                path.display()
            ));
        }

        // La musica comienza automaticamente.
        let music_running = start_music();

        Self {
            music_running,
            music_paused: false,
            active_effects: Vec::new(),
        }
    }

    // ========================================================
    // CONTROL DE MUSICA
    // ========================================================

    pub fn sync_music(&mut self, _should_play: bool, paused: bool) {
        // Si la musica no esta funcionando,
        // intentar iniciarla nuevamente.
        if !self.music_running {
            self.music_running = start_music();

            if self.music_running {
                self.music_paused = false;
            }
        }

        if !self.music_running {
            return;
        }

        // Pausar musica.
        if paused && !self.music_paused {
            if command("pause music") {
                self.music_paused = true;
            }
        }

        // Reanudar musica.
        else if !paused && self.music_paused {
            if command("resume music") {
                self.music_paused = false;
            }
        }
    }

    // ========================================================
    // UPDATE
    // ========================================================

    pub fn update(&mut self, delta_seconds: f32) {
        // ----------------------------------------------------
        // LOOP DE MUSICA
        // ----------------------------------------------------

        if self.music_running && !self.music_paused {
            match query("status music mode").as_deref() {
                Some("playing") => {}

                Some("stopped") => {
                    command("seek music to start");

                    self.music_running =
                        command("play music");
                }

                Some("paused") => {
                    self.music_paused = true;
                }

                Some(_) => {}

                None => {
                    self.music_running = false;
                }
            }
        }

        // ----------------------------------------------------
        // EFECTOS DE SONIDO
        // ----------------------------------------------------

        for effect in &mut self.active_effects {
            effect.elapsed += delta_seconds;

            let entrance =
                (effect.elapsed / effect.fade_in)
                    .clamp(0.0, 1.0);

            let exit =
                ((effect.duration - effect.elapsed)
                    / effect.fade_out)
                    .clamp(0.0, 1.0);

            let gain = entrance.min(exit);

            set_volume(
                effect.alias,
                effect.volume,
                gain,
            );
        }

        self.active_effects
            .retain(|effect| effect.elapsed < effect.duration);
    }

    // ========================================================
    // REPRODUCIR EFECTO
    // ========================================================

    pub fn play(&mut self, event: SoundEvent) {
        let (
            alias,
            duration,
            fade_in,
            fade_out,
            volume,
        ) = match event {
            SoundEvent::GrassLanding => (
                "grass",
                0.50,
                0.035,
                0.07,
                GRASS_VOLUME,
            ),

            SoundEvent::RockLanding => (
                "rocks",
                0.50,
                0.035,
                0.07,
                ROCKS_VOLUME,
            ),

            SoundEvent::LogLanding => (
                "log_water",
                0.55,
                0.040,
                0.08,
                LOG_WATER_VOLUME,
            ),

            SoundEvent::RailLanding => (
                "rail",
                0.50,
                0.035,
                0.07,
                RAIL_VOLUME,
            ),

            SoundEvent::TrainWarning => (
                "train_warning",
                2.00,
                0.12,
                0.16,
                TRAIN_WARNING_VOLUME,
            ),

            SoundEvent::TrainCrossing => (
                "train",
                2.50,
                0.12,
                0.20,
                TRAIN_VOLUME,
            ),
        };

        // Detener efecto anterior.
        command(&format!("stop {alias}"));

        // Regresar al inicio.
        command(&format!(
            "seek {alias} to start"
        ));

        // Comenzar desde volumen 0.
        set_volume(
            alias,
            volume,
            0.0,
        );

        // Reproducir.
        command(&format!(
            "play {alias}"
        ));

        // Evitar duplicados del mismo efecto.
        self.active_effects
            .retain(|effect| effect.alias != alias);

        self.active_effects.push(
            ActiveEffect {
                alias,
                elapsed: 0.0,
                duration,
                fade_in,
                fade_out,
                volume,
            }
        );
    }
}

// ============================================================
// CERRAR AUDIO
// ============================================================

impl Drop for Audio {
    fn drop(&mut self) {
        for (alias, _) in AUDIO_ASSETS {
            command(&format!(
                "close {alias}"
            ));
        }
    }
}

// ============================================================
// MUSICA
// ============================================================

fn start_music() -> bool {
    command("seek music to start");

    command("play music")
}

// ============================================================
// COMANDO MCI
// ============================================================

fn command(value: &str) -> bool {
    let wide: Vec<u16> =
        value
            .encode_utf16()
            .chain(Some(0))
            .collect();

    let result = unsafe {
        mci_send_string_w(
            wide.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        )
    };

    result == 0
}

// ============================================================
// CONSULTA MCI
// ============================================================

fn query(value: &str) -> Option<String> {
    let wide: Vec<u16> =
        value
            .encode_utf16()
            .chain(Some(0))
            .collect();

    let mut buffer = [0u16; 256];

    let result = unsafe {
        mci_send_string_w(
            wide.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            std::ptr::null_mut(),
        )
    };

    if result != 0 {
        return None;
    }

    let length = buffer
        .iter()
        .position(|&character| character == 0)
        .unwrap_or(buffer.len());

    let result =
        String::from_utf16_lossy(
            &buffer[..length]
        );

    Some(
        result
            .trim()
            .to_lowercase()
    )
}

// ============================================================
// VOLUMEN
// ============================================================

fn set_volume(
    alias: &str,
    maximum: u32,
    gain: f32,
) {
    let volume =
        (maximum as f32 * gain)
            .round() as u32;

    command(&format!(
        "setaudio {alias} volume to {volume}"
    ));
}

// ============================================================
// WINDOWS MCI
// ============================================================

#[link(name = "winmm")]
unsafe extern "system" {
    #[link_name = "mciSendStringW"]
    fn mci_send_string_w(
        command: *const u16,
        return_buffer: *mut u16,
        return_length: u32,
        callback: *mut core::ffi::c_void,
    ) -> u32;
}