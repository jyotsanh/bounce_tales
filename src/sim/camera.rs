//! The view onto the level. Lives in the sim (not the renderer) so it is
//! deterministic, checkable from replays and unit-testable; the renderer only
//! blends `prev_pos` → `pos`.
//!
//! Behaviour:
//! - Horizontal: the camera keeps a focus point inside a narrow dead zone
//!   around the screen centre. The focus sits `look` px ahead of the ball in
//!   the direction it is rolling, so you see more of where you are going.
//!   `look` eases over at `LOOK_RATE` when the ball turns, so turning pans
//!   smoothly instead of jumping.
//! - Vertical: on the ground the camera eases so the ball sits at `FOCUS_Y`.
//!   In the air it holds still (jumps don't bob the view) unless the ball
//!   leaves the dead zone above or below.
//! - Always clamped to the level; levels smaller than the view are centred.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::{Ball, approach};
use super::map::TileMap;

/// Size of the view in pixels (the original Nokia screen).
pub const VIEW_W: f32 = 240.0;
pub const VIEW_H: f32 = 320.0;

/// How far ahead of the ball the camera looks while rolling, px.
pub const LOOK_AHEAD: f32 = 36.0;
/// How fast the look-ahead swings when the ball turns, px/s.
pub const LOOK_RATE: f32 = 90.0;
/// Ball speed needed before the look-ahead picks a direction, px/s.
const LOOK_MIN_SPEED: f32 = 30.0;
/// Half-width of the horizontal dead zone, px.
pub const DEAD_X: f32 = 8.0;
/// Where the ball rests on screen vertically (from the top), px. A bit
/// below the middle so more of what is above is visible.
pub const FOCUS_Y: f32 = VIEW_H * 0.6;
/// How far above `FOCUS_Y` the ball may go in the air before the camera
/// follows, px. A full jump (52 px) stays inside.
pub const DEAD_UP: f32 = 64.0;
/// How far below `FOCUS_Y` the ball may go before the camera follows, px.
pub const DEAD_DOWN: f32 = 24.0;
/// Fastest vertical re-centring while the ball is on the ground, px/s.
pub const GROUND_FOLLOW: f32 = 240.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    /// Top-left corner of the view in level pixels.
    pub pos: Vec2,
    /// `pos` at the start of the last step, for render interpolation.
    pub prev_pos: Vec2,
    /// Current look-ahead offset, px (negative = looking left).
    pub look: f32,
}

impl Camera {
    /// A camera centred on the ball with no look-ahead (level start,
    /// respawn).
    pub fn snapped_to(ball: &Ball, map: &TileMap) -> Camera {
        let pos = clamp_to_level(vec2(ball.pos.x - VIEW_W / 2.0, ball.pos.y - FOCUS_Y), map);
        Camera {
            pos,
            prev_pos: pos,
            look: 0.0,
        }
    }

    pub fn update(&mut self, ball: &Ball, map: &TileMap) {
        self.prev_pos = self.pos;

        // Look-ahead: swing toward the rolling direction; keep it when slow
        // so stopping does not pan back.
        let want = if ball.vel.x > LOOK_MIN_SPEED {
            LOOK_AHEAD
        } else if ball.vel.x < -LOOK_MIN_SPEED {
            -LOOK_AHEAD
        } else {
            self.look
        };
        self.look = approach(self.look, want, LOOK_RATE * DT);

        // Horizontal dead zone around the screen centre.
        let focus_x = ball.pos.x + self.look;
        let centre_x = self.pos.x + VIEW_W / 2.0;
        if focus_x > centre_x + DEAD_X {
            self.pos.x = focus_x - DEAD_X - VIEW_W / 2.0;
        } else if focus_x < centre_x - DEAD_X {
            self.pos.x = focus_x + DEAD_X - VIEW_W / 2.0;
        }

        // Vertical: ease to the focus line on the ground...
        if ball.grounded {
            let target = ball.pos.y - FOCUS_Y;
            self.pos.y = approach(self.pos.y, target, GROUND_FOLLOW * DT);
        }
        // ...and never let the ball leave the dead zone.
        let screen_y = ball.pos.y - self.pos.y;
        if screen_y < FOCUS_Y - DEAD_UP {
            self.pos.y = ball.pos.y - (FOCUS_Y - DEAD_UP);
        } else if screen_y > FOCUS_Y + DEAD_DOWN {
            self.pos.y = ball.pos.y - (FOCUS_Y + DEAD_DOWN);
        }

        self.pos = clamp_to_level(self.pos, map);
    }

