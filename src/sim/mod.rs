//! Pure game simulation. Nothing in here may call macroquad drawing or
//! windowing functions, so it can run headless in tests. Only macroquad's
//! math types (glam) are used.
//!
//! Determinism rule: `Sim::step` must depend only on the current state and
//! the given `InputFrame`. No wall-clock time, no global RNG, no HashMap
//! iteration order. Replays rely on this.

pub mod ball;
pub mod camera;
pub mod crates;
pub mod entities;
pub mod hazards;
pub mod input;
pub mod map;
pub mod physics;
pub mod platforms;
pub mod rng;
pub mod seesaw;
pub mod water;

pub use ball::Ball;
pub use camera::Camera;
pub use entities::{Entity, EntityKind, Event, Item};
pub use input::InputFrame;
pub use map::TileMap;

use macroquad::math::Vec2;

use crate::level::Level;
use hazards::Cause;
pub use rng::Rng;

/// Simulation rate. The sim always advances in steps of exactly `DT`.
pub const TICKS_PER_SECOND: u32 = 60;
pub const DT: f32 = 1.0 / TICKS_PER_SECOND as f32;

/// Entering/leaving water slower than this makes no `Splash`, px/s.
pub const SPLASH_MIN_SPEED: f32 = 60.0;

/// Landings slower than this don't produce a `Landed` event, px/s.
pub const LAND_EVENT_MIN_SPEED: f32 = 100.0;

/// Level loaded when none is given.
pub const DEFAULT_LEVEL: &str = "big";

/// Steps between dying and reappearing (1 s: the burst plays out).
pub const DEATH_TICKS: u32 = 60;
/// Lives at the start of a level (and after a game over).
pub const START_LIVES: u32 = 3;
/// What a level starts with, carried over from the previous level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carry {
    pub lives: u32,
    pub score: u32,
}

impl Default for Carry {
    fn default() -> Carry {
        Carry {
            lives: START_LIVES,
            score: 0,
        }
    }
}

/// A crumbling block in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crumble {
    pub x: i32,
    pub y: i32,
    /// Steps left in the current phase (shaking, then gone).
    pub ticks: u32,
    pub gone: bool,
}

/// Steps a crumbling block shakes before falling away.
pub const CRUMBLE_SHAKE_TICKS: u32 = 30;
/// Steps before a fallen crumbling block comes back.
pub const CRUMBLE_RETURN_TICKS: u32 = 300;
/// Hitting a cracked block at least this fast breaks it (if the ball
/// `smashes`), px/s.
pub const SMASH_SPEED: f32 = 60.0;

/// Most balls you can hold.
pub const MAX_LIVES: u32 = 9;
/// Steps the "game over" pause lasts before the level restarts.
pub const GAME_OVER_TICKS: u32 = 150;
/// Steps of the transformation flash (drawing only).
pub const TRANSFORM_ANIM_TICKS: u32 = 20;
/// A timed form blinks for this many steps before it runs out.
pub const FORM_WARNING_TICKS: u32 = 120;
/// Steps of the grow-in effect after respawning (drawing only).
pub const RESPAWN_ANIM_TICKS: u32 = 12;

/// Whole game state.
#[derive(Debug, Clone, PartialEq)]
pub struct Sim {
    /// Number of steps taken since start.
    pub tick: u64,
    /// Collision map plus drawable layers as they are now (blocks can
    /// crumble, break, open or appear during play).
    pub level: Level,
    /// The level as loaded, for restoring blocks and restarting.
    pub pristine: std::rc::Rc<Level>,
    /// Crumbling blocks that are shaking or gone.
    pub crumbles: Vec<Crumble>,
    pub ball: Ball,
    pub camera: Camera,
    /// Interactive objects built from the level's object specs.
    pub entities: Vec<Entity>,
    /// What happened during the latest step (cleared at the start of each).
    pub events: Vec<Event>,
    /// The ball has reached the exit (stays true).
    pub exit_reached: bool,
    /// Tick at which the exit was reached (stops the level clock).
    pub exit_tick: Option<u64>,
    /// Where the ball comes back after dying (level start; checkpoints move
    /// it in step 12). Ball centre, px.
    pub respawn_point: Vec2,
    /// Steps left before respawning; 0 = alive. While dying the ball is
    /// hidden and input is ignored.
    pub dying: u32,
    /// Counts down from `RESPAWN_ANIM_TICKS` after a respawn (drawing).
    pub respawn_anim: u32,
    /// Index (into `entities`) of the moving platform the ball rides.
    pub riding: Option<usize>,
    /// Steps left of a timed form (`None` = the form lasts).
    pub form_ticks: Option<u32>,
    /// Counts down the transformation flash (drawing only).
    pub transform_anim: u32,
    /// Form the ball comes back in after dying (its form at the last
    /// checkpoint, if that form was not timed).
    pub respawn_form: ball::Form,
    /// Seesaw the ball rests on (entity index, position along the plank).
    pub on_seesaw: Option<(usize, f32)>,
    /// Catapult holding the ball (entity index, steps until it fires).
    pub held: Option<(usize, u32)>,
    /// Total deaths this level.
    pub deaths: u32,
    /// Balls left, including the one in play. Losing the last one is game
    /// over.
    pub lives: u32,
    /// Steps left of the game-over pause; 0 = not game over.
    pub game_over: u32,
    /// How many times the level was restarted after a game over.
    pub restarts: u32,
    /// Total score, including what was carried into the level.
    pub score: u32,
    /// Lives and score this level started with (a game over goes back to
    /// these).
    pub start_carry: Carry,
    /// Tick at which the current attempt started (reset by a game-over
    /// restart); level time = `tick - started_tick`.
    pub started_tick: u64,
    /// Gems and eggs collected this level, and how many the level has.
    pub gems: u32,
    pub gems_total: u32,
    pub eggs: u32,
    pub eggs_total: u32,
    pub rng: Rng,
    /// Input used in the most recent step.
    pub input: InputFrame,
    /// Input used in the step before that (for "just pressed" checks).
    pub prev_input: InputFrame,
}

impl Default for Sim {
    fn default() -> Sim {
        Sim::new()
    }
}

impl Sim {
    /// The default level.
    pub fn new() -> Sim {
        Sim::for_level(DEFAULT_LEVEL).expect("default level loads")
    }

    /// Starts the named level (see `level::level_names`).
    pub fn for_level(name: &str) -> Result<Sim, String> {
        Level::load(name).map(Sim::with_level)
    }

    pub fn with_level(level: Level) -> Sim {
        Sim::with_level_carry(level, Carry::default())
    }

    /// Starts a level with lives and score carried in.
    pub fn with_level_carry(level: Level, carry: Carry) -> Sim {
        let pristine = std::rc::Rc::new(level.clone());
        let mut level = level;
        let ball = Ball::new(level.map.spawn);
        let entities =
            entities::build_all(&level.objects).expect("objects are validated when loading");
        let count = |want: Item| {
            entities
                .iter()
                .filter(|e| matches!(e.kind, EntityKind::Pickup { item, .. } if item == want))
                .count() as u32
        };
        let (gems_total, eggs_total) = (count(Item::Gem), count(Item::Egg));
        // Closed doors are solid from the start.
        for e in &entities {
            if let EntityKind::Door { bridge: false, .. } = e.kind {
                for (x, y) in cells_of(e) {
                    level.map.set(x, y, map::Tile::Solid);
                }
            }
        }
        Sim {
            tick: 0,
            entities,
            events: Vec::new(),
            exit_reached: false,
            exit_tick: None,
            respawn_point: level.map.spawn,
            dying: 0,
            respawn_anim: 0,
            riding: None,
            held: None,
            form_ticks: None,
            transform_anim: 0,
            respawn_form: ball::Form::Normal,
            on_seesaw: None,
            deaths: 0,
            lives: carry.lives,
            game_over: 0,
            restarts: 0,
            score: carry.score,
            start_carry: carry,
            started_tick: 0,
            gems: 0,
            gems_total,
            eggs: 0,
            eggs_total,
            camera: Camera::snapped_to(&ball, &level.map),
            ball,
            level,
            pristine,
            crumbles: Vec::new(),
            rng: Rng::new(1),
            input: InputFrame::NONE,
            prev_input: InputFrame::NONE,
        }
    }

