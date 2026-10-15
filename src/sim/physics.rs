//! Collision between the ball and the tile map.
//!
//! The ball is a circle; every solid tile is a small convex polygon (square
//! or slope, see [`Tile::polygon`]). Each step the ball's motion is split
//! into sub-steps no longer than half its radius, so even very fast balls
//! cannot pass through a one-tile wall. After each sub-step the ball is
//! pushed out of the tile it overlaps most, a few times over.
//!
//! Seams: an edge covered by the neighbouring tile ([`TileMap::edge_hidden`])
//! is not a surface and is skipped. On an open surface made of several tiles
//! the ball touches every tile's edge at the same depth, so taking the
//! deepest contact never picks a seam corner.
//!
//! Staying on the ground: a ball that was rolling (not jumping) and lost
//! contact this step is pulled down up to [`SNAP_DISTANCE`] to find the floor
//! again, so it follows convex bends (slope crests) instead of hopping off.
//! When the floor angle changes under a rolling ball, its speed is kept and
//! redirected along the new surface.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::Ball;
use super::map::{TILE, Tile, TileMap};

/// Contact normals with `y` below this count as floor (up to ~53° slopes).
pub const FLOOR_NORMAL_Y: f32 = -0.6;
/// Contact normals with `y` above this count as ceiling.
pub const CEILING_NORMAL_Y: f32 = 0.6;
/// How far a rolling ball is pulled down to stay on the ground, px.
pub const SNAP_DISTANCE: f32 = 6.0;
/// Most sub-steps per step (limits cost of absurd speeds).
const MAX_SUBSTEPS: u32 = 16;
/// Push-out passes per sub-step (handles corners touching two tiles).
const RESOLVE_PASSES: u32 = 4;

/// What the ball touched during one step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Contacts {
    /// Cells the ball was pushed out of, with the speed it hit them at
    /// (px/s into the surface; 0 when just resting against them).
    pub hits: Vec<(i32, i32, f32)>,
    pub floor: bool,
    pub wall: bool,
    pub ceiling: bool,
    /// Speed into the floor at impact, px/s (0 if no floor hit).
    pub impact_speed: f32,
    /// Normal of the hardest floor hit (points away from the floor).
    pub floor_normal: Vec2,
    /// Extra solid boxes (crates) the ball pressed sideways into, with the
    /// ball's horizontal speed at the time: (index into `solids`, vx).
    pub pushes: Vec<(usize, f32)>,
}

/// An axis-aligned solid box other than the tile map (a crate): (min, max).
pub type SolidBox = (Vec2, Vec2);

/// Moves the ball by its velocity for one step, colliding with the map, then
/// applies ground-following and landing/bounce rules.
pub fn move_and_collide(ball: &mut Ball, map: &TileMap) -> Contacts {
    move_and_collide_with(ball, map, &[])
}

/// [`move_and_collide`] with extra solid boxes (crates). Pressing sideways
/// into a box doesn't bounce the ball off; it records a push instead.
pub fn move_and_collide_with(ball: &mut Ball, map: &TileMap, solids: &[SolidBox]) -> Contacts {
    // An up-draft lifting the ball must not be undone by ground snapping.
    let rolling = ball.grounded && !ball.jumping && ball.wind.y >= 0.0;
    let old_normal = ball.ground_normal;
    // Speed along the old surface (ignores the small push into the floor
    // that keeps a rolling ball in contact).
    let surface_speed = (ball.vel - old_normal * ball.vel.dot(old_normal)).length();

    let distance = ball.vel.length() * DT;
    let substeps = ((distance / (ball.radius * 0.5)).ceil() as u32).clamp(1, MAX_SUBSTEPS);
    let mut c = Contacts::default();
    for _ in 0..substeps {
        // Velocity can change mid-step (wall hit), so re-read it each time.
        ball.pos += ball.vel * (DT / substeps as f32);
        resolve(ball, map, solids, &mut c);
    }

    if rolling && !c.floor {
        snap_to_floor(ball, map, solids, old_normal, &mut c);
    }
    if rolling && c.floor {
        // A rolling ball meeting a new floor angle is not a landing: no
        // bounce, however sharp the bend.
        c.impact_speed = 0.0;
        let n = c.floor_normal;
        if !c.wall && n.distance(old_normal) > 1e-3 {
            // Floor angle changed under a rolling ball: keep its speed and
            // send it along the new surface in the same horizontal direction.
            let t = vec2(-n.y, n.x);
            let dir = if ball.vel.dot(t) >= 0.0 { t } else { -t };
            ball.vel = dir * surface_speed;
        }
    }
    land(ball, &c);
    c
}

