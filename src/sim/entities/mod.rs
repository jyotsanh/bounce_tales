//! Interactive level objects. Each Tiled object type maps to one
//! [`EntityKind`]; [`build`] turns an [`ObjectSpec`] into an [`Entity`].
//!
//! To add a new kind (checkpoint, egg, enemy…): add a variant, a match arm
//! in [`build`] (validate properties there so bad levels fail at load
//! time), behaviour in [`Entity::update`], and a sprite in the renderer.

use macroquad::math::Vec2;

use crate::level::ObjectSpec;
use crate::sim::ball::Ball;

/// Something that happened during a step, for sound/effects/UI to react to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The ball reached the level exit (fires once).
    ExitReached,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityKind {
    /// Level goal: reaching its area finishes the level (step 14 adds the
    /// results screen).
    Exit { reached: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Top-left corner, level pixels.
    pub pos: Vec2,
    pub size: Vec2,
    pub kind: EntityKind,
}

/// Object types that `build` understands.
pub const KNOWN_TYPES: &[&str] = &["exit"];

/// Turns a level object into an entity, or explains what is wrong with it.
pub fn build(spec: &ObjectSpec) -> Result<Entity, String> {
    let kind = match spec.kind.as_str() {
        "exit" => {
            if spec.size.x <= 0.0 || spec.size.y <= 0.0 {
                return Err(format!("exit '{}' must be a rectangle object", spec.name));
            }
            EntityKind::Exit { reached: false }
        }
        other => {
            return Err(format!(
                "unknown object type '{other}' ('{}' at {},{}); known: spawn, {}",
                spec.name,
                spec.pos.x,
                spec.pos.y,
                KNOWN_TYPES.join(", ")
            ));
        }
    };
    Ok(Entity {
        pos: spec.pos,
        size: spec.size,
        kind,
    })
}

/// Builds every object, failing on the first bad one.
pub fn build_all(specs: &[ObjectSpec]) -> Result<Vec<Entity>, String> {
    specs.iter().map(build).collect()
}

impl Entity {
    /// True if the ball's centre is inside this entity's rectangle.
    pub fn contains_ball(&self, ball: &Ball) -> bool {
        let p = ball.pos;
        p.x >= self.pos.x
            && p.x <= self.pos.x + self.size.x
            && p.y >= self.pos.y
            && p.y <= self.pos.y + self.size.y
    }

    /// Runs one step of this entity's behaviour.
    pub fn update(&mut self, ball: &Ball, events: &mut Vec<Event>) {
        let inside = self.contains_ball(ball);
        match &mut self.kind {
            EntityKind::Exit { reached } => {
                if !*reached && inside {
                    *reached = true;
                    events.push(Event::ExitReached);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ball::ball_at;
    use macroquad::math::vec2;

    fn spec(kind: &str, size: Vec2) -> ObjectSpec {
        ObjectSpec {
            kind: kind.into(),
            name: "x".into(),
            pos: vec2(100.0, 100.0),
            size,
            points: Vec::new(),
            props: Vec::new(),
        }
    }

    #[test]
    fn builds_exit() {
        let e = build(&spec("exit", vec2(32.0, 32.0))).unwrap();
        assert_eq!(e.kind, EntityKind::Exit { reached: false });
    }

    #[test]
    fn rejects_bad_objects() {
        assert!(
            build(&spec("exit", Vec2::ZERO))
                .unwrap_err()
                .contains("rectangle")
        );
        let e = build(&spec("exitt", vec2(32.0, 32.0))).unwrap_err();
        assert!(
            e.contains("unknown object type 'exitt'") && e.contains("known:"),
            "{e}"
        );
    }

    #[test]
    fn exit_fires_once_when_the_ball_centre_enters() {
        let mut e = build(&spec("exit", vec2(32.0, 32.0))).unwrap();
        let mut events = Vec::new();
        e.update(&ball_at(95.0, 116.0), &mut events);
        assert!(events.is_empty(), "centre still outside");
        e.update(&ball_at(101.0, 116.0), &mut events);
        e.update(&ball_at(110.0, 116.0), &mut events);
        assert_eq!(events, vec![Event::ExitReached]);
        assert_eq!(e.kind, EntityKind::Exit { reached: true });
    }
}
