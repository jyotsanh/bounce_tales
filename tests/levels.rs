//! Level geometry checks that a single replay can't cover: optional
//! routes to the secret eggs.

use bounce_tales::level::Level;
use bounce_tales::sim::enemies::EnemyKind;
use bounce_tales::sim::{EntityKind, InputFrame, Sim};
use macroquad::math::vec2;

fn inp(left: bool, right: bool, jump: bool) -> InputFrame {
    InputFrame { left, right, jump }
}

/// Chapter 1: the blue egg hangs over the mole; stomping it while it is
/// out, with jump held, bounces the ball up to the egg.
#[test]
fn ch01_egg_from_the_mole() {
    let mut sim = Sim::with_level(Level::load("ch01").unwrap());
    let mole = sim
        .entities
        .iter()
        .position(|e| matches!(&e.kind, EntityKind::Enemy(en) if matches!(en.kind, EnemyKind::Mole { .. })))
        .unwrap();
    for _ in 0..400 {
        sim.step(InputFrame::NONE);
        if let EntityKind::Enemy(en) = &sim.entities[mole].kind
            && en.out() == 1.0
        {
            break;
        }
    }
    sim.ball.pos = sim.entities[mole].pos + vec2(8.0, -30.0);
    sim.ball.vel = vec2(0.0, 100.0);
    for _ in 0..120 {
        sim.step(inp(false, false, true));
    }
    assert_eq!(sim.enemies_freed, 1);
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Chapter 2: the spring launches the ball onto the high ledge; breaking
/// the hypno machine there bounces it up to the blue egg.
#[test]
fn ch02_spring_ledge_machine_egg() {
    let mut sim = Sim::with_level(Level::load("ch02").unwrap());
    // Rolling onto the spring from just left of it.
    sim.ball.pos = vec2(78.0 * 16.0, 17.0 * 16.0 - 8.0);
    for step in 0..300 {
        let b = &sim.ball;
        let on_ledge = b.pos.y < 175.0 && b.pos.x > 82.0 * 16.0;
        let right = step < 30 || (b.pos.y < 175.0 && b.pos.x < 1384.0);
        let left = on_ledge && b.pos.x > 1392.0;
        sim.step(inp(left, right, on_ledge && (b.grounded || b.vel.y < 0.0)));
    }
    assert_eq!(sim.machines_broken, 1);
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Every chapter has exactly one secret egg and a fair number of gems.
#[test]
fn chapters_have_one_egg_and_gems() {
    for name in ["ch01", "ch02"] {
        let sim = Sim::with_level(Level::load(name).unwrap());
        assert_eq!(sim.eggs_total, 1, "{name}");
        assert!(sim.gems_total >= 20, "{name}: {}", sim.gems_total);
    }
}
