//! Sound generated in code: a small synthesiser, the game's effects and
//! its music. Pure data (samples / WAV bytes); the binary hands them to the
//! mixer.

pub mod music;
pub mod sfx;
pub mod synth;

/// Volume steps, 0 (off) ..= [`Volume::MAX`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume {
    pub music: u8,
    pub sfx: u8,
    /// Everything off, whatever the levels.
    pub muted: bool,
}

impl Volume {
    pub const MAX: u8 = 10;

    pub const DEFAULT: Volume = Volume {
        music: 6,
        sfx: 8,
        muted: false,
    };

    /// Music gain, 0..1 (curved: steps sound evenly spaced).
    pub fn music_gain(&self) -> f32 {
        if self.muted { 0.0 } else { gain(self.music) }
    }

    /// Effects gain, 0..1.
    pub fn sfx_gain(&self) -> f32 {
        if self.muted { 0.0 } else { gain(self.sfx) }
    }

    /// Moves a level one step up or down, clamped.
    pub fn nudge(level: &mut u8, up: bool) {
        *level = if up {
            (*level + 1).min(Volume::MAX)
        } else {
            level.saturating_sub(1)
        };
    }
}

impl Default for Volume {
    fn default() -> Volume {
        Volume::DEFAULT
    }
}

fn gain(level: u8) -> f32 {
    let x = level.min(Volume::MAX) as f32 / Volume::MAX as f32;
    x * x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_steps() {
        let mut v = Volume::DEFAULT;
        for _ in 0..20 {
            Volume::nudge(&mut v.music, true);
        }
        assert_eq!(v.music, Volume::MAX);
        assert_eq!(v.music_gain(), 1.0);
        for _ in 0..20 {
            Volume::nudge(&mut v.music, false);
        }
        assert_eq!(v.music_gain(), 0.0);
        assert!(gain(5) < 0.5 && gain(5) > 0.0);
        v.muted = true;
        assert_eq!(v.sfx_gain(), 0.0);
    }
}
