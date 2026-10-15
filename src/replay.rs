//! Replay files: scripted input plus inline checks.
//!
//! Plain text, one item per line, `#` starts a comment:
//!
//! ```text
//! # roll right for half a second, then stop
//! level forest1          # optional: level to load (used from step 8)
//! 30 R                   # hold Right for 30 steps
//! RJ                     # count omitted = 1 step
//! 20 -                   # nothing held for 20 steps
//! expect ball.x > 150    # checked after all steps above have run
//! expect tick == 51
//! expect ball.y == 252 +- 0.5   # booleans read as 0 / 1
//! ```
//!
//! Inputs use `L`, `R`, `J` (any order) or `-` for nothing.
//! Comparison ops: `<`, `<=`, `>`, `>=`, `==`, `!=`. `==` and `!=` allow
//! `+- tol` (default 0.001). Field names are listed in [`FIELDS`].

use std::fmt::Write as _;

use crate::sim::{InputFrame, Sim};

/// Reads one checkable value from the sim.
pub type Probe = fn(&Sim) -> f64;

/// Values a replay can check, and how to read them from the sim.
pub const FIELDS: &[(&str, Probe)] = &[
    ("tick", |s| s.tick as f64),
    ("ball.x", |s| s.ball.pos.x as f64),
    ("ball.y", |s| s.ball.pos.y as f64),
    ("ball.vx", |s| s.ball.vel.x as f64),
    ("ball.vy", |s| s.ball.vel.y as f64),
    ("ball.speed", |s| s.ball.vel.length() as f64),
    ("ball.grounded", |s| s.ball.grounded as u8 as f64),
    ("ball.angle", |s| s.ball.angle as f64),
    ("ball.squash", |s| s.ball.squash as f64),
    ("ball.water", |s| s.ball.water as f64),
    ("riding", |s| s.riding.is_some() as u8 as f64),
    ("held", |s| s.held.is_some() as u8 as f64),
    // 0 normal, 1 stone, 2 balloon.
    ("form", |s| s.ball.form as u8 as f64),
    ("seesaw.angle", |s| first_seesaw_angle(s) as f64),
    ("on_seesaw", |s| s.on_seesaw.is_some() as u8 as f64),
    ("crate.x", |s| first_crate(s).0 as f64),
    ("crate.y", |s| first_crate(s).1 as f64),
    ("cam.x", |s| s.camera.pos.x as f64),
    ("cam.y", |s| s.camera.pos.y as f64),
    ("exit", |s| s.exit_reached as u8 as f64),
    ("deaths", |s| s.deaths as f64),
    ("dying", |s| (s.dying > 0) as u8 as f64),
    ("lives", |s| s.lives as f64),
    ("game_over", |s| (s.game_over > 0) as u8 as f64),
    ("restarts", |s| s.restarts as f64),
    ("respawn.x", |s| s.respawn_point.x as f64),
    ("respawn.y", |s| s.respawn_point.y as f64),
    ("score", |s| s.score as f64),
    ("gems", |s| s.gems as f64),
    ("eggs", |s| s.eggs as f64),
];

/// Position of the level's first crate (0,0 if none), for replay checks.
fn first_crate(s: &Sim) -> (f32, f32) {
    s.entities
        .iter()
        .find(|e| matches!(e.kind, crate::sim::EntityKind::Crate(_)))
        .map_or((0.0, 0.0), |e| (e.pos.x, e.pos.y))
}

/// Angle (degrees, clockwise) of the level's first seesaw (0 if none).
fn first_seesaw_angle(s: &Sim) -> f32 {
    s.entities
        .iter()
        .find_map(|e| match &e.kind {
            crate::sim::EntityKind::Seesaw(w) => Some(w.angle.to_degrees()),
            _ => None,
        })
        .unwrap_or(0.0)
}

