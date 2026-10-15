//! A tiny chiptune synthesiser: square, triangle and noise voices with
//! pitch slides, vibrato and ADSR envelopes, mixed to mono samples and
//! wrapped in a 16-bit PCM WAV file the audio backend can load.
//!
//! Everything is deterministic (noise comes from a fixed LFSR), so a sound
//! renders to the same bytes every time.

/// Output rate; the playback mixer runs at 44.1 kHz, so no resampling.
pub const SAMPLE_RATE: u32 = 44_100;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Wave {
    /// Pulse wave; `duty` is the high share of each period (0.5 = square).
    Square {
        duty: f32,
    },
    Triangle,
    /// NES-style noise: a new random level every 1/freq seconds, so the
    /// "pitch" sets how hissy (high) or rumbly (low) it sounds.
    Noise,
}

/// Attack/decay/sustain/release envelope, times in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Envelope {
    pub attack: f32,
    pub decay: f32,
    /// Level held after the decay (0..1).
    pub sustain: f32,
    pub release: f32,
}

impl Envelope {
    /// Instant attack, then a fall to silence over the whole note.
    pub const PLUCK: Envelope = Envelope {
        attack: 0.002,
        decay: 0.0,
        sustain: 1.0,
        release: 1.0,
    };

    /// Organ-like: on at full level, short fade at the end.
    pub const FLAT: Envelope = Envelope {
        attack: 0.003,
        decay: 0.0,
        sustain: 1.0,
        release: 0.02,
    };

    /// Level at time `t` into a note lasting `len` seconds. The release
    /// starts so that the note is silent exactly at `len`; a release of
    /// 1.0 or more means "fade over everything after the attack/decay".
    pub fn level(&self, t: f32, len: f32) -> f32 {
        if t < 0.0 || t >= len {
            return 0.0;
        }
        let release = if self.release >= 1.0 {
            (len - self.attack - self.decay).max(0.001)
        } else {
            self.release.min(len)
        };
        let held = if t < self.attack {
            t / self.attack
        } else if t < self.attack + self.decay {
            let k = (t - self.attack) / self.decay;
            1.0 + (self.sustain - 1.0) * k
        } else {
            self.sustain
        };
        let left = len - t;
        if left < release {
            held * left / release
        } else {
            held
        }
    }
}

/// One note of a sound effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    pub wave: Wave,
    /// Start time within the effect, seconds.
    pub at: f32,
    pub len: f32,
    /// Pitch at the start and end of the note, Hz (exponential slide).
    pub from: f32,
    pub to: f32,
    pub env: Envelope,
    pub volume: f32,
    /// Vibrato (rate Hz, depth as a pitch ratio, e.g. 0.05 = ±5 %).
    pub vibrato: (f32, f32),
}

impl Tone {
    pub fn new(wave: Wave, at: f32, len: f32, from: f32, to: f32) -> Tone {
        Tone {
            wave,
            at,
            len,
            from,
            to,
            env: Envelope::PLUCK,
            volume: 0.5,
            vibrato: (0.0, 0.0),
        }
    }

    pub fn env(self, env: Envelope) -> Tone {
        Tone { env, ..self }
    }

    pub fn volume(self, volume: f32) -> Tone {
        Tone { volume, ..self }
    }

    pub fn vibrato(self, rate: f32, depth: f32) -> Tone {
        Tone {
            vibrato: (rate, depth),
            ..self
        }
    }

    fn end(&self) -> f32 {
        self.at + self.len
    }

