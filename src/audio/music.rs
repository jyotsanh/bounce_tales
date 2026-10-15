//! Chiptune music: a tiny pattern sequencer and the game's tunes.
//!
//! A track is a few parts (lead, harmony, bass, drums), each written as a
//! string of steps separated by spaces (`|` bar lines are ignored):
//! - a note such as `C4`, `F#5` or `Bb3` starts a note,
//! - `-` holds the previous note one more step,
//! - `.` is a rest.
//!
//! Drum parts use `k` kick, `s` snare, `h` closed hat, `o` open hat.
//! A part shorter than the track repeats; every part's length must divide
//! the track's length. The whole loop renders to one WAV that is played
//! looped, so every note ends inside the loop (no clicks at the seam).

use super::sfx::note;
use super::synth::{Envelope, SAMPLE_RATE, Tone, Wave, render, wav_bytes};
use crate::game::{Game, Mode};

/// Track names, as used by the level property `music`.
pub const TRACKS: &[&str] = &["title", "forest", "cave", "castle", "boss"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice {
    /// Melody: 25 % pulse, a touch of vibrato on long notes.
    Lead,
    /// Arpeggios / echoes: thin 12.5 % pulse, quiet.
    Harmony,
    /// Triangle bass.
    Bass,
    /// Noise and triangle percussion.
    Drums,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub voice: Voice,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub name: &'static str,
    pub bpm: f32,
    /// Steps per beat (2 = eighth notes).
    pub steps_per_beat: u32,
    pub parts: Vec<Part>,
}

impl Track {
    /// Seconds per step.
    pub fn step_secs(&self) -> f32 {
        60.0 / self.bpm / self.steps_per_beat as f32
    }

    /// Steps in one loop (the longest part).
    pub fn len_steps(&self) -> usize {
        self.parts.iter().map(|p| p.steps.len()).max().unwrap_or(0)
    }

    pub fn len_secs(&self) -> f32 {
        self.len_steps() as f32 * self.step_secs()
    }

    /// Every note of the loop. Errors name the bad token.
    pub fn tones(&self) -> Result<Vec<Tone>, String> {
        let total = self.len_steps();
        let dt = self.step_secs();
        let mut out = Vec::new();
        for part in &self.parts {
            let n = part.steps.len();
            if n == 0 || !total.is_multiple_of(n) {
                return Err(format!(
                    "{}: a {:?} part of {n} steps doesn't divide the {total}-step loop",
                    self.name, part.voice
                ));
            }
            let steps: Vec<&str> = (0..total).map(|i| part.steps[i % n].as_str()).collect();
            let mut i = 0;
            while i < total {
                let tok = steps[i];
                let held = 1 + steps[i + 1..].iter().take_while(|t| **t == "-").count();
                let at = i as f32 * dt;
                let len = held as f32 * dt;
                match tok {
                    "." | "-" => {}
                    _ if part.voice == Voice::Drums => out.extend(
                        drum(tok, at, dt)
                            .ok_or_else(|| format!("{}: unknown drum '{tok}'", self.name))?,
                    ),
                    _ => {
                        let midi = parse_note(tok)
                            .ok_or_else(|| format!("{}: bad note '{tok}'", self.name))?;
                        out.push(voiced(part.voice, at, len, note(midi)));
                    }
                }
                i += if tok == "." || part.voice == Voice::Drums {
                    1
                } else {
                    held
                };
            }
        }
        Ok(out)
    }

    /// One loop as mono samples.
    pub fn samples(&self) -> Result<Vec<f32>, String> {
        let mut s = render(&self.tones()?);
        // Pad to the exact loop length (the last note may end early).
        s.resize((self.len_secs() * SAMPLE_RATE as f32).round() as usize, 0.0);
        Ok(s)
    }

    pub fn wav(&self) -> Result<Vec<u8>, String> {
        Ok(wav_bytes(&self.samples()?))
    }
}

/// MIDI number of a note name like `C4`, `F#5`, `Bb3` (C4 = 60).
pub fn parse_note(s: &str) -> Option<i32> {
    let mut chars = s.chars();
    let base = match chars.next()? {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let (shift, octave) = match rest.strip_prefix('#') {
        Some(o) => (1, o),
        None => match rest.strip_prefix('b') {
            Some(o) => (-1, o),
            None => (0, rest.as_str()),
        },
    };
    let octave: i32 = octave.parse().ok().filter(|o| (0..=8).contains(o))?;
    Some(12 * (octave + 1) + base + shift)
}

fn voiced(voice: Voice, at: f32, len: f32, freq: f32) -> Tone {
    // Leave a hair of silence between notes so repeats articulate.
    let len = (len - 0.012).max(0.02);
    match voice {
        Voice::Lead => {
            let t = Tone::new(Wave::Square { duty: 0.25 }, at, len, freq, freq)
                .env(Envelope {
                    attack: 0.004,
                    decay: 0.08,
                    sustain: 0.65,
                    release: 0.03,
                })
                .volume(0.15);
            if len > 0.3 { t.vibrato(5.5, 0.012) } else { t }
        }
        Voice::Harmony => Tone::new(Wave::Square { duty: 0.125 }, at, len, freq, freq)
            .env(Envelope {
                attack: 0.003,
                decay: 0.06,
                sustain: 0.4,
                release: 0.03,
            })
            .volume(0.08),
        Voice::Bass => Tone::new(Wave::Triangle, at, len, freq, freq)
            .env(Envelope::FLAT)
            .volume(0.42),
        Voice::Drums => unreachable!("drums use drum()"),
    }
}

fn drum(tok: &str, at: f32, step: f32) -> Option<Vec<Tone>> {
    let t = Tone::new;
    let short = |len: f32| len.min(step);
    Some(match tok {
        "k" => vec![
            t(Wave::Triangle, at, short(0.14), 160.0, 45.0).volume(0.6),
            t(Wave::Noise, at, short(0.02), 1500.0, 500.0).volume(0.1),
        ],
        "s" => vec![
            t(Wave::Noise, at, short(0.13), 3500.0, 1800.0).volume(0.2),
            t(Wave::Triangle, at, short(0.06), 260.0, 160.0).volume(0.25),
        ],
        "h" => vec![t(Wave::Noise, at, short(0.03), 10000.0, 10000.0).volume(0.07)],
        "o" => vec![t(Wave::Noise, at, short(0.14), 10000.0, 9000.0).volume(0.06)],
        _ => return None,
    })
}

/// Splits a pattern into steps, dropping bar lines.
fn steps(s: &str) -> Vec<String> {
    s.split_whitespace()
        .filter(|t| *t != "|")
        .map(str::to_owned)
        .collect()
}

fn part(voice: Voice, s: &str) -> Part {
    Part {
        voice,
        steps: steps(s),
    }
}

/// A chord for one bar of eight steps: `"C3 C4 E4 G4"` = bass note then a
/// triad.
fn chord_notes(chord: &str) -> [&str; 4] {
    let v: Vec<&str> = chord.split_whitespace().collect();
    [v[0], v[1], v[2], v[3]]
}

/// Bass bouncing between the root and its octave, one bar per chord.
fn octave_bass(chords: &[&str]) -> String {
    chords
        .iter()
        .map(|c| {
            let b = chord_notes(c)[0];
            let up = parse_note(b).map(|m| name(m + 12)).unwrap_or_default();
            format!("{b} . {up} . {b} . {up} . ")
        })
        .collect()
}

/// Bass playing the root on beats, held.
fn held_bass(chords: &[&str]) -> String {
    chords
        .iter()
        .map(|c| {
            let b = chord_notes(c)[0];
            format!("{b} - - - {b} - - . ")
        })
        .collect()
}

/// Driving bass: root on every step, octave on 3 and 7.
fn driving_bass(chords: &[&str]) -> String {
    chords
        .iter()
        .map(|c| {
            let b = chord_notes(c)[0];
            let up = parse_note(b).map(|m| name(m + 12)).unwrap_or_default();
            format!("{b} {b} {up} {b} {b} {b} {up} {b} ")
        })
        .collect()
}

/// Up-and-down arpeggio over each chord's triad.
fn arpeggio(chords: &[&str]) -> String {
    chords
        .iter()
        .map(|c| {
            let [_, a, b, c] = chord_notes(c);
            format!("{a} {b} {c} {b} {a} {b} {c} {b} ")
        })
        .collect()
}

/// The same line `delay` steps later (an echo), wrapping round the loop.
fn echo(line: &str, delay: usize) -> String {
    let mut s = steps(line);
    // A held note cut by the shift must not start with '-'.
    s.rotate_right(delay);
    for t in s.iter_mut().take(delay) {
        if t == "-" {
            *t = ".".into();
        }
    }
    s.join(" ")
}

/// Note name for a MIDI number (sharps).
fn name(midi: i32) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NAMES[midi.rem_euclid(12) as usize], midi / 12 - 1)
}

