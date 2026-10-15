//! How deep the ball is in water.

use super::ball::Ball;
use super::map::{TILE, TileMap, flags};

/// Fraction (0..1) of the ball's height that is inside water cells, looking
/// at the column under its centre (the ball is one tile wide).
pub fn submersion(ball: &Ball, map: &TileMap) -> f32 {
    let r = ball.radius;
    let (top, bottom) = (ball.pos.y - r, ball.pos.y + r);
    let col = (ball.pos.x / TILE).floor() as i32;
    let (row0, row1) = ((top / TILE).floor() as i32, (bottom / TILE).floor() as i32);
    let mut wet = 0.0;
    for row in row0..=row1 {
        if map.flags(col, row) & flags::WATER == 0 {
            continue;
        }
        let (cell_top, cell_bottom) = (row as f32 * TILE, (row + 1) as f32 * TILE);
        wet += (bottom.min(cell_bottom) - top.max(cell_top)).max(0.0);
    }
    (wet / (2.0 * r)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ball::ball_at;

    fn pool() -> TileMap {
        // Water in rows 1-2 of column 1.
        TileMap::from_ascii("o..\n.~.\n.~.\n###").unwrap()
    }

    #[test]
    fn depth_fractions() {
        let m = pool();
        assert_eq!(submersion(&ball_at(24.0, 0.0), &m), 0.0, "above the water");
        assert_eq!(
            submersion(&ball_at(24.0, 16.0), &m),
            0.5,
            "centre on the surface"
        );
        assert_eq!(submersion(&ball_at(24.0, 32.0), &m), 1.0, "fully under");
        assert_eq!(submersion(&ball_at(24.0, 20.0), &m), 0.75);
        assert_eq!(
            submersion(&ball_at(8.0, 32.0), &m),
            0.0,
            "next column is dry"
        );
    }
}