    /// Adds this note into `out` (which already spans the whole effect).
    fn render_into(&self, out: &mut [f32]) {
        let rate = SAMPLE_RATE as f32;
        let start = (self.at * rate) as usize;
        let count = (self.len * rate) as usize;
        let mut phase = 0.0f32;
        let mut lfsr: u16 = 0xACE1;
        let mut noise = 1.0f32;
        let ratio = self.to / self.from;
        for i in 0..count {
            let Some(slot) = out.get_mut(start + i) else {
                break;
            };
            let t = i as f32 / rate;
            let k = t / self.len;
            let (vib_rate, vib_depth) = self.vibrato;
            let mut freq = if ratio == 1.0 {
                self.from
            } else {
                self.from * ratio.powf(k)
            };
            if vib_depth != 0.0 {
                freq *= 1.0 + vib_depth * (std::f32::consts::TAU * vib_rate * t).sin();
            }
            let prev = phase;
            phase = (phase + freq / rate).fract();
            let sample = match self.wave {
                Wave::Square { duty } => {
                    if phase < duty {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Noise => {
                    if phase < prev {
                        // Galois LFSR, period 65535.
                        let bit = lfsr & 1;
                        lfsr >>= 1;
                        if bit == 1 {
                            lfsr ^= 0xB400;
                        }
                        noise = if lfsr & 1 == 1 { 1.0 } else { -1.0 };
                    }
                    noise
                }
            };
            *slot += sample * self.env.level(t, self.len) * self.volume;
        }
    }
}

/// Mixes notes into mono samples in -1..1 (hard-clipped).
pub fn render(tones: &[Tone]) -> Vec<f32> {
    let len = tones.iter().map(Tone::end).fold(0.0, f32::max);
    let mut out = vec![0.0; (len * SAMPLE_RATE as f32).ceil() as usize];
    for tone in tones {
        tone.render_into(&mut out);
    }
    for s in &mut out {
        *s = s.clamp(-1.0, 1.0);
    }
    out
}

/// Wraps mono samples in a 16-bit PCM WAV file.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s * i16::MAX as f32) as i16).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: Wave = Wave::Square { duty: 0.5 };

    #[test]
    fn length_covers_the_last_note() {
        let s = render(&[
            Tone::new(SQUARE, 0.0, 0.1, 440.0, 440.0),
            Tone::new(Wave::Triangle, 0.05, 0.1, 440.0, 440.0),
        ]);
        assert_eq!(s.len(), (0.15 * SAMPLE_RATE as f32).ceil() as usize);
    }

    #[test]
    fn square_wave_has_the_right_pitch() {
        let s = render(&[Tone::new(SQUARE, 0.0, 1.0, 441.0, 441.0).env(Envelope::FLAT)]);
        // Count rising zero crossings over the first half second.
        let half = &s[..SAMPLE_RATE as usize / 2];
        let rises = half.windows(2).filter(|w| w[0] < 0.0 && w[1] > 0.0).count();
        assert!((219..=221).contains(&rises), "{rises}");
    }

    #[test]
    fn slides_reach_the_end_pitch() {
        let s = render(&[Tone::new(SQUARE, 0.0, 1.0, 200.0, 800.0).env(Envelope::FLAT)]);
        let rises = |a: &[f32]| a.windows(2).filter(|w| w[0] < 0.0 && w[1] > 0.0).count();
        let tenth = SAMPLE_RATE as usize / 10;
        let first = rises(&s[..tenth]);
        let last = rises(&s[s.len() - tenth..]);
        assert!(first < 25 && last > 70, "{first} {last}");
    }

    #[test]
    fn envelope_shapes() {
        let e = Envelope::PLUCK;
        assert!(e.level(0.01, 0.5) > 0.95);
        assert!((e.level(0.25, 0.5) - 0.5).abs() < 0.02);
        assert_eq!(e.level(0.5, 0.5), 0.0);
        let f = Envelope::FLAT;
        assert_eq!(f.level(0.25, 0.5), 1.0);
        assert!(f.level(0.49, 0.5) < 0.6);
        let adsr = Envelope {
            attack: 0.1,
            decay: 0.1,
            sustain: 0.5,
            release: 0.1,
        };
        assert!((adsr.level(0.05, 1.0) - 0.5).abs() < 1e-5);
        assert!((adsr.level(0.5, 1.0) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn noise_is_deterministic_and_noisy() {
        let tone = Tone::new(Wave::Noise, 0.0, 0.2, 8000.0, 8000.0);
        let a = render(&[tone]);
        assert_eq!(a, render(&[tone]));
        let flips = a
            .windows(2)
            .filter(|w| w[0].signum() != w[1].signum())
            .count();
        assert!(flips > 200, "{flips}");
    }

    #[test]
    fn mixing_clips() {
        let loud = Tone::new(SQUARE, 0.0, 0.1, 100.0, 100.0)
            .env(Envelope::FLAT)
            .volume(1.0);
        let s = render(&[loud, loud, loud]);
        assert!(s.iter().all(|v| v.abs() <= 1.0));
    }

    #[test]
    fn wav_header() {
        let w = wav_bytes(&[0.0, 1.0, -1.0]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(w[4..8].try_into().unwrap()), 36 + 6);
        assert_eq!(
            u32::from_le_bytes(w[24..28].try_into().unwrap()),
            SAMPLE_RATE
        );
        assert_eq!(&w[36..40], b"data");
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(i16::from_le_bytes([w[46], w[47]]), i16::MAX);
    }
}