/// What should be playing now: the level's tune while it is played, the
/// title tune on the finished screen, silence on the results screen, during
/// a game over and once the level is won (the jingles play then).
pub fn for_game(game: &Game) -> Option<&str> {
    match game.mode {
        Mode::Finished { .. } => Some("title"),
        Mode::Results { .. } => None,
        Mode::Playing if game.sim.exit_reached || game.sim.game_over > 0 => None,
        Mode::Playing => Some(game.sim.level.music.as_str()),
    }
}

/// A track by name (one of [`TRACKS`]).
pub fn track(name: &str) -> Option<Track> {
    Some(match name {
        "title" => title(),
        "forest" => forest(),
        "cave" => cave(),
        "castle" => castle(),
        "boss" => boss(),
        _ => return None,
    })
}

fn title() -> Track {
    let chords = [
        "C3 C4 E4 G4",
        "G2 B3 D4 G4",
        "A2 A3 C4 E4",
        "F2 A3 C4 F4",
        "C3 C4 E4 G4",
        "G2 B3 D4 G4",
        "F2 A3 C4 F4",
        "C3 C4 E4 G4",
    ];
    let lead = "G5 - - - E5 - C5 - | D5 - - - B4 - G4 - | C5 - E5 - A5 - G5 E5 | F5 - - - - - . . |
                G5 - - - E5 - C6 - | B5 - - - G5 - D5 - | C5 - E5 - G5 - F5 D5 | C5 - - - - - . .";
    Track {
        name: "title",
        bpm: 104.0,
        steps_per_beat: 2,
        parts: vec![
            part(Voice::Lead, lead),
            part(Voice::Harmony, &arpeggio(&chords)),
            part(Voice::Bass, &held_bass(&chords)),
            part(Voice::Drums, "k . . . s . . h | k . h . s . . h"),
        ],
    }
}

