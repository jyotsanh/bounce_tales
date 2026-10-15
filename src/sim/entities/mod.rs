//! Interactive level objects. Each Tiled object type maps to one
//! [`EntityKind`]; [`build`] turns an [`ObjectSpec`] into an [`Entity`].
//!
//! To add a new kind (checkpoint, egg, enemy…): add a variant, a match arm
//! in [`build`] (validate properties there so bad levels fail at load
//! time), behaviour in [`Entity::update`], and a sprite in the renderer.

use macroquad::math::{Vec2, vec2};

use crate::level::{ObjectSpec, Prop};
use crate::sim::ball::{Ball, Form};
use crate::sim::boss::Boss;
use crate::sim::crates::Crate;
use crate::sim::enemies::{self, Enemy};
use crate::sim::hazards::Cause;
use crate::sim::platforms::{self, Platform};
use crate::sim::seesaw::{self, Seesaw};

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
    /// The ball went into or out of water at `pos` moving at `speed` px/s.
    Splash { pos: Vec2, speed: f32 },
    /// A crumbling block fell away (`pos` = cell centre).
    Crumbled { pos: Vec2 },
    /// A cracked block was smashed (`pos` = cell centre).
    Smashed { pos: Vec2 },
    /// A switch was pressed (`pos` = its centre).
    Switched { pos: Vec2 },
    /// A door opened or a bridge was built (`pos` = its centre).
    GateMoved { pos: Vec2 },
    /// The ball changed form at `pos` (by a transformer, or a timed form
    /// running out).
    Transformed { form: Form, pos: Vec2 },
    /// The boss was stomped while dazed (`pos` = top of the cube).
    BossHit { pos: Vec2 },
    /// The boss's last hit: it shatters (`pos` = its centre).
    BossDefeated { pos: Vec2 },
    /// A shooter fired a pellet from `pos`.
    Shot { pos: Vec2 },
    /// A pellet shattered on the stone ball at `pos`.
    ShotBlocked { pos: Vec2 },
    /// An enemy was freed (stomped, or crushed by the stone) at `pos`.
    EnemyFreed { pos: Vec2 },
    /// A spring or catapult launched the ball from `pos`.
    Launched { pos: Vec2 },
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
    /// Moving one-way platform (moved by the sim before the ball).
    Platform(Platform),
    /// Floor button: the first touch opens every door / builds every bridge
    /// whose name is `target` (handled by `Sim::step`).
    Switch { target: String, on: bool },
    /// Solid cells over its rectangle until a switch opens it; a `bridge`
    /// is the reverse (empty until switched). `anim` counts down the
    /// open/build animation (drawing only).
    Door {
        name: String,
        bridge: bool,
        switched: bool,
        anim: u32,
    },
    /// Launches a ball that lands on it straight up, high enough to lift
    /// its centre `height` px. `squish` counts down the drawn compression.
    Spring { height: f32, squish: u32 },
    /// Holds a ball that rolls into its cup for a moment, then flings it
    /// along `launch` (px/s). `cooldown` stops it re-catching at once;
    /// `fired` counts down the drawn arm swing.
    Catapult {
        launch: Vec2,
        cooldown: u32,
        fired: u32,
    },
    /// Wind over its rectangle: accelerates the ball by `force` (px/s²).
    Fan { force: Vec2 },
    /// Pushable box (moved by `Sim::step`; solid to the ball).
    Crate(Crate),
    /// Plank on a pivot (turned by `Sim::step`); also motor-driven
    /// spinners.
    Seesaw(Seesaw),
    /// A hypnotized creature (moved by `Sim::step`).
    Enemy(Enemy),
    /// The Hypno Cube (moved by `Sim::step`); beating it wins the level.
    Boss(Boss),
    /// Touching it turns the ball into `form`; `duration` steps (0 = until
    /// changed again).
    Transformer { form: Form, duration: u32 },
    /// Hub and spokes of a wheel whose cars are separate `Platform`
    /// entities (drawing only). `angle` follows the cars.
    Wheel {
        centre: Vec2,
        radius: f32,
        count: u32,
        angle: f32,
        speed: f32,
    },
    /// Sucks in a ball that enters its base and fires it from the muzzle
    /// along `aim` (rad, clockwise from right). `auto` fires after `delay`
    /// steps, otherwise on a jump press; the barrel swings `swing` rad
    /// either side of `base_aim` while loaded or idle.
    Cannon {
        aim: f32,
        base_aim: f32,
        swing: f32,
        swing_speed: f32,
        phase: f32,
        power: f32,
        auto: bool,
        delay: u32,
        cooldown: u32,
        fired: u32,
    },
}