    /// Advances the world by exactly one `DT`.
    pub fn step(&mut self, input: InputFrame) {
        self.prev_input = self.input;
        self.input = input;
        self.tick += 1;
        self.events.clear();
        self.respawn_anim = self.respawn_anim.saturating_sub(1);

        // Level finished: everything holds still (the game flow takes over).
        if self.exit_reached {
            return;
        }
        self.tick_crumbles();
        if self.game_over > 0 {
            self.game_over -= 1;
            if self.game_over == 0 {
                self.restart_level();
            }
            return;
        }
        if self.dying > 0 {
            self.dying -= 1;
            if self.dying == 0 {
                if self.lives == 0 {
                    self.game_over = GAME_OVER_TICKS;
                    self.events.push(Event::GameOver);
                } else {
                    self.respawn();
                }
            }
            return;
        }

        // Moving platforms go first, carrying a riding ball with them.
        for e in &mut self.entities {
            if let EntityKind::Platform(p) = &mut e.kind {
                p.advance();
                e.pos = p.top_left();
            }
        }
        self.update_crates();
        self.update_seesaws();
        self.ball.wind = self.wind_at(self.ball.pos);
        if !self.hold_in_catapult() {
            self.move_ball(input);
            self.use_launchers();
            self.use_transformers();
        }
        self.tick_form();

        self.camera.update(&self.ball, &self.level.map);
        if let Some(cause) = hazards::check(&self.ball, &self.level.map) {
            self.die(cause);
            return;
        }

        for e in &mut self.entities {
            e.update(&self.ball, &mut self.events);
        }
        for e in &self.events {
            match *e {
                Event::ExitReached => {
                    self.exit_reached = true;
                    self.exit_tick = Some(self.tick);
                }
                Event::CheckpointReached { respawn } => {
                    self.respawn_point = respawn;
                    // Timed forms don't survive a respawn.
                    self.respawn_form = if self.form_ticks.is_some() {
                        ball::Form::Normal
                    } else {
                        self.ball.form
                    };
                }
                Event::Collected { item, .. } => {
                    self.score += item.points();
                    match item {
                        Item::Gem => self.gems += 1,
                        Item::Egg => self.eggs += 1,
                        Item::Life => self.lives = (self.lives + 1).min(MAX_LIVES),
                    }
                }
                _ => {}
            }
        }
    }

    /// Ball physics for one step: platform carry, input, movement and
    /// collision, landing/water events, breakable blocks and switches.
    fn move_ball(&mut self, input: InputFrame) {
        let carried_from = self.ball.pos;
        if let Some(p) = self.riding.and_then(|i| self.platform(i)) {
            self.ball.pos += p.delta();
        }

        let jump_pressed = self.jump_pressed();
        let bottom = |b: &Ball| b.pos + macroquad::math::vec2(0.0, b.radius);
        let jumped = self.ball.update(input, jump_pressed);
        // Draw the carried ball moving smoothly with its platform.
        self.ball.prev_pos = carried_from;
        if jumped {
            self.events.push(Event::Jumped {
                pos: bottom(&self.ball),
            });
        }
        let prev_bottom = self.ball.pos.y + self.ball.radius;
        let (solid_ids, solids) = self.crate_boxes(None);
        let contacts = physics::move_and_collide_with(&mut self.ball, &self.level.map, &solids);
        for &(i, vx) in &contacts.pushes {
            if let EntityKind::Crate(c) = &mut self.entities[solid_ids[i]].kind {
                c.push = Some(vx);
            }
        }
        let platform_impact = self
            .collide_platforms(prev_bottom)
            .max(self.collide_seesaws());
        let impact = contacts.impact_speed.max(platform_impact);
        if impact > LAND_EVENT_MIN_SPEED {
            let normal = if platform_impact > contacts.impact_speed {
                macroquad::math::vec2(0.0, -1.0)
            } else {
                contacts.floor_normal
            };
            self.events.push(Event::Landed {
                pos: self.ball.pos - normal * self.ball.radius,
                speed: impact,
            });
        }
        self.ball.update_spin();
        self.ball.update_squash(impact, jumped);

        let was_wet = self.ball.water > 0.0;
        self.ball.water = water::submersion(&self.ball, &self.level.map);
        let speed = self.ball.vel.length();
        if (self.ball.water > 0.0) != was_wet && speed > SPLASH_MIN_SPEED {
            // At the water line under the ball's centre.
            let col_top = (self.ball.pos.y / map::TILE).floor() * map::TILE;
            let surface = if self.ball.water > 0.0 {
                col_top + map::TILE
            } else {
                col_top
            };
            self.events.push(Event::Splash {
                pos: macroquad::math::vec2(
                    self.ball.pos.x,
                    surface.min(self.ball.pos.y + self.ball.radius),
                ),
                speed,
            });
        }

        self.update_blocks(&contacts.hits);
        self.press_switches();
    }

    /// Boxes of the crates still in play (optionally leaving one out), with
    /// their entity indices.
    fn crate_boxes(&self, except: Option<usize>) -> (Vec<usize>, Vec<physics::SolidBox>) {
        self.entities
            .iter()
            .enumerate()
            .filter(|&(i, e)| {
                Some(i) != except && matches!(&e.kind, EntityKind::Crate(c) if !c.gone)
            })
            .map(|(i, e)| (i, (e.pos, e.pos + e.size)))
            .unzip()
    }

    /// Moves every crate one step (in level order, so results never depend
    /// on anything else).
    fn update_crates(&mut self) {
        for i in 0..self.entities.len() {
            if !matches!(self.entities[i].kind, EntityKind::Crate(_)) {
                continue;
            }
            let (_, others) = self.crate_boxes(Some(i));
            let e = &mut self.entities[i];
            let (pos, size) = (e.pos, e.size);
            if let EntityKind::Crate(c) = &mut e.kind {
                e.pos = c.advance(pos, size, &self.level.map, &others);
            }
        }
    }

    /// Touching a transformer turns the ball into its form (or refreshes a
    /// timed form).
    fn use_transformers(&mut self) {
        let mut change = None;
        for e in &self.entities {
            if let EntityKind::Transformer { form, duration } = e.kind
                && inside(e, self.ball.pos)
            {
                let refresh = form == self.ball.form && duration > 0;
                if form != self.ball.form || refresh {
                    change = Some((form, duration, e.centre(), !refresh));
                }
            }
        }
        if let Some((form, duration, pos, announce)) = change {
            self.ball.set_form(form);
            self.form_ticks = (duration > 0 && form != ball::Form::Normal).then_some(duration);
            if announce {
                self.transform_anim = TRANSFORM_ANIM_TICKS;
                self.events.push(Event::Transformed { form, pos });
            }
        }
    }

    /// Counts down a timed form and the flash; a timed form running out
    /// turns the ball back to normal.
    fn tick_form(&mut self) {
        self.transform_anim = self.transform_anim.saturating_sub(1);
        if let Some(t) = self.form_ticks {
            if t > 1 {
                self.form_ticks = Some(t - 1);
            } else {
                self.form_ticks = None;
                self.ball.set_form(ball::Form::Normal);
                self.transform_anim = TRANSFORM_ANIM_TICKS;
                self.events.push(Event::Transformed {
                    form: ball::Form::Normal,
                    pos: self.ball.pos,
                });
            }
        }
    }

    /// Fan push on the ball at `p`: the forces whose air stream holds it.
    /// Heavier forms are pushed less (force / mass): stone barely moves in
    /// an updraft, a balloon shoots up.
    fn wind_at(&self, p: Vec2) -> Vec2 {
        let force: Vec2 = self
            .entities
            .iter()
            .filter_map(|e| match e.kind {
                EntityKind::Fan { force } if inside(e, p) => Some(force),
                _ => None,
            })
            .sum();
        force / self.ball.params.mass
    }

    /// While a catapult holds the ball, keep it in the cup; fire when the
    /// hold is over. Returns true if the ball is held this step (its own
    /// physics is skipped).
    fn hold_in_catapult(&mut self) -> bool {
        let Some((i, ticks)) = self.held else {
            return false;
        };
        let jump = self.jump_pressed();
        let e = &mut self.entities[i];
        if let EntityKind::Cannon {
            aim,
            power,
            auto,
            cooldown,
            fired,
            ..
        } = &mut e.kind
        {
            let pivot = e.pos + e.size / 2.0;
            self.ball.prev_pos = pivot;
            self.ball.pos = pivot;
            self.ball.vel = Vec2::ZERO;
            let ready = if *auto { ticks <= 1 } else { jump };
            if !ready {
                self.held = Some((i, ticks.saturating_sub(1)));
                return true;
            }
            let dir = macroquad::math::vec2(aim.cos(), aim.sin());
            let muzzle = pivot + dir * entities::CANNON_BARREL;
            self.ball.pos = muzzle;
            self.ball.prev_pos = muzzle;
            self.ball.vel = dir * *power;
            self.ball.grounded = false;
            self.ball.jumping = true;
            *cooldown = entities::CATAPULT_COOLDOWN_TICKS;
            *fired = entities::LAUNCH_ANIM_TICKS;
            self.held = None;
            self.events.push(Event::Launched { pos: muzzle });
            return true;
        }
        let cup = cup_of(e, self.ball.radius);
        let EntityKind::Catapult {
            launch,
            cooldown,
            fired,
        } = &mut e.kind
        else {
            self.held = None;
            return false;
        };
        self.ball.prev_pos = self.ball.pos;
        self.ball.pos = cup;
        self.ball.vel = Vec2::ZERO;
        if ticks > 1 {
            self.held = Some((i, ticks - 1));
            return true;
        }
        // Fire.
        self.ball.vel = *launch;
        self.ball.grounded = false;
        self.ball.jumping = true;
        *cooldown = entities::CATAPULT_COOLDOWN_TICKS;
        *fired = entities::LAUNCH_ANIM_TICKS;
        self.held = None;
        self.events.push(Event::Launched { pos: cup });
        true
    }

