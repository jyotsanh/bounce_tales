//! Plays the synthesised effects for each sim step's events, and the music
//! the game state calls for.

use std::collections::HashMap;

use bounce_tales::audio::sfx::{self, Sfx};
use bounce_tales::audio::{Volume, music};
use bounce_tales::sim::Sim;
use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
};

/// Effects at full volume, before the player's level.
const SFX_MASTER: f32 = 0.6;
/// Music at full volume.
const MUSIC_MASTER: f32 = 0.5;

pub struct Sounds {
    /// One sound per [`Sfx::ALL`] entry; `None` if it failed to load. Empty
    /// for a silent (scripted) run.
    table: Vec<Option<Sound>>,
    /// Tracks rendered so far (on first use: ~0.1 s each).
    tracks: HashMap<String, Option<Sound>>,
    /// The track playing, if any.
    playing: Option<String>,
    pub volume: Volume,
}

impl Sounds {
    /// Renders and loads every effect (a few ms of synthesis).
    pub async fn load(volume: Volume) -> Sounds {
        let mut table = Vec::with_capacity(Sfx::ALL.len());
        for s in Sfx::ALL {
            table.push(load_sound_from_bytes(&s.wav()).await.ok());
        }
        Sounds {
            table,
            tracks: HashMap::new(),
            playing: None,
            volume,
        }
    }

    /// No sounds at all (scripted runs never open the audio device).
    pub fn silent() -> Sounds {
        Sounds {
            table: Vec::new(),
            tracks: HashMap::new(),
            playing: None,
            volume: Volume {
                muted: true,
                ..Volume::DEFAULT
            },
        }
    }

    fn silent_run(&self) -> bool {
        self.table.is_empty()
    }

    pub fn play(&self, sfx: Sfx, volume: f32) {
        let gain = self.volume.sfx_gain();
        if gain == 0.0 {
            return;
        }
        if let Some(Some(sound)) = self.table.get(sfx.index()) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: volume * gain * SFX_MASTER,
                },
            );
        }
    }

    /// Call after every sim step.
    pub fn on_step(&self, sim: &Sim) {
        for (s, volume) in sfx::for_step(&sim.events, sim.ball.form) {
            self.play(s, volume);
        }
    }

    /// Starts, stops or switches the music to `want` (a track name). Call
    /// once per frame.
    pub async fn update_music(&mut self, want: Option<&str>) {
        if self.silent_run() {
            return;
        }
        if want == self.playing.as_deref() {
            return;
        }
        if let Some(Some(old)) = self.playing.take().and_then(|n| self.tracks.get(&n)) {
            stop_sound(old);
        }
        let Some(name) = want else {
            return;
        };
        if !self.tracks.contains_key(name) {
            let sound = match music::track(name).map(|t| t.wav()) {
                Some(Ok(wav)) => load_sound_from_bytes(&wav).await.ok(),
                _ => None,
            };
            self.tracks.insert(name.to_owned(), sound);
        }
        if let Some(Some(sound)) = self.tracks.get(name) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: true,
                    volume: self.volume.music_gain() * MUSIC_MASTER,
                },
            );
        }
        self.playing = Some(name.to_owned());
    }

    /// Applies a volume change to the music already playing.
    pub fn volume_changed(&self) {
        if let Some(Some(sound)) = self.playing.as_ref().and_then(|n| self.tracks.get(n)) {
            set_sound_volume(sound, self.volume.music_gain() * MUSIC_MASTER);
        }
    }

    /// Volume keys: M mutes, -/= music, [/] effects. Returns a short
    /// message to show when something changed.
    pub fn handle_keys(&mut self, pressed: impl Fn(VolumeKey) -> bool) -> Option<String> {
        if self.silent_run() {
            return None;
        }
        let v = &mut self.volume;
        let msg = if pressed(VolumeKey::Mute) {
            v.muted = !v.muted;
            if v.muted { "SOUND OFF" } else { "SOUND ON" }.to_owned()
        } else if pressed(VolumeKey::MusicDown) || pressed(VolumeKey::MusicUp) {
            Volume::nudge(&mut v.music, pressed(VolumeKey::MusicUp));
            v.muted = false;
            format!("MUSIC {}/{}", v.music, Volume::MAX)
        } else if pressed(VolumeKey::SfxDown) || pressed(VolumeKey::SfxUp) {
            Volume::nudge(&mut v.sfx, pressed(VolumeKey::SfxUp));
            v.muted = false;
            let msg = format!("SOUND {}/{}", v.sfx, Volume::MAX);
            self.play(Sfx::Gem, 1.0); // a sample at the new level
            msg
        } else {
            return None;
        };
        self.volume_changed();
        Some(msg)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeKey {
    Mute,
    MusicDown,
    MusicUp,
    SfxDown,
    SfxUp,
}