/// Barrel length of a cannon (pivot to muzzle), px.
pub const CANNON_BARREL: f32 = 20.0;
/// Defaults for wheel, spinner and cannon properties.
pub const DEFAULT_WHEEL_RADIUS: f32 = 48.0;
pub const DEFAULT_WHEEL_CARS: i32 = 4;
pub const DEFAULT_WHEEL_SPEED_DEG: f32 = 40.0;
pub const DEFAULT_SPINNER_SPEED_DEG: f32 = 45.0;
pub const DEFAULT_CANNON_ANGLE_DEG: f32 = -45.0;
pub const DEFAULT_CANNON_POWER: f32 = 420.0;
pub const DEFAULT_CANNON_DELAY: f32 = 0.5;
pub const DEFAULT_CANNON_SWING_SPEED_DEG: f32 = 90.0;

/// Steps a catapult holds the ball before firing.
pub const CATAPULT_HOLD_TICKS: u32 = 12;
/// Steps after firing before a catapult can catch again.
pub const CATAPULT_COOLDOWN_TICKS: u32 = 40;
/// Steps a spring/catapult shows its "fired" pose.
pub const LAUNCH_ANIM_TICKS: u32 = 10;
/// Defaults for launcher and fan properties.
pub const DEFAULT_SPRING_TILES: f32 = 6.0;
pub const DEFAULT_CATAPULT_SPEED: f32 = 420.0;
pub const DEFAULT_CATAPULT_ANGLE: f32 = 50.0;
pub const DEFAULT_FAN_FORCE: f32 = 1500.0;

/// Steps of the door-opening / bridge-building animation.
pub const GATE_ANIM_TICKS: u32 = 15;

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Top-left corner, level pixels.
    pub pos: Vec2,
    pub size: Vec2,
    pub kind: EntityKind,
}

/// Object types that `build` understands.
pub const KNOWN_TYPES: &[&str] = &[
    "exit",
    "checkpoint",
    "gem",
    "egg",
    "life",
    "platform",
    "switch",
    "door",
    "bridge",
    "spring",
    "catapult",
    "fan",
    "crate",
    "seesaw",
    "spinner",
    "wheel",
    "cannon",
    "transformer",
    "walker",
    "flyer",
    "jumper",
    "shooter",
    "boss",
];

/// Reads a string property with a default.
fn str_prop<'a>(spec: &'a ObjectSpec, key: &str, default: &'a str) -> Result<&'a str, String> {
    match spec.prop(key) {
        None => Ok(default),
        Some(Prop::Str(s)) => Ok(s),
        Some(other) => Err(format!(
            "{} '{}': property '{key}' must be a string, got {other:?}",
            spec.kind, spec.name
        )),
    }
}

/// Unit vector for a direction name.
fn direction(spec: &ObjectSpec, name: &str) -> Result<Vec2, String> {
    Ok(match name {
        "up" => vec2(0.0, -1.0),
        "down" => vec2(0.0, 1.0),
        "left" => vec2(-1.0, 0.0),
        "right" => vec2(1.0, 0.0),
        other => {
            return Err(format!(
                "{} '{}': dir must be up/down/left/right, got '{other}'",
                spec.kind, spec.name
            ));
        }
    })
}

/// Reads a number property (int or float) with a default.
fn num_prop(spec: &ObjectSpec, key: &str, default: f32) -> Result<f32, String> {
    match spec.prop(key) {
        None => Ok(default),
        Some(Prop::Int(i)) => Ok(*i as f32),
        Some(Prop::Float(f)) => Ok(*f),
        Some(other) => Err(format!(
            "{} '{}': property '{key}' must be a number, got {other:?}",
            spec.kind, spec.name
        )),
    }
}

fn seconds_to_ticks(s: f32) -> u32 {
    (s * crate::sim::TICKS_PER_SECOND as f32).round().max(0.0) as u32
}

