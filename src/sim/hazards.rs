//! Things that kill the ball: spike cells and falling out of the level.

use macroquad::math::{Vec2, vec2};

use super::ball::Ball;
use super::map::{TILE, TileMap, flags};

/// Why the ball died.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Spikes,
    /// Fell out of the bottom of the level.
    Pit,
    /// Touched an enemy the wrong way.
    Enemy,
    /// Hit by a falling stalactite.
    Rock,
    /// Under a crusher.
    Crushed,
}

/// Danger box of a spike cell, relative to the cell's top-left. A little
/// smaller than the drawn spikes (x 1..15, 12 px tall) so a graze of a
/// tip is forgiven.
const SPIKE_UP: (Vec2, Vec2) = (Vec2::new(2.0, 7.0), Vec2::new(14.0, 16.0));
const SPIKE_DOWN: (Vec2, Vec2) = (Vec2::new(2.0, 0.0), Vec2::new(14.0, 9.0));

fn circle_hits_box(centre: Vec2, r: f32, min: Vec2, max: Vec2) -> bool {
    centre.distance(centre.clamp(min, max)) < r
}

/// What (if anything) kills the ball where it is now.
pub fn check(ball: &Ball, map: &TileMap) -> Option<Cause> {
    // Fully below the level: gone for good.
    if ball.pos.y - ball.radius > map.pixel_size().y {
        return Some(Cause::Pit);
    }
    let r = ball.radius;
    let (x0, y0) = TileMap::cell_at(ball.pos - Vec2::splat(r));
    let (x1, y1) = TileMap::cell_at(ball.pos + Vec2::splat(r));
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let f = map.flags(tx, ty);
            if f == 0 {
                continue;
            }
            let origin = vec2(tx as f32, ty as f32) * TILE;
            for (bit, (min, max)) in [(flags::SPIKE, SPIKE_UP), (flags::SPIKE_DOWN, SPIKE_DOWN)] {
                if f & bit != 0 && circle_hits_box(ball.pos, r, origin + min, origin + max) {
                    return Some(Cause::Spikes);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ball::ball_at;

    fn map() -> TileMap {
        // Spike up at (1,2), spike down at (3,0), floor below.
        TileMap::from_ascii("o..v.\n.....\n.^...\n#####").unwrap()
    }

    #[test]
    fn touching_spikes_kills() {
        let m = map();
        // Resting on the floor right on top of the spike cell.
        assert_eq!(check(&ball_at(24.0, 40.0), &m), Some(Cause::Spikes));
        // Hanging spikes: ball just under them.
        assert_eq!(check(&ball_at(56.0, 14.0), &m), Some(Cause::Spikes));
    }

    #[test]
    fn grazing_beside_spikes_is_forgiven() {
        let m = map();
        // Next to the spike cell, edges touching the cell but not the box.
        assert_eq!(check(&ball_at(24.0 - 14.5, 40.0), &m), None);
        // Above the spike tips (box starts 7 px into the cell).
        assert_eq!(check(&ball_at(24.0, 32.0 + 7.0 - 8.5), &m), None);
        assert_eq!(check(&ball_at(72.0, 40.0), &m), None);
    }

    #[test]
    fn falling_out_of_the_level_is_a_pit() {
        let m = map();
        let h = m.pixel_size().y;
        assert_eq!(
            check(&ball_at(40.0, h + 7.0), &m),
            None,
            "still partly inside"
        );
        assert_eq!(check(&ball_at(40.0, h + 9.0), &m), Some(Cause::Pit));
    }
}
