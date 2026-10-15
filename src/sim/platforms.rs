//! Moving platforms: they follow a path, carry a ball that rides them, and
//! only collide from above (one-way), so they can never crush the ball.
//!
//! Riding works in the platform's frame: while riding, the ball's own
//! velocity is relative to the platform and the platform's movement is
//! added each step. Landing subtracts the platform's velocity; leaving
//! (jump or rolling off) adds it back, so momentum carries over.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::Ball;

/// Platform thickness, px (drawn as one row of 16 px pieces).
pub const PLATFORM_H: f32 = 16.0;
/// Defaults for Tiled properties.
pub const DEFAULT_WIDTH_TILES: u32 = 3;
pub const DEFAULT_SPEED: f32 = 40.0;
pub const DEFAULT_WAIT_SECONDS: f32 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct Platform {
    /// Path of the platform's top-centre, level px.
    pub path: Vec<Vec2>,
    /// Loop around (closed path) instead of going back and forth.
    pub looped: bool,
    /// px/s along the path.
    pub speed: f32,
    /// Steps to pause at each end (back-and-forth paths only).
    pub wait: u32,
    pub width: f32,
    /// Distance travelled along the path.
    along: f32,
    forward: bool,
    waiting: u32,
    pub top_centre: Vec2,
    pub prev_top_centre: Vec2,
    /// Velocity during the last step, px/s.
    pub vel: Vec2,
}

impl Platform {
    pub fn new(path: Vec<Vec2>, looped: bool, speed: f32, wait: u32, width: f32) -> Platform {
        Platform::starting_at(path, looped, speed, wait, width, 0.0)
    }

    /// A platform that starts `along` px into its path (wheel cars are
    /// spread out this way).
    pub fn starting_at(
        path: Vec<Vec2>,
        looped: bool,
        speed: f32,
        wait: u32,
        width: f32,
        along: f32,
    ) -> Platform {
        let mut p = Platform {
            path,
            looped,
            speed,
            wait,
            width,
            along,
            forward: true,
            waiting: 0,
            top_centre: Vec2::ZERO,
            prev_top_centre: Vec2::ZERO,
            vel: Vec2::ZERO,
        };
        p.top_centre = p.point_at(along);
        p.prev_top_centre = p.top_centre;
        p
    }

    fn segments(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        let n = self.path.len();
        let count = if self.looped { n } else { n - 1 };
        (0..count).map(move |i| (self.path[i], self.path[(i + 1) % n]))
    }

    pub fn length(&self) -> f32 {
        self.segments().map(|(a, b)| a.distance(b)).sum()
    }

    /// Point at `s` px along the path.
    pub fn point_at(&self, mut s: f32) -> Vec2 {
        for (a, b) in self.segments() {
            let len = a.distance(b);
            if s <= len {
                return if len > 0.0 { a.lerp(b, s / len) } else { a };
            }
            s -= len;
        }
        *self.path.last().unwrap()
    }

    /// Moves one step along the path.
    pub fn advance(&mut self) {
        self.prev_top_centre = self.top_centre;
        if self.waiting > 0 {
            self.waiting -= 1;
            self.vel = Vec2::ZERO;
            return;
        }
        let len = self.length();
        let step = self.speed * DT;
        if self.looped {
            self.along = (self.along + step).rem_euclid(len.max(f32::EPSILON));
        } else if self.forward {
            self.along += step;
            if self.along >= len {
                self.along = len;
                self.forward = false;
                self.waiting = self.wait;
            }
        } else {
            self.along -= step;
            if self.along <= 0.0 {
                self.along = 0.0;
                self.forward = true;
                self.waiting = self.wait;
            }
        }
        self.top_centre = self.point_at(self.along);
        self.vel = (self.top_centre - self.prev_top_centre) / DT;
    }

    pub fn delta(&self) -> Vec2 {
        self.top_centre - self.prev_top_centre
    }

    /// Top-left corner (current position).
    pub fn top_left(&self) -> Vec2 {
        self.top_centre - vec2(self.width / 2.0, 0.0)
    }

