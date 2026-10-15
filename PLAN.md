# Bounce Tales (Rust) — 40-Step Build Plan

## Context
Goal: recreate the 2008 Nokia game **Bounce Tales** (red ball, Hypno Cube villain) as a native Linux game in Rust, built over **40 prompts**. Each prompt does one focused step and tests it; the user reviews and commits. The repo is currently a bare `cargo new` (`Cargo.toml`, `src/main.rs`), with Rust 1.99 installed.

Gameplay is copied as closely as possible. All art, audio and level files are **our own originals** in the same style, so the repo is safe to share.

### Decisions (from Q&A)
| Topic | Choice |
|---|---|
| Engine | **macroquad** |
| Physics | **Custom** (circle vs tiles/segments, tuned to match the original feel) |
| Levels | **Tiled** `.tmx` files, loaded with the `tiled` crate. Plus an `ascii2tmx` tool so I can sketch levels as text |
| Screen | **240×320 portrait** offscreen render target, integer-scaled, nearest-filter |
| Art | Our own pixel art: sprites written as ASCII grids, turned into PNGs by `tools/artgen` |
| Levels scope | Full story set + bonus levels. You share screenshots/notes before each level prompt |
| Features | Transformations, enemies + Hypno Cube boss, cutscenes + menus, eggs/lives/checkpoints/save |
| Audio | Chiptune SFX + music synthesized in code (no external files) |
| Controls | Keyboard: arrows/WASD to roll, Up/W/Space to jump, Esc to pause |
| Difficulty | Like the original: limited lives, game over restarts the level |
| Testing | `cargo test` + headless replay sim + a screenshot from a real run |
| Git | **You commit, not me.** At the end of each step I leave the changes uncommitted and give you a ready-to-paste commit message (`step NN: <title>` + short body). Branch rename `master` → `main` is your command to run too (`git branch -m master main`) |
| Plan tracking | `PLAN.md` in the repo with checkboxes + a progress log |
| Ship | Release binary with all assets embedded (`include_bytes!`) |

## Architecture (set up in steps 1–2, kept throughout)
```
src/
  main.rs          // macroquad window, CLI flags, fixed 60 Hz loop
  sim/             // PURE game logic, no macroquad rendering → headless-testable
    mod.rs         // Sim { world, entities, ball, rng } ; step(&InputFrame)
    physics.rs     // circle vs AABB/segment collision, integration
    ball.rs        // ball state, forms (Normal/Stone/Balloon…)
    map.rs         // collision grid built from Tiled tile properties
    entities/      // enum-based entities: pickups, platforms, enemies, props
  render/          // draws Sim state into 240x320 target, camera, parallax, HUD
  audio/           // synth.rs (SFX -> WAV bytes), music.rs (sequencer)
  ui/              // state machine: Title, Menu, LevelSelect, Playing, Pause, Cutscene, GameOver
  save.rs          // serde + RON at $XDG_DATA_HOME/bounce_tales/save.ron (`directories` crate)
  assets.rs        // include_bytes! registry (single-binary ship)
tools/artgen/      // ASCII sprite -> PNG atlas (cargo workspace member)
tools/ascii2tmx/   // ASCII level sketch -> Tiled .tmx
assets/{sprites,tilesets,levels}/
tests/replays/     // input scripts + expected outcomes
```
Key rules:
- **Fixed timestep** (60 Hz sim, rendering interpolated). Deterministic, so replays always give the same result.
- CLI flags: `--level <name>`, `--frames N`, `--replay <file>`, `--screenshot <png>` (saves the screen after N frames, then quits). Used for the visual check in every step.
- `scripts/check.sh` = `cargo fmt --check && cargo clippy -D warnings && cargo test`.

