//! Plays the synthesised effects for each sim step's events.

use bounce_tales::audio::sfx::{self, Sfx};
use bounce_tales::sim::Sim;
use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};

/// Overall effects volume.
const MASTER: f32 = 0.6;

pub struct Sounds {
    /// One sound per [`Sfx::ALL`] entry; `None` if it failed to load.
    table: Vec<Option<Sound>>,
    pub muted: bool,
}

impl Sounds {
    /// Renders and loads every effect (a few ms of synthesis).
    pub async fn load(muted: bool) -> Sounds {
        let mut table = Vec::with_capacity(Sfx::ALL.len());
        for s in Sfx::ALL {
            table.push(load_sound_from_bytes(&s.wav()).await.ok());
        }
        Sounds { table, muted }
    }

    /// No sounds at all (scripted runs never open the audio device).
    pub fn silent() -> Sounds {
        Sounds {
            table: Vec::new(),
            muted: true,
        }
    }

    pub fn play(&self, sfx: Sfx, volume: f32) {
        if self.muted {
            return;
        }
        if let Some(Some(sound)) = self.table.get(sfx.index()) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: volume * MASTER,
                },
            );
        }
    }

    /// Call after every sim step.
    pub fn on_step(&self, sim: &Sim) {
        if self.muted {
            return;
        }
        for (s, volume) in sfx::for_step(&sim.events, sim.ball.form) {
            self.play(s, volume);
        }
    }
}