pub fn probe(sim: &Sim, field: &str) -> Option<f64> {
    FIELDS
        .iter()
        .find(|(n, _)| *n == field)
        .map(|(_, f)| f(sim))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl Op {
    fn parse(s: &str) -> Option<Op> {
        Some(match s {
            "<" => Op::Lt,
            "<=" => Op::Le,
            ">" => Op::Gt,
            ">=" => Op::Ge,
            "==" => Op::Eq,
            "!=" => Op::Ne,
            _ => return None,
        })
    }

    fn symbol(self) -> &'static str {
        match self {
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
            Op::Eq => "==",
            Op::Ne => "!=",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// Checked once this many steps have run.
    pub after: u64,
    pub field: String,
    pub op: Op,
    pub value: f64,
    pub tol: f64,
    /// 1-based line in the source file, for error messages.
    pub line: usize,
}

impl Check {
    /// `None` if it passes, otherwise a message describing the failure.
    pub fn evaluate(&self, sim: &Sim) -> Option<String> {
        let actual = probe(sim, &self.field).expect("field validated at parse time");
        let ok = match self.op {
            Op::Lt => actual < self.value,
            Op::Le => actual <= self.value,
            Op::Gt => actual > self.value,
            Op::Ge => actual >= self.value,
            Op::Eq => (actual - self.value).abs() <= self.tol,
            Op::Ne => (actual - self.value).abs() > self.tol,
        };
        (!ok).then(|| {
            format!(
                "line {}: expected {} {} {} at tick {}, got {}",
                self.line,
                self.field,
                self.op.symbol(),
                self.value,
                self.after,
                actual
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Replay {
    pub level: Option<String>,
    /// One entry per sim step.
    pub inputs: Vec<InputFrame>,
    pub checks: Vec<Check>,
}

impl Replay {
    pub fn parse(text: &str) -> Result<Replay, String> {
        let mut r = Replay::default();
        for (i, raw) in text.lines().enumerate() {
            let line_no = i + 1;
            let err = |m: &str| format!("line {line_no}: {m}: '{}'", raw.trim());
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let tok: Vec<&str> = line.split_whitespace().collect();
            match tok[0] {
                "level" => {
                    if tok.len() != 2 {
                        return Err(err("usage: level <name>"));
                    }
                    r.level = Some(tok[1].to_owned());
                }
                "expect" => {
                    let (field, op, value, tol) = match tok[1..] {
                        [f, o, v] => (f, o, v, None),
                        [f, o, v, "+-", t] => (f, o, v, Some(t)),
                        _ => return Err(err("usage: expect <field> <op> <value> [+- tol]")),
                    };
                    if probe(&Sim::new(), field).is_none() {
                        let names: Vec<_> = FIELDS.iter().map(|(n, _)| *n).collect();
                        return Err(err(&format!("unknown field (known: {})", names.join(", "))));
                    }
                    let op = Op::parse(op).ok_or_else(|| err("unknown operator"))?;
                    let value = value.parse().map_err(|_| err("bad number"))?;
                    let tol = match tol {
                        Some(t) => t.parse().map_err(|_| err("bad tolerance"))?,
                        None => 0.001,
                    };
                    r.checks.push(Check {
                        after: r.inputs.len() as u64,
                        field: field.to_owned(),
                        op,
                        value,
                        tol,
                        line: line_no,
                    });
                }
                _ => {
                    let (count, code) = match tok[..] {
                        [code] => (1, code),
                        [n, code] => (n.parse().map_err(|_| err("bad step count"))?, code),
                        _ => return Err(err("expected '<count> <keys>'")),
                    };
                    let input = InputFrame::from_code(code).ok_or_else(|| err("bad keys"))?;
                    r.inputs.extend(std::iter::repeat_n(input, count));
                }
            }
        }
        Ok(r)
    }

    pub fn load(path: &std::path::Path) -> Result<Replay, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Replay::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// A fresh sim for this replay's level (or `default_level` if the file
    /// names none).
    pub fn start_sim(&self, default_level: &str) -> Result<Sim, String> {
        let name = self.level.as_deref().unwrap_or(default_level);
        Sim::for_level(name)
    }

    /// Input for the step that starts at `tick` (nothing once the replay ends).
    pub fn input_at(&self, tick: u64) -> InputFrame {
        self.inputs
            .get(tick as usize)
            .copied()
            .unwrap_or(InputFrame::NONE)
    }

    /// Failures of the checks due at the sim's current tick.
    pub fn failures_now(&self, sim: &Sim) -> Vec<String> {
        self.checks
            .iter()
            .filter(|c| c.after == sim.tick)
            .filter_map(|c| c.evaluate(sim))
            .collect()
    }

    /// Runs the whole replay on `sim` and returns every failed check.
    pub fn run(&self, sim: &mut Sim) -> Vec<String> {
        let mut failures = self.failures_now(sim);
        let last_check = self.checks.iter().map(|c| c.after).max().unwrap_or(0);
        let end = (self.inputs.len() as u64).max(last_check);
        while sim.tick < end {
            sim.step(self.input_at(sim.tick));
            failures.extend(self.failures_now(sim));
        }
        failures
    }

    /// Writes inputs back out as run-length text (checks are not kept).
    pub fn to_text(&self) -> String {
        let mut out = String::from("# bounce_tales replay\n");
        if let Some(level) = &self.level {
            let _ = writeln!(out, "level {level}");
        }
        let mut i = 0;
        while i < self.inputs.len() {
            let f = self.inputs[i];
            let n = self.inputs[i..].iter().take_while(|&&x| x == f).count();
            let _ = writeln!(out, "{n} {}", f.to_code());
            i += n;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "
        # sample
        level test
        3 R      # three steps right
        RJ
        2 -
        expect tick == 6
        expect ball.x > 72
        expect ball.vx != 0 +- 0.5
    ";

    #[test]
    fn parses_sample() {
        let r = Replay::parse(SAMPLE).unwrap();
        assert_eq!(r.level.as_deref(), Some("test"));
        assert_eq!(r.inputs.len(), 6);
        assert_eq!(r.inputs[3].to_code(), "RJ");
        assert_eq!(r.checks.len(), 3);
        assert!(r.checks.iter().all(|c| c.after == 6));
        assert_eq!(r.checks[2].tol, 0.5);
    }

    #[test]
    fn sample_passes() {
        let r = Replay::parse(SAMPLE).unwrap();
        let mut sim = r.start_sim("flat").unwrap();
        assert_eq!(r.run(&mut sim), Vec::<String>::new());
    }

    #[test]
    fn failing_check_reports_line() {
        let r = Replay::parse("5 L\nexpect ball.x > 1000").unwrap();
        let f = r.run(&mut Sim::new());
        assert_eq!(f.len(), 1);
        assert!(f[0].starts_with("line 2:"), "{}", f[0]);
    }

    #[test]
    fn checks_past_the_end_still_run() {
        let r = Replay::parse("2 R\nexpect tick == 2\n").unwrap();
        assert!(r.run(&mut Sim::new()).is_empty());
    }

    #[test]
    fn parse_errors() {
        for bad in [
            "x R",
            "3 Q",
            "1 2 3",
            "expect nope == 1",
            "expect tick ~ 1",
            "expect tick == z",
            "level",
        ] {
            let e = Replay::parse(bad).unwrap_err();
            assert!(e.starts_with("line 1:"), "{bad}: {e}");
        }
    }

    #[test]
    fn text_round_trip() {
        let r = Replay::parse(SAMPLE).unwrap();
        let back = Replay::parse(&r.to_text()).unwrap();
        assert_eq!(back.inputs, r.inputs);
        assert_eq!(back.level, r.level);
    }
}