fn need_rect(spec: &ObjectSpec) -> Result<(), String> {
    if spec.size.x <= 0.0 || spec.size.y <= 0.0 {
        return Err(format!(
            "{} '{}' must be a rectangle object",
            spec.kind, spec.name
        ));
    }
    Ok(())
}

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
        "switch" => {
            let Some(Prop::Str(target)) = spec.prop("target") else {
                return Err(format!(
                    "switch '{}' needs a string property 'target' (the door/bridge name)",
                    spec.name
                ));
            };
            if spec.size.x <= 0.0 || spec.size.y <= 0.0 {
                return Err(format!("switch '{}' must be a rectangle object", spec.name));
            }
            EntityKind::Switch {
                target: target.clone(),
                on: false,
            }
        }
        kind @ ("door" | "bridge") => {
            if spec.name.is_empty() {
                return Err(format!(
                    "{kind} at {},{} needs a name",
                    spec.pos.x, spec.pos.y
                ));
            }
            if spec.size.x <= 0.0 || spec.size.y <= 0.0 {
                return Err(format!("{kind} '{}' must be a rectangle object", spec.name));
            }
            EntityKind::Door {
                name: spec.name.clone(),
                bridge: kind == "bridge",
                switched: false,
                anim: 0,
            }
        }
        "seesaw" => {
            // The rect's width is the plank; its top edge is the plank's
            // centre line and its bottom is where the post stands.
            need_rect(spec)?;
            let max = num_prop(spec, "max_angle", seesaw::DEFAULT_MAX_ANGLE_DEG)?;
            if !(1.0..=60.0).contains(&max) {
                return Err(format!(
                    "seesaw '{}': max_angle must be 1..60 degrees",
                    spec.name
                ));
            }
            let pivot = vec2(spec.pos.x + spec.size.x / 2.0, spec.pos.y);
            EntityKind::Seesaw(Seesaw::new(
                pivot,
                spec.size.x / 2.0,
                max.to_radians(),
                spec.pos.y + spec.size.y,
            ))
        }
        "walker" => {
            need_rect(spec)?;
            let dir = match str_prop(spec, "dir", "left")? {
                "left" => -1.0,
                "right" => 1.0,
                other => {
                    return Err(format!(
                        "walker '{}': dir must be left or right, got '{other}'",
                        spec.name
                    ));
                }
            };
            let speed = num_prop(spec, "speed", enemies::WALKER_SPEED)?;
            EntityKind::Enemy(Enemy::walker(spec.pos, dir, speed))
        }
        "boss" => {
            // Top-left of the cube where it hovers; its size is fixed.
            return Ok(Entity {
                pos: spec.pos,
                size: Vec2::splat(crate::sim::boss::BOSS_SIZE),
                kind: EntityKind::Boss(Boss::new(spec.pos)),
            });
        }
        "flyer" => {
            need_rect(spec)?;
            let axis = match str_prop(spec, "axis", "x")? {
                "x" => vec2(1.0, 0.0),
                "y" => vec2(0.0, 1.0),
                other => {
                    return Err(format!(
                        "flyer '{}': axis must be x or y, got '{other}'",
                        spec.name
                    ));
                }
            };
            let range = num_prop(spec, "range", 4.0)? * crate::sim::map::TILE;
            let speed = num_prop(spec, "speed", 40.0)?;
            if range <= 0.0 || speed <= 0.0 {
                return Err(format!(
                    "flyer '{}': range and speed must be > 0",
                    spec.name
                ));
            }
            EntityKind::Enemy(Enemy::flyer(spec.pos, axis, range, speed))
        }
        "jumper" => {
            need_rect(spec)?;
            let period = num_prop(spec, "period", 1.25)?;
            EntityKind::Enemy(Enemy::jumper(spec.pos, seconds_to_ticks(period).max(10)))
        }
        "shooter" => {
            need_rect(spec)?;
            let dir = match str_prop(spec, "dir", "left")? {
                "left" => -1.0,
                "right" => 1.0,
                other => {
                    return Err(format!(
                        "shooter '{}': dir must be left or right, got '{other}'",
                        spec.name
                    ));
                }
            };
            let period = num_prop(spec, "period", 2.0)?;
            let aim = match spec.prop("aim") {
                None => false,
                Some(Prop::Bool(b)) => *b,
                Some(other) => {
                    return Err(format!(
                        "shooter '{}': aim must be a bool, got {other:?}",
                        spec.name
                    ));
                }
            };
            EntityKind::Enemy(Enemy::shooter(
                spec.pos,
                dir,
                seconds_to_ticks(period).max(20),
                aim,
            ))
        }
        "transformer" => {
            need_rect(spec)?;
            let name = str_prop(spec, "form", "")?;
            let form = Form::from_name(name).ok_or_else(|| {
                format!(
                    "transformer '{}': form must be normal, stone or balloon, got '{name}'",
                    spec.name
                )
            })?;
            let seconds = num_prop(spec, "duration", 0.0)?;
            if seconds < 0.0 {
                return Err(format!(
                    "transformer '{}': duration must be >= 0",
                    spec.name
                ));
            }
            EntityKind::Transformer {
                form,
                duration: (seconds * crate::sim::TICKS_PER_SECOND as f32).round() as u32,
            }
        }
        "spinner" => {
            // The rect's width is the plank; it turns about the rect's
            // centre.
            need_rect(spec)?;
            let speed = num_prop(spec, "speed", DEFAULT_SPINNER_SPEED_DEG)?.to_radians();
            EntityKind::Seesaw(Seesaw::spinner(
                spec.pos + spec.size / 2.0,
                spec.size.x / 2.0,
                speed,
            ))
        }
        "cannon" => {
            need_rect(spec)?;
            let fire = str_prop(spec, "fire", "auto")?;
            if fire != "auto" && fire != "jump" {
                return Err(format!("cannon '{}': fire must be auto or jump", spec.name));
            }
            let aim = num_prop(spec, "angle", DEFAULT_CANNON_ANGLE_DEG)?.to_radians();
            EntityKind::Cannon {
                aim,
                base_aim: aim,
                swing: num_prop(spec, "swing", 0.0)?.to_radians(),
                swing_speed: num_prop(spec, "swing_speed", DEFAULT_CANNON_SWING_SPEED_DEG)?
                    .to_radians(),
                phase: 0.0,
                power: num_prop(spec, "power", DEFAULT_CANNON_POWER)?,
                auto: fire == "auto",
                delay: (num_prop(spec, "delay", DEFAULT_CANNON_DELAY)?
                    * crate::sim::TICKS_PER_SECOND as f32)
                    .round()
                    .max(1.0) as u32,
                cooldown: 0,
                fired: 0,
            }
        }
        "wheel" => {
            return Err("wheels are built by build_all (several entities)".into());
        }
        "crate" => {
            need_rect(spec)?;
            EntityKind::Crate(Crate::new(spec.pos))
        }
        "spring" => {
            need_rect(spec)?;
            let tiles = num_prop(spec, "height", DEFAULT_SPRING_TILES)?;
            EntityKind::Spring {
                height: tiles * crate::sim::map::TILE,
                squish: 0,
            }
        }
        "catapult" => {
            need_rect(spec)?;
            let dir = direction(spec, str_prop(spec, "dir", "right")?)?;
            if dir.y != 0.0 {
                return Err(format!(
                    "catapult '{}': dir must be left or right",
                    spec.name
                ));
            }
            let speed = num_prop(spec, "speed", DEFAULT_CATAPULT_SPEED)?;
            let angle = num_prop(spec, "angle", DEFAULT_CATAPULT_ANGLE)?.to_radians();
            EntityKind::Catapult {
                launch: vec2(dir.x * angle.cos(), -angle.sin()) * speed,
                cooldown: 0,
                fired: 0,
            }
        }
        "fan" => {
            need_rect(spec)?;
            let dir = direction(spec, str_prop(spec, "dir", "up")?)?;
            let force = num_prop(spec, "force", DEFAULT_FAN_FORCE)?;
            EntityKind::Fan { force: dir * force }
        }
        "platform" => {
            if spec.points.len() < 2 {
                return Err(format!(
                    "platform '{}' must be a polyline or polygon with 2+ points",
                    spec.name
                ));
            }
            let width = num_prop(spec, "width", platforms::DEFAULT_WIDTH_TILES as f32)?;
            let speed = num_prop(spec, "speed", platforms::DEFAULT_SPEED)?;
            let wait = num_prop(spec, "wait", platforms::DEFAULT_WAIT_SECONDS)?;
            if width < 1.0 || speed <= 0.0 || wait < 0.0 {
                return Err(format!(
                    "platform '{}': width >= 1 tile, speed > 0, wait >= 0",
                    spec.name
                ));
            }
            let p = Platform::new(
                spec.points.clone(),
                spec.closed,
                speed,
                (wait * crate::sim::TICKS_PER_SECOND as f32).round() as u32,
                width.round() * crate::sim::map::TILE,
            );
            return Ok(Entity {
                pos: p.top_left(),
                size: vec2(p.width, platforms::PLATFORM_H),
                kind: EntityKind::Platform(p),
            });
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
    let mut out = Vec::new();
    for spec in specs {
        if spec.kind == "wheel" {
            out.extend(build_wheel(spec)?);
        } else {
            out.push(build(spec)?);
        }
    }
    Ok(out)
}

