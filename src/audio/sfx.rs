//! The game's sound effects, each built from a few synth notes, and which
//! sim event plays which effect.

use super::synth::{Envelope, Tone, Wave, render, wav_bytes};
use crate::sim::ball::Form;
use crate::sim::entities::{Event, Item};

const SQUARE: Wave = Wave::Square { duty: 0.5 };
const PULSE: Wave = Wave::Square { duty: 0.25 };
const THIN: Wave = Wave::Square { duty: 0.125 };
const TRI: Wave = Wave::Triangle;
const NOISE: Wave = Wave::Noise;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sfx {
    Bounce,
    Jump,
    Gem,
    Egg,
    Life,
    Splash,
    Death,
    Pop,
    Respawn,
    Checkpoint,
    TransformStone,
    TransformBalloon,
    TransformNormal,
    Crumble,
    Smash,
    Switch,
    Gate,
    Spring,
    Shot,
    Tink,
    Freed,
    BossHit,
    BossDefeated,
    Exit,
    GameOver,
    /// A hypno machine breaking.
    MachineBreak,
    /// Menu cursor moved.
    MenuMove,
    /// Menu item chosen.
    MenuSelect,
    /// Back out / pause.
    MenuBack,
}

impl Sfx {
    pub const ALL: [Sfx; 29] = [
        Sfx::Bounce,
        Sfx::Jump,
        Sfx::Gem,
        Sfx::Egg,
        Sfx::Life,
        Sfx::Splash,
        Sfx::Death,
        Sfx::Pop,
        Sfx::Respawn,
        Sfx::Checkpoint,
        Sfx::TransformStone,
        Sfx::TransformBalloon,
        Sfx::TransformNormal,
        Sfx::Crumble,
        Sfx::Smash,
        Sfx::Switch,
        Sfx::Gate,
        Sfx::Spring,
        Sfx::Shot,
        Sfx::Tink,
        Sfx::Freed,
        Sfx::BossHit,
        Sfx::BossDefeated,
        Sfx::Exit,
        Sfx::GameOver,
        Sfx::MachineBreak,
        Sfx::MenuMove,
        Sfx::MenuSelect,
        Sfx::MenuBack,
    ];