/// Runs push-out passes until the ball overlaps nothing (or passes run out).
fn resolve(ball: &mut Ball, map: &TileMap, solids: &[SolidBox], c: &mut Contacts) {
    for _ in 0..RESOLVE_PASSES {
        if !resolve_deepest(ball, map, solids, c) {
            break;
        }
    }
}

/// Pulls a ball that just lost the ground back onto a floor within reach,
/// if nothing else is in the way. The pull goes into the surface it was
/// rolling on (`old_normal` reversed), not straight down: at a 45° crest a
/// straight-down pull lands back on the slope, whose push-out would shove
/// the ball backwards every step.
fn snap_to_floor(
    ball: &mut Ball,
    map: &TileMap,
    solids: &[SolidBox],
    old_normal: Vec2,
    c: &mut Contacts,
) {
    let (pos, vel) = (ball.pos, ball.vel);
    ball.pos -= old_normal * SNAP_DISTANCE;
    let mut probe = Contacts::default();
    resolve(ball, map, solids, &mut probe);
    let moved_into_surface = (ball.pos - pos).dot(old_normal) <= 1e-3;
    if probe.floor && !probe.wall && !probe.ceiling && moved_into_surface {
        c.floor = true;
        c.floor_normal = probe.floor_normal;
        ball.vel = vel;
    } else {
        ball.pos = pos;
        ball.vel = vel;
    }
}

/// Pushes the ball out of the tile it overlaps most and updates its
/// velocity. Returns false if it overlaps nothing.
fn resolve_deepest(ball: &mut Ball, map: &TileMap, solids: &[SolidBox], c: &mut Contacts) -> bool {
    let p = ball.pos;
    let r = ball.radius;
    let (x0, y0) = TileMap::cell_at(p - Vec2::splat(r));
    let (x1, y1) = TileMap::cell_at(p + Vec2::splat(r));

    let mut best: Option<(f32, Vec2, (i32, i32))> = None; // (penetration, normal, cell)
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            if let Some((pen, n)) = penetration(map, tx, ty, p, r)
                && best.is_none_or(|(b, _, _)| pen > b)
            {
                best = Some((pen, n, (tx, ty)));
            }
        }
    }
    // A solid box overlapped more deeply than any tile wins.
    let mut best_box: Option<(f32, Vec2, usize)> = None;
    for (i, &(min, max)) in solids.iter().enumerate() {
        if let Some((pen, n)) = box_penetration(min, max, p, r)
            && best.is_none_or(|(b, _, _)| pen > b)
            && best_box.is_none_or(|(b, _, _)| pen > b)
        {
            best_box = Some((pen, n, i));
        }
    }
    if let Some((pen, n, i)) = best_box {
        ball.pos += n * pen;
        if n.x.abs() > 0.7 && ball.vel.x * -n.x > 0.0 {
            c.pushes.push((i, ball.vel.x));
            c.wall = true;
            // Keep pressing at crate speed instead of bouncing off, so the
            // ball and crate travel together.
            let max = super::crates::PUSH_SPEED;
            ball.vel.x = ball.vel.x.clamp(-max, max);
        } else {
            respond(ball, n, c);
        }
        return true;
    }
    let Some((pen, n, (tx, ty))) = best else {
        return false;
    };
    ball.pos += n * pen;
    let speed_in = (-ball.vel.dot(n)).max(0.0);
    match c.hits.iter_mut().find(|h| (h.0, h.1) == (tx, ty)) {
        Some(h) => h.2 = h.2.max(speed_in),
        None => c.hits.push((tx, ty, speed_in)),
    }
    respond(ball, n, c);
    true
}

/// How far a circle at `p` overlaps tile (tx, ty), and the direction to push
/// it out. Only edges that are real surfaces (not hidden by a neighbour)
/// count.
fn penetration(map: &TileMap, tx: i32, ty: i32, p: Vec2, r: f32) -> Option<(f32, Vec2)> {
    let tile = map.get(tx, ty);
    if !tile.is_solid() {
        return None;
    }
    let origin = vec2(tx as f32, ty as f32) * TILE;
    let local = p - origin;

    // Nearest open edge: (distance, closest point, outward normal).
    let mut nearest: Option<(f32, Vec2, Vec2)> = None;
    for (a, b) in tile.edges() {
        if map.edge_hidden(tx, ty, a, b) {
            continue;
        }
        let q = closest_on_segment(local, a, b);
        let d = local.distance(q);
        if nearest.is_none_or(|(best, _, _)| d < best) {
            let e = b - a;
            // Clockwise outline (y down): outward normal is (e.y, -e.x).
            nearest = Some((d, q, vec2(e.y, -e.x).normalize()));
        }
    }
    let (dist, q, outward) = nearest?;

    if inside_convex(tile, local) {
        // Centre inside the tile (only after a deep hit or a spawn inside):
        // leave through the nearest open edge.
        return Some((dist + r, outward));
    }
    if dist >= r {
        return None;
    }
    let n = if dist > 1e-4 {
        (local - q) / dist
    } else {
        outward
    };
    Some((r - dist, n))
}