fn forest() -> Track {
    let chords = [
        // A
        "C3 C4 E4 G4",
        "A2 A3 C4 E4",
        "F2 A3 C4 F4",
        "G2 B3 D4 G4",
        "C3 C4 E4 G4",
        "A2 A3 C4 E4",
        "G2 B3 D4 G4",
        "C3 C4 E4 G4",
        // B
        "F2 A3 C4 F4",
        "G2 B3 D4 G4",
        "E2 B3 E4 G4",
        "A2 A3 C4 E4",
        "F2 A3 C4 F4",
        "G2 B3 D4 G4",
        "C3 C4 E4 G4",
        "G2 B3 D4 G4",
    ];
    let lead = "E5 - G5 - C6 - B5 G5 | A5 - - E5 C5 - E5 - | F5 - A5 - C6 - A5 F5 | G5 - - - D5 - B4 - |
                E5 - G5 - C6 - B5 G5 | A5 - - E5 C6 - B5 A5 | G5 - F5 - D5 - B4 - | C5 - - - - - . . |
                A5 - - G5 F5 - - - | G5 - - F5 E5 - D5 - | E5 - - D5 E5 - G5 - | A5 - - - - - . . |
                A5 - C6 - A5 - F5 - | B5 - D6 - B5 - G5 - | C6 - - - G5 - E5 - | D5 - - - - - . .";
    let drums = "k . h . s . h . | k . h k s . h . | k . h . s . h . | k . s . s s s s";
    Track {
        name: "forest",
        bpm: 138.0,
        steps_per_beat: 2,
        parts: vec![
            part(Voice::Lead, lead),
            part(Voice::Harmony, &arpeggio(&chords)),
            part(Voice::Bass, &octave_bass(&chords)),
            part(Voice::Drums, drums),
        ],
    }
}