    /// Springs launch a ball that comes down on them; catapults catch a
    /// ball that rolls into their cup.
    fn use_launchers(&mut self) {
        let r = self.ball.radius;
        for (i, e) in self.entities.iter_mut().enumerate() {
            let ball_inside = inside(e, self.ball.pos);
            match &mut e.kind {
                EntityKind::Spring { height, squish } => {
                    let over = self.ball.pos.x >= e.pos.x && self.ball.pos.x <= e.pos.x + e.size.x;
                    let touching = self.ball.pos.y + r >= e.pos.y + 4.0
                        && self.ball.pos.y < e.pos.y + e.size.y;
                    if over && touching && self.ball.vel.y >= -10.0 {
                        let top = e.pos.y + 4.0;
                        self.ball.pos.y = self.ball.pos.y.min(top - r);
                        self.ball.vel.y = -self.ball.params.launch_speed(*height);
                        self.ball.grounded = false;
                        self.ball.jumping = true;
                        self.riding = None;
                        *squish = entities::LAUNCH_ANIM_TICKS;
                        self.events.push(Event::Launched {
                            pos: macroquad::math::vec2(self.ball.pos.x, top),
                        });
                    }
                }
                EntityKind::Catapult { cooldown: 0, .. }
                | EntityKind::Cannon { cooldown: 0, .. }
                    if self.held.is_none() && ball_inside =>
                {
                    let ticks = match e.kind {
                        EntityKind::Cannon { delay, .. } => delay,
                        _ => entities::CATAPULT_HOLD_TICKS,
                    };
                    self.held = Some((i, ticks));
                    self.riding = None;
                }
                _ => {}
            }
        }
    }

    /// Changes a cell during play, keeping the ground layer's picture in
    /// step: removed with the block, restored from the pristine level.
    fn set_cell(&mut self, x: i32, y: i32, tile: map::Tile) {
        self.level.map.set(x, y, tile);
        let restore = tile.is_solid();
        let pristine = &self.pristine;
        if let Some((li, layer)) = self
            .level
            .layers
            .iter_mut()
            .enumerate()
            .find(|(_, l)| l.name == "ground")
            && x >= 0
            && y >= 0
            && (x as usize) < layer.width
            && (y as usize) < layer.height
        {
            let i = y as usize * layer.width + x as usize;
            layer.cells[i] = if restore {
                pristine.layers[li].cells[i]
            } else {
                None
            };
        }
    }

    /// Crumbling blocks under the ball start shaking; cracked blocks hit
    /// hard by a smashing ball break. (`tick_crumbles` drops and restores.)
    fn update_blocks(&mut self, hits: &[(i32, i32, f32)]) {
        let centre =
            |x: i32, y: i32| macroquad::math::vec2(x as f32 + 0.5, y as f32 + 0.5) * map::TILE;
        for &(x, y, speed) in hits {
            let f = self.level.map.flags(x, y);
            let below_ball = centre(x, y).y > self.ball.pos.y;
            if f & map::flags::CRUMBLE != 0
                && below_ball
                && self.level.map.is_solid(x, y)
                && !self.crumbles.iter().any(|c| (c.x, c.y) == (x, y))
            {
                self.crumbles.push(Crumble {
                    x,
                    y,
                    ticks: CRUMBLE_SHAKE_TICKS,
                    gone: false,
                });
            }
            if f & map::flags::CRACKED != 0
                && self.ball.params.smashes
                && speed >= SMASH_SPEED
                && self.level.map.is_solid(x, y)
            {
                self.set_cell(x, y, map::Tile::Empty);
                self.events.push(Event::Smashed { pos: centre(x, y) });
            }
        }
    }

    /// Runs crumbling-block timers (every step, also while dying, so a
    /// block's return time doesn't depend on what the ball is doing).
    fn tick_crumbles(&mut self) {
        let centre =
            |x: i32, y: i32| macroquad::math::vec2(x as f32 + 0.5, y as f32 + 0.5) * map::TILE;
        let mut i = 0;
        while i < self.crumbles.len() {
            let c = self.crumbles[i];
            if c.ticks > 1 {
                self.crumbles[i].ticks -= 1;
                i += 1;
                continue;
            }
            if !c.gone {
                self.set_cell(c.x, c.y, map::Tile::Empty);
                self.events.push(Event::Crumbled {
                    pos: centre(c.x, c.y),
                });
                self.crumbles[i] = Crumble {
                    ticks: CRUMBLE_RETURN_TICKS,
                    gone: true,
                    ..c
                };
                i += 1;
            } else if ball_overlaps_cell(&self.ball, c.x, c.y) {
                i += 1; // wait until the ball has moved away
            } else {
                let original = self.pristine.map.get(c.x, c.y);
                self.set_cell(c.x, c.y, original);
                self.crumbles.remove(i);
            }
        }
    }

    /// The first touch of a switch opens its doors and builds its bridges.
    fn press_switches(&mut self) {
        let mut targets = Vec::new();
        for e in &mut self.entities {
            let inside = e.contains_ball(&self.ball);
            let centre = e.centre();
            if let EntityKind::Switch { target, on } = &mut e.kind
                && !*on
                && inside
            {
                *on = true;
                targets.push(target.clone());
                self.events.push(Event::Switched { pos: centre });
            }
        }
        for target in targets {
            let mut changes = Vec::new();
            for e in &mut self.entities {
                let centre = e.centre();
                let cells = cells_of(e);
                if let EntityKind::Door {
                    name,
                    bridge,
                    switched,
                    anim,
                } = &mut e.kind
                    && *name == target
                    && !*switched
                {
                    *switched = true;
                    *anim = entities::GATE_ANIM_TICKS;
                    let tile = if *bridge {
                        map::Tile::Solid
                    } else {
                        map::Tile::Empty
                    };
                    changes.extend(cells.into_iter().map(|(x, y)| (x, y, tile)));
                    self.events.push(Event::GateMoved { pos: centre });
                }
            }
            for (x, y, tile) in changes {
                self.level.map.set(x, y, tile);
            }
        }
    }

    fn platform(&self, i: usize) -> Option<&platforms::Platform> {
        match &self.entities.get(i)?.kind {
            EntityKind::Platform(p) => Some(p),
            _ => None,
        }
    }

    /// Keeps the rider on its platform or lands the ball on one. Returns
    /// the landing impact speed (0 if none).
    fn collide_platforms(&mut self, prev_bottom: f32) -> f32 {
        if let Some(i) = self.riding {
            let p = self.platform(i).expect("riding a platform").clone();
            if p.keep_rider(&mut self.ball) {
                return 0.0;
            }
            self.riding = None;
        }
        // Only a ball moving down can land.
        for i in 0..self.entities.len() {
            let Some(p) = self.platform(i).cloned() else {
                continue;
            };
            if let Some(impact) = p.try_land(&mut self.ball, prev_bottom) {
                if self.ball.grounded {
                    self.riding = Some(i);
                }
                return impact;
            }
        }
        0.0
    }

    /// Turns every seesaw, loaded by the ball if it rests on one.
    fn update_seesaws(&mut self) {
        let mass = self.ball.params.mass;
        for (i, e) in self.entities.iter_mut().enumerate() {
            if let EntityKind::Seesaw(s) = &mut e.kind {
                let load = self
                    .on_seesaw
                    .filter(|&(j, _)| j == i)
                    .map(|(_, at)| (at, mass));
                s.advance(load);
            }
        }
    }

    /// Keeps the ball on its seesaw or lands it on one. Returns the landing
    /// impact speed (0 if none).
    fn collide_seesaws(&mut self) -> f32 {
        if let Some((i, _)) = self.on_seesaw {
            if let EntityKind::Seesaw(s) = &self.entities[i].kind
                && let Some(at) = s.keep_rider(&mut self.ball)
            {
                self.on_seesaw = Some((i, at));
                return 0.0;
            }
            self.on_seesaw = None;
        }
        for i in 0..self.entities.len() {
            if let EntityKind::Seesaw(s) = &mut self.entities[i].kind
                && let Some((at, impact)) = s.try_land(&mut self.ball)
            {
                if self.ball.grounded {
                    self.on_seesaw = Some((i, at));
                }
                return impact;
            }
        }
        0.0
    }