    /// Camera position blended between the last two steps, rounded to whole
    /// pixels so tiles stay crisp.
    pub fn render_pos(&self, alpha: f32) -> Vec2 {
        self.prev_pos.lerp(self.pos, alpha).round()
    }
}

/// Keeps the view inside the level. On an axis where the level is smaller
/// than the view, the level is centred.
fn clamp_to_level(pos: Vec2, map: &TileMap) -> Vec2 {
    let size = map.pixel_size();
    let axis = |p: f32, level: f32, view: f32| {
        if level <= view {
            (level - view) / 2.0
        } else {
            p.clamp(0.0, level - view)
        }
    };
    vec2(axis(pos.x, size.x, VIEW_W), axis(pos.y, size.y, VIEW_H))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ball::ball_at;

    /// Open map `w` x `h` tiles with a floor along the bottom.
    fn open_map(w: usize, h: usize) -> TileMap {
        let mut text = String::new();
        for y in 0..h {
            let row = if y >= h - 2 {
                "#".repeat(w)
            } else if y == h - 3 {
                "o".to_owned() + &".".repeat(w - 1)
            } else {
                ".".repeat(w)
            };
            text.push_str(&row);
            text.push('\n');
        }
        TileMap::from_ascii(&text).unwrap()
    }

    fn grounded_ball(x: f32, y: f32) -> Ball {
        let mut b = ball_at(x, y);
        b.grounded = true;
        b
    }

    #[test]
    fn snap_centres_the_ball_and_clamps() {
        let map = open_map(100, 100);
        let b = ball_at(800.0, 800.0);
        let cam = Camera::snapped_to(&b, &map);
        assert_eq!(cam.pos, vec2(800.0 - VIEW_W / 2.0, 800.0 - FOCUS_Y));

        let corner = Camera::snapped_to(&ball_at(10.0, 10.0), &map);
        assert_eq!(corner.pos, Vec2::ZERO);
        let far = Camera::snapped_to(&ball_at(1590.0, 1590.0), &map);
        assert_eq!(far.pos, vec2(1600.0 - VIEW_W, 1600.0 - VIEW_H));
    }

    #[test]
    fn small_levels_are_centred() {
        let map = open_map(10, 10); // 160 x 160, smaller than the view
        let cam = Camera::snapped_to(&ball_at(80.0, 80.0), &map);
        assert_eq!(
            cam.pos,
            vec2((160.0 - VIEW_W) / 2.0, (160.0 - VIEW_H) / 2.0)
        );
    }

    #[test]
    fn exact_screen_sized_level_never_scrolls() {
        let map = open_map(15, 20);
        let mut b = grounded_ball(120.0, 280.0);
        let mut cam = Camera::snapped_to(&b, &map);
        for i in 0..200 {
            b.pos.x = 20.0 + i as f32;
            b.vel.x = 60.0;
            cam.update(&b, &map);
            assert_eq!(cam.pos, Vec2::ZERO);
        }
    }

    #[test]
    fn rolling_right_leads_with_look_ahead() {
        let map = open_map(200, 30);
        let mut b = grounded_ball(400.0, 440.0);
        let mut cam = Camera::snapped_to(&b, &map);
        b.vel.x = 120.0;
        for _ in 0..180 {
            b.pos.x += b.vel.x * DT;
            cam.update(&b, &map);
            let screen_x = b.pos.x - cam.pos.x;
            // The ball never leaves the screen's middle band.
            assert!((80.0..=160.0).contains(&screen_x), "screen x {screen_x}");
        }
        assert_eq!(cam.look, LOOK_AHEAD);
        let screen_x = b.pos.x - cam.pos.x;
        assert_eq!(screen_x, VIEW_W / 2.0 - LOOK_AHEAD + DEAD_X);
    }

    #[test]
    fn turning_pans_smoothly() {
        let map = open_map(200, 30);
        let mut b = grounded_ball(1600.0, 440.0);
        let mut cam = Camera::snapped_to(&b, &map);
        b.vel.x = 120.0;
        for _ in 0..120 {
            b.pos.x += b.vel.x * DT;
            cam.update(&b, &map);
        }
        b.vel.x = -120.0;
        for _ in 0..120 {
            let before = cam.pos.x;
            b.pos.x += b.vel.x * DT;
            cam.update(&b, &map);
            // Never jumps more than ball speed + look rate per step.
            let moved = (cam.pos.x - before).abs();
            assert!(moved <= (120.0 + LOOK_RATE) * DT + 1e-3, "jumped {moved}");
        }
        assert_eq!(cam.look, -LOOK_AHEAD);
    }

    #[test]
    fn stopping_does_not_pan_back() {
        let map = open_map(200, 30);
        let mut b = grounded_ball(1600.0, 440.0);
        let mut cam = Camera::snapped_to(&b, &map);
        b.vel.x = 120.0;
        for _ in 0..120 {
            b.pos.x += b.vel.x * DT;
            cam.update(&b, &map);
        }
        let stopped_at = cam.pos;
        b.vel.x = 0.0;
        for _ in 0..120 {
            cam.update(&b, &map);
        }
        assert_eq!(cam.pos, stopped_at);
    }

    #[test]
    fn a_jump_does_not_move_the_camera_vertically() {
        let map = open_map(100, 100);
        let rest_y = 800.0;
        let mut b = grounded_ball(800.0, rest_y);
        let mut cam = Camera::snapped_to(&b, &map);
        let y0 = cam.pos.y;
        b.grounded = false;
        for i in 0..40 {
            // Roughly a full jump arc, 52 px high.
            let t = i as f32 / 40.0;
            b.pos.y = rest_y - 52.0 * 4.0 * t * (1.0 - t);
            cam.update(&b, &map);
            assert_eq!(cam.pos.y, y0);
        }
    }

    #[test]
    fn landing_higher_recentres_gradually() {
        let map = open_map(100, 100);
        let mut b = grounded_ball(800.0, 800.0);
        let mut cam = Camera::snapped_to(&b, &map);
        b.pos.y = 768.0; // landed on a platform two tiles up
        let mut steps = 0;
        while cam.pos.y != b.pos.y - FOCUS_Y {
            let before = cam.pos.y;
            cam.update(&b, &map);
            assert!((before - cam.pos.y).abs() <= GROUND_FOLLOW * DT + 1e-3);
            steps += 1;
            assert!(steps < 100, "never settled");
        }
        assert!(steps > 1, "should ease, not snap");
    }

    #[test]
    fn falling_ball_is_followed() {
        let map = open_map(100, 200);
        let mut b = ball_at(800.0, 400.0);
        let mut cam = Camera::snapped_to(&b, &map);
        for _ in 0..200 {
            b.pos.y += 7.0; // max fall speed
            cam.update(&b, &map);
            let screen_y = b.pos.y - cam.pos.y;
            assert!(
                screen_y <= FOCUS_Y + DEAD_DOWN + 1e-3,
                "ball fell off screen"
            );
        }
    }

    #[test]
    fn render_pos_is_whole_pixels() {
        let cam = Camera {
            pos: vec2(10.6, 3.2),
            prev_pos: vec2(10.0, 3.0),
            look: 0.0,
        };
        assert_eq!(cam.render_pos(1.0), vec2(11.0, 3.0));
        assert_eq!(cam.render_pos(0.0), vec2(10.0, 3.0));
    }
}
