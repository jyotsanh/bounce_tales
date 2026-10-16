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

/// Chapter 3: the egg nook sits above the lift's highest point; jump left
/// off the lift at the top.
#[test]
fn ch03_egg_from_the_lift() {
    let mut sim = Sim::with_level(Level::load("ch03").unwrap());
    let lift = sim
        .entities
        .iter()
        .position(|e| matches!(e.kind, EntityKind::Platform(_)) && e.pos.y < 268.0)
        .unwrap();
    // Ride it from the bottom.
    while sim.entities[lift].pos.y < 255.0 {
        sim.step(InputFrame::NONE);
    }
    let p = sim.entities[lift].pos;
    sim.ball.pos = p + vec2(24.0, -8.0);
    sim.ball.vel = vec2(0.0, 0.0);
    while sim.entities[lift].pos.y > 12.0 * 16.0 + 1.0 {
        sim.step(InputFrame::NONE);
    }
    for _ in 0..90 {
        let b = &sim.ball;
        let over_nook = b.pos.x < 46.5 * 16.0;
        sim.step(inp(!over_nook || b.pos.x > 44.5 * 16.0, false, b.grounded));
    }
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Chapter 6: from the top ledge of the zig-zag shaft, jump left over the
/// little stepping ledge into the alcove with the egg (same height).
#[test]
fn ch06_egg_alcove_above_the_shaft() {
    let mut sim = Sim::with_level(Level::load("ch06").unwrap());
    // Start at the right end of the ledge for a run-up, jump off each edge.
    sim.ball.pos = vec2(72.4 * 16.0, 12.0 * 16.0 - 8.0);
    for _ in 0..240 {
        let b = &sim.ball;
        let top_edge = (b.pos.y - 184.0).abs() < 2.0 && b.pos.x < 69.4 * 16.0;
        sim.step(inp(true, false, b.grounded && top_edge));
    }
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Chapter 7: the egg sits on a one-tile ledge in the dark chamber's corner;
/// jump left to it off the ledge beside it.
#[test]
fn ch07_egg_in_the_dark_corner() {
    let mut sim = Sim::with_level(Level::load("ch07").unwrap());
    sim.ball.pos = vec2(89.0 * 16.0, 11.0 * 16.0 - 8.0);
    for _ in 0..120 {
        let b = &sim.ball;
        let jump = b.grounded && b.pos.x < 87.6 * 16.0;
        sim.step(inp(true, false, jump));
    }
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Chapter 9: break the hypno machine at the end of the backwards conveyor
/// with jump held; its bounce reaches the egg above it.
#[test]
fn ch09_egg_from_the_machine() {
    let mut sim = Sim::with_level(Level::load("ch09").unwrap());
    sim.ball.pos = vec2(77.5 * 16.0, 18.0 * 16.0 - 8.0);
    for _ in 0..200 {
        let x = sim.ball.pos.x;
        sim.step(inp(x > 1276.0, x < 1268.0, true));
    }
    assert_eq!(sim.machines_broken, 1);
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Chapter 10: hop onto the crate where it starts and jump from its top.
#[test]
fn ch10_egg_from_the_crate() {
    let mut sim = Sim::with_level(Level::load("ch10").unwrap());
    sim.ball.pos = vec2(46.0 * 16.0, 14.0 * 16.0 - 8.0);
    for _ in 0..90 {
        let g = sim.ball.grounded;
        sim.step(inp(false, false, g));
    }
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Chapter 11: ride the big wheel and jump from the top car.
#[test]
fn ch11_egg_from_the_wheel_top() {
    let mut sim = Sim::with_level(Level::load("ch11").unwrap());
    let top_car = |sim: &Sim| {
        sim.entities
            .iter()
            .filter(|e| matches!(e.kind, EntityKind::Platform(_)))
            .map(|e| e.pos)
            .min_by(|a, b| a.y.total_cmp(&b.y))
            .unwrap()
    };
    // Wait until a car is right at the top, then stand on it and jump.
    while (top_car(&sim).x + 16.0 - (48.0 * 16.0 + 8.0)).abs() > 2.0 {
        sim.step(InputFrame::NONE);
    }
    let p = top_car(&sim);
    sim.ball.pos = p + vec2(16.0, -8.0);
    sim.ball.vel = vec2(0.0, 0.0);
    for _ in 0..60 {
        let g = sim.ball.grounded;
        sim.step(inp(false, false, g));
    }
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Chapter 12: from the top of the red climb, jump left into the nook.
#[test]
fn ch12_egg_in_the_nook() {
    let mut sim = Sim::with_level(Level::load("ch12").unwrap());
    sim.ball.pos = vec2(23.4 * 16.0, 12.0 * 16.0 - 8.0);
    for _ in 0..150 {
        let b = &sim.ball;
        let edge = (b.pos.y - 184.0).abs() < 2.0 && b.pos.x < 20.4 * 16.0;
        sim.step(inp(true, false, b.grounded && edge));
    }
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Chapter 13: break the machine in the upper hall with jump held.
#[test]
fn ch13_egg_from_the_machine() {
    let mut sim = Sim::with_level(Level::load("ch13").unwrap());
    sim.ball.pos = vec2(115.5 * 16.0, 8.0 * 16.0 - 8.0);
    for _ in 0..200 {
        let x = sim.ball.pos.x;
        sim.step(inp(x > 118.5 * 16.0 + 4.0, x < 118.5 * 16.0 - 4.0, true));
    }
    assert_eq!(sim.machines_broken, 1);
    assert_eq!(sim.eggs, 1);
    assert_eq!(sim.deaths, 0);
}

/// Chapter 14: jump for the egg from the second ride's platform.
#[test]
fn ch14_egg_from_the_second_ride() {
    let mut sim = Sim::with_level(Level::load("ch14").unwrap());
    let second = |sim: &Sim| {
        sim.entities
            .iter()
            .filter(|e| matches!(e.kind, EntityKind::Platform(_)))
            .map(|e| e.pos)
            .find(|p| p.x > 21.5 * 16.0 && p.x < 33.0 * 16.0)
            .unwrap()
    };
    // Wait for the platform's middle to pass under the egg (col 27).
    while (second(&sim).x + 24.0 - 27.5 * 16.0).abs() > 1.5 {
        sim.step(InputFrame::NONE);
    }
    sim.ball.pos = second(&sim) + vec2(24.0, -8.0);
    sim.ball.vel = vec2(0.0, 0.0);
    for _ in 0..60 {
        let g = sim.ball.grounded;
        sim.step(inp(false, false, g));
    }
    assert_eq!(sim.eggs, 1);
}

/// Chapter 15: the egg above the big wheel's top car.
#[test]
fn ch15_egg_from_the_wheel_top() {
    let mut sim = Sim::with_level(Level::load("ch15").unwrap());
    let top_car = |sim: &Sim| {
        sim.entities
            .iter()
            .filter(|e| matches!(e.kind, EntityKind::Platform(_)))
            .map(|e| e.pos)
            .min_by(|a, b| a.y.total_cmp(&b.y))
            .unwrap()
    };
    while (top_car(&sim).x + 16.0 - (48.0 * 16.0 + 8.0)).abs() > 2.0 {
        sim.step(InputFrame::NONE);
    }
    sim.ball.pos = top_car(&sim) + vec2(16.0, -8.0);
    sim.ball.vel = vec2(0.0, 0.0);
    for _ in 0..60 {
        let g = sim.ball.grounded;
        sim.step(inp(false, false, g));
    }
    assert_eq!(sim.eggs, 1);
}

/// Every chapter has exactly one secret egg and a fair number of gems.
#[test]
fn chapters_have_one_egg_and_gems() {
    for name in [
        "ch01", "ch02", "ch03", "ch04", "ch05", "ch06", "ch07", "ch08", "ch09", "ch10", "ch11",
        "ch12", "ch13", "ch14", "ch15",
    ] {
        let sim = Sim::with_level(Level::load(name).unwrap());
        assert_eq!(sim.eggs_total, 1, "{name}");
        assert!(sim.gems_total >= 15, "{name}: {}", sim.gems_total);
    }
}

/// The bonus level needs eggs from the campaign: leave some slack so a
/// player can miss a few and still open it.
#[test]
fn enough_campaign_eggs_for_the_bonus() {
    use bounce_tales::game::{BONUS, BONUS_EGGS, CAMPAIGN};
    let eggs: u32 = CAMPAIGN
        .iter()
        .map(|n| Sim::with_level(Level::load(n).unwrap()).eggs_total)
        .sum();
    assert!(eggs >= BONUS_EGGS + 3, "{eggs} campaign eggs");
    for n in BONUS {
        Level::load(n).unwrap();
    }
}