fn cave() -> Track {
    let chords = [
        "A2 A3 C4 E4",
        "A2 A3 C4 E4",
        "F2 A3 C4 F4",
        "E2 G#3 B3 E4",
        "A2 A3 C4 E4",
        "A2 A3 C4 E4",
        "D2 A3 D4 F4",
        "E2 G#3 B3 E4",
    ];
    let lead = "A4 - - - C5 - E5 - | D5 - C5 - B4 - - - | A4 - - - F4 - A4 - | G#4 - - - - - . . |
                A4 - - - C5 - E5 - | A5 - G5 - E5 - - - | F5 - E5 - D5 - C5 - | B4 - - - G#4 - . .";
    let bass: String = chords
        .iter()
        .map(|c| {
            let b = chord_notes(c)[0];
            format!("{b} - - {b} - - . . ")
        })
        .collect();
    Track {
        name: "cave",
        bpm: 96.0,
        steps_per_beat: 2,
        parts: vec![
            part(Voice::Lead, lead),
            // A faint echo of the melody: dripping, cavernous.
            part(Voice::Harmony, &echo(lead, 3)),
            part(Voice::Bass, &bass),
            part(Voice::Drums, "k . . h . . s . | k . . h . k s h"),
        ],
    }
}

fn castle() -> Track {
    let chords = [
        "D2 A3 D4 F4",
        "C2 G3 C4 E4",
        "A#1 A#3 D4 F4",
        "A1 A3 C#4 E4",
        "D2 A3 D4 F4",
        "C2 G3 C4 E4",
        "A#1 A#3 D4 F4",
        "A1 A3 C#4 E4",
    ];
    let lead = "D5 - D5 F5 A5 - F5 D5 | E5 - C5 E5 G5 - E5 C5 | D5 - Bb4 D5 F5 - D5 Bb4 | C#5 - E5 - A5 - - - |
                D6 - A5 - F5 - D5 - | E5 - G5 - C6 - G5 - | F5 - E5 - D5 - C#5 - | D5 - - - A4 - . .";
    Track {
        name: "castle",
        bpm: 150.0,
        steps_per_beat: 2,
        parts: vec![
            part(Voice::Lead, lead),
            part(Voice::Harmony, &arpeggio(&chords)),
            part(Voice::Bass, &octave_bass(&chords)),
            part(Voice::Drums, "k . s . k k s . | k . s . k k s s"),
        ],
    }
}