    /// Destroys the ball; it comes back after `DEATH_TICKS` steps.
    pub fn die(&mut self, cause: Cause) {
        self.events.push(Event::Died {
            pos: self.ball.pos,
            cause,
        });
        self.deaths += 1;
        self.lives = self.lives.saturating_sub(1);
        self.dying = DEATH_TICKS;
        self.ball.vel = Vec2::ZERO;
        self.riding = None;
        self.held = None;
        self.on_seesaw = None;
    }

    fn respawn(&mut self) {
        self.ball = Ball::with_form(self.respawn_point, self.respawn_form);
        self.form_ticks = None;
        self.camera = Camera::snapped_to(&self.ball, &self.level.map);
        self.respawn_anim = RESPAWN_ANIM_TICKS;
        self.events.push(Event::Respawned {
            pos: self.respawn_point,
        });
    }

    /// Starts the level over: fresh entities, full lives, back to the
    /// start. Keeps the step counter, input history and restart count.
    fn restart_level(&mut self) {
        let mut fresh = Sim::with_level_carry((*self.pristine).clone(), self.start_carry);
        fresh.tick = self.tick;
        fresh.started_tick = self.tick;
        fresh.input = self.input;
        fresh.prev_input = self.prev_input;
        fresh.restarts = self.restarts + 1;
        fresh.respawn_anim = RESPAWN_ANIM_TICKS;
        fresh.events.push(Event::LevelRestarted);
        *self = fresh;
    }

    /// Seconds played on the current attempt (the clock stops at the exit).
    pub fn level_seconds(&self) -> f32 {
        let end = self.exit_tick.unwrap_or(self.tick);
        (end - self.started_tick) as f32 / TICKS_PER_SECOND as f32
    }

    /// True while the ball should be drawn (not popped, not game over).
    pub fn ball_visible(&self) -> bool {
        let in_cannon = self
            .held
            .is_some_and(|(i, _)| matches!(self.entities[i].kind, EntityKind::Cannon { .. }));
        self.dying == 0 && self.game_over == 0 && !in_cannon
    }

    /// True on the first step the jump button is held.
    pub fn jump_pressed(&self) -> bool {
        self.input.jump && !self.prev_input.jump
    }
}

/// Map cells covered by an entity's rectangle.
pub fn cells_of(e: &Entity) -> Vec<(i32, i32)> {
    let (x0, y0) = map::TileMap::cell_at(e.pos + Vec2::splat(0.5));
    let (x1, y1) = map::TileMap::cell_at(e.pos + e.size - Vec2::splat(0.5));
    (y0..=y1)
        .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
        .collect()
}

/// Is `p` inside an entity's rectangle?
fn inside(e: &Entity, p: Vec2) -> bool {
    p.x >= e.pos.x && p.x <= e.pos.x + e.size.x && p.y >= e.pos.y && p.y <= e.pos.y + e.size.y
}

/// Where a catapult holds the ball: centred, resting in the cup on top.
fn cup_of(e: &Entity, radius: f32) -> Vec2 {
    macroquad::math::vec2(e.pos.x + e.size.x / 2.0, e.pos.y + 6.0 - radius)
}

fn ball_overlaps_cell(ball: &Ball, x: i32, y: i32) -> bool {
    let min = macroquad::math::vec2(x as f32, y as f32) * map::TILE;
    let max = min + Vec2::splat(map::TILE);
    ball.pos.distance(ball.pos.clamp(min, max)) < ball.radius
}

/// Fixed-timestep accumulator: turns variable frame times into a whole number
/// of sim steps, plus a 0..1 blend factor for interpolated rendering.
#[derive(Debug, Clone, Default)]
pub struct FixedStep {
    accumulator: f32,
}

impl FixedStep {
    /// Most steps we run per rendered frame; drops time after a long stall
    /// (window drag, debugger) instead of spiralling.
    pub const MAX_STEPS_PER_FRAME: u32 = 5;

    /// Adds `frame_time` seconds and returns how many sim steps to run now.
    pub fn advance(&mut self, frame_time: f32) -> u32 {
        self.accumulator += frame_time.max(0.0);
        let mut steps = 0;
        while self.accumulator >= DT && steps < Self::MAX_STEPS_PER_FRAME {
            self.accumulator -= DT;
            steps += 1;
        }
        if steps == Self::MAX_STEPS_PER_FRAME {
            self.accumulator = self.accumulator.min(DT);
        }
        steps
    }

