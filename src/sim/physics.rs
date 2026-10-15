//! Collision between the ball and the tile map.
//!
//! The ball is a circle; solid tiles are squares. Each step the ball's motion
//! is split into sub-steps no longer than half its radius, so even very fast
//! balls cannot pass through a one-tile wall. After each sub-step the ball is
//! pushed out of every tile it overlaps.
//!
//! "Ghost edges": where two solid tiles meet, the shared face is not a real
//! surface. Rolling across a floor of separate tiles must not catch on the
//! corners between them, so a tile's box is treated as extending into any
//! solid neighbour (see [`closest_point`]).

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::Ball;
use super::map::{TILE, TileMap};

/// Contact normals with `y` below this count as floor (up to ~53° slopes).
pub const FLOOR_NORMAL_Y: f32 = -0.6;
/// Contact normals with `y` above this count as ceiling.
pub const CEILING_NORMAL_Y: f32 = 0.6;
/// Most sub-steps per step (limits cost of absurd speeds).
const MAX_SUBSTEPS: u32 = 16;
/// Push-out passes per sub-step (handles corners touching two tiles).
const RESOLVE_PASSES: u32 = 4;

/// What the ball touched during one step.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Contacts {
    pub floor: bool,
    pub wall: bool,
    pub ceiling: bool,
    /// Speed into the floor at impact, px/s (0 if no floor hit).
    pub impact_speed: f32,
    /// Normal of the hardest floor hit (points away from the floor).
    pub floor_normal: Vec2,
}

/// Moves the ball by its velocity for one step, colliding with the map, then
/// applies landing/bounce rules.
pub fn move_and_collide(ball: &mut Ball, map: &TileMap) -> Contacts {
    let distance = ball.vel.length() * DT;
    let substeps = ((distance / (ball.radius * 0.5)).ceil() as u32).clamp(1, MAX_SUBSTEPS);
    let mut c = Contacts::default();
    for _ in 0..substeps {
        // Velocity can change mid-step (wall hit), so re-read it each time.
        ball.pos += ball.vel * (DT / substeps as f32);
        for _ in 0..RESOLVE_PASSES {
            if !resolve_deepest(ball, map, &mut c) {
                break;
            }
        }
    }
    land(ball, c);
    c
}

/// Pushes the ball out of the tile it overlaps most and updates its
/// velocity. Returns false if it overlaps nothing.
fn resolve_deepest(ball: &mut Ball, map: &TileMap, c: &mut Contacts) -> bool {
    let p = ball.pos;
    let r = ball.radius;
    let (x0, y0) = TileMap::cell_at(p - Vec2::splat(r));
    let (x1, y1) = TileMap::cell_at(p + Vec2::splat(r));

    let mut best: Option<(f32, Vec2)> = None; // (penetration, normal)
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            if !map.is_solid(tx, ty) {
                continue;
            }
            if let Some((pen, n)) = penetration(map, tx, ty, p, r)
                && best.is_none_or(|(b, _)| pen > b)
            {
                best = Some((pen, n));
            }
        }
    }
    let Some((pen, n)) = best else {
        return false;
    };
    ball.pos += n * pen;
    respond(ball, n, c);
    true
}

/// How far a circle at `p` overlaps tile (tx, ty), and the direction to push
/// it out.
fn penetration(map: &TileMap, tx: i32, ty: i32, p: Vec2, r: f32) -> Option<(f32, Vec2)> {
    let q = closest_point(map, tx, ty, p)?;
    let d = p - q;
    let dist = d.length();
    if dist >= r {
        return None;
    }
    if dist > 1e-4 {
        return Some((r - dist, d / dist));
    }
    // Centre inside the tile (only after a deep hit): leave through the
    // nearest face that is open to empty space.
    let min = vec2(tx as f32, ty as f32) * TILE;
    let max = min + Vec2::splat(TILE);
    let faces = [
        (p.y - min.y, vec2(0.0, -1.0), (tx, ty - 1)),
        (max.y - p.y, vec2(0.0, 1.0), (tx, ty + 1)),
        (p.x - min.x, vec2(-1.0, 0.0), (tx - 1, ty)),
        (max.x - p.x, vec2(1.0, 0.0), (tx + 1, ty)),
    ];
    faces
        .iter()
        .filter(|(_, _, (nx, ny))| !map.is_solid(*nx, *ny))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|&(depth, n, _)| (depth + r, n))
        .or(Some((r, vec2(0.0, -1.0))))
}

/// Closest point on tile (tx, ty) to `p`, treating the tile as extending into
/// solid neighbours so their shared faces and corners never push the ball.
///
/// Returns `None` when `p` sits in the diagonal cell of an inside corner
/// (both side neighbours solid, the diagonal cell empty): the two neighbours
/// own that contact, and extending both ways would fill the empty corner.
fn closest_point(map: &TileMap, tx: i32, ty: i32, p: Vec2) -> Option<Vec2> {
    let min = vec2(tx as f32, ty as f32) * TILE;
    let max = min + Vec2::splat(TILE);
    let mut q = p.clamp(min, max);
    let side_x = if p.x < min.x {
        Some(tx - 1)
    } else if p.x > max.x {
        Some(tx + 1)
    } else {
        None
    };
    let side_y = if p.y < min.y {
        Some(ty - 1)
    } else if p.y > max.y {
        Some(ty + 1)
    } else {
        None
    };
    let extend_x = side_x.is_some_and(|nx| map.is_solid(nx, ty));
    let extend_y = side_y.is_some_and(|ny| map.is_solid(tx, ny));
    if extend_x && extend_y {
        let (nx, ny) = (side_x.unwrap(), side_y.unwrap());
        if !map.is_solid(nx, ny) {
            return None;
        }
    }
    if extend_x {
        q.x = p.x;
    }
    if extend_y {
        q.y = p.y;
    }
    Some(q)
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
fn land(ball: &mut Ball, c: Contacts) {
    let p = ball.params;
    if c.floor {
        ball.jumping = false;
        ball.air_ticks = 0;
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
        w.ball.vel.x = 300.0; // e.g. from a spring later on
        w.step("R");
        assert!(w.ball.vel.x > 290.0, "{}", w.ball.vel.x);
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
    fn closest_point_ignores_faces_shared_with_solid_tiles() {
        let m = map_with(&["##"]);
        // Ball over the seam between (0,17) and (1,17), slightly to the
        // right: tile 0's right face is shared, so its closest point is
        // straight below the ball, not its corner.
        let p = vec2(16.5, 17.0 * TILE - 5.0);
        assert_eq!(closest_point(&m, 0, 17, p), Some(vec2(16.5, 17.0 * TILE)));
    }

    #[test]
    fn inside_corner_cell_is_not_filled() {
        // Floor row 18 with a block on top at column 6: the empty cell (5,17)
        // is an inside corner of floor tile (6,18).
        let m = map_with(&["......#"]);
        let p = vec2(5.5 * TILE, 17.5 * TILE);
        assert_eq!(closest_point(&m, 6, 18, p), None);
    }
}