/// Overlap of a circle at `p` with an axis-aligned box, and the direction
/// to push it out.
fn box_penetration(min: Vec2, max: Vec2, p: Vec2, r: f32) -> Option<(f32, Vec2)> {
    let q = p.clamp(min, max);
    let d = p - q;
    let dist = d.length();
    if dist >= r {
        return None;
    }
    if dist > 1e-4 {
        return Some((r - dist, d / dist));
    }
    // Centre inside the box: leave through the nearest face.
    let faces = [
        (p.y - min.y, vec2(0.0, -1.0)),
        (max.y - p.y, vec2(0.0, 1.0)),
        (p.x - min.x, vec2(-1.0, 0.0)),
        (max.x - p.x, vec2(1.0, 0.0)),
    ];
    let (depth, n) = faces
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .expect("four faces");
    Some((depth + r, n))
}

fn closest_on_segment(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    a + ab * t
}

/// Point strictly inside a tile's (clockwise, convex) outline.
fn inside_convex(tile: Tile, p: Vec2) -> bool {
    tile.edges().all(|(a, b)| (b - a).perp_dot(p - a) > 0.0)
}

/// Velocity response to touching a surface with normal `n`.
fn respond(ball: &mut Ball, n: Vec2, c: &mut Contacts) {
    let vn = ball.vel.dot(n);
    if n.y < FLOOR_NORMAL_Y {
        c.floor = true;
        if vn < 0.0 {
            // Bounce is applied once per step in `land`, not per sub-step.
            ball.vel -= n * vn;
            if -vn > c.impact_speed {
                c.impact_speed = -vn;
                c.floor_normal = n;
            }
        }
        if c.floor_normal == Vec2::ZERO {
            c.floor_normal = n;
        }
    } else if vn < 0.0 {
        if n.y > CEILING_NORMAL_Y {
            c.ceiling = true;
            ball.vel -= n * vn; // bonk: stop rising, no bounce
        } else {
            c.wall = true;
            ball.vel -= n * vn * (1.0 + ball.params.wall_restitution);
        }
    }
}