## Per-Prompt Protocol (every one of the 40 prompts)
1. Read `PLAN.md` (progress log) to see the current state.
2. Build only that step's feature.
3. Add unit tests and/or a replay test for it.
4. Run `scripts/check.sh`. Then run the game with `--frames N --screenshot` (under `xvfb-run` if there's no display) and look at the PNG.
5. Tick the step in `PLAN.md` and add notes (tuning values, known issues).
6. Do **not** commit. Give you: the commit message (`step NN: <title>` + body), a list of changed files, and what to playtest by hand.

## The 40 Steps

### Phase A — Foundation
- [x] **01.** **Bootstrap**: (you run `git branch -m master main`). Add macroquad. Module skeleton, `PLAN.md`, `scripts/check.sh`. Fixed-timestep loop, 240×320 target integer-scaled to the window, CLI flags + screenshot capture.
- [x] **02.** **Headless sim + replays**: `Sim`/`InputFrame`, deterministic stepping, replay file format, test harness in `tests/replays`, F1 debug overlay (FPS, ball pos/vel).
- [x] **03.** **Ball on flat ground**: gravity, rolling acceleration, friction, max speed, jump, bounce restitution. Tests for jump height and speed cap.
- [x] **04.** **Tile collisions**: hardcoded test grid, circle-vs-AABB resolution (floor, walls, ceiling, corners), no tunneling at high speed (sub-stepping).
- [x] **05.** **Slopes**: 45° and 22.5° tiles, circle-vs-segment, rolling down/up slopes, smooth hand-off between slope and flat tiles.
- [x] **06.** **Camera**: follow with deadzone and look-ahead, clamped to level bounds.

### Phase B — Content pipeline
- [x] **07.** **Art pipeline**: `tools/artgen` (ASCII + palette → PNG). Ball sprite with roll rotation, first forest tileset.
- [x] **08.** **Tiled loader**: load `.tmx` from embedded bytes. Tile properties (`solid`, `slope`, `spike`, `water`) → collision map. Render tile layers.
- [x] **09.** **Object layers + ascii2tmx**: spawn, exit, decoration objects. Entity list. Level sketch tool.
- [x] **10.** **Visual polish base**: parallax backgrounds, ball squash/stretch on landing, simple particles.

### Phase C — Core gameplay
- [x] **11.** **Hazards & death**: spikes, bottomless pits, death animation, respawn.
- [x] **12.** **Lives & checkpoints**: checkpoint objects, lives counter, game over → level restart.
- [x] **13.** **Collectibles & HUD**: eggs/gems, score, bitmap font, HUD.
- [x] **14.** **Level exit & completion**: exit gate, results screen (eggs found, time), next level.
- [x] **15.** **Water**: buoyancy, drag, splash particles.
- [x] **16.** **Moving platforms**: paths from Tiled polylines, ball rides and inherits velocity.
- [x] **17.** **Breakables, switches, doors**: crumbling blocks, cracked walls, switch-linked doors/bridges.
- [ ] **18.** **Springs, catapults, wind fans**.

### Phase D — Physics props
- [ ] **19.** **Pushable boxes** (gravity, ball pushes them, they stack on tiles).
- [ ] **20.** **Seesaws** (pivot plank, torque from ball weight, ball rolls on the rotated segment).
- [ ] **21.** **Rotating platforms / wheels & cannons** (ball launched from cannons).

### Phase E — Transformations
- [ ] **22.** **Transformation framework**: pickup objects, form state machine, swap visuals and physics parameters, revert pickups.
- [ ] **23.** **Stone ball**: heavy, sinks in water, smashes cracked blocks, tips seesaws, low bounce.
- [ ] **24.** **Balloon/light ball** (+ any other form from your references): floats up, slow fall, pops on spikes.

### Phase F — Enemies & boss
- [ ] **25.** **Enemy framework + walkers** (hypnotized creatures): patrol, damage vs stomp rules.
- [ ] **26.** **Flyers, jumpers, shooters**, projectiles.
- [ ] **27.** **Hypno Cube boss**: arena, attack patterns, phases, win condition.

### Phase G — Audio
- [ ] **28.** **SFX synth**: square/triangle/noise + envelopes → WAV bytes. Bounce, jump, pickup, splash, death, transform sounds.
- [ ] **29.** **Chiptune music**: small pattern sequencer. Title, forest, cave, castle and boss tracks. Volume control.

### Phase H — Menus & story
- [ ] **30.** **Game state machine & menus**: title, main menu, pause, game over, screen transitions.
- [ ] **31.** **Level select / world map** with locks, egg counts, best times.
- [ ] **32.** **Save & options**: progress, eggs, settings (volume, scale, fullscreen) saved to an XDG path.
- [ ] **33.** **Cutscene engine**: comic panels + text boxes. Intro, between-level and ending scenes.

### Phase I — Levels (you share references before each one)
- [ ] **34.** **Levels 1–2** (forest) + tileset.
- [ ] **35.** **Levels 3–4** + second tileset (caves/water).
- [ ] **36.** **Levels 5–6** + third tileset.
- [ ] **37.** **Final levels + Hypno Cube castle**, boss arena wired in. Exact count checked against your references.
- [ ] **38.** **Bonus levels**, egg placement, collectible balance.

### Phase J — Finish
- [ ] **39.** **Tuning & bug bash**: physics feel pass against the original, difficulty curve. Regression replays: every level loads and runs 3000 frames without panic, plus scripted runs through key sections. Performance check.
- [ ] **40.** **Release**: all assets embedded, release profile (LTO), README (build, controls, credits), screenshots, and a suggested `git tag v1.0` command for you to run.

Spare capacity: if a step overruns, the next prompt finishes it before moving on. Steps 38–39 are the buffer we give up first.

## Critical files
`Cargo.toml` (workspace + deps: macroquad, tiled, serde, ron, directories, png for artgen), `src/main.rs`, `src/sim/*`, `src/render/*`, `PLAN.md`, `scripts/check.sh`, `assets/levels/*.tmx`.

## Verification (each step + at the end)
- `scripts/check.sh` passes (fmt, clippy, unit + replay tests).
- `cargo run --release -- --level <lvl> --frames 300 --screenshot shot.png` produces the expected image (I check it visually).
- Your manual playtest notes after each step go into the PLAN.md log.
- Final check: play the whole game from new save → ending on a release binary moved outside the repo (proves assets are embedded).

## Progress Log

### Step 01 — Bootstrap (done)
- macroquad 0.4.16. Modules: `cli` (hand-rolled flag parser), `sim` (`Sim`, `FixedStep` accumulator, 60 Hz, max 5 steps/frame), `render` (240x320 target, integer letterbox).
- `--frames N` runs exactly one sim step per rendered frame (deterministic); `--screenshot` saves the 240x320 target after N frames.
- Gotcha: render-target textures are bottom-up. `present()` draws with `flip_y: true`; `export_png` flips on its own, so screenshots are already the right way up. Verified both via PNG and a real window capture (`import -window "Bounce Tales"`).
- Placeholder scene: sky, grass, sliding red ball, tick counter. Replaced in step 3.
- No Xvfb on this machine; a real display (`DISPLAY=:1`) is used for screenshot runs.
- Branch is still `master`; user to run `git branch -m master main`.

### Step 02 — Headless sim + replays (done)
- Crate is now lib + bin: `src/lib.rs` exports `cli`, `replay`, `sim`; `render` stays in the binary (`src/main.rs`).
- `sim::InputFrame` {left,right,jump} = buttons held for one step; edges derived in sim (`Sim::jump_pressed`). `sim::Ball` has `pos/prev_pos/vel/radius`; `render_pos(alpha)` interpolates. `sim::Rng` = seeded xorshift32 (never use global RNG in sim).
- **Determinism rule** (documented in `sim/mod.rs`): `step` depends only on state + input. `Sim: PartialEq` so tests compare whole states.
- Replay format (`src/replay.rs` doc comment): `<count> <LRJ|->`, `level <name>`, `expect <field> <op> <value> [+- tol]`. Checkable fields live in `replay::FIELDS`; **add new fields there** as features arrive (lives, eggs, form…).
- `tests/replays.rs` runs every `tests/replays/*.replay` (each must have ≥1 expect) and checks determinism. New regression test = new `.replay` file.
- CLI: `--replay` drives input, prints check failures, exits 1 on failure, runs to end if no `--frames`. `--record <file>` saves live input on exit (Esc or window close; `prevent_quit` used). `--debug` / F1 overlay: fps, tick, pos, vel, held input.
- Placeholder movement (constant 90 px/s, flat floor `GROUND_Y`=260, clamped to 240 px wide). **Step 3 must rewrite `placeholder_roll.replay` and `wall_clamp.replay`.**
- No xdotool on this machine, so live keyboard recording was tested by hand only; the write path was verified with `--frames 30 --record`.

### Step 03 — Ball on flat ground (done)
- `sim::ball::BallParams` holds all movement tuning; `BallParams::NORMAL` is the red ball. Forms (step 22) = other param sets.
- NORMAL values: gravity 900, max_fall 420, ground_accel 600, air_accel 420, ground_friction 450, air_drag 60, max_speed 120, jump_height 52 (≈3.25 ball heights, ~0.7 s air time), restitution 0.35, min_bounce_speed 60, wall_restitution 0.3, coyote 5 ticks, jump buffer 8 ticks.
- `jump_speed()` adds ½·g·DT to make up for semi-implicit Euler, so the apex really is `jump_height` (tested to ±1 px).
- Holding jump re-jumps on every landing (like the original). A tap in the air is buffered for 8 steps. Coyote time is blocked after a real jump (`jumping` flag).
- Input can't push past max_speed, but extra speed (springs or slopes later) bleeds off by friction instead of snapping back.
- `sim::physics::collide_flat` = temporary flat floor + screen-edge walls; `land()` holds the shared bounce/rest rules. **Step 4 replaces collide_flat with tile collision but should reuse `land()`.**
- `Ball.angle/spin`: spin = vx / r on the ground, kept in the air. The render draws a rotating stripe.
- New replay fields: `ball.grounded`, `ball.angle`. Replays: `flat_roll`, `jump`, `hold_jump`, `wall_clamp`.
- 15 movement unit tests in `physics.rs` use a 100000 px wide world and start at `MID`, so walls don't interfere.

### Step 04 — Tile collisions (done)
- `sim::map`: `TILE` = 16 px (ball = exactly one tile wide). `TileMap::from_ascii` (`#` solid, `.` empty, `o` spawn resting on that cell's floor). Out of bounds: left/right = solid, above/below = open (falling out the bottom → respawn at spawn for now; real deaths in step 11).
- Built-in levels via `map::builtin(name)` / `BUILTIN_NAMES`: `test` (default: step block, low platform, staircase, tunnel) and `flat` (empty room for physics replays). `Sim::for_level(name)`, `Sim::with_map(map)`. `--level` works; a replay's `level` line wins over `--level`; `Replay::start_sim`.
- `physics::move_and_collide`: sub-steps of at most r/2 (max 16) → no tunnelling even at 5000 px/s. Each sub-step pushes out of the deepest overlapping tile (up to 4 passes). Normal y < -0.6 = floor, > 0.6 = ceiling (bonk, no bounce), else wall (reflect × wall_restitution). Bounce is applied once per step in `land()` along the floor normal.
- **Gravity now always applies** (the floor contact cancels it). This is ready for slopes in step 5. `Ball::update` only changes velocity; `update_spin()` runs after collision.
- Ghost edges: `closest_point` extends a tile into solid neighbours (no seams on floors or walls). **Bug found and fixed:** extending both ways at an inside corner filled the empty diagonal cell and lifted the ball 8 px when it hit a step. Now returns `None` if the diagonal cell is empty. Covered by `inside_corner_cell_is_not_filled` and the y-check in `one_tile_step_blocks_rolling`.
- Tests: 12 new collision unit tests (still resting, seams, ceiling, wall slide, step, corner roll-off, ledge + coyote, 1-tile tunnel, fast tunnelling x/y). Replays moved to `level flat` (rest y = 280, walls at x 16/224); new `ceiling_bonk`, `step_block` on `test`.
- Debug trick: a throwaway `tests/zz_trace.rs` that prints per-step state is quick for tuning replay timings (delete afterwards).

### Step 05 — Slopes (done)
- `map::Tile` shapes: `Solid`, `Slope45Right` `/`, `Slope45Left` `\`, 1:2 (≈26.6°) two-tile slopes `r R` (rising right: low, high) and `L l` (rising left: high, low). `Tile::polygon()` gives a clockwise convex outline in tile-local px; `side_span(side)` says how much of each square side the shape covers.
- `TileMap::edge_hidden(x, y, a, b)`: an edge is hidden when the neighbour's opposite side covers it fully. This **replaces the step 4 "extend into neighbours" trick**. Seams are safe because the deepest contact on an open surface is always the flat one, never a seam corner.
- `physics::penetration` = circle vs the nearest open polygon edge (centre-inside case leaves through the nearest open edge).
- `Ball::roll` (ground movement) works in speed along the surface tangent: downhill pull = gravity·t.y; pushing uphill only fights `uphill_gravity` (0.5) of it; friction × grip (1 on flat → 0 at 45°), so the ball never sticks on slopes; `max_roll_speed` 260; a small push into the surface keeps contact. Air movement unchanged (`drive()` helper shared).
- Rolling ball (grounded and not jumping):
  - meeting a new floor angle = no bounce; speed kept and redirected along the new surface;
  - losing contact → `snap_to_floor` pulls up to 6 px **along the old normal** (straight down failed at 45° crests: the slope push-out shoved the ball backwards every step).
- `Ball.ground_normal` is new; spin uses speed along the surface.
- Maps: `test` gained a 1:2 hill + 45° ramp on the floor; new `slopes` level (30 wide: 45° down, 1:2 hill, 45° up). Built with a script so rows are exact.
- Tests: 10 slope unit tests (roll down, gentle slower, hugging surface across seams ±0.05 px, climb + crest without leaving the ground, gentle climbs faster, follow downslope off a plateau, speed kept at slope→flat, downhill cap, jump on slope, slope underside = ceiling) + map tests (convex/clockwise outlines, spans, hidden edges). Replays `slope_valley`, `slope_rollback`.
- Not done: upside-down (ceiling) slopes. Add them if the reference levels need them.

### Step 06 — Camera (done)
- `sim::camera::Camera` lives **in the sim** (deterministic, replay-checkable via `cam.x`/`cam.y`); the renderer only blends `prev_pos→pos` and rounds to whole pixels (`render_pos`). `VIEW_W/H` (240x320) are defined there; `render::SCREEN_W/H` derive from them.
- Horizontal: focus = ball.x + `look`; dead zone ±8 px around the centre; look-ahead 36 px toward the rolling direction (needs |vx| > 30), swinging at 90 px/s. Stopping keeps the current look (no pan back).
- Vertical: ball rests at `FOCUS_Y` = 60% down the screen. On the ground the camera eases there at ≤240 px/s. In the air it holds still unless the ball goes more than 64 px above or 24 px below the focus line (a full 52 px jump never moves the view).
- Clamped to the level; levels smaller than the view are centred. `Camera::snapped_to` is used at start and on respawn.
- New default level **`big`** (64x40: step, ramp, plateau, 1:2 slope, 3-tile pit, 12-platform zigzag tower, top ledge back left). `test` stays for replays.
- Renderer: `target_camera(target, view)` builds the Camera2D per frame; `screen_camera` for the overlay (now shows cam); tiles outside the view are culled.
- Tests: 10 camera unit tests (snap/clamp, centring small levels, screen-sized level never scrolls, look-ahead lead, smooth turn pan, no pan back on stop, jump doesn't bob, gradual recentre, falls followed, whole-pixel render). Replays `camera_slopes`, `camera_big` (hop path depends on tuning; re-check if physics changes).

### Step 07 — Art pipeline (done)
- Cargo **workspace**: root `bounce_tales` + `tools/artgen` (dep: `png`). `scripts/check.sh` now runs clippy/test with `--workspace`.
- `artgen` source format (doc comment in `tools/artgen/src/lib.rs`): `sheet <name> cell WxH columns N`, `palette … end` (`#RRGGBB[AA]`, `.` = transparent), `sprite <name> … end`, `spin <prefix> from <sprite> frames N fill <char> [over <sprite>]` (pre-rotated frames, clockwise; `over` = fixed lighting, masked to the shape). `;` comments outside sprites.
- `cargo run -p artgen` writes `assets/gen/<sheet>.png` + `.idx` (`name x y w h`). **Generated files are committed.** `tools/artgen/tests/up_to_date.rs` fails if they drift from `assets/art/*.txt` (verified by changing a pixel). `cargo run -p artgen -- --check` does the same from the CLI.
- Art: `assets/art/ball.txt` (body with seam + mark, shade overlay with highlight/outline, 16 roll frames `roll_0..15`); `assets/art/forest.txt` (14 tiles, **order matters for Tiled, append only**: grass_top, grass_left, grass_right, grass_platform, dirt, dirt_stones, dirt_bottom, dirt_roots, slope45_r, slope45_l, slope2_r_low, slope2_r_high, slope2_l_high, slope2_l_low). First drafts came from a one-off Python script (circle mask, light-based shading, slope grass lines); the `.txt` files are the source of truth now.
- Game: `src/render/atlas.rs`: `Atlas` (embedded via `include_bytes!`/`include_str!`, nearest filter, `draw(name, pos)` rounds to whole pixels), `Sprites::load()`. Ball frame = `ball_frame(angle)` (nearest of 16). `forest_tile(map, x, y)` auto-tiles ASCII maps from the shape plus open sides (top row of the map counts as closed). Dirt variety comes from a fixed per-cell hash.
- Screenshots now go to `shots/` (gitignored). Live window verified the right way up with textures.

### Step 08 — Tiled loader (done)
- `tiled` 0.16 with `default-features = false` (avoids the zstd C dependency; Tiled's default CSV/base64/zlib layers still work).
- artgen: sprite lines take `key=value` properties; a `sheet … tileset` line also writes `assets/gen/<sheet>.tsx` (tile id = position, `name` property + extras, `true`/`false` → bool). Covered by the up-to-date test. `forest.txt` tiles carry `shape=…`.
- `src/level.rs` (lib): `Level { map, layers: Vec<TileLayer>, sheets }`, `TileRef { sheet, id }`. `Level::load(name)`: Tiled levels in `TMX_LEVELS` win over ASCII built-ins. **Conventions** (module doc): 16x16 finite maps, no flipped tiles (error), tile layer named `ground` = collision (`shape` property → `Tile::from_shape_name`; `spike`/`water` bools → `map::flags`), layer bool `front=true` → drawn over the ball, object of type `spawn` (point where the ball's bottom rests; Tiled `type` or `class` both work).
- `EmbeddedReader` serves the map and embedded tilesets by **file name** (so `../gen/forest.tsx` from the editor works).
- `src/tileset.rs` (lib): forest names/ids from the embedded `.idx`, `autotile()` (moved out of render).
- `src/bin/ascii2tmx.rs` (pulled forward from step 9): `cargo run --bin ascii2tmx -- builtin:test out.tmx` or a `.txt` file. `Cargo.toml` has `default-run = "bounce_tales"`.
- `big` is now `assets/levels/big.tmx` (converted; the ASCII copy was deleted). Its screenshot is **pixel-identical** to step 7, and `camera_big` still passes → loader parity. ASCII built-ins: `test`, `flat`, `slopes`.
- `Sim` holds `level: Level` (was `map`); `Sim::for_level` returns `Result<Sim, String>` with the list of known levels.
- Render: `draw_layer` per tile layer (back layers, ball, front layers), `Atlas::draw_index(id)`, `Sprites::by_name(tileset)`.
- Tests: TMX round-trip of every ASCII map (collision + pictures equal), every level loads, every `.tmx` is registered, property → shape/flag parsing with an inline test tileset, front layers, error cases (no ground, no spawn, flipped tile, missing tileset).
- Editing levels: install Tiled (e.g. `sudo apt install tiled` or Flatpak `org.mapeditor.Tiled`), open `assets/levels/*.tmx`, save, and **add new files to `TMX_LEVELS`** (a test fails if you forget).

### Step 09 — Objects, exit, decorations, entities (done)
- `level::ObjectSpec { kind, name, pos, size, points, props }` (generic; `Prop` Bool/Int/Float/Str, sorted by key). Tiled rect → pos = top-left + size; polyline/polygon → absolute `points`; tile objects with no type → `Decoration { tile, pos (top-left; Tiled anchors tile objects bottom-left), front }` (front from the object or its layer property). Untyped non-tile objects, flipped/rotated decorations and unknown types are **load errors**.
- `sim::entities`: `Entity { pos, size, kind: EntityKind }`, `build(spec)` validates (unknown types list the known ones), `Entity::update(ball, events)`. **Recipe for new kinds** is in the module doc. First kind: `Exit { reached }`; the ball centre inside the rect → `Event::ExitReached` (once).
- `Sim`: `entities`, `events` (cleared every step, for audio/FX later), `exit_reached`. Replay field `exit`.
- ASCII marker `E` = 32x32 exit standing on that cell's floor, centred (`level::EXIT_SIZE`). `TileMap::from_ascii` treats it as empty; `ascii2tmx` writes it as a rect object. `flat` and `test` maps have exits.
- Art: `assets/art/deco.txt` (tileset, append-only: flower_red, flower_yellow, mushroom, tuft, rock, sign_right, bush_l, bush_r) and `assets/art/objects.txt` (32x32 entity sprites: `exit` gate with checkered flag). `Sprites` now has `deco`, `objects`; `render::ENTITY_SPRITES` is checked against the index.
- `big.tmx`: deco tileset (firstgid 15), 20 decorations + 4 front tufts, exit at the left end of the top ledge (24,160).
- Draw order: back tile layers → back decorations → entities → ball → front tile layers → front decorations.
- Tests: entity build/validate/trigger, ASCII exit marker, TMX decorations/front/props/typed objects, unknown/untyped object errors, round trip now compares objects. Replay `exit_flat`.

### Step 10 — Visual polish base (done)
- **Squash/stretch** in the sim (deterministic, replay field `ball.squash`): `Ball::update_squash(impact, jumped)`. A landing > 60 px/s sets squash = impact/1200 (max 0.35); a jump sets −0.15 (stretch); an under-damped spring (stiffness 450, damping 13; ≈21 rad/s, ζ≈0.3) wobbles it back to round. Rendering scales the sprite (1+s, 1−s), keeping the bottom on the ground (`Atlas::draw_scaled`). The first tuning (stiffness 800) was too quick to see (~2 frames); softened after checking screenshots.
- **Events**: `Event::Jumped { pos }` and `Event::Landed { pos, speed }` (only from the air, impact > `LAND_EVENT_MIN_SPEED` = 100; rolling over bends never emits). `Event` is now `PartialEq` only (has Vec2). `Ball::update` returns `jumped`.
- **Particles** (`src/render/particles.rs`, cosmetic, render-side): advanced once per sim step via `Renderer::on_step(&sim)` (called in main after every `sim.step`), own seeded `Rng` → repeatable screenshots. Dust on land (count ∝ speed), jump, fast rolling (every 6 ticks above 110 px/s); 36-piece confetti on `ExitReached`. Puffs alternate sides and fly outward (the first version clumped into a white block under the ball). Capped at 400.
- **Parallax** (`src/render/background.rs`): 6 sky bands, clouds (x factor 0.08 + drift), far hills (0.2), near hills with round trees (0.45). Textures are generated at start-up from sines with whole-number periods per 256 px (seamless, tested). Layers sink as the camera climbs (`climb` = how far above the level's lowest camera position; y factors 0.05/0.1/0.25). Drawn in screen space before the world.
- Map property `background` (default `forest`; must be in `level::BACKGROUNDS`, else load error).
- Tests: squash on landing + return to round, rebound wobble, jump stretch + event, no landings while rolling across slopes, dust appears and dies, effects repeatable, exit confetti, hill tiling/fit, background property.

### Step 11 — Hazards & death (done)
- Spikes are **flags, not shapes** (no collision): `flags::SPIKE` (up, from the cell bottom) and the new `flags::SPIKE_DOWN` = 4 (hanging from the cell top). Tile properties `spike=true` / `spike_down=true` (the loader reads both; the round-trip test caught `spike_down` missing at first). ASCII glyphs `^` / `v`. Forest tiles 14 `spikes`, 15 `spikes_down` (appended; two stepped metal spikes per tile). The auto-tiler picks them from flags.
- `sim::hazards::check(ball, map) -> Option<Cause>`: spike danger boxes are smaller than the art (up: x 2..14, y 7..16 in the cell; down: y 0..9), so grazes are forgiven. `Cause::Pit` when the ball is fully below the level (replaces the old silent respawn).
- `Sim`: `die(cause)` → `Event::Died { pos, cause }`, `deaths += 1`, `dying = DEATH_TICKS` (60). While dying, `step` only counts down (input ignored, ball hidden); then it respawns at `respawn_point` (= level start; **step 12 checkpoints set it**), snaps the camera, emits `Event::Respawned`, and sets `respawn_anim` (12 ticks grow-in, drawing only).
- FX: popping burst (16 rubber fragments; none for pit deaths, which happen off screen), sparkle ring on respawn, no rolling dust while dying. Overlay shows deaths.
- New ASCII level `hazards` (24 wide): floor spikes, ceiling with hanging spikes, 2-tile pit, exit. The first 20-wide layout had a **1-tick jump window** (found by brute-force searching jump ticks); it was widened to an 18-tick spike window and a 32-tick pit window.
- Replay fields `deaths`, `dying`. Replays `hazards_course` (no-death run to the exit), `hazards_spikes`, `hazards_pit`. Unit tests: hazard boxes, grace, pit, death→freeze→respawn, pit death, burst FX.

### Step 12 — Lives & checkpoints (done)
- `checkpoint` object type (rect; ASCII marker `C`, 32x32 like the exit). `map::OBJECT_MARKERS` + `level::ascii_marker()` drive markers generically (a test checks every marker has a kind). `EntityKind::Checkpoint { active }`: the first touch → `Event::CheckpointReached { respawn }` (ball resting on the rect's floor, centred); Sim sets `respawn_point`. The latest checkpoint touched wins.
- Lives: `START_LIVES` = 3 (balls left including the one in play). `die()` takes one. When dying ends with 0 lives → `Event::GameOver` + `game_over` = `GAME_OVER_TICKS` (150) pause → `restart_level()`: a fresh `Sim::with_level` keeping tick, input history and `restarts + 1`, emitting `Event::LevelRestarted`. `Sim::ball_visible()` (false while dying or game over; **the screenshot caught the ball still drawn during game over**).
- Lives will need to carry across levels in step 14 (session state); for now they live in `Sim`.
- HUD (`Renderer::draw_hud`, called from main): mini-ball life icons at the top left, "GAME OVER" banner (default font; bitmap font in step 13).
- Art: `objects.txt` + `checkpoint_off` (limp grey flag), `checkpoint_on` (red flag, gold star). FX: gold sparkles on activation, white sparkle on restart.
- Maps: `hazards` has a checkpoint after the floor spikes (respawn x 168); `big.tmx` has 3 (plateau, tower foot, top ledge start).
- Replay fields `lives`, `game_over`, `restarts`, `respawn.x/y`. Replays: `hazards_pit` (now respawns at the checkpoint), `game_over`. Unit tests: checkpoint activation/respawn spot, full game-over → restart cycle, respawn at checkpoint after a pit death.

### Step 13 — Collectibles & HUD (done)
- Object types `gem`, `egg`, `life` (Tiled rect → centred on its middle, or a point → centred on it; always a 16x16 entity). ASCII markers `*`, `e`, `+`. `EntityKind::Pickup { item: Item, taken }`; picked up when the ball centre is within radius + `PICKUP_RADIUS` (6) of the item centre → `Event::Collected { item, pos }`.
- `Item::points()`: gem 10, egg 500, life 100. `Sim`: `score`, `gems`/`gems_total`, `eggs`/`eggs_total` (totals counted at level start), life → `lives + 1` capped at `MAX_LIVES` (9). Items stay taken after a death; a game-over restart resets them (fresh Sim). Replay fields `score`, `gems`, `eggs`.
- Art: `assets/art/font.txt` (5x7 pixel font in 6x8 cells: 0-9, A-Z and named symbols), `items.txt` (`gem_0..3` shimmer frames, `egg`, `life`), `icons.txt` (8x8 HUD icons). The first egg looked like a cookie; it was reshaped into a taller oval.
- `src/render/font.rs`: `glyph_name` (lowercase → capitals, `*` draws ×, unknown → `?`), `width`, `draw` (tint + 1 px shadow), `draw_centred`, any integer `scale`. `Atlas::draw_tinted`.
- HUD: `icon_ball ×N`, gem count, `eggs/total` (only if the level has eggs), 6-digit score at the top right; GAME OVER now in the pixel font at 2x. Items bob (±1.5 px, phase by x) and gems animate (8 ticks per frame). Pickups emit sparkles plus a floating popup ("+10", "+500", "1UP").
- Content: `hazards` has 3 gems after the spikes and an egg at the pit-jump apex (placed by tracing the jump path; the first guess was too low). `big.tmx` has 34 items: gem trails (step, plateau, pit arc, every tower platform, top ledge), 3 eggs (behind the tower on the floor, the top-right tower platform, above the ledge start), 1 extra life. The plateau checkpoint moved left so it doesn't overlap the gem row.
- Replay `hazards_collect` (clean run = 3 gems + egg = 530). Tests: pickup build from rect/point, collect-once radius, scoring/counting, lives cap, popups, every glyph in the font index.

### Step 14 — Level completion & game flow (done)
- `src/game.rs` (lib, deterministic, unit-tested): `Game { sim, mode, campaign, index, level_name, results }`, `Mode::{Playing, Results { result, ticks }, Finished { ticks }}`. `CAMPAIGN` = ["big", "hazards"] for now (real levels in steps 34-38). `--level X` starts there; a level outside the campaign goes to Finished after its results.
- Flow: exit touched → `COMPLETE_DELAY` (75 ticks of confetti) → Results (lines reveal every 10 ticks; continue needs a **fresh** jump press after `CONTINUE_AFTER` = 30, so a held jump can't skip it) → next level with `Carry { lives, score }` → … → Finished ("WELL DONE!", total score, eggs) → jump restarts the campaign fresh.
- `LevelResult::from_sim`: time (seconds of the current attempt), par, gems, eggs, deaths, time bonus = (par − seconds) × 10, all-eggs bonus 1000, final score. Results are kept in `Game::results` (for saving in step 32).
- Sim: `Carry` + `Sim::with_level_carry`; `start_carry` (a game over restarts with the lives/score you entered the level with); `started_tick` (reset on restart); **the sim freezes once the exit is reached**; `exit_tick` stops the level clock (the screenshot showed the 75-tick celebration being counted as level time).
- Level map property `par` (int seconds, default 120; ASCII maps 60).
- main: drives `Game`; uses its own `step_count` for frame limits and replay input (the sim tick pauses on Finished, which would loop a scripted run forever). Renderer `draw_flow(&game)`: results panel and finished screen in the pixel font, blinking PRESS JUMP.
- Tests: results + bonuses, held jump doesn't skip, carry of lives (lost lives stay lost) and score into the next level, Finished → fresh start, campaign levels load, par property, exit freeze + clock stop, game-over returns to the carried values.

### Step 15 — Water (done)
- Water = `flags::WATER` cells (no shape); ASCII `~`; forest tiles 16 `water_top`, 17 `water` (see-through, `water=true`). Auto-tile: `water_top` where the cell above is neither water nor solid; **no grass on blocks under water** (it looked wrong in the first screenshot).
- `sim::water::submersion(ball, map)` = share (0..1) of the ball's height inside water cells in the column under its centre. Stored as `Ball.water` after each move (replay field `ball.water`).
- `BallParams`: `buoyancy` 1.6 (floats at 1/1.6 ≈ 62% under; **the stone ball in step 23 sets < 1 to sink**), `water_drag` (2.5, 6.0)/s × depth, `water_speed` 0.75 × max speed, `water_jump` 0.9. In `Ball::update` buoyancy applies even when grounded (lifts the ball off the floor); you can jump at the surface (0.15 < depth < 0.9, not already rising fast).
- `Event::Splash { pos, speed }` when the ball enters or leaves water above 60 px/s of **total** speed (vertical-only missed rolling in). FX: droplets (blue/white, more for harder hits).
- Rendering: water cells of non-front layers are skipped in the back pass and drawn **after the ball** (`draw_layer(…, keep)` filter), so a floating ball looks half-submerged and tinted.
- New ASCII level `water`: pool (x 96..288, 4 deep), gems above it (reachable by hopping), exit on the far bank. Measured: the ball floats with its centre at y 226, swims at 88 px/s, and can jump out at the far bank any time.
- Tests: submersion fractions, floating settles at ≈1/buoyancy, splash + bob on a drop, splash when rolling in, jump out, slower swimming, no grass under water. Replay `water_cross`.

### Step 16 — Moving platforms (done)
- Tiled object type `platform` = polyline (back and forth, pausing `wait` s at each end) or polygon (loops). Points are the platform's **top-centre** path. Properties: `width` (tiles, default 3), `speed` (px/s, default 40), `wait` (s, default 0.5). `ObjectSpec.closed` added; `ascii2tmx` writes polyline/polygon objects (round-trip tested).
- ASCII markers `H` (3 wide from that cell, travels 8 tiles right and back) and `V` (rises 4 tiles and back).
- `sim::platforms::Platform`: path walking by distance (`point_at`, `advance`), `vel`, `delta`. **One-way** (only landable from above), so nothing can crush the ball.
- Riding (`Sim.riding: Option<entity index>`, replay field `riding`): platforms advance first; the rider is carried by `delta` (and `prev_pos` is fixed so it's drawn smoothly). The ball's own velocity is **relative** while riding: landing subtracts the platform velocity, leaving (jump / rolling off the end) adds it back. Hard landings bounce (relative impact > min_bounce); soft ones ride. Platform landings feed the same Landed event and squash. Death clears riding.
- Art: `items.txt` + `plat_l/m/r` wooden plank pieces; drawn interpolated.
- New ASCII level `platforms`: ferry across a bottomless gap, lift up to the exit ledge, gems over the gap (hop for them). Replay `platforms_route` was **recorded from a small scripted controller** (wait for the ferry, ride, hop onto the lift, steer onto it, ride up) — a reusable trick for long routes.
- Tests: path back-and-forth with pause, loop, point_at, build with properties, carried exactly with the platform, jump-off keeps platform speed, rolling off the end, one-way from below, lift raises the ball.

### Step 17 — Breakables, switches, doors (done)
- **Mutable map**: `Sim.level` is now the live copy; `Sim.pristine: Rc<Level>` is the level as loaded (blocks come back from it; game-over restarts clone it). `TileMap::set`, `Sim::set_cell` (keeps the ground layer's picture in step: removed, or restored from pristine).
- `physics::Contacts` (no longer `Copy`) has `hits: Vec<(x, y, speed into it)>` for every cell the ball was pushed out of.
- **Crumbling** (`flags::CRUMBLE`, tile `crumble=true`, ASCII `x`, forest tile `crumble`): standing on one (hit from above) → `Sim.crumbles` entry, shakes 30 ticks (drawn jittering) → falls (`Event::Crumbled`, sand debris) → comes back after 300 ticks once the ball is clear. `tick_crumbles` runs every step, **also while dying** (the first version paused timers during death).
- **Cracked** (`flags::CRACKED`, `cracked=true`, ASCII `k`, tile `cracked`): broken for good when hit at ≥ `SMASH_SPEED` (150) **by a ball whose `BallParams.smashes`** is true. False for the red ball; the stone ball (step 23) sets it. `Event::Smashed`, stone debris.
- **Switch / door / bridge** objects: `switch` (rect, string property `target`), `door` and `bridge` (rect, `name`). The first touch of a switch → every door/bridge with that name switches: doors stop being solid (drawn sliding up into their frame), bridges become solid (planks appear left to right), `GATE_ANIM_TICKS` 15. Events `Switched`, `GateMoved`. Closed doors are made solid at level start. ASCII `S`, `D`, `B` are all linked by the name `gate`. `ascii2tmx` now writes object string properties.
- Art: forest +`crumble` (sandstone), +`cracked` (boulder with a crack) (ids 18, 19); items +`switch_off/on`, `door`, `bridge`.
- New ASCII level `switches` (crumbling bridge, switch, roofed door, bridge, exit, roofed cracked wall with an egg behind for the stone ball). Replays `switches_route` (holding right clears it), `crumble_fall`. Tests: crumble fall + return, fast crossing, switch → door + bridge, closed door blocks, smashing only with `smashes`, game over restores blocks.