    /// Index into [`Sfx::ALL`] (for a sound table).
    pub fn index(self) -> usize {
        Sfx::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    /// The notes making up this effect.
    pub fn tones(self) -> Vec<Tone> {
        let t = Tone::new;
        match self {
            // Soft rubbery thud.
            Sfx::Bounce => vec![
                t(TRI, 0.0, 0.09, 260.0, 110.0).volume(0.7),
                t(NOISE, 0.0, 0.03, 2000.0, 800.0).volume(0.12),
            ],
            // Quick upward "bwip".
            Sfx::Jump => vec![t(PULSE, 0.0, 0.12, 330.0, 700.0).volume(0.25)],
            Sfx::Gem => vec![
                t(PULSE, 0.0, 0.05, note(76), note(76)).volume(0.22),
                t(PULSE, 0.05, 0.12, note(83), note(83)).volume(0.22),
            ],
            Sfx::Egg => arpeggio(PULSE, &[72, 76, 79, 84, 88], 0.06, 0.25, 0.25),
            Sfx::Life => {
                let mut v = arpeggio(SQUARE, &[67, 72, 76, 79, 84], 0.07, 0.2, 0.2);
                v.push(t(TRI, 0.35, 0.3, note(91), note(91)).volume(0.4));
                v
            }
            Sfx::Splash => vec![
                t(NOISE, 0.0, 0.3, 6000.0, 1500.0).volume(0.35),
                t(TRI, 0.0, 0.1, 400.0, 150.0).volume(0.3),
            ],
            // Burst then a falling whine.
            Sfx::Death => vec![
                t(NOISE, 0.0, 0.15, 4000.0, 1000.0).volume(0.4),
                t(SQUARE, 0.05, 0.45, 700.0, 90.0).volume(0.22),
            ],
            // The balloon bursting: one sharp crack.
            Sfx::Pop => vec![
                t(NOISE, 0.0, 0.12, 9000.0, 3000.0).volume(0.55),
                t(TRI, 0.0, 0.05, 900.0, 200.0).volume(0.4),
            ],
            Sfx::Respawn => vec![
                t(TRI, 0.0, 0.25, 300.0, 900.0)
                    .volume(0.4)
                    .vibrato(18.0, 0.04),
            ],
            Sfx::Checkpoint => vec![
                t(TRI, 0.0, 0.18, note(79), note(79)).volume(0.45),
                t(TRI, 0.12, 0.35, note(84), note(84)).volume(0.45),
            ],
            // Heavy, falling and warbly.
            Sfx::TransformStone => vec![
                t(SQUARE, 0.0, 0.4, 500.0, 90.0)
                    .volume(0.25)
                    .vibrato(22.0, 0.08),
                t(NOISE, 0.25, 0.15, 600.0, 300.0).volume(0.25),
            ],
            // Light, rising and wobbly.
            Sfx::TransformBalloon => vec![
                t(THIN, 0.0, 0.4, 250.0, 1200.0)
                    .volume(0.22)
                    .vibrato(16.0, 0.07),
            ],
            Sfx::TransformNormal => vec![
                t(PULSE, 0.0, 0.25, 900.0, 400.0)
                    .volume(0.22)
                    .vibrato(20.0, 0.06),
            ],
            Sfx::Crumble => vec![t(NOISE, 0.0, 0.25, 900.0, 300.0).volume(0.35)],
            Sfx::Smash => vec![
                t(NOISE, 0.0, 0.3, 1800.0, 400.0).volume(0.55),
                t(SQUARE, 0.0, 0.12, 120.0, 60.0).volume(0.3),
            ],
            Sfx::Switch => vec![
                t(SQUARE, 0.0, 0.03, 1200.0, 1200.0).volume(0.2),
                t(SQUARE, 0.05, 0.06, 1800.0, 1800.0).volume(0.2),
            ],
            Sfx::Gate => vec![
                t(SQUARE, 0.0, 0.35, 70.0, 110.0)
                    .env(Envelope::FLAT)
                    .volume(0.18),
                t(NOISE, 0.0, 0.35, 400.0, 400.0).volume(0.12),
            ],
            // "Boing".
            Sfx::Spring => vec![
                t(TRI, 0.0, 0.3, 140.0, 620.0)
                    .volume(0.6)
                    .vibrato(25.0, 0.1),
            ],
            Sfx::Shot => vec![
                t(NOISE, 0.0, 0.05, 7000.0, 7000.0).volume(0.2),
                t(PULSE, 0.0, 0.08, 1000.0, 400.0).volume(0.15),
            ],
            Sfx::Tink => vec![t(TRI, 0.0, 0.08, 2400.0, 2200.0).volume(0.4)],
            Sfx::Freed => vec![
                t(PULSE, 0.0, 0.07, note(72), note(72)).volume(0.2),
                t(PULSE, 0.07, 0.07, note(79), note(79)).volume(0.2),
                t(PULSE, 0.14, 0.15, note(84), note(84)).volume(0.2),
            ],
            Sfx::BossHit => vec![
                t(NOISE, 0.0, 0.25, 2500.0, 500.0).volume(0.45),
                t(SQUARE, 0.0, 0.35, 260.0, 55.0).volume(0.3),
            ],
            Sfx::BossDefeated => {
                let mut v = vec![
                    t(NOISE, 0.0, 1.2, 3000.0, 200.0).volume(0.5),
                    t(SQUARE, 0.0, 0.8, 400.0, 40.0).volume(0.25),
                ];
                v.extend(
                    arpeggio(PULSE, &[72, 76, 79, 84, 88, 91], 0.09, 0.3, 0.2)
                        .into_iter()
                        .map(|n| Tone {
                            at: n.at + 0.9,
                            ..n
                        }),
                );
                v
            }
            // Little fanfare.
            Sfx::Exit => {
                let mut v = arpeggio(SQUARE, &[67, 72, 76], 0.1, 0.1, 0.2);
                v.push(
                    t(SQUARE, 0.3, 0.45, note(79), note(79))
                        .env(Envelope {
                            attack: 0.005,
                            decay: 0.1,
                            sustain: 0.6,
                            release: 0.15,
                        })
                        .volume(0.2),
                );
                v.push(t(TRI, 0.3, 0.45, note(55), note(55)).volume(0.45));
                v
            }
            Sfx::MachineBreak => {
                let mut v = vec![
                    t(NOISE, 0.0, 0.25, 2200.0, 600.0).volume(0.45),
                    t(SQUARE, 0.0, 0.3, 500.0, 120.0)
                        .volume(0.2)
                        .vibrato(30.0, 0.1),
                ];
                v.extend(
                    arpeggio(PULSE, &[84, 88, 91], 0.05, 0.12, 0.18)
                        .into_iter()
                        .map(|n| Tone {
                            at: n.at + 0.2,
                            ..n
                        }),
                );
                v
            }
            Sfx::MenuMove => vec![t(PULSE, 0.0, 0.04, note(84), note(84)).volume(0.18)],
            Sfx::MenuSelect => vec![
                t(PULSE, 0.0, 0.05, note(79), note(79)).volume(0.2),
                t(PULSE, 0.05, 0.1, note(91), note(91)).volume(0.2),
            ],
            Sfx::MenuBack => vec![t(PULSE, 0.0, 0.08, note(79), note(72)).volume(0.18)],
            // Slow sad descent.
            Sfx::GameOver => {
                let mut v = Vec::new();
                for (i, n) in [67, 66, 65, 64].into_iter().enumerate() {
                    let len = if i == 3 { 0.7 } else { 0.25 };
                    v.push(
                        t(SQUARE, i as f32 * 0.28, len, note(n), note(n))
                            .env(Envelope::FLAT)
                            .volume(0.18)
                            .vibrato(if i == 3 { 6.0 } else { 0.0 }, 0.01),
                    );
                    v.push(t(TRI, i as f32 * 0.28, len, note(n - 12), note(n - 12)).volume(0.35));
                }
                v
            }
        }
    }

    /// Mono samples of this effect.
    pub fn samples(self) -> Vec<f32> {
        render(&self.tones())
    }

    /// The effect as a WAV file.
    pub fn wav(self) -> Vec<u8> {
        wav_bytes(&self.samples())
    }
}

/// Frequency of MIDI note `n` (69 = A4 = 440 Hz).
pub fn note(n: i32) -> f32 {
    440.0 * 2f32.powf((n - 69) as f32 / 12.0)
}

/// Notes one after another every `step` s, each ringing `len` s.
fn arpeggio(wave: Wave, notes: &[i32], step: f32, len: f32, volume: f32) -> Vec<Tone> {
    notes
        .iter()
        .enumerate()
        .map(|(i, &n)| Tone::new(wave, i as f32 * step, len, note(n), note(n)).volume(volume))
        .collect()
}

/// Landing speed (px/s) that plays the bounce at full volume.
const LOUD_LANDING: f32 = 500.0;

/// The effect an event plays and its volume (0..1). `form` is the ball's
/// form at the time (the balloon pops instead of bursting).
pub fn for_event(event: &Event, form: Form) -> Option<(Sfx, f32)> {
    let sfx = match *event {
        Event::Landed { speed, .. } => {
            return Some((Sfx::Bounce, (speed / LOUD_LANDING).clamp(0.25, 1.0)));
        }
        Event::Splash { speed, .. } => {
            return Some((Sfx::Splash, (speed / 300.0).clamp(0.3, 1.0)));
        }
        Event::Jumped { .. } => Sfx::Jump,
        Event::Collected { item, .. } => match item {
            Item::Gem => Sfx::Gem,
            Item::Egg => Sfx::Egg,
            Item::Life => Sfx::Life,
        },
        Event::Died { .. } if form == Form::Balloon => Sfx::Pop,
        Event::Died { .. } => Sfx::Death,
        Event::Respawned { .. } | Event::LevelRestarted => Sfx::Respawn,
        Event::CheckpointReached { .. } => Sfx::Checkpoint,
        Event::Transformed { form, .. } => match form {
            Form::Normal => Sfx::TransformNormal,
            Form::Stone => Sfx::TransformStone,
            Form::Balloon => Sfx::TransformBalloon,
        },
        Event::Crumbled { .. } => Sfx::Crumble,
        Event::Smashed { .. } => Sfx::Smash,
        Event::Switched { .. } => Sfx::Switch,
        Event::GateMoved { .. } => Sfx::Gate,
        Event::Launched { .. } => Sfx::Spring,
        Event::Shot { .. } => Sfx::Shot,
        Event::ShotBlocked { .. } => Sfx::Tink,
        Event::EnemyFreed { .. } => Sfx::Freed,
        Event::MachineBroken { .. } => Sfx::MachineBreak,
        Event::BossHit { .. } => Sfx::BossHit,
        Event::BossDefeated { .. } => Sfx::BossDefeated,
        Event::ExitReached => Sfx::Exit,
        Event::GameOver => Sfx::GameOver,
    };
    Some((sfx, 1.0))
}

/// The effects to play for one step's events: one per kind (the loudest),
/// so a burst of identical events doesn't stack into a blast.
pub fn for_step(events: &[Event], form: Form) -> Vec<(Sfx, f32)> {
    let mut out: Vec<(Sfx, f32)> = Vec::new();
    for e in events {
        let Some((sfx, vol)) = for_event(e, form) else {
            continue;
        };
        match out.iter_mut().find(|(s, _)| *s == sfx) {
            Some(slot) => slot.1 = slot.1.max(vol),
            None => out.push((sfx, vol)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::hazards::Cause;
    use macroquad::math::vec2;

    #[test]
    fn every_effect_renders_short_audible_and_unclipped_enough() {
        for sfx in Sfx::ALL {
            let s = sfx.samples();
            let secs = s.len() as f32 / super::super::synth::SAMPLE_RATE as f32;
            assert!(secs > 0.02 && secs < 2.5, "{sfx:?} {secs}");
            let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.1, "{sfx:?} too quiet: {peak}");
            // Ends in silence (no click when the sound stops).
            assert!(s.last().unwrap().abs() < 0.02, "{sfx:?}");
        }
    }

    #[test]
    fn table_indices_are_unique() {
        for (i, sfx) in Sfx::ALL.iter().enumerate() {
            assert_eq!(sfx.index(), i);
        }
    }

    #[test]
    fn notes() {
        assert_eq!(note(69), 440.0);
        assert!((note(81) - 880.0).abs() < 0.01);
        assert!((note(60) - 261.63).abs() < 0.01);
    }

    #[test]
    fn events_map_to_effects() {
        let p = vec2(0.0, 0.0);
        let died = Event::Died {
            pos: p,
            cause: Cause::Spikes,
        };
        assert_eq!(for_event(&died, Form::Normal), Some((Sfx::Death, 1.0)));
        assert_eq!(for_event(&died, Form::Balloon), Some((Sfx::Pop, 1.0)));
        let soft = for_event(
            &Event::Landed {
                pos: p,
                speed: 100.0,
            },
            Form::Normal,
        );
        let hard = for_event(
            &Event::Landed {
                pos: p,
                speed: 900.0,
            },
            Form::Normal,
        );
        assert_eq!(hard, Some((Sfx::Bounce, 1.0)));
        assert!(soft.unwrap().1 < 0.5);
        let gem = Event::Collected {
            item: Item::Egg,
            pos: p,
        };
        assert_eq!(for_event(&gem, Form::Normal).unwrap().0, Sfx::Egg);
    }

    #[test]
    fn a_step_plays_each_effect_once() {
        let p = vec2(0.0, 0.0);
        let gem = Event::Collected {
            item: Item::Gem,
            pos: p,
        };
        let events = [
            gem,
            gem,
            Event::Landed {
                pos: p,
                speed: 100.0,
            },
            Event::Landed {
                pos: p,
                speed: 400.0,
            },
        ];
        let out = for_step(&events, Form::Normal);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], (Sfx::Gem, 1.0));
        assert_eq!(out[1], (Sfx::Bounce, 0.8));
    }
}
