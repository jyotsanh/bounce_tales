//! Interactive level objects. Each Tiled object type maps to one
//! [`EntityKind`]; [`build`] turns an [`ObjectSpec`] into an [`Entity`].
//!
//! To add a new kind (checkpoint, egg, enemy…): add a variant, a match arm
//! in [`build`] (validate properties there so bad levels fail at load
//! time), behaviour in [`Entity::update`], and a sprite in the renderer.

use macroquad::math::{Vec2, vec2};

use crate::level::ObjectSpec;
use crate::sim::ball::Ball;
use crate::sim::hazards::Cause;

/// Something that happened during a step, for sound/effects/UI to react to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// The ball reached the level exit (fires once).
    ExitReached,
    /// The ball took off; `pos` is the contact point under it.
    Jumped { pos: Vec2 },
    /// The ball hit the floor from the air at `speed` px/s (only hard
    /// enough landings, not rolling bumps); `pos` is the contact point.
    Landed { pos: Vec2, speed: f32 },
    /// The ball was destroyed at `pos`.
    Died { pos: Vec2, cause: Cause },
    /// The ball reappeared at the respawn point.
    Respawned { pos: Vec2 },
    /// A checkpoint was touched for the first time; `respawn` is where the
    /// ball will come back from now on (ball centre).
    CheckpointReached { respawn: Vec2 },
    /// A collectible was picked up at `pos` (its centre).
    Collected { item: Item, pos: Vec2 },
    /// The last life was lost; the level restarts after a pause.
    GameOver,
    /// The level started over (after game over).
    LevelRestarted,
}

/// Collectible kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// Common sparkly gem: a few points.
    Gem,
    /// Rare hidden egg: many points; counted per level.
    Egg,
    /// Extra ball.
    Life,
}

impl Item {
    /// Points for picking it up.
    pub fn points(self) -> u32 {
        match self {
            Item::Gem => 10,
            Item::Egg => 500,
            Item::Life => 100,
        }
    }
}

/// How close (beyond the ball's radius) the ball's centre must come to an
/// item's centre to pick it up, px.
pub const PICKUP_RADIUS: f32 = 6.0;
/// Side of an item's box, px.
pub const ITEM_SIZE: f32 = 16.0;

#[derive(Debug, Clone, PartialEq)]
pub enum EntityKind {
    /// Level goal: reaching its area finishes the level (step 14 adds the
    /// results screen).
    Exit { reached: bool },
    /// Touching it moves the respawn point here (once).
    Checkpoint { active: bool },
    /// Collectible; stays collected after a death (not after a game over).
    Pickup { item: Item, taken: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Top-left corner, level pixels.
    pub pos: Vec2,
    pub size: Vec2,
    pub kind: EntityKind,
}

/// Object types that `build` understands.
pub const KNOWN_TYPES: &[&str] = &["exit", "checkpoint", "gem", "egg", "life"];

/// Turns a level object into an entity, or explains what is wrong with it.
pub fn build(spec: &ObjectSpec) -> Result<Entity, String> {
    let kind = match spec.kind.as_str() {
        "exit" => {
            if spec.size.x <= 0.0 || spec.size.y <= 0.0 {
                return Err(format!("exit '{}' must be a rectangle object", spec.name));
            }
            EntityKind::Exit { reached: false }
        }
        "checkpoint" => {
            if spec.size.x <= 0.0 || spec.size.y <= 0.0 {
                return Err(format!(
                    "checkpoint '{}' must be a rectangle object",
                    spec.name
                ));
            }
            EntityKind::Checkpoint { active: false }
        }
        kind @ ("gem" | "egg" | "life") => {
            let item = match kind {
                "gem" => Item::Gem,
                "egg" => Item::Egg,
                _ => Item::Life,
            };
            // Rectangles are centred on their middle, points on themselves.
            let centre = spec.pos + spec.size / 2.0;
            return Ok(Entity {
                pos: centre - Vec2::splat(ITEM_SIZE / 2.0),
                size: Vec2::splat(ITEM_SIZE),
                kind: EntityKind::Pickup { item, taken: false },
            });
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

    pub fn centre(&self) -> Vec2 {
        self.pos + self.size / 2.0
    }

    /// Runs one step of this entity's behaviour.
    pub fn update(&mut self, ball: &Ball, events: &mut Vec<Event>) {
        let inside = self.contains_ball(ball);
        let centre = self.centre();
        match &mut self.kind {
            EntityKind::Exit { reached } => {
                if !*reached && inside {
                    *reached = true;
                    events.push(Event::ExitReached);
                }
            }
            EntityKind::Checkpoint { active } => {
                if !*active && inside {
                    *active = true;
                    // Respawn resting on the rectangle's floor, centred.
                    let respawn = vec2(
                        self.pos.x + self.size.x / 2.0,
                        self.pos.y + self.size.y - ball.radius,
                    );
                    events.push(Event::CheckpointReached { respawn });
                }
            }
            EntityKind::Pickup { item, taken } => {
                if !*taken && ball.pos.distance(centre) < ball.radius + PICKUP_RADIUS {
                    *taken = true;
                    events.push(Event::Collected {
                        item: *item,
                        pos: centre,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ball::ball_at;

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
    fn checkpoint_activates_once_with_floor_respawn() {
        let mut c = build(&spec("checkpoint", vec2(32.0, 32.0))).unwrap();
        let mut events = Vec::new();
        c.update(&ball_at(116.0, 124.0), &mut events);
        c.update(&ball_at(118.0, 124.0), &mut events);
        assert_eq!(
            events,
            vec![Event::CheckpointReached {
                respawn: vec2(116.0, 124.0)
            }]
        );
        assert_eq!(c.kind, EntityKind::Checkpoint { active: true });
    }

    #[test]
    fn pickups_from_rects_and_points() {
        let rect = build(&spec("gem", vec2(16.0, 16.0))).unwrap();
        assert_eq!(rect.centre(), vec2(108.0, 108.0));
        let point = build(&spec("egg", Vec2::ZERO)).unwrap();
        assert_eq!(point.centre(), vec2(100.0, 100.0));
        assert!(matches!(
            point.kind,
            EntityKind::Pickup {
                item: Item::Egg,
                taken: false
            }
        ));
    }

    #[test]
    fn pickup_is_collected_once_when_touched() {
        let mut gem = build(&spec("gem", Vec2::ZERO)).unwrap();
        let mut events = Vec::new();
        gem.update(&ball_at(100.0 + 8.0 + 6.5, 100.0), &mut events);
        assert!(events.is_empty(), "just out of reach");
        gem.update(&ball_at(100.0 + 13.0, 100.0), &mut events);
        gem.update(&ball_at(100.0, 100.0), &mut events);
        assert_eq!(
            events,
            vec![Event::Collected {
                item: Item::Gem,
                pos: vec2(100.0, 100.0)
            }]
        );
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
