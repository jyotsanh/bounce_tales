//! The player ball.

use macroquad::math::Vec2;

/// Radius of the normal ball in pixels (world units are pixels, y points down).
pub const BALL_RADIUS: f32 = 8.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Ball {
    pub pos: Vec2,
    /// Position at the start of the last step, for render interpolation.
    pub prev_pos: Vec2,
    /// Pixels per second.
    pub vel: Vec2,
    pub radius: f32,
}

impl Ball {
    pub fn new(pos: Vec2) -> Ball {
        Ball {
            pos,
            prev_pos: pos,
            vel: Vec2::ZERO,
            radius: BALL_RADIUS,
        }
    }

    /// Position blended between the last two steps (`alpha` in 0..1).
    pub fn render_pos(&self, alpha: f32) -> Vec2 {
        self.prev_pos.lerp(self.pos, alpha)
    }
}