fn boss() -> Track {
    let chords = [
        "E2 G3 B3 E4",
        "E2 G3 B3 E4",
        "C2 G3 C4 E4",
        "D2 F#3 A3 D4",
        "E2 G3 B3 E4",
        "E2 G3 B3 E4",
        "C2 G3 C4 E4",
        "B1 F#3 B3 D#4",
    ];
    let lead = "E5 . E5 G5 - E5 B5 - | A5 G5 F#5 E5 - - . . | E5 . E5 G5 - E5 C6 - | B5 A5 G5 F#5 - - . . |
                E6 - B5 - G5 - E5 - | F#5 G5 A5 B5 - - . . | C6 - B5 - A5 - G5 - | F#5 - D#5 - B4 - . .";
    Track {
        name: "boss",
        bpm: 168.0,
        steps_per_beat: 2,
        parts: vec![
            part(Voice::Lead, lead),
            part(Voice::Harmony, &arpeggio(&chords)),
            part(Voice::Bass, &driving_bass(&chords)),
            part(Voice::Drums, "k h s h k k s h | k h s h k k s s"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_names() {
        assert_eq!(parse_note("C4"), Some(60));
        assert_eq!(parse_note("A4"), Some(69));
        assert_eq!(parse_note("F#5"), Some(78));
        assert_eq!(parse_note("Bb3"), Some(58));
        assert_eq!(parse_note("H4"), None);
        assert_eq!(parse_note("C"), None);
        assert_eq!(parse_note("C9"), None);
        for m in 24..100 {
            assert_eq!(parse_note(&name(m)), Some(m));
        }
    }

    #[test]
    fn holds_and_rests() {
        let t = Track {
            name: "t",
            bpm: 120.0,
            steps_per_beat: 2,
            parts: vec![part(Voice::Lead, "C4 - - . E4 . G4 -")],
        };
        let tones = t.tones().unwrap();
        assert_eq!(tones.len(), 3);
        // 0.25 s per step: C4 held three steps, E4 one, G4 two.
        assert_eq!(tones[0].at, 0.0);
        assert!((tones[0].len - 0.75).abs() < 0.02);
        assert_eq!(tones[1].at, 1.0);
        assert_eq!(tones[2].at, 1.5);
        assert!((tones[2].len - 0.5).abs() < 0.02);
    }

    #[test]
    fn short_parts_repeat_and_must_divide_the_loop() {
        let mut t = Track {
            name: "t",
            bpm: 120.0,
            steps_per_beat: 2,
            parts: vec![
                part(Voice::Lead, "C4 - - - - - - -"),
                part(Voice::Drums, "k . s ."),
            ],
        };
        let kicks = |t: &Track| {
            t.tones()
                .unwrap()
                .iter()
                .filter(|n| n.from == 160.0)
                .count()
        };
        assert_eq!(kicks(&t), 2);
        t.parts[1] = part(Voice::Drums, "k . s");
        assert!(t.tones().unwrap_err().contains("divide"));
    }

    #[test]
    fn bad_tokens_are_errors() {
        let t = Track {
            name: "t",
            bpm: 120.0,
            steps_per_beat: 2,
            parts: vec![part(Voice::Lead, "C4 X4")],
        };
        assert!(t.tones().unwrap_err().contains("X4"));
        let d = Track {
            parts: vec![part(Voice::Drums, "k z")],
            ..t
        };
        assert!(d.tones().unwrap_err().contains("'z'"));
    }

    #[test]
    fn echo_shifts_and_never_starts_on_a_hold() {
        assert_eq!(echo("C4 - - E4", 2), ". E4 C4 -");
        assert_eq!(echo("C4 D4 E4 F4", 1), "F4 C4 D4 E4");
    }

    #[test]
    fn every_track_builds_and_loops_cleanly() {
        for name in TRACKS {
            let t = track(name).unwrap();
            assert!(t.len_steps().is_multiple_of(16), "{name}: whole bars");
            let secs = t.len_secs();
            assert!((8.0..40.0).contains(&secs), "{name}: {secs}s");
            let s = t.samples().unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                s.len(),
                (secs * SAMPLE_RATE as f32).round() as usize,
                "{name}"
            );
            let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.3 && peak <= 1.0, "{name}: peak {peak}");
            // Seamless loop: silent at both ends.
            assert!(s[0].abs() < 0.05 && s[s.len() - 1].abs() < 0.05, "{name}");
            // Every part fits the loop.
            for p in &t.parts {
                assert!(
                    t.len_steps().is_multiple_of(p.steps.len()),
                    "{name} {:?}",
                    p.voice
                );
            }
        }
        assert!(track("nope").is_none());
    }

    #[test]
    fn music_follows_the_game() {
        let mut game = Game::with_campaign(vec!["flat".into()], "flat").unwrap();
        assert_eq!(for_game(&game), Some("forest"));
        game.sim.exit_reached = true;
        assert_eq!(for_game(&game), None);
        game.sim.exit_reached = false;
        game.sim.game_over = 10;
        assert_eq!(for_game(&game), None);
        game.mode = Mode::Finished { ticks: 0 };
        assert_eq!(for_game(&game), Some("title"));
        let boss = Game::new("hypno").unwrap();
        assert_eq!(for_game(&boss), Some("boss"));
    }

    #[test]
    fn tracks_are_deterministic() {
        let a = track("cave").unwrap().wav().unwrap();
        assert_eq!(a, track("cave").unwrap().wav().unwrap());
    }
}