    /// Is the ball's centre over the platform (with a little give at the
    /// ends so the ball doesn't slip off the very corner)?
    fn under_ball(&self, ball: &Ball) -> bool {
        let give = ball.radius * 0.5;
        let half = self.width / 2.0 + give;
        (ball.pos.x - self.top_centre.x).abs() <= half
    }

    /// Keeps a riding ball on top. Returns false if it has left (it then
    /// gets the platform's velocity added).
    pub fn keep_rider(&self, ball: &mut Ball) -> bool {
        let top = self.top_centre.y;
        let bottom = ball.pos.y + ball.radius;
        let on =
            !ball.jumping && self.under_ball(ball) && bottom >= top - 3.0 && bottom <= top + 8.0;
        if on {
            stand(ball, top);
        } else {
            ball.vel += self.vel;
        }
        on
    }

    /// Lands a falling ball whose bottom crossed the platform's top this
    /// step. Returns the impact speed (relative to the platform) if it
    /// landed; a hard landing bounces instead of riding (`grounded` false).
    pub fn try_land(&self, ball: &mut Ball, prev_bottom: f32) -> Option<f32> {
        let (prev_top, top) = (self.prev_top_centre.y, self.top_centre.y);
        let bottom = ball.pos.y + ball.radius;
        let relative = ball.vel.y - self.vel.y;
        if relative < 0.0 || prev_bottom > prev_top + 1.0 || bottom < top || !self.under_ball(ball)
        {
            return None;
        }
        let p = ball.params;
        if relative > p.min_bounce_speed {
            ball.pos.y = top - ball.radius;
            ball.vel.y = self.vel.y - relative * p.restitution;
            ball.grounded = false;
            ball.jumping = false;
            ball.air_ticks = 0;
        } else {
            ball.vel.x -= self.vel.x; // into the platform's frame
            stand(ball, top);
        }
        Some(relative)
    }
}

fn stand(ball: &mut Ball, top: f32) {
    ball.pos.y = top - ball.radius;
    ball.vel.y = 0.0;
    ball.grounded = true;
    ball.jumping = false;
    ball.air_ticks = 0;
    ball.ground_normal = vec2(0.0, -1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> Platform {
        Platform::new(
            vec![vec2(0.0, 100.0), vec2(60.0, 100.0)],
            false,
            60.0,
            30,
            48.0,
        )
    }

    #[test]
    fn goes_back_and_forth_with_a_pause() {
        let mut p = line();
        for _ in 0..60 {
            p.advance();
        }
        assert!((p.top_centre.x - 60.0).abs() < 1e-3, "{}", p.top_centre.x);
        for _ in 0..30 {
            p.advance();
            assert_eq!(p.vel, Vec2::ZERO, "waiting at the end");
        }
        p.advance();
        assert!(p.vel.x < 0.0, "heading back");
        for _ in 0..60 {
            p.advance();
        }
        assert!(p.top_centre.x.abs() < 1e-3);
    }

    #[test]
    fn closed_paths_loop() {
        let square = vec![
            vec2(0.0, 0.0),
            vec2(30.0, 0.0),
            vec2(30.0, 30.0),
            vec2(0.0, 30.0),
        ];
        let mut p = Platform::new(square, true, 60.0, 30, 48.0);
        assert_eq!(p.length(), 120.0);
        for _ in 0..120 {
            p.advance(); // two seconds = 120 px = once around
        }
        assert!(
            p.top_centre.distance(Vec2::ZERO) < 1e-2,
            "{:?}",
            p.top_centre
        );
        assert_ne!(p.vel, Vec2::ZERO, "loops never pause");
    }

    #[test]
    fn point_at_walks_segments() {
        let p = Platform::new(
            vec![vec2(0.0, 0.0), vec2(10.0, 0.0), vec2(10.0, 10.0)],
            false,
            1.0,
            0,
            16.0,
        );
        assert_eq!(p.point_at(5.0), vec2(5.0, 0.0));
        assert_eq!(p.point_at(15.0), vec2(10.0, 5.0));
        assert_eq!(p.point_at(99.0), vec2(10.0, 10.0));
    }
}
