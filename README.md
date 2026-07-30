# Snooker Aim Trainer

A small 3D visualization tool for practicing snooker aim. It renders a
table, a cue ball, an object (red) ball, and a cue, with a set of
geometric aiming aids layered on top — no physics engine, no shot
simulation, no spin. Just plane/sphere geometry and straight-line math,
built to answer one question: *if I hit the ball dead straight from here,
where does it go?*

## Stack

Rust + [raylib](https://www.raylib.com/) (via the `raylib` crate, v6),
plus `rand` for random table layouts. Chosen for fast iteration and a
tiny, immediate-mode-friendly API — no game engine, no ECS, no asset
pipeline.

## Controls

Every action has both a keyboard hotkey and an on-screen button, so it
works the same with a mouse or a finger. The on-screen controls (a
pan/rotate/zoom cluster bottom-left, action buttons bottom-right, `?`
top-right) render on every platform, not just touch devices.

| Input | On-screen button | Action |
|---|---|---|
| Drag (left mouse) / D-pad | pan/rotate cluster | Orbit the camera |
| Scroll wheel | `+`/`-` | Zoom toward the cursor (wheel) or the camera target (buttons) |
| `W`/`A`/`S`/`D` or arrows | D-pad | Pan the camera (position + target together) |
| `Q`/`E` | `Q`/`E` | Rotate (yaw) the camera at a fixed speed |
| `C` | `CENTER` | Re-center the orbit pivot on the cue ball (keeps zoom/angle) |
| `V` | `LOOK` | Toggle view mode (see below) |
| `G` | `GHOST` | Toggle the ghost cue ball |
| `H` | `AIM` | Toggle the object-ball aim line (only while the ghost ball is shown) |
| `Space` | `HIT` | Test the current aim: trace both balls' paths, evaluate the pot |
| `R` | `CLEAR` / `NEXT` | Clear a tested shot's paths, or (if none showing) reposition both balls |
| `?` | `?` | Toggle a help popup listing all of the above |

The window (and, on the web build, the browser canvas) is resizable —
both the 3D view and the on-screen controls adapt to the current
window/screen size every frame, so it works down to a phone-sized
viewport.

## Features and design choices

**Camera starts "down the cue."** On launch and on reposition, the camera
sits behind the cue ball (away from the object ball) at a fixed ~15°
elevation — the vantage a player sights down the cue from when addressing
a shot. It's deliberately *not* placed exactly at the ball: the cue itself
is ~1.45m long and always renders along the camera-to-ball line (see
below), so the camera has to sit far enough back to avoid ending up inside
the cue's own geometry.

**The cue follows the camera's look direction, not its position.** The
cue's azimuth is derived each frame from the camera's horizontal forward
vector (`target − position`), not from where the camera happens to be
relative to the ball — so orbiting (which rotates that forward vector)
swings the cue, but panning with `W`/`A`/`S`/`D` (which translates
position and target together, preserving the vector between them) doesn't
disturb the aim. It ignores the camera's *pitch*: the cue stays
near-horizontal, raised by a fixed small angle from tip to butt, so
looking down from above doesn't tilt it into the table.

**Ghost ball, aim line, and the potting "gate" are pure geometry, no
physics.** Three related aids, all computed as straight-line raycasts in
the table plane:
- *Ghost ball* (`G`): where the cue ball's center would be at first contact
  — either the object ball or a cushion — if struck dead straight along
  the current cue direction.
- *Aim line* (`H`): a line from the ghost ball's top through the object
  ball's top, continuing to the cushion — i.e., where the object ball
  would travel if hit, using the standard no-spin "ghost ball"
  approximation (it departs along the line from the contact point through
  its own center).
- *Gate* (always visible): two posts a bit wider than a ball's diameter,
  spanning the *best* pocket for the current layout, perpendicular to the
  object ball's ideal path in. "Best" means smallest cut angle among
  pockets with a physically reachable contact point — 0° is a straight
  in-line pot, beyond ~90° is impossible. Falls back to the nearest pocket
  if every cut is too thin.

**`Space` freezes a shot test.** Pressing it traces the cue ball to its
first contact, then (if it hit the object ball) traces the object ball's
resulting path to *its* first event — passing through the gate (green,
potted) or hitting a cushion (red, missed) — and draws both paths as flat
"stadium" stripes (a ball-width-wide rectangle with circular end caps).
The result is frozen until the next `Space` or `R`, so you can orbit
around and inspect it without recomputing.

**Rotation sensitivity is cursor-aware, with two different rules for two
different cases.** Fixed radians-per-pixel felt wrong: pointing at
something close should turn fast, pointing at the horizon or off the table
should turn slow. On the table, sensitivity scales against the real
distance to the point under the cursor. Off the table — where the y=0
plane is infinite but the rendered table isn't, so a naive raycast could
still report a deceptively short distance just past the table's edge — it
falls back to a "virtual cone": a continuous function of the cursor ray's
downward angle alone, no intersection test, tuned to be reliably slower
than the on-table case.

**Zoom is also cursor-aware** (scroll toward whatever's under the cursor,
not just the orbit target), and clamped to a sane distance range so you
can't zoom through the table or out to nothing.

**View mode (`V`) freezes the cue's aim while you move the camera freely.**
Useful for walking around a shot you've already set up without disturbing
it. Entering it the first time (per layout) re-centers the orbit pivot on
the cue ball; toggling it off and back on resumes exactly where you left
off, until `R` generates a new layout.

**Random layouts are constrained to be "realistic."** Ball positions are
rerolled (up to 300 attempts) until the two balls have reasonable
separation *and* the best available pocket offers a cut angle of 65° or
less — otherwise every layout would technically be a "shot" but plenty of
them would be near-impossible slivers.

**Lighting is a small hand-rolled Blinn-Phong shader**, not raylib's
default flat-shaded primitives (`DrawSphere` etc. have no lighting at
all). The overhead light is modeled as three segmented rectangular panels
along the table's length — matching the look of a real snooker table's
LED light bank — each also acting as a point light in the shader, rather
than a single point light.

## Building for the web (WebAssembly)

The desktop build (`cargo run`) is the primary target. There's also a
working web build via Emscripten, running in the browser over WebGL2.

**Prerequisites** (one-time setup):
```sh
brew install emscripten
rustup target add wasm32-unknown-emscripten
```

**Build and serve locally:**
```sh
make serve   # builds for web and serves it at http://localhost:8765
# or just:
make web     # builds only, output in target/wasm32-unknown-emscripten/release/
make run     # native desktop build, for comparison
```

A couple of things had to be worked around to make this build actually
work, worth knowing about if it breaks again on a raylib/raylib-sys
upgrade:

- **`vendor/raylib-sys-6.0.0/`** is a locally patched copy of the
  `raylib-sys` crate, wired in via a `[patch.crates-io]` entry in
  `Cargo.toml`. Its `opengl_es_30` Cargo feature (needed to get
  WebGL2/GLSL ES 3.00 instead of the older WebGL1/ES 2.00) unconditionally
  links `-lGLESv2 -lGLdispatch` — real system libraries on native Linux/
  Raspberry Pi GLES targets, but raylib's own official Makefile links
  neither of them for `PLATFORM_WEB` (Emscripten provides its own GL
  implementation internally), so those two link flags just don't exist for
  this target and broke the build. The patch skips them specifically when
  `TARGET` contains `emscripten`; nothing else about `raylib-sys` was
  touched, and the native desktop build is unaffected (it never enables
  this feature — see `Cargo.toml`'s target-specific `[dependencies]`).
- **Two dialects of the three custom shaders** (`BALL_VS`/`BALL_FS`/
  `GHOST_FS` in `main.rs`), picked via `cfg(target_os = "emscripten")`.
  Desktop compiles against desktop OpenGL 3.3 core (GLSL 330); the browser
  gets GLSL ES 3.00 instead (`#version 300 es` + a `precision` qualifier —
  WebGL's compiler is stricter here than desktop's, e.g. it also requires
  `#version` to be the literal first line of the source, no leading blank
  line). The two dialects are close enough (both use `in`/`out` and an
  explicit output variable) that only the preamble differs, not the shader
  logic itself.
- **`web/index.html`** is a minimal page (canvas + script tag) that `make
  web` copies alongside the build output — Cargo's Emscripten target only
  produces the `.js`/`.wasm` files, not a host page.

## Limitations

- **No physics.** No collision response, no momentum, no cushion
  rebounds, no spin/swerve/throw. All ball travel is a straight line;
  "shot testing" is a single ball-ball contact plus a single subsequent
  straight segment, not a simulation.
- **Two balls only.** No full ball set, no break, no multi-ball
  interactions.
- **The pot model is a straight, dead-center pot.** No side spin, no
  screw, no positional play after the shot.
- **Table geometry is simplified.** Pocket jaws are flat cylinders, not
  the angled/rounded real-world pocket shape; cushions are plain boxes
  with no nose profile.
- **Single, fixed light rig.** No day/night or venue variation; the three
  panel lights are positioned by a fixed formula relative to table length,
  not configurable at runtime.