    /// How far we are between the last step and the next one (0..1).
    pub fn alpha(&self) -> f32 {
        (self.accumulator / DT).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_second_is_sixty_steps() {
        let mut fs = FixedStep::default();
        let total: u32 = (0..60).map(|_| fs.advance(1.0 / 60.0 + 1e-6)).sum();
        assert_eq!(total, 60);
    }

    #[test]
    fn slow_frames_run_several_steps() {
        let mut fs = FixedStep::default();
        assert_eq!(fs.advance(DT * 3.5), 3);
        assert!((fs.alpha() - 0.5).abs() < 1e-3);
    }

    #[test]
    fn long_stall_is_capped() {
        let mut fs = FixedStep::default();
        assert_eq!(fs.advance(10.0), FixedStep::MAX_STEPS_PER_FRAME);
        assert!(fs.advance(0.0) <= 1);
    }

    fn press(code: &str) -> InputFrame {
        InputFrame::from_code(code).unwrap()
    }

    #[test]
    fn sim_step_counts_ticks() {
        let mut sim = Sim::new();
        for _ in 0..10 {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(sim.tick, 10);
    }

    #[test]
    fn jump_pressed_is_an_edge() {
        let mut sim = Sim::new();
        sim.step(press("J"));
        assert!(sim.jump_pressed());
        sim.step(press("J"));
        assert!(!sim.jump_pressed());
        sim.step(press("-"));
        sim.step(press("J"));
        assert!(sim.jump_pressed());
    }

    #[test]
    fn prev_pos_tracks_last_step() {
        let mut sim = Sim::new();
        sim.step(press("R"));
        let before = sim.ball.pos;
        sim.step(press("R"));
        assert_eq!(sim.ball.prev_pos, before);
        assert!(sim.ball.render_pos(0.5).x > before.x);
    }

    #[test]
    fn same_inputs_same_state() {
        let inputs: Vec<_> = ["R", "R", "RJ", "-", "L", "LJ", "-"]
            .iter()
            .cycle()
            .take(500)
            .map(|c| press(c))
            .collect();
        let run = || {
            let mut sim = Sim::new();
            for &i in &inputs {
                sim.step(i);
            }
            sim
        };
        assert_eq!(run(), run());
    }

    fn flat() -> Sim {
        let mut sim = Sim::for_level("flat").unwrap();
        for _ in 0..5 {
            sim.step(InputFrame::NONE);
        }
        sim
    }

    #[test]
    fn spikes_kill_then_respawn_at_the_start() {
        let mut sim = Sim::for_level("hazards").unwrap();
        let start = sim.ball.pos;
        let mut died_at = None;
        for i in 0..200 {
            sim.step(press("R"));
            if let Some(Event::Died { cause, .. }) = sim.events.first().copied() {
                assert_eq!(cause, Cause::Spikes);
                died_at = Some(i);
                break;
            }
        }
        assert!(died_at.is_some(), "never hit the spikes");
        assert_eq!(sim.deaths, 1);
        let frozen = sim.ball.pos;
        for _ in 0..DEATH_TICKS - 1 {
            sim.step(press("R"));
            assert_eq!(sim.ball.pos, frozen, "input ignored while dying");
            assert!(sim.dying > 0);
        }
        sim.step(press("R"));
        assert!(
            matches!(sim.events[..], [Event::Respawned { .. }]),
            "{:?}",
            sim.events
        );
        assert_eq!(sim.dying, 0);
        assert_eq!(sim.ball.pos, start);
        assert_eq!(sim.respawn_anim, RESPAWN_ANIM_TICKS);
    }

    /// Runs `code` until the ball dies (or panics after `limit` steps).
    fn until_death(sim: &mut Sim, code: &str, limit: usize) {
        let deaths = sim.deaths;
        for _ in 0..limit {
            sim.step(press(code));
            if sim.deaths > deaths {
                return;
            }
        }
        panic!("no death within {limit} steps");
    }

    fn wait(sim: &mut Sim, steps: u32) {
        for _ in 0..steps {
            sim.step(InputFrame::NONE);
        }
    }

    #[test]
    fn losing_every_life_is_game_over_then_a_fresh_restart() {
        let mut sim = Sim::for_level("hazards").unwrap();
        assert_eq!(sim.lives, START_LIVES);
        for left in (1..START_LIVES).rev() {
            until_death(&mut sim, "R", 200);
            assert_eq!(sim.lives, left);
            wait(&mut sim, DEATH_TICKS);
            assert_eq!(sim.dying, 0, "respawned with lives left");
        }
        until_death(&mut sim, "R", 200);
        assert_eq!(sim.lives, 0);
        wait(&mut sim, DEATH_TICKS);
        assert!(sim.game_over > 0, "game over instead of a respawn");
        assert!(!sim.ball_visible(), "no ball during game over");
        let tick = sim.tick;
        wait(&mut sim, GAME_OVER_TICKS);
        assert!(sim.events.contains(&Event::LevelRestarted));
        assert_eq!(sim.lives, START_LIVES);
        assert_eq!(sim.deaths, 0);
        assert_eq!(sim.restarts, 1);
        assert_eq!(sim.started_tick, sim.tick);
        assert_eq!(
            sim.tick,
            tick + GAME_OVER_TICKS as u64,
            "tick keeps counting"
        );
        assert_eq!(sim.ball.pos, sim.level.map.spawn);
    }

    #[test]
    fn checkpoint_moves_the_respawn_point() {
        let mut sim = Sim::for_level("hazards").unwrap();
        // Clear the floor spikes (jump window 13-30) and pass the checkpoint.
        for t in 0..60 {
            sim.step(press(if t == 20 { "RJ" } else { "R" }));
        }
        assert!(
            sim.entities
                .iter()
                .any(|e| e.kind == EntityKind::Checkpoint { active: true }),
            "checkpoint not touched"
        );
        let cp = sim.respawn_point;
        assert!(cp.x > sim.level.map.spawn.x);
        until_death(&mut sim, "R", 300); // into the pit
        wait(&mut sim, DEATH_TICKS);
        assert_eq!(sim.ball.pos, cp);
    }

    #[test]
    fn collecting_scores_and_counts() {
        let mut sim = Sim::with_level(Level::from_ascii("o*e+.\n#####").unwrap());
        assert_eq!((sim.gems_total, sim.eggs_total), (1, 1));
        for _ in 0..90 {
            sim.step(press("R"));
        }
        assert_eq!((sim.gems, sim.eggs), (1, 1));
        assert_eq!(sim.lives, START_LIVES + 1);
        assert_eq!(sim.score, 10 + 500 + 100);
    }

    #[test]
    fn carry_sets_start_values_and_game_over_returns_to_them() {
        let level = Level::load("hazards").unwrap();
        let mut sim = Sim::with_level_carry(
            level,
            Carry {
                lives: 1,
                score: 700,
            },
        );
        assert_eq!((sim.lives, sim.score), (1, 700));
        until_death(&mut sim, "R", 200);
        wait(&mut sim, DEATH_TICKS + GAME_OVER_TICKS);
        assert_eq!(sim.restarts, 1);
        assert_eq!((sim.lives, sim.score), (1, 700));
    }

    #[test]
    fn reaching_the_exit_freezes_the_level() {
        let mut sim = Sim::for_level("flat").unwrap();
        while !sim.exit_reached {
            sim.step(press("R"));
        }
        let (pos, tick) = (sim.ball.pos, sim.tick);
        for _ in 0..30 {
            sim.step(press("L"));
        }
        assert_eq!(sim.ball.pos, pos);
        assert_eq!(sim.tick, tick + 30, "ticks still count for the game flow");
        assert_eq!(
            sim.level_seconds(),
            (tick as f32) / 60.0,
            "but the level clock stopped"
        );
    }

    fn pool_drop() -> Sim {
        let mut sim = Sim::for_level("water").unwrap();
        sim.ball.pos = macroquad::math::vec2(192.0, 150.0);
        sim
    }

    #[test]
    fn the_ball_floats_and_settles() {
        let mut sim = pool_drop();
        for _ in 0..240 {
            sim.step(InputFrame::NONE);
        }
        let w = sim.ball.water;
        // Equilibrium: 1 / buoyancy of the ball under water.
        assert!((w - 1.0 / 1.6).abs() < 0.08, "submersion {w}");
        assert!(
            sim.ball.vel.y.abs() < 5.0,
            "still bobbing: {}",
            sim.ball.vel.y
        );
        assert_eq!(sim.deaths, 0);
    }

    #[test]
    fn landing_in_water_splashes_and_bobs() {
        let mut sim = pool_drop();
        let mut splash = false;
        let mut lowest: f32 = 0.0;
        for _ in 0..120 {
            sim.step(InputFrame::NONE);
            splash |= sim.events.iter().any(|e| matches!(e, Event::Splash { .. }));
            lowest = lowest.max(sim.ball.pos.y);
        }
        assert!(splash);
        assert!(
            lowest > 230.0,
            "sank in a little before bobbing up: {lowest}"
        );
    }

    #[test]
    fn rolling_into_the_pool_splashes() {
        let mut sim = Sim::for_level("water").unwrap();
        let mut splash = false;
        for _ in 0..60 {
            sim.step(press("R"));
            splash |= sim.events.iter().any(|e| matches!(e, Event::Splash { .. }));
        }
        assert!(splash);
    }

    #[test]
    fn can_jump_out_of_the_water() {
        let mut sim = pool_drop();
        for _ in 0..240 {
            sim.step(InputFrame::NONE);
        }
        sim.step(press("J"));
        let mut left_water = false;
        for _ in 0..30 {
            sim.step(InputFrame::NONE);
            left_water |= sim.ball.water == 0.0 && sim.ball.pos.y < 200.0;
        }
        assert!(left_water);
    }

    #[test]
    fn swimming_is_slower() {
        let mut sim = pool_drop();
        for _ in 0..240 {
            sim.step(InputFrame::NONE);
        }
        for _ in 0..60 {
            sim.step(press("R"));
        }
        let vx = sim.ball.vel.x;
        assert!(vx > 20.0 && vx < 120.0 * 0.75, "vx {vx}");
    }

    fn platforms_level() -> Sim {
        Sim::for_level("platforms").unwrap()
    }

    fn ferry(sim: &Sim) -> &platforms::Platform {
        sim.entities
            .iter()
            .find_map(|e| match &e.kind {
                EntityKind::Platform(p) if p.path[1].y == p.path[0].y => Some(p),
                _ => None,
            })
            .unwrap()
    }

    /// Drops the ball onto the ferry and waits until it rides.
    fn on_ferry() -> Sim {
        let mut sim = platforms_level();
        let top = ferry(&sim).top_centre;
        sim.ball.pos = top - macroquad::math::vec2(0.0, 30.0);
        // A drop this high bounces once or twice before it settles.
        for _ in 0..90 {
            sim.step(InputFrame::NONE);
            if sim.riding.is_some() {
                return sim;
            }
        }
        panic!("never settled on the ferry");
    }

    #[test]
    fn riding_carries_the_ball() {
        let mut sim = on_ferry();
        // A ball dropped straight down slides back a little until friction
        // matches it to the platform; measure after that.
        for _ in 0..10 {
            sim.step(InputFrame::NONE);
        }
        let (x0, px0) = (sim.ball.pos.x, ferry(&sim).top_centre.x);
        for _ in 0..60 {
            sim.step(InputFrame::NONE);
            assert!(sim.riding.is_some());
            let top = ferry(&sim).top_centre.y;
            assert_eq!(sim.ball.pos.y, top - 8.0, "stays on top");
        }
        let moved = sim.ball.pos.x - x0;
        let plat_moved = ferry(&sim).top_centre.x - px0;
        assert!(plat_moved > 30.0);
        assert!((moved - plat_moved).abs() < 0.01, "{moved} vs {plat_moved}");
    }

    #[test]
    fn jumping_off_keeps_the_platform_speed() {
        let mut sim = on_ferry();
        for _ in 0..10 {
            sim.step(InputFrame::NONE);
        }
        let plat_vx = ferry(&sim).vel.x;
        assert!(plat_vx > 30.0);
        sim.step(press("J"));
        assert!(sim.riding.is_none());
        assert!(
            (sim.ball.vel.x - plat_vx).abs() < 2.0,
            "{} vs {plat_vx}",
            sim.ball.vel.x
        );
    }

    #[test]
    fn rolling_off_the_end_falls() {
        let mut sim = on_ferry();
        let mut fell = false;
        for _ in 0..90 {
            sim.step(press("L"));
            fell |= sim.riding.is_none() && sim.ball.pos.y > ferry(&sim).top_centre.y;
        }
        assert!(fell);
    }

    #[test]
    fn platforms_are_one_way() {
        let mut sim = platforms_level();
        let top = ferry(&sim).top_centre;
        // Start just below the ferry, moving up fast: passes through.
        sim.ball.pos = top + macroquad::math::vec2(0.0, 30.0);
        sim.ball.vel.y = -320.0;
        let mut landed_on_top = false;
        for _ in 0..60 {
            sim.step(InputFrame::NONE);
            landed_on_top |= sim.riding.is_some();
        }
        assert!(landed_on_top, "came up through and landed on it");
    }

    #[test]
    fn the_lift_raises_the_ball() {
        let mut sim = platforms_level();
        let lift_top = sim
            .entities
            .iter()
            .find_map(|e| match &e.kind {
                EntityKind::Platform(p) if p.path[1].x == p.path[0].x => Some(p.top_centre),
                _ => None,
            })
            .unwrap();
        sim.ball.pos = lift_top - macroquad::math::vec2(0.0, 20.0);
        let mut highest = sim.ball.pos.y;
        for _ in 0..150 {
            sim.step(InputFrame::NONE);
            highest = highest.min(sim.ball.pos.y);
        }
        assert!(
            highest <= lift_top.y - 64.0 - 8.0 + 0.5,
            "highest {highest}"
        );
    }

    fn switches_level() -> Sim {
        Sim::for_level("switches").unwrap()
    }

    #[test]
    fn crumbling_block_falls_after_standing_then_returns() {
        let mut sim = switches_level();
        // Stand on the first crumbling block (cell 8,14).
        sim.ball.pos = macroquad::math::vec2(8.5 * 16.0, 14.0 * 16.0 - 8.0);
        let mut fell = false;
        for _ in 0..(CRUMBLE_SHAKE_TICKS + 20) {
            sim.step(InputFrame::NONE);
            fell |= sim
                .events
                .iter()
                .any(|e| matches!(e, Event::Crumbled { .. }));
        }
        assert!(fell);
        assert_eq!(sim.level.map.get(8, 14), map::Tile::Empty);
        // The ball drops into the pit and dies; the block comes back later.
        for _ in 0..CRUMBLE_RETURN_TICKS {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(sim.deaths, 1);
        assert_eq!(sim.level.map.get(8, 14), map::Tile::Solid);
        assert!(sim.crumbles.is_empty());
    }

    #[test]
    fn rolling_fast_crosses_a_crumbling_bridge() {
        let mut sim = switches_level();
        for _ in 0..150 {
            sim.step(press("R"));
        }
        assert_eq!(sim.deaths, 0);
        assert!(sim.ball.pos.x > 14.0 * 16.0, "x {}", sim.ball.pos.x);
        assert!(
            !sim.crumbles.is_empty(),
            "the bridge started crumbling behind"
        );
    }

    #[test]
    fn switch_opens_the_door_and_builds_the_bridge() {
        let mut sim = switches_level();
        assert!(sim.level.map.is_solid(20, 12), "door closed");
        assert!(!sim.level.map.is_solid(27, 14), "no bridge yet");
        // Put the ball just left of the switch and roll onto it.
        sim.ball.pos = macroquad::math::vec2(15.5 * 16.0, 14.0 * 16.0 - 8.0);
        let mut switched = false;
        for _ in 0..30 {
            sim.step(press("R"));
            switched |= sim
                .events
                .iter()
                .any(|e| matches!(e, Event::Switched { .. }));
        }
        assert!(switched);
        assert!(!sim.level.map.is_solid(20, 12), "door open");
        assert!(sim.level.map.is_solid(27, 14), "bridge built");
        for _ in 0..240 {
            sim.step(press("R"));
        }
        assert!(sim.exit_reached, "reaches the exit at x {}", sim.ball.pos.x);
    }

    #[test]
    fn closed_door_blocks_the_way() {
        let mut sim = switches_level();
        sim.ball.pos = macroquad::math::vec2(18.5 * 16.0, 14.0 * 16.0 - 8.0);
        for _ in 0..120 {
            sim.step(press("R"));
        }
        assert!(sim.ball.pos.x < 20.0 * 16.0, "x {}", sim.ball.pos.x);
    }

    #[test]
    fn only_a_smashing_ball_breaks_cracked_blocks() {
        for smashes in [false, true] {
            let mut sim = switches_level();
            sim.ball.params.smashes = smashes;
            // Just right of the exit area, rolling hard at the wall (x 560).
            sim.ball.pos = macroquad::math::vec2(553.0, 14.0 * 16.0 - 8.0);
            sim.ball.vel.x = 250.0;
            for _ in 0..10 {
                sim.step(press("R"));
            }
            assert_eq!(
                !sim.level.map.is_solid(35, 13),
                smashes,
                "smashes={smashes}"
            );
        }
    }

    #[test]
    fn game_over_restores_broken_blocks() {
        let mut sim = switches_level();
        sim.lives = 1;
        sim.ball.pos = macroquad::math::vec2(8.5 * 16.0, 14.0 * 16.0 - 8.0);
        for _ in 0..(CRUMBLE_SHAKE_TICKS + 60 + DEATH_TICKS + GAME_OVER_TICKS) {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(sim.restarts, 1);
        assert_eq!(sim.level.map.get(8, 14), map::Tile::Solid);
        assert!(sim.crumbles.is_empty());
    }

    fn launchers_level() -> Sim {
        Sim::for_level("launchers").unwrap()
    }

    #[test]
    fn spring_launches_to_its_height() {
        let mut sim = launchers_level();
        // Drop straight onto the spring (x 96..112, floor y 288).
        sim.ball.pos = macroquad::math::vec2(104.0, 250.0);
        let mut launched_from = None;
        let mut highest = f32::MAX;
        for _ in 0..90 {
            sim.step(InputFrame::NONE);
            if launched_from.is_none()
                && let Some(Event::Launched { .. }) = sim.events.first()
            {
                launched_from = Some(sim.ball.pos.y);
            }
            highest = highest.min(sim.ball.pos.y);
        }
        let from = launched_from.expect("never launched");
        let rise = from - highest;
        assert!((rise - 96.0).abs() < 2.0, "rose {rise}");
    }

    #[test]
    fn catapult_holds_then_flings() {
        let mut sim = launchers_level();
        sim.ball.pos = macroquad::math::vec2(240.0, 280.0);
        let mut held_steps = 0;
        let mut fired_vel = None;
        for _ in 0..60 {
            sim.step(press("R"));
            if sim.held.is_some() {
                held_steps += 1;
            }
            if sim
                .events
                .iter()
                .any(|e| matches!(e, Event::Launched { .. }))
            {
                fired_vel = Some(sim.ball.vel);
            }
        }
        // The catch step itself counts as held.
        assert_eq!(held_steps, entities::CATAPULT_HOLD_TICKS);
        let v = fired_vel.expect("never fired");
        assert!(v.x > 200.0 && v.y < -250.0, "{v:?}");
    }

    #[test]
    fn fan_lifts_and_holds_the_ball_at_the_top() {
        let mut sim = launchers_level();
        sim.ball.pos = macroquad::math::vec2(552.0, 280.0);
        for _ in 0..180 {
            sim.step(InputFrame::NONE);
        }
        // The air stream ends at y 160: the ball bobs around there.
        assert!(
            (sim.ball.pos.y - 160.0).abs() < 16.0,
            "y {}",
            sim.ball.pos.y
        );
        assert!(sim.ball.vel.y.abs() <= ball::MAX_WIND_RISE);
    }

    #[test]
    fn dying_releases_a_held_ball() {
        let mut sim = launchers_level();
        sim.held = Some((0, 5));
        sim.die(Cause::Spikes);
        assert!(sim.held.is_none());
    }

    fn crate_pos(sim: &Sim) -> Vec2 {
        sim.entities
            .iter()
            .find(|e| matches!(e.kind, EntityKind::Crate(_)))
            .unwrap()
            .pos
    }

    #[test]
    fn the_ball_pushes_a_crate_at_crate_speed() {
        let mut sim = Sim::for_level("crates").unwrap();
        for _ in 0..90 {
            sim.step(press("R"));
        }
        let x0 = crate_pos(&sim).x;
        for _ in 0..60 {
            sim.step(press("R"));
        }
        let moved = crate_pos(&sim).x - x0;
        assert!(
            (moved - crates::PUSH_SPEED).abs() < 3.0,
            "moved {moved} in 1 s"
        );
        // The ball keeps pace, pressed against the crate's side.
        assert!((crate_pos(&sim).x - (sim.ball.pos.x + 8.0)).abs() < 1.0);
    }

    #[test]
    fn the_wall_needs_the_crate() {
        let mut sim = Sim::for_level("crates").unwrap();
        // Drop the ball between the crate and the wall and try jumping.
        sim.ball.pos = macroquad::math::vec2(300.0, 200.0);
        for _ in 0..300 {
            sim.step(press("RJ"));
        }
        assert!(sim.ball.pos.x < 320.0, "got over: {:?}", sim.ball.pos);
    }

    #[test]
    fn standing_on_a_crate_over_spikes_is_safe() {
        let level = Level::from_ascii("o......\n.......\n...^...\n#######").unwrap();
        let mut sim = Sim::with_level(level);
        sim.entities.push(
            entities::build(&crate::level::ObjectSpec {
                kind: "crate".into(),
                name: "crate".into(),
                pos: macroquad::math::vec2(48.0, 0.0),
                size: Vec2::splat(16.0),
                points: Vec::new(),
                closed: false,
                props: Vec::new(),
            })
            .unwrap(),
        );
        for _ in 0..30 {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(
            crate_pos(&sim),
            macroquad::math::vec2(48.0, 32.0),
            "rests on the spike cell's floor"
        );
        sim.ball.pos = macroquad::math::vec2(56.0, 0.0);
        for _ in 0..60 {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(sim.deaths, 0);
        assert!(sim.ball.grounded);
        assert_eq!(sim.ball.pos.y, 32.0 - 8.0, "on top of the crate");
    }

    #[test]
    fn game_over_puts_crates_back() {
        let mut sim = Sim::for_level("crates").unwrap();
        let start = crate_pos(&sim);
        for _ in 0..150 {
            sim.step(press("R"));
        }
        assert_ne!(crate_pos(&sim), start);
        sim.restart_level();
        assert_eq!(crate_pos(&sim), start);
    }

    fn seesaw(sim: &Sim) -> &seesaw::Seesaw {
        sim.entities
            .iter()
            .find_map(|e| match &e.kind {
                EntityKind::Seesaw(w) => Some(w),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn landing_on_one_side_tips_the_seesaw_that_way() {
        for (x, sign) in [(184.0, -1.0), (248.0, 1.0)] {
            let mut sim = Sim::for_level("seesaws").unwrap();
            sim.ball.pos = macroquad::math::vec2(x, 180.0);
            for _ in 0..60 {
                sim.step(InputFrame::NONE);
            }
            let a = seesaw(&sim).angle.to_degrees();
            assert!(a * sign > 10.0, "x {x}: angle {a}");
        }
    }

    #[test]
    fn a_ball_on_a_tilted_plank_rolls_downhill() {
        let mut sim = Sim::for_level("seesaws").unwrap();
        for e in &mut sim.entities {
            if let EntityKind::Seesaw(w) = &mut e.kind {
                w.angle = 20f32.to_radians(); // right end down
            }
        }
        // Drop onto the high (left) part, a little left of the pivot.
        sim.ball.pos = macroquad::math::vec2(200.0, 180.0);
        let mut rolled_right = false;
        for _ in 0..40 {
            sim.step(InputFrame::NONE);
            rolled_right |= sim.on_seesaw.is_some() && sim.ball.vel.x > 20.0;
        }
        assert!(rolled_right);
    }

    #[test]
    fn heavier_ball_tips_the_seesaw_faster() {
        let angle_after = |mass: f32| {
            let mut sim = Sim::for_level("seesaws").unwrap();
            sim.ball.params.mass = mass;
            sim.ball.pos = macroquad::math::vec2(250.0, 210.0);
            for _ in 0..12 {
                sim.step(InputFrame::NONE);
            }
            seesaw(&sim).angle
        };
        assert!(angle_after(3.0) > angle_after(1.0));
    }

    fn wheels_level() -> Sim {
        Sim::for_level("wheels").unwrap()
    }

    fn cannon_index(sim: &Sim, auto: bool) -> usize {
        sim.entities
            .iter()
            .position(|e| matches!(e.kind, EntityKind::Cannon { auto: a, .. } if a == auto))
            .unwrap()
    }

    #[test]
    fn auto_cannon_hides_then_fires_after_its_delay() {
        let mut sim = wheels_level();
        let i = cannon_index(&sim, true);
        let pivot = sim.entities[i].centre();
        sim.ball.pos = pivot - macroquad::math::vec2(10.0, 0.0);
        sim.step(press("R"));
        assert_eq!(sim.held.map(|h| h.0), Some(i));
        assert!(!sim.ball_visible(), "hidden inside the cannon");
        let mut fired_at = None;
        for t in 0..120 {
            sim.step(InputFrame::NONE);
            if sim
                .events
                .iter()
                .any(|e| matches!(e, Event::Launched { .. }))
            {
                fired_at = Some(t);
                break;
            }
        }
        let t = fired_at.expect("never fired");
        assert!((25..=35).contains(&t), "fired after {t} steps");
        assert!(sim.ball_visible());
        // -60°, 500 px/s.
        assert!(
            sim.ball.vel.y < -400.0 && sim.ball.vel.x > 200.0,
            "{:?}",
            sim.ball.vel
        );
    }

    #[test]
    fn jump_cannon_waits_for_a_press() {
        let mut sim = wheels_level();
        let i = cannon_index(&sim, false);
        sim.ball.pos = sim.entities[i].centre();
        for _ in 0..120 {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(sim.held.map(|h| h.0), Some(i), "still waiting");
        sim.step(press("J"));
        assert!(sim.held.is_none());
        assert!(sim.ball.vel.length() > 400.0);
    }

    #[test]
    fn wheel_cars_carry_a_rider_round() {
        let mut sim = wheels_level();
        let car = sim
            .entities
            .iter()
            .position(|e| matches!(e.kind, EntityKind::Platform(_)))
            .unwrap();
        sim.ball.pos = sim.entities[car].pos + macroquad::math::vec2(16.0, -12.0);
        let mut rode = 0;
        let mut lowest_y = f32::MIN;
        let mut highest_y = f32::MAX;
        for _ in 0..240 {
            sim.step(InputFrame::NONE);
            if sim.riding == Some(car) {
                rode += 1;
                lowest_y = lowest_y.max(sim.ball.pos.y);
                highest_y = highest_y.min(sim.ball.pos.y);
            }
        }
        assert!(rode > 150, "rode {rode} steps");
        assert!(
            lowest_y - highest_y > 40.0,
            "went round: {highest_y}..{lowest_y}"
        );
    }

    #[test]
    fn a_spinner_tips_a_passive_ball_off() {
        let mut sim = wheels_level();
        let centre = sim
            .entities
            .iter()
            .find_map(|e| match &e.kind {
                EntityKind::Seesaw(w) if w.motor.is_some() => Some(w.pivot),
                _ => None,
            })
            .unwrap();
        sim.ball.pos = centre - macroquad::math::vec2(0.0, 14.0);
        let mut was_on = false;
        let mut fell = false;
        for _ in 0..360 {
            sim.step(InputFrame::NONE);
            was_on |= sim.on_seesaw.is_some();
            fell |= sim.deaths > 0;
        }
        assert!(was_on && fell, "on {was_on}, fell {fell}");
    }

    fn forms_level() -> Sim {
        Sim::for_level("forms").unwrap()
    }

    #[test]
    fn transformer_changes_form_once_with_an_event() {
        let mut sim = forms_level();
        let mut transformed = 0;
        for _ in 0..40 {
            sim.step(press("R"));
            transformed += sim
                .events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        Event::Transformed {
                            form: ball::Form::Stone,
                            ..
                        }
                    )
                })
                .count();
        }
        assert_eq!(sim.ball.form, ball::Form::Stone);
        assert_eq!(sim.ball.params, ball::BallParams::STONE);
        assert_eq!(transformed, 1, "touching it longer doesn't re-trigger");
    }

    #[test]
    fn timed_form_runs_out_back_to_normal() {
        let level = Level::from_ascii("o2...\n#####").unwrap();
        let mut sim = Sim::with_level(level);
        for e in &mut sim.entities {
            if let EntityKind::Transformer { duration, .. } = &mut e.kind {
                *duration = 90;
            }
        }
        let mut became = None;
        for t in 0..200 {
            sim.step(press("R"));
            if became.is_none() && sim.ball.form == ball::Form::Balloon {
                became = Some(t);
            }
            if let Some(start) = became
                && sim.ball.form == ball::Form::Normal
            {
                // Touching the transformer keeps refreshing the timer, so it
                // runs 90 steps after the ball has rolled through it (32 px
                // at ~120 px/s ≈ 16-20 steps).
                let lasted = t - start;
                assert!((90..=115).contains(&lasted), "lasted {lasted}");
                return;
            }
        }
        panic!("never ran out");
    }

    #[test]
    fn respawn_keeps_the_checkpoint_form() {
        let mut sim = forms_level();
        for _ in 0..70 {
            sim.step(press("R"));
        }
        assert_eq!(sim.ball.form, ball::Form::Stone);
        assert_eq!(
            sim.respawn_form,
            ball::Form::Stone,
            "checkpoint touched as stone"
        );
        sim.die(Cause::Spikes);
        wait(&mut sim, DEATH_TICKS);
        assert_eq!(sim.ball.form, ball::Form::Stone);
        assert_eq!(sim.ball.params, ball::BallParams::STONE);
    }

    #[test]
    fn the_red_ball_cannot_break_the_cracked_wall() {
        let mut sim = forms_level();
        sim.ball.pos = macroquad::math::vec2(160.0, 232.0); // past the transformer
        for _ in 0..120 {
            sim.step(press("R"));
        }
        assert!(sim.ball.pos.x < 192.0, "x {}", sim.ball.pos.x);
        assert!(sim.level.map.is_solid(12, 14));
    }

    #[test]
    fn only_the_balloon_clears_the_high_wall() {
        for (form, clears) in [(ball::Form::Normal, false), (ball::Form::Balloon, true)] {
            let mut sim = forms_level();
            sim.ball = Ball::with_form(macroquad::math::vec2(504.0, 232.0), form);
            for _ in 0..240 {
                sim.step(press("RJ"));
            }
            assert_eq!(
                sim.ball.pos.x > 528.0,
                clears,
                "{form:?} at {:?}",
                sim.ball.pos
            );
        }
    }

    fn stone_at(sim: &mut Sim, x: f32, y: f32) {
        sim.ball = Ball::with_form(macroquad::math::vec2(x, y), ball::Form::Stone);
    }

    #[test]
    fn stone_sinks_to_the_bottom_and_rolls_slower_there() {
        let mut sim = Sim::for_level("stone").unwrap();
        stone_at(&mut sim, 340.0, 200.0); // over the pool (bed top y 288)
        for _ in 0..120 {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(sim.ball.pos.y, 280.0, "on the bed");
        assert!(sim.ball.grounded);
        // A short roll (the pool's normal transformer is further right).
        for _ in 0..25 {
            sim.step(press("R"));
        }
        assert_eq!(sim.ball.form, ball::Form::Stone);
        let under_water = sim.ball.vel.x;
        assert!(under_water > 20.0 && under_water < 90.0, "vx {under_water}");
    }

    #[test]
    fn normal_ball_floats_up_from_a_pool_bed() {
        let mut sim = Sim::for_level("stone").unwrap();
        sim.ball.pos = macroquad::math::vec2(340.0, 280.0);
        sim.ball.grounded = true;
        for _ in 0..120 {
            sim.step(InputFrame::NONE);
        }
        assert!(sim.ball.pos.y < 236.0, "floated up to y {}", sim.ball.pos.y);
    }

    #[test]
    fn fans_barely_move_a_stone() {
        let mut sim = Sim::for_level("stone").unwrap();
        stone_at(&mut sim, 488.0, 216.0); // standing in the fan's stream
        let mut highest = f32::MAX;
        for _ in 0..120 {
            sim.step(InputFrame::NONE);
            highest = highest.min(sim.ball.pos.y);
        }
        assert!(highest > 200.0, "lifted to {highest}");
    }

    #[test]
    fn rolling_over_cracked_floor_keeps_it_landing_breaks_it() {
        let mut sim = Sim::for_level("stone").unwrap();
        stone_at(&mut sim, 120.0, 216.0);
        for _ in 0..90 {
            sim.step(press("R"));
        }
        assert!(
            (9..=11).all(|x| sim.level.map.is_solid(x, 14)),
            "rolling broke it"
        );
        stone_at(&mut sim, 168.0, 150.0); // drop onto the middle cell
        for _ in 0..40 {
            sim.step(InputFrame::NONE);
        }
        assert!(!sim.level.map.is_solid(10, 14), "landing should break it");
    }

    #[test]
    fn stone_bounces_less_and_cannot_jump_two_tiles() {
        let bounce = |form| {
            let mut sim = Sim::for_level("flat").unwrap();
            sim.ball = Ball::with_form(macroquad::math::vec2(120.0, 180.0), form);
            let mut peak_after = f32::MAX;
            let mut landed = false;
            for _ in 0..90 {
                sim.step(InputFrame::NONE);
                landed |= sim.ball.pos.y >= 279.0;
                if landed {
                    peak_after = peak_after.min(sim.ball.pos.y);
                }
            }
            280.0 - peak_after
        };
        assert!(bounce(ball::Form::Stone) < bounce(ball::Form::Normal) / 3.0);
    }

    fn balloon_at(sim: &mut Sim, x: f32, y: f32) {
        sim.ball = Ball::with_form(macroquad::math::vec2(x, y), ball::Form::Balloon);
    }

    #[test]
    fn balloon_floats_up_while_jump_is_held_and_sinks_slowly() {
        let mut sim = Sim::for_level("balloon").unwrap();
        balloon_at(&mut sim, 272.0, 400.0); // mid-shaft, right side
        let y0 = sim.ball.pos.y;
        // (A jump off the floor starts faster than max_rise; only the
        // floating lift itself is capped.)
        for _ in 0..60 {
            sim.step(press("J"));
        }
        assert!(
            y0 - sim.ball.pos.y > 60.0,
            "rose only {}",
            y0 - sim.ball.pos.y
        );
        for _ in 0..90 {
            sim.step(InputFrame::NONE);
            assert!(sim.ball.vel.y <= ball::BallParams::BALLOON.max_fall + 1e-3);
        }
        assert!(sim.ball.vel.y > 0.0, "sinks again");
    }

    #[test]
    fn the_red_ball_cannot_climb_the_balloon_shaft() {
        let mut sim = Sim::for_level("balloon").unwrap();
        sim.ball.pos = macroquad::math::vec2(272.0, 472.0);
        let mut highest = f32::MAX;
        for _ in 0..300 {
            sim.step(press("RJ"));
            highest = highest.min(sim.ball.pos.y);
        }
        assert!(highest > 400.0, "reached {highest}");
    }

    #[test]
    fn balloon_pops_on_spikes() {
        let mut sim = Sim::for_level("balloon").unwrap();
        balloon_at(&mut sim, 200.0, 420.0); // under the spiked ledge
        let mut cause = None;
        for _ in 0..120 {
            sim.step(press("J"));
            if let Some(Event::Died { cause: c, .. }) = sim.events.first() {
                cause = Some(*c);
                break;
            }
        }
        assert_eq!(cause, Some(Cause::Spikes));
    }

    #[test]
    fn balloon_rides_high_in_water_and_fans_fling_it() {
        let mut sim = Sim::for_level("water").unwrap();
        balloon_at(&mut sim, 192.0, 200.0);
        for _ in 0..240 {
            sim.step(InputFrame::NONE);
        }
        assert!(
            sim.ball.water > 0.0 && sim.ball.water < 0.35,
            "w {}",
            sim.ball.water
        );

        let mut sim = Sim::for_level("launchers").unwrap();
        balloon_at(&mut sim, 552.0, 280.0); // in the fan's stream
        let mut fastest: f32 = 0.0;
        for _ in 0..30 {
            sim.step(InputFrame::NONE);
            fastest = fastest.max(-sim.ball.vel.y);
        }
        assert!(fastest >= ball::MAX_WIND_RISE - 1.0, "rose at {fastest}");
    }

    #[test]
    fn lives_are_capped() {
        let mut sim = Sim::with_level(Level::from_ascii("o+++++++++.\n###########").unwrap());
        for _ in 0..200 {
            sim.step(press("R"));
        }
        assert_eq!(sim.lives, MAX_LIVES);
    }

    #[test]
    fn falling_out_of_the_level_kills() {
        let mut sim = Sim::for_level("hazards").unwrap();
        sim.ball.pos = macroquad::math::vec2(288.0, 200.0); // above the pit
        let mut cause = None;
        for _ in 0..120 {
            sim.step(InputFrame::NONE);
            if let Some(Event::Died { cause: c, .. }) = sim.events.first().copied() {
                cause = Some(c);
                break;
            }
        }
        assert_eq!(cause, Some(Cause::Pit));
    }

    #[test]
    fn hard_landing_emits_event_and_squashes() {
        let mut sim = flat();
        sim.ball.pos.y -= 120.0;
        sim.ball.grounded = false;
        let mut landed = None;
        for _ in 0..60 {
            sim.step(InputFrame::NONE);
            if let Some(Event::Landed { pos, speed }) = sim.events.first().copied() {
                landed = Some((pos, speed, sim.ball.squash));
                break;
            }
        }
        let (pos, speed, squash) = landed.expect("no Landed event");
        assert!(speed > 300.0, "speed {speed}");
        assert_eq!(pos.y, 288.0, "contact point is on the floor");
        assert!(squash > 0.2, "squash {squash}");
        for _ in 0..120 {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(sim.ball.squash, 0.0, "springs back to round");
    }

    #[test]
    fn squash_wobbles_past_round() {
        let mut sim = flat();
        sim.ball.pos.y -= 120.0;
        sim.ball.grounded = false;
        let mut min_squash: f32 = 0.0;
        for _ in 0..90 {
            sim.step(InputFrame::NONE);
            min_squash = min_squash.min(sim.ball.squash);
        }
        assert!(min_squash < -0.02, "no rebound stretch: {min_squash}");
    }

    #[test]
    fn jump_emits_event_and_stretches() {
        let mut sim = flat();
        sim.step(press("J"));
        assert!(
            matches!(sim.events[..], [Event::Jumped { .. }]),
            "{:?}",
            sim.events
        );
        assert!(sim.ball.squash < -0.1, "squash {}", sim.ball.squash);
    }

    #[test]
    fn rolling_emits_no_landings() {
        let mut sim = Sim::for_level("slopes").unwrap();
        for _ in 0..300 {
            sim.step(press("R"));
            assert!(
                !sim.events.iter().any(|e| matches!(e, Event::Landed { .. })),
                "landing while rolling at x={}",
                sim.ball.pos.x
            );
        }
    }
}