/// A wheel: a hub entity plus `count` level platforms ("cars") spaced
/// evenly on a circle, looping clockwise for positive `speed` (deg/s).
/// Centre = the rect's centre (or the point); radius = half the rect's
/// width, or the `radius` property.
fn build_wheel(spec: &ObjectSpec) -> Result<Vec<Entity>, String> {
    let centre = spec.pos + spec.size / 2.0;
    let radius = if spec.size.x > 0.0 {
        spec.size.x / 2.0
    } else {
        num_prop(spec, "radius", DEFAULT_WHEEL_RADIUS)?
    };
    let count = num_prop(spec, "count", DEFAULT_WHEEL_CARS as f32)?.round() as i32;
    let speed = num_prop(spec, "speed", DEFAULT_WHEEL_SPEED_DEG)?.to_radians();
    let width = num_prop(spec, "width", 2.0)?.round().max(1.0) * crate::sim::map::TILE;
    if !(1..=12).contains(&count) || radius < 16.0 || speed == 0.0 {
        return Err(format!(
            "wheel '{}': count 1..12, radius >= 16, speed not 0",
            spec.name
        ));
    }
    // Circle as a fine polygon, ordered in the direction of travel.
    const SIDES: usize = 48;
    let dir = speed.signum();
    let path: Vec<Vec2> = (0..SIDES)
        .map(|i| {
            let a = dir * i as f32 / SIDES as f32 * std::f32::consts::TAU;
            centre + vec2(a.cos(), a.sin()) * radius
        })
        .collect();
    let probe = Platform::new(path.clone(), true, 1.0, 0, width);
    let length = probe.length();
    let mut out = vec![Entity {
        pos: centre - Vec2::splat(radius),
        size: Vec2::splat(radius * 2.0),
        kind: EntityKind::Wheel {
            centre,
            radius,
            count: count as u32,
            angle: 0.0,
            speed,
        },
    }];
    for k in 0..count {
        let along = length * k as f32 / count as f32;
        let p = Platform::starting_at(path.clone(), true, speed.abs() * radius, 0, width, along);
        out.push(Entity {
            pos: p.top_left(),
            size: vec2(p.width, platforms::PLATFORM_H),
            kind: EntityKind::Platform(p),
        });
    }
    Ok(out)
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
            // Moved / handled by `Sim::step`.
            EntityKind::Platform(_)
            | EntityKind::Switch { .. }
            | EntityKind::Fan { .. }
            | EntityKind::Crate(_)
            | EntityKind::Seesaw(_)
            | EntityKind::Transformer { .. }
            | EntityKind::Enemy(_)
            | EntityKind::Boss(_) => {}
            EntityKind::Wheel { angle, speed, .. } => *angle += *speed * crate::sim::DT,
            EntityKind::Cannon {
                aim,
                base_aim,
                swing,
                swing_speed,
                phase,
                cooldown,
                fired,
                ..
            } => {
                if *swing > 0.0 {
                    *phase += *swing_speed * crate::sim::DT;
                    *aim = *base_aim + *swing * phase.sin();
                }
                *cooldown = cooldown.saturating_sub(1);
                *fired = fired.saturating_sub(1);
            }
            EntityKind::Spring { squish, .. } => *squish = squish.saturating_sub(1),
            EntityKind::Catapult {
                cooldown, fired, ..
            } => {
                *cooldown = cooldown.saturating_sub(1);
                *fired = fired.saturating_sub(1);
            }
            EntityKind::Door { anim, .. } => *anim = anim.saturating_sub(1),
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
            closed: false,
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
    fn builds_platform_with_properties() {
        let mut s = spec("platform", Vec2::ZERO);
        s.points = vec![vec2(100.0, 100.0), vec2(200.0, 100.0)];
        s.props = vec![
            ("speed".into(), Prop::Int(80)),
            ("width".into(), Prop::Int(4)),
            ("wait".into(), Prop::Float(1.0)),
        ];
        let e = build(&s).unwrap();
        let EntityKind::Platform(p) = &e.kind else {
            panic!("not a platform")
        };
        assert_eq!((p.speed, p.width, p.wait), (80.0, 64.0, 60));
        assert_eq!(e.pos, vec2(68.0, 100.0));
        s.points.pop();
        assert!(build(&s).unwrap_err().contains("polyline"));
    }

    #[test]
    fn switches_and_doors_need_their_properties() {
        let mut sw = spec("switch", vec2(16.0, 16.0));
        assert!(build(&sw).unwrap_err().contains("target"));
        sw.props = vec![("target".into(), Prop::Str("gate1".into()))];
        assert!(matches!(
            build(&sw).unwrap().kind,
            EntityKind::Switch { on: false, .. }
        ));
        let mut door = spec("bridge", vec2(48.0, 16.0));
        door.name = String::new();
        assert!(build(&door).unwrap_err().contains("name"));
        door.name = "gate1".into();
        assert!(matches!(
            build(&door).unwrap().kind,
            EntityKind::Door {
                bridge: true,
                switched: false,
                ..
            }
        ));
    }

    #[test]
    fn launchers_and_fans_from_properties() {
        let mut c = spec("catapult", vec2(32.0, 16.0));
        c.props = vec![
            ("dir".into(), Prop::Str("left".into())),
            ("angle".into(), Prop::Int(45)),
            ("speed".into(), Prop::Int(100)),
        ];
        let EntityKind::Catapult { launch, .. } = build(&c).unwrap().kind else {
            panic!()
        };
        assert!(
            (launch - vec2(-70.71, -70.71)).length() < 0.05,
            "{launch:?}"
        );
        c.props[0].1 = Prop::Str("up".into());
        assert!(build(&c).is_err());

        let mut f = spec("fan", vec2(32.0, 96.0));
        f.props = vec![("dir".into(), Prop::Str("right".into()))];
        assert_eq!(
            build(&f).unwrap().kind,
            EntityKind::Fan {
                force: vec2(DEFAULT_FAN_FORCE, 0.0)
            }
        );
        let s = build(&spec("spring", vec2(16.0, 16.0))).unwrap();
        assert_eq!(
            s.kind,
            EntityKind::Spring {
                height: 96.0,
                squish: 0
            }
        );
        assert!(build(&spec("spring", Vec2::ZERO)).is_err());
    }

    #[test]
    fn wheel_expands_into_hub_and_evenly_spaced_cars() {
        let mut w = spec("wheel", vec2(96.0, 96.0));
        w.props = vec![("count".into(), Prop::Int(3))];
        let es = build_all(&[w]).unwrap();
        assert_eq!(es.len(), 4);
        assert!(matches!(es[0].kind, EntityKind::Wheel { count: 3, .. }));
        let centre = vec2(148.0, 148.0);
        for e in &es[1..] {
            let EntityKind::Platform(p) = &e.kind else {
                panic!("car is not a platform")
            };
            assert!((p.top_centre.distance(centre) - 48.0).abs() < 0.5);
        }
    }

    #[test]
    fn cannon_properties() {
        let mut c = spec("cannon", vec2(32.0, 32.0));
        c.props = vec![
            ("fire".into(), Prop::Str("jump".into())),
            ("swing".into(), Prop::Int(30)),
        ];
        let EntityKind::Cannon { auto, swing, .. } = build(&c).unwrap().kind else {
            panic!()
        };
        assert!(!auto);
        assert!((swing - 30f32.to_radians()).abs() < 1e-6);
        c.props[0].1 = Prop::Str("never".into());
        assert!(build(&c).is_err());
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