/// Shared landing rules: bounce if hitting hard, otherwise come to rest.
/// Also updates the ground/air counters.
fn land(ball: &mut Ball, c: &Contacts) {
    let p = ball.params;
    if c.floor {
        ball.jumping = false;
        ball.air_ticks = 0;
        ball.ground_normal = c.floor_normal;
        if c.impact_speed > p.min_bounce_speed {
            ball.vel += c.floor_normal * c.impact_speed * p.restitution;
            ball.grounded = false;
        } else {
            ball.grounded = true;
        }
    } else {
        ball.grounded = false;
        ball.air_ticks = ball.air_ticks.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::InputFrame;
    use crate::sim::ball::{BallParams, ball_at};

    /// Floor surface of the generated test maps (row 18 of 20).
    const FLOOR: f32 = 18.0 * TILE;
    /// Width in tiles of the wide flat test map.
    const WIDE_TILES: usize = 2000;
    /// Start position far from both walls of the wide test map.
    const MID: f32 = WIDE_TILES as f32 * TILE / 2.0;

    /// 20-row map: `rows` drawn above a two-row floor, padded with empty
    /// space. Each string is one row; all must have the same width.
    fn map_with(rows: &[&str]) -> TileMap {
        let w = rows[0].len();
        let mut text = String::new();
        for i in 0..18 {
            let row = if i >= 18 - rows.len() {
                rows[i - (18 - rows.len())].to_owned()
            } else {
                ".".repeat(w)
            };
            text.push_str(&row);
            text.push('\n');
        }
        let floor = "#".repeat(w) + "\n";
        text.push_str(&floor);
        text.push_str(&floor);
        // Spawn is unused by these tests but the format requires one.
        text.replace_range(0..1, "o");
        TileMap::from_ascii(&text).unwrap()
    }

    fn flat(width_tiles: usize) -> TileMap {
        map_with(&[&".".repeat(width_tiles)])
    }

    /// Ball in a map, stepped like the real sim does it.
    struct World {
        ball: Ball,
        prev: InputFrame,
        map: TileMap,
    }

    impl World {
        fn new(map: TileMap, x: f32, y: f32) -> World {
            World {
                ball: ball_at(x, y),
                prev: InputFrame::NONE,
                map,
            }
        }

        /// Settled on the floor of a wide flat map.
        fn resting(x: f32) -> World {
            let mut w = World::new(flat(WIDE_TILES), x, FLOOR - BallParams::NORMAL.radius);
            w.run("-", 5);
            assert!(w.ball.grounded);
            w
        }

        fn step(&mut self, code: &str) -> Contacts {
            let input = InputFrame::from_code(code).unwrap();
            let pressed = input.jump && !self.prev.jump;
            self.prev = input;
            self.ball.update(input, pressed);
            let c = move_and_collide(&mut self.ball, &self.map);
            self.ball.update_spin();
            c
        }

        fn run(&mut self, code: &str, steps: usize) {
            for _ in 0..steps {
                self.step(code);
            }
        }

        /// Highest the ball's bottom gets above the floor during `steps`.
        fn max_rise(&mut self, code: &str, steps: usize) -> f32 {
            let rest_y = FLOOR - self.ball.radius;
            let mut best: f32 = 0.0;
            for _ in 0..steps {
                self.step(code);
                best = best.max(rest_y - self.ball.pos.y);
            }
            best
        }
    }

    #[test]
    fn falling_ball_bounces_then_settles() {
        let mut w = World::resting(MID);
        w.ball.pos.y = FLOOR - 100.0;
        w.ball.grounded = false;
        let mut bounces = 0;
        for _ in 0..240 {
            let c = w.step("-");
            if c.floor && w.ball.vel.y < 0.0 {
                bounces += 1;
            }
        }
        assert!(bounces >= 1, "should bounce at least once");
        assert!(w.ball.grounded);
        assert_eq!(w.ball.vel.y, 0.0);
        assert_eq!(w.ball.pos.y, FLOOR - w.ball.radius);
    }

    #[test]
    fn jump_reaches_jump_height() {
        let p = BallParams::NORMAL;
        let mut w = World::resting(MID);
        w.step("J");
        let rise = w.max_rise("-", 60);
        assert!(
            (rise - p.jump_height).abs() < 1.0,
            "rise {rise} vs {}",
            p.jump_height
        );
    }

    #[test]
    fn jump_air_time_is_snappy() {
        let mut w = World::resting(MID);
        w.step("J");
        let mut air = 1;
        while !w.step("-").floor {
            air += 1;
            assert!(air < 120, "never landed");
        }
        // About 0.7 s, like the original's quick hop.
        assert!((36..=48).contains(&air), "air time {air} steps");
    }

    #[test]
    fn holding_jump_keeps_jumping() {
        let mut w = World::resting(MID);
        let mut takeoffs = 0;
        let mut was_rising = false;
        for _ in 0..180 {
            w.step("J");
            let rising = w.ball.vel.y < -200.0;
            if rising && !was_rising {
                takeoffs += 1;
            }
            was_rising = rising;
        }
        assert!(takeoffs >= 3, "only {takeoffs} jumps in 3 s");
    }

    #[test]
    fn buffered_jump_just_before_landing_works() {
        let mut w = World::resting(MID);
        w.step("J");
        // Wait until falling and close to the floor, then tap jump in the air.
        while !(w.ball.vel.y > 0.0 && FLOOR - w.ball.pos.y < 30.0) {
            w.step("-");
        }
        w.step("J");
        w.run("-", 20);
        let rise = w.max_rise("-", 30);
        assert!(rise > 30.0, "buffered jump did not happen (rise {rise})");
    }

    #[test]
    fn early_press_is_forgotten() {
        let mut w = World::resting(MID);
        w.step("J");
        w.run("-", 5);
        w.step("J"); // pressed while still rising: far too early
        w.run("-", 80);
        assert!(w.ball.grounded, "should have landed and stayed down");
    }

    #[test]
    fn no_double_jump_in_air() {
        let mut w = World::resting(MID);
        w.step("J");
        w.run("-", 10);
        let vy = w.ball.vel.y;
        w.step("J");
        assert!(w.ball.vel.y > vy, "second press in air changed velocity");
    }

    #[test]
    fn rolling_is_capped_at_max_speed() {
        let p = BallParams::NORMAL;
        let mut w = World::resting(MID);
        w.run("R", 60);
        assert!(
            (w.ball.vel.x - p.max_speed).abs() < 1e-3,
            "{}",
            w.ball.vel.x
        );
        w.run("L", 120);
        assert!(
            (w.ball.vel.x + p.max_speed).abs() < 1e-3,
            "{}",
            w.ball.vel.x
        );
    }

    #[test]
    fn reaches_top_speed_quickly() {
        let mut w = World::resting(MID);
        w.run("R", 15); // a quarter second
        assert!(w.ball.vel.x > 100.0, "{}", w.ball.vel.x);
    }

    #[test]
    fn friction_stops_the_ball() {
        let mut w = World::resting(MID);
        w.run("R", 60);
        w.run("-", 30); // half a second
        assert_eq!(w.ball.vel.x, 0.0);
    }

    #[test]
    fn extra_speed_bleeds_off_instead_of_snapping() {
        let mut w = World::resting(MID);
        w.ball.vel.x = 250.0; // e.g. from a slope or spring
        w.step("R");
        assert!(w.ball.vel.x > 240.0, "{}", w.ball.vel.x);
        w.run("R", 120);
        assert!((w.ball.vel.x - BallParams::NORMAL.max_speed).abs() < 1e-3);
    }

    #[test]
    fn air_control_is_weaker() {
        let mut ground = World::resting(MID);
        ground.run("R", 6);
        let mut air = World::resting(MID);
        air.step("J");
        air.run("R", 6);
        assert!(air.ball.vel.x < ground.ball.vel.x);
        assert!(air.ball.vel.x > 0.0);
    }

    #[test]
    fn walls_stop_and_bounce_back() {
        // Wall tile at column 12 (x 192..208).
        let mut w = World::new(map_with(&["............#"]), 100.0, FLOOR - 8.0);
        let mut hit = false;
        for _ in 0..120 {
            hit |= w.step("R").wall;
        }
        assert!(hit);
        assert!(w.ball.pos.x <= 192.0 - w.ball.radius + 1e-3);
        w.ball.vel.x = 120.0;
        w.step("-");
        assert!(w.ball.vel.x < 0.0, "should rebound off the wall");
    }

    #[test]
    fn coyote_jump_after_leaving_ground() {
        // Fake walking off a ledge: ball in the air, not from a jump.
        let mut w = World::resting(MID);
        w.ball.pos.y -= 20.0;
        w.ball.grounded = false;
        w.ball.air_ticks = 2;
        w.ball.update(InputFrame::from_code("J").unwrap(), true);
        assert!(w.ball.vel.y < -200.0, "coyote jump expected");
    }

    #[test]
    fn spin_matches_rolling() {
        let mut w = World::resting(MID);
        w.run("R", 60);
        let expected = w.ball.vel.x / w.ball.radius;
        use crate::sim::DT;
        assert!((w.ball.spin - expected).abs() < 1e-4);
        let a0 = w.ball.angle;
        w.step("R");
        let da = (w.ball.angle - a0).rem_euclid(std::f32::consts::TAU);
        assert!((da - expected * DT).abs() < 1e-4);
    }

    // ---- tile collision ----

    #[test]
    fn resting_on_tiles_is_perfectly_still() {
        let mut w = World::resting(MID);
        let y = w.ball.pos.y;
        for _ in 0..600 {
            let c = w.step("-");
            assert!(c.floor);
            assert!(w.ball.grounded);
            assert_eq!(w.ball.pos.y, y);
        }
        assert_eq!(y, FLOOR - 8.0);
    }

    #[test]
    fn rolling_across_tile_seams_never_bumps() {
        let mut w = World::resting(MID);
        for _ in 0..300 {
            w.step("R");
            assert_eq!(w.ball.pos.y, FLOOR - 8.0, "bumped at x={}", w.ball.pos.x);
            assert!(w.ball.grounded);
        }
    }

    #[test]
    fn ceiling_stops_a_jump() {
        // Ceiling tile row 15 (bottom at y=256) over the ball at x=24.
        let mut w = World::new(map_with(&["###", "...", "..."]), 24.0, FLOOR - 8.0);
        w.run("-", 3);
        let mut bonked = false;
        w.step("J");
        for _ in 0..30 {
            bonked |= w.step("-").ceiling;
            assert!(w.ball.pos.y - 8.0 >= 256.0 - 1e-3, "went into the ceiling");
        }
        assert!(bonked);
    }

    #[test]
    fn falling_along_a_wall_does_not_snag() {
        // Tall wall at column 5; ball pressed against it while falling.
        let mut w = World::new(map_with(&[".....#"; 18]), 5.0 * TILE - 8.0, 2.0 * TILE);
        for _ in 0..40 {
            w.step("R");
            assert!(!w.step("R").floor, "caught on a seam at y={}", w.ball.pos.y);
            if w.ball.pos.y > FLOOR - 40.0 {
                break;
            }
        }
        assert!(w.ball.vel.y > 200.0, "fall was slowed: vy={}", w.ball.vel.y);
    }

    #[test]
    fn one_tile_step_blocks_rolling() {
        let mut w = World::new(map_with(&["......#"]), 24.0, FLOOR - 8.0);
        w.run("-", 3);
        for _ in 0..120 {
            w.step("R");
            // Hitting the wall must not lift the ball off the floor.
            assert_eq!(w.ball.pos.y, FLOOR - 8.0, "lifted at x={}", w.ball.pos.x);
        }
        assert!(w.ball.pos.x <= 6.0 * TILE - 8.0 + 1e-3);
        assert!(w.ball.grounded);
    }

    #[test]
    fn jump_onto_a_one_tile_step() {
        let mut w = World::new(map_with(&["......#######"]), 24.0, FLOOR - 8.0);
        w.run("R", 30);
        w.step("RJ");
        w.run("R", 60);
        assert!(w.ball.pos.x > 7.0 * TILE);
        assert_eq!(
            w.ball.pos.y,
            FLOOR - TILE - 8.0,
            "should be on top of the step"
        );
    }

    #[test]
    fn landing_on_a_corner_pushes_off_the_edge() {
        // Single block, ball dropped onto its right corner.
        let mut w = World::new(
            map_with(&["....#....", "........."]),
            5.0 * TILE + 3.0,
            10.0 * TILE,
        );
        w.run("-", 60);
        assert!(
            w.ball.pos.x > 5.0 * TILE + 8.0,
            "should roll off: x={}",
            w.ball.pos.x
        );
        assert_eq!(w.ball.pos.y, FLOOR - 8.0);
    }

    #[test]
    fn roll_off_ledge_then_coyote_jump() {
        // Platform row 17 ends at column 5; floor is one tile lower after it.
        let mut w = World::new(map_with(&["#####......"]), 3.0 * TILE, FLOOR - TILE - 8.0);
        w.run("-", 3);
        assert!(w.ball.grounded);
        while w.ball.grounded || w.ball.air_ticks == 0 {
            w.step("R");
            assert!(w.ball.pos.x < 8.0 * TILE, "never left the ledge");
        }
        assert!(w.ball.air_ticks <= w.ball.params.coyote_ticks);
        w.step("RJ");
        assert!(w.ball.vel.y < -200.0, "coyote jump should work");
    }

    #[test]
    fn fits_through_a_one_tile_tunnel() {
        let mut w = World::new(
            map_with(&["...#########...", "..............."]),
            24.0,
            FLOOR - 8.0,
        );
        w.run("R", 180);
        assert!(w.ball.pos.x > 13.0 * TILE, "stuck at x={}", w.ball.pos.x);
    }

    #[test]
    fn very_fast_ball_does_not_tunnel_through_thin_walls() {
        // One-tile wall at column 10.
        let mut w = World::new(map_with(&["..........#....."; 6]), 5.0 * TILE, FLOOR - 8.0);
        w.run("-", 3);
        w.ball.vel.x = 5000.0;
        w.step("-");
        assert!(
            w.ball.pos.x <= 10.0 * TILE - 8.0 + 1e-3,
            "x={}",
            w.ball.pos.x
        );
    }

    #[test]
    fn very_fast_fall_does_not_tunnel_through_floor() {
        let mut w = World::resting(MID);
        w.ball.pos.y -= 100.0;
        w.ball.vel.y = 5000.0;
        w.step("-");
        assert!(w.ball.pos.y <= FLOOR - 8.0 + 1e-3, "y={}", w.ball.pos.y);
    }

    #[test]
    fn seam_contact_is_never_deeper_than_the_flat_contact() {
        let m = map_with(&["##"]);
        // Ball just right of the seam between (0,17) and (1,17): tile 0 only
        // offers its corner, tile 1 its flat top, which must win.
        let p = vec2(16.5, 17.0 * TILE - 5.0);
        let (corner_pen, _) = penetration(&m, 0, 17, p, 8.0).unwrap();
        let (flat_pen, n) = penetration(&m, 1, 17, p, 8.0).unwrap();
        assert!(flat_pen > corner_pen);
        assert_eq!(n, vec2(0.0, -1.0));
    }

    #[test]
    fn tile_with_only_hidden_edges_never_collides() {
        // Floor tile (6,18) under a block: all its edges are covered, so a
        // ball in the inside corner beside the block only touches real faces.
        let m = map_with(&["......#"]);
        let p = vec2(5.5 * TILE, 17.5 * TILE);
        assert_eq!(penetration(&m, 6, 18, p, 8.0), None);
    }

    // ---- slopes ----

    /// First slope column of the generated slope maps.
    const C0: usize = 6;

    /// Map with a slope rising to the right from x = C0*16 on the floor,
    /// `rise` tiles high, then a plateau. `kind` 1 = 45°, 2 = 1:2.
    fn rising_right(kind: usize, rise: usize) -> TileMap {
        let width = C0 + kind * rise + 8;
        let rows: Vec<String> = (0..rise)
            .map(|i| {
                let level = rise - 1 - i; // 0 = bottom row
                let start = C0 + kind * level;
                (0..width)
                    .map(|x| match (kind, x) {
                        _ if x < start => '.',
                        (1, x) if x == start => '/',
                        (2, x) if x == start => 'r',
                        (2, x) if x == start + 1 => 'R',
                        _ => '#',
                    })
                    .collect()
            })
            .collect();
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        map_with(&refs)
    }

    /// The same, mirrored: plateau on the left, slope falling to the right,
    /// reaching the floor at x = C0*16 + kind*rise*16.
    fn falling_right(kind: usize, rise: usize) -> TileMap {
        let width = C0 + kind * rise + 8;
        let rows: Vec<String> = (0..rise)
            .map(|i| {
                let start = C0 + kind * i;
                (0..width)
                    .map(|x| match (kind, x) {
                        _ if x < start => '#',
                        (1, x) if x == start => '\\',
                        (2, x) if x == start => 'L',
                        (2, x) if x == start + 1 => 'l',
                        _ => '.',
                    })
                    .collect()
            })
            .collect();
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        map_with(&refs)
    }

    /// Surface height of a `rising_right` slope at x.
    fn rising_surface(kind: usize, x: f32) -> f32 {
        FLOOR - (x - C0 as f32 * TILE) / kind as f32
    }

    /// Ball resting on a `rising_right` slope at x (centre pushed out along
    /// the normal).
    fn on_rising(kind: usize, rise: usize, x: f32) -> World {
        let n = vec2(-1.0, -(kind as f32)).normalize();
        let surface = vec2(x, rising_surface(kind, x));
        let p = surface + n * 8.0;
        let mut w = World::new(rising_right(kind, rise), p.x, p.y);
        w.ball.grounded = true;
        w.ball.ground_normal = n;
        w
    }

    /// Distance from the ball centre to a `rising_right` slope's line.
    fn gap_to_rising(kind: usize, b: &Ball) -> f32 {
        let k = kind as f32;
        // Line: x + k*y = C0*16 + k*FLOOR, normal (1, k)/len.
        (C0 as f32 * TILE + k * FLOOR - b.pos.x - k * b.pos.y) / (1.0 + k * k).sqrt()
    }

    #[test]
    fn rolls_down_a_45_slope_with_no_input() {
        let mut w = on_rising(1, 8, C0 as f32 * TILE + 100.0);
        let x0 = w.ball.pos.x;
        w.run("-", 30);
        assert!(w.ball.vel.x < -100.0, "vx {}", w.ball.vel.x);
        assert!(w.ball.pos.x < x0 - 20.0);
    }

    #[test]
    fn rolls_down_a_gentle_slope_more_slowly() {
        let mut steep = on_rising(1, 8, C0 as f32 * TILE + 100.0);
        let mut gentle = on_rising(2, 4, C0 as f32 * TILE + 100.0);
        steep.run("-", 30);
        gentle.run("-", 30);
        assert!(gentle.ball.vel.x < -5.0, "gentle slope should still roll");
        assert!(gentle.ball.vel.length() < steep.ball.vel.length());
    }

    #[test]
    fn rolling_down_hugs_the_surface_across_seams() {
        for kind in [1, 2] {
            let mut w = on_rising(kind, 6, C0 as f32 * TILE + 90.0);
            for _ in 0..60 {
                w.step("L");
                if w.ball.pos.x < C0 as f32 * TILE + 12.0 {
                    break; // reached the bottom
                }
                let gap = gap_to_rising(kind, &w.ball);
                assert!(
                    w.ball.grounded,
                    "kind {kind}: airborne at x={}",
                    w.ball.pos.x
                );
                assert!(
                    (gap - 8.0).abs() < 0.05,
                    "kind {kind}: gap {gap} at x={}",
                    w.ball.pos.x
                );
            }
        }
    }

    #[test]
    fn climbs_a_45_slope_and_crests_without_hopping() {
        let rise = 5;
        let mut w = World::new(rising_right(1, rise), 2.0 * TILE, FLOOR - 8.0);
        w.run("-", 3);
        let top = FLOOR - rise as f32 * TILE - 8.0;
        for i in 0..240 {
            w.step("R");
            assert!(w.ball.grounded, "airborne at step {i}, x={}", w.ball.pos.x);
            if w.ball.pos.x > (C0 + rise + 2) as f32 * TILE {
                assert_eq!(w.ball.pos.y, top, "should be on the plateau");
                return;
            }
        }
        panic!("never reached the plateau: x={}", w.ball.pos.x);
    }

    #[test]
    fn climbs_a_gentle_slope_faster_than_a_steep_one() {
        let time_to_top = |kind: usize| {
            let rise = 3;
            let mut w = World::new(
                rising_right(kind, rise),
                C0 as f32 * TILE - 8.0,
                FLOOR - 8.0,
            );
            let top_x = (C0 + kind * rise) as f32 * TILE;
            let mut steps = 0;
            while w.ball.pos.x < top_x {
                w.step("R");
                steps += 1;
                assert!(steps < 600, "kind {kind} never reached the top");
            }
            steps as f32 / (kind * rise) as f32 // steps per tile of run
        };
        assert!(time_to_top(2) < time_to_top(1));
    }

    #[test]
    fn follows_a_downslope_off_a_plateau() {
        for kind in [1, 2] {
            let rise = 4;
            let mut w = World::new(
                falling_right(kind, rise),
                2.0 * TILE,
                FLOOR - rise as f32 * TILE - 8.0,
            );
            w.run("-", 3);
            let bottom_x = (C0 + kind * rise) as f32 * TILE;
            for i in 0..300 {
                w.step("R");
                assert!(
                    w.ball.grounded,
                    "kind {kind}: airborne at step {i}, x={}",
                    w.ball.pos.x
                );
                if w.ball.pos.x > bottom_x + 32.0 {
                    assert_eq!(w.ball.pos.y, FLOOR - 8.0);
                    break;
                }
            }
        }
    }

    #[test]
    fn speed_is_kept_when_a_slope_meets_flat_ground() {
        let mut w = on_rising(1, 8, C0 as f32 * TILE + 110.0);
        let mut last_speed = 0.0;
        for _ in 0..120 {
            w.step("-");
            if w.ball.ground_normal == vec2(0.0, -1.0) {
                let speed = w.ball.vel.length();
                assert!(
                    speed > last_speed * 0.97,
                    "lost speed: {last_speed} -> {speed}"
                );
                assert!(speed > 150.0, "should arrive fast: {speed}");
                return;
            }
            last_speed = w.ball.vel.length();
        }
        panic!("never reached flat ground");
    }

    #[test]
    fn rolling_speed_is_capped_downhill() {
        let mut w = on_rising(1, 16, C0 as f32 * TILE + 250.0);
        let mut fastest: f32 = 0.0;
        for _ in 0..90 {
            w.step("L");
            fastest = fastest.max(w.ball.vel.length());
        }
        assert!(
            fastest <= BallParams::NORMAL.max_roll_speed + 1.0,
            "{fastest}"
        );
    }

    #[test]
    fn jumping_on_a_slope_goes_straight_up() {
        let mut w = on_rising(1, 8, C0 as f32 * TILE + 60.0);
        w.step("J");
        assert!(w.ball.vel.y < -250.0, "vy {}", w.ball.vel.y);
        assert!(!w.ball.grounded);
    }

    #[test]
    fn slope_underside_is_a_ceiling() {
        // A lone 45° tile floating in the air; jump into its flat bottom.
        let mut w = World::new(
            map_with(&["..../....", ".........", "........."]),
            4.5 * TILE,
            FLOOR - 8.0,
        );
        w.run("-", 3);
        w.step("J");
        let mut bonked = false;
        for _ in 0..30 {
            bonked |= w.step("-").ceiling;
        }
        assert!(bonked);
    }
}
