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
- [ ] **08.** **Tiled loader**: load `.tmx` from embedded bytes. Tile properties (`solid`, `slope`, `spike`, `water`) → collision map. Render tile layers.
- [ ] **09.** **Object layers + ascii2tmx**: spawn, exit, decoration objects. Entity list. Level sketch tool.
- [ ] **10.** **Visual polish base**: parallax backgrounds, ball squash/stretch on landing, simple particles.

### Phase C — Core gameplay
- [ ] **11.** **Hazards & death**: spikes, bottomless pits, death animation, respawn.
- [ ] **12.** **Lives & checkpoints**: checkpoint objects, lives counter, game over → level restart.
- [ ] **13.** **Collectibles & HUD**: eggs/gems, score, bitmap font, HUD.
- [ ] **14.** **Level exit & completion**: exit gate, results screen (eggs found, time), next level.
- [ ] **15.** **Water**: buoyancy, drag, splash particles.
- [ ] **16.** **Moving platforms**: paths from Tiled polylines, ball rides and inherits velocity.
- [ ] **17.** **Breakables, switches, doors**: crumbling blocks, cracked walls, switch-linked doors/bridges.
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
