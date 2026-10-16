//! Bug bash: every level, several input patterns (idle, rolling either
//! way, jumping, seeded random play), 3000 steps each. Nothing may panic,
//! and the ball must stay sane: finite, within reach of the level, never
//! stuck inside solid ground.

use bounce_tales::level::{Level, level_names};
use bounce_tales::sim::map::{TILE, Tile, TileMap};
use bounce_tales::sim::rng::Rng;
use bounce_tales::sim::{InputFrame, Sim};

const STEPS: u32 = 3000;

#[derive(Clone, Copy, Debug)]
enum Pattern {
    Idle,
    Right,
    Left,
    RightJumping,
    Random(u32),
}

fn input(p: Pattern, step: u32, rng: &mut Rng, held: &mut InputFrame) -> InputFrame {
    let f = |l, r, j| InputFrame {
        left: l,
        right: r,
        jump: j,
    };
    match p {
        Pattern::Idle => InputFrame::NONE,
        Pattern::Right => f(false, true, false),
        Pattern::Left => f(true, false, false),
        Pattern::RightJumping => f(false, true, step % 40 < 20),
        Pattern::Random(_) => {
            // Change what is held every so often, like a restless player.
            if step.is_multiple_of(8 + rng.next_u32() % 30) {
                let k = rng.next_u32() % 6;
                *held = f(
                    k == 0 || k == 3,
                    k == 1 || k == 4 || k == 5,
                    k >= 3 || rng.next_u32().is_multiple_of(4),
                );
            }
            *held
        }
    }
}

/// Is the point inside a fully solid tile?
fn in_solid(map: &TileMap, p: macroquad::math::Vec2) -> bool {
    let (cx, cy) = TileMap::cell_at(p);
    map.get(cx, cy) == Tile::Solid
}

fn run(name: &str, pattern: Pattern) -> Result<(), String> {
    let mut sim = Sim::with_level(Level::load(name).map_err(|e| format!("{name}: {e}"))?);
    let seed = match pattern {
        Pattern::Random(s) => s,
        _ => 1,
    };
    let mut rng = Rng::new(seed);
    let mut held = InputFrame::NONE;
    let size = sim.level.map.pixel_size();
    let mut embedded = 0;
    for step in 0..STEPS {
        let i = input(pattern, step, &mut rng, &mut held);
        sim.step(i);
        let b = &sim.ball;
        let at = format!(
            "{name} {pattern:?} step {step}: ball {:?} vel {:?}",
            b.pos, b.vel
        );
        if !(b.pos.is_finite() && b.vel.is_finite()) {
            return Err(format!("not finite: {at}"));
        }
        if b.pos.x < -4.0 * TILE
            || b.pos.x > size.x + 4.0 * TILE
            || b.pos.y < -20.0 * TILE
            || b.pos.y > size.y + 40.0 * TILE
        {
            return Err(format!("escaped the level: {at}"));
        }
        // A ball being carried, held in a cannon or dying may overlap
        // briefly; it must never stay inside rock.
        if sim.dying == 0 && sim.held.is_none() && in_solid(&sim.level.map, b.pos) {
            embedded += 1;
            if embedded > 10 {
                return Err(format!("stuck inside solid ground: {at}"));
            }
        } else {
            embedded = 0;
        }
        for e in &sim.entities {
            if !(e.pos.is_finite() && e.size.is_finite()) {
                return Err(format!(
                    "{name} {pattern:?} step {step}: entity not finite {:?}",
                    e.kind
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn every_level_survives_every_input_pattern() {
    let patterns = [
        Pattern::Idle,
        Pattern::Right,
        Pattern::Left,
        Pattern::RightJumping,
        Pattern::Random(7),
        Pattern::Random(1234),
    ];
    let mut failures = Vec::new();
    for name in level_names() {
        for p in patterns {
            let r = std::panic::catch_unwind(|| run(name, p));
            match r {
                Ok(Ok(())) => {}
                Ok(Err(e)) => failures.push(e),
                Err(_) => failures.push(format!("{name} {p:?}: panicked")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Menus, map, cutscenes, pause, options, levels: random key mashing
/// through the whole app never panics, and every level it enters runs.
#[test]
fn mashing_keys_through_the_app_never_panics() {
    use bounce_tales::app::{App, Hotkey, MenuKeys};
    for seed in [3u32, 99, 2024] {
        let mut app = App::new();
        app.progress.unlock_all = true;
        let mut rng = Rng::new(seed);
        let mut held = InputFrame::NONE;
        let mut levels_seen = std::collections::BTreeSet::new();
        for step in 0..20_000u32 {
            let r = rng.next_u32() % 100;
            let keys = MenuKeys {
                up: r < 6,
                down: (6..12).contains(&r),
                left: (12..15).contains(&r),
                right: (15..20).contains(&r),
                confirm: (20..28).contains(&r),
                back: r == 28,
            };
            if step.is_multiple_of(20) {
                let k = rng.next_u32() % 4;
                held = InputFrame {
                    left: k == 0,
                    right: k >= 2,
                    jump: k == 3,
                };
            }
            if rng.next_u32().is_multiple_of(500) {
                app.hotkey(Hotkey::MusicUp);
            }
            app.step(held, keys);
            if let Some(g) = &app.game {
                levels_seen.insert(g.level_name.clone());
                assert!(g.sim.ball.pos.is_finite());
            }
            if app.quit {
                app = App::new();
                app.progress.unlock_all = true;
            }
        }
        assert!(
            !levels_seen.is_empty(),
            "seed {seed}: never got into a level"
        );
    }
}
