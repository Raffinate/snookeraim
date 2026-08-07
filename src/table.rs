use raylib::prelude::*;

use crate::cushion_segments::{CUSHION_BOUNDARY, SHORT_RAIL_BOUNDARY};

// Real-world snooker dimensions, in meters.
pub const TABLE_LENGTH: f32 = 3.569; // long axis (Z)
pub const TABLE_WIDTH: f32 = 1.778; // short axis (X)
pub const CUSHION_HEIGHT: f32 = 0.05;
pub const CUSHION_THICKNESS: f32 = 0.06;
pub const BALL_RADIUS: f32 = 0.02625;
pub const CORNER_POCKET_RADIUS: f32 = 0.045;
pub const MIDDLE_POCKET_RADIUS: f32 = 0.05;

// A random layout is only "realistic" if the balls have some breathing room
// and at least one pocket offers a pot that isn't a near-impossible sliver
// of a cut.
pub const MIN_BALL_SEPARATION: f32 = 0.18;
pub const MAX_REALISTIC_CUT_DEG: f32 = 65.0;

// Real cushion nose boundaries, read directly from the table model's own
// mesh (see scripts/extract_cushion_segments.py). The nose sits recessed
// at a near-constant distance from the table's center along each straight
// run, then flares back out toward the flat cloth bed's edge in a window
// right at every pocket mouth (a real cushion-facing/shoulder feature,
// not noise) before holding flat past the corners' flare peak (the mesh
// beyond that is the pocket throat, not a wall a ball bounces off). Both
// tables cover only their non-negative half; each rail is symmetric about
// its own center.
// A real ball resting against the cushion touches it -- essentially zero
// gap. This is only a tiny epsilon to stop the ball's rendered surface
// from z-fighting/clipping into the nose mesh, not a real-world buffer.
pub const CUSHION_CLEARANCE: f32 = 0.001;

/// Piecewise-linear lookup shared by safe_half_width/safe_half_length:
/// interpolates `table`'s second column at `along`, clamped to the
/// table's own range at either end.
fn boundary_lookup(table: &[[f32; 2]], along: f32) -> f32 {
    let n = table.len();
    if along <= table[0][0] {
        return table[0][1];
    }
    if along >= table[n - 1][0] {
        return table[n - 1][1];
    }
    let mut result = table[n - 1][1];
    for w in table.windows(2) {
        let ([a0, b0], [a1, b1]) = (w[0], w[1]);
        if along >= a0 && along <= a1 {
            let t = (along - a0) / (a1 - a0);
            result = b0 + (b1 - b0) * t;
            break;
        }
    }
    result
}

/// Draws a ball's collision footprint as a ring in the table plane (its
/// actual physical radius, not the point rendered on screen) -- for the
/// collision-debug overlay, so the boundary a ball actually occupies is
/// visible, not just the cushion lines it collides against.
pub fn draw_ball_collision_ring(d: &mut impl RaylibDraw3D, center: Vector3, color: Color) {
    const SEGMENTS: usize = 24;
    let y = 0.021; // just above the cushion-boundary debug lines (0.02)
    let mut prev = Vector3::new(center.x + BALL_RADIUS, y, center.z);
    for i in 1..=SEGMENTS {
        let a = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
        let next = Vector3::new(center.x + BALL_RADIUS * a.cos(), y, center.z + BALL_RADIUS * a.sin());
        d.draw_line3D(prev, next, color);
        prev = next;
    }
}

/// Safe X boundary (for a ball's *center*) at a given |z|, against the
/// long rails: the measured cushion boundary minus the ball's own radius
/// and a clearance margin.
pub fn safe_half_width(abs_z: f32) -> f32 {
    boundary_lookup(CUSHION_BOUNDARY, abs_z) - BALL_RADIUS - CUSHION_CLEARANCE
}

/// Safe Z boundary (for a ball's *center*) at a given |x|, against the
/// short rails.
pub fn safe_half_length(abs_x: f32) -> f32 {
    boundary_lookup(SHORT_RAIL_BOUNDARY, abs_x) - BALL_RADIUS - CUSHION_CLEARANCE
}
pub const MAX_PLACEMENT_ATTEMPTS: u32 = 300;

/// Distance along a ray from `(x, z)` in direction `(dx, dz)` to the
/// first cushion — i.e. where a ball's center would leave the safe X
/// range (against the long rails, safe_half_width) or the safe Z range
/// (against the short rails, safe_half_length), both against the
/// model's measured geometry.
pub fn cushion_t(x: f32, z: f32, dx: f32, dz: f32) -> f32 {
    let calc_t_x = |x_max: f32| {
        if dx > 1e-6 {
            (x_max - x) / dx
        } else if dx < -1e-6 {
            (-x_max - x) / dx
        } else {
            f32::INFINITY
        }
    };
    let calc_t_z = |z_max: f32| {
        if dz > 1e-6 {
            (z_max - z) / dz
        } else if dz < -1e-6 {
            (-z_max - z) / dz
        } else {
            f32::INFINITY
        }
    };

    // Both boundaries vary along the rail (narrower near every pocket):
    // refine each once using the coordinate the ray would actually reach
    // by its first estimate, since that's what decides which it really
    // hits.
    let mut t_x = calc_t_x(safe_half_width(z.abs()));
    if t_x.is_finite() {
        t_x = calc_t_x(safe_half_width((z + dz * t_x).abs()));
    }
    let mut t_z = calc_t_z(safe_half_length(x.abs()));
    if t_z.is_finite() {
        t_z = calc_t_z(safe_half_length((x + dx * t_z).abs()));
    }

    t_x.min(t_z)
}

pub const CLOTH_COLOR: Color = Color::new(20, 110, 60, 255);
pub const CUSHION_COLOR: Color = Color::new(15, 80, 45, 255);
pub const POCKET_COLOR: Color = Color::BLACK;

// Loaded table model (assets/snooker_table.glb, stripped of balls/lamp/cues
// by scripts/strip_table_model.py -- see that script for provenance and
// licensing). Set to `false` as an escape hatch back to the procedural
// table below, in case the model's alignment needs adjusting.
pub const USE_TABLE_MODEL: bool = true;
pub const TABLE_MODEL_PATH: &str = "assets/snooker_table.glb";
// Measured directly from the model's "Baize" (cloth) mesh bounding box:
// the flat bed sits at local (X center 0.1, Z center -1.879, Y top
// 0.8697), not at its own origin. This offset brings that point to our
// world's table center / cloth height (0, 0, 0), matching where the
// procedural table, balls, and gameplay math all assume the cloth is.
pub const TABLE_MODEL_OFFSET_X: f32 = -0.1;
pub const TABLE_MODEL_OFFSET_Y: f32 = -0.8697;
pub const TABLE_MODEL_OFFSET_Z: f32 = 1.879;

// Real cue and ball meshes/textures, extracted from the same source model
// by scripts/extract_props.py (see that script for how/why).
pub const USE_MODEL_PROPS: bool = true;

// balls.glb: mesh indices and baked centers for the one cue-ball mesh and
// one (of 15 identical) red-ball mesh we actually use, measured the same
// way as the table's Baize offset. Node transforms were left untouched by
// extract_props.py, so these centers come straight from the original
// rack layout in the source file.
pub const BALLS_MODEL_PATH: &str = "assets/balls.glb";
pub const CUE_BALL_MESH_INDEX: usize = 5;
pub const RED_BALL_MESH_INDEX: usize = 7;
pub const CUE_BALL_MODEL_CENTER: Vector3 = Vector3 { x: 0.23129013, y: 0.89566159, z: -0.64475494 };
pub const RED_BALL_MODEL_CENTER: Vector3 = Vector3 { x: 0.10001251, y: 0.89566159, z: -3.01039052 };

pub const CUE_BALL_COLOR: Color = Color::WHITE;
pub const OBJECT_BALL_COLOR: Color = Color::new(200, 30, 30, 255); // red ball

// Room backdrop so the table doesn't render in empty space ("abandoned VR
// gallery" -- downloaded separately, not part of this repo's other assets).
// Measured its two meshes' (RoomBaked/PropsBaked) glTF node bounding boxes
// directly: floor sits at local Y 0, and the room spans X [0, 9.8] / Z
// [-15, 0.1] (not centered on its own origin) -- the X/Z offsets recenter
// it under the table.
//
// The Y offset is *not* 0, even though the room's floor is already at its
// own local 0: world Y 0 is the table's *cloth* height (every game-logic
// coordinate assumes that), not the floor the table's legs stand on. The
// table model's own lowest point (its feet, checked directly against
// snooker_table.glb) sits at local Y ~0, and TABLE_MODEL_OFFSET_Y is
// -0.8697 (measured from the Baize/cloth mesh, see below) -- so in world
// space the feet actually rest at Y ~-0.87, not Y 0. The room's floor has
// to line up with that, or the table looks like it's sunk into the floor
// up to the cloth.
pub const USE_GALLERY_MODEL: bool = true;
pub const GALLERY_MODEL_PATH: &str = "assets/gallery.glb";
pub const GALLERY_MODEL_OFFSET_X: f32 = -4.868618;
pub const GALLERY_MODEL_OFFSET_Y: f32 = -0.8697;
pub const GALLERY_MODEL_OFFSET_Z: f32 = 7.451210;

// Sky backdrop ("free_-_skybox_anime_sky", downloaded separately) so the
// gallery's windows show a bright daytime sky instead of the near-black
// clear-color -- a single sphere of radius 500, already centered on the
// origin, so no offset is needed. It's drawn unlit (no shader assigned, see
// Assets::load) since a skybox shouldn't respond to the table's overhead
// lights, and like the gallery its interior-facing normals need back-face
// culling disabled to render from inside.
pub const USE_SKY_MODEL: bool = true;
pub const SKY_MODEL_PATH: &str = "assets/sky.glb";

// Overhead LED light bank: a row of wide rectangular panels, like the
// segmented shade units over a real snooker table, rather than one point.
pub const LIGHT_PANEL_COUNT: usize = 3;
pub const LIGHT_HEIGHT: f32 = 1.0;
pub const LIGHT_PANEL_WIDTH: f32 = TABLE_WIDTH * 0.65;
pub const LIGHT_PANEL_THICKNESS: f32 = 0.04;
pub const LIGHT_PANEL_GAP: f32 = 0.08;
pub const LIGHT_PANEL_COLOR: Color = Color::new(255, 250, 235, 255);
pub const LIGHT_COLOR_INTENSITY: f32 = 0.42; // per light, so 3 lights don't overblow brightness

pub struct Pocket {
    pub position: Vector3,
    pub radius: f32,
}

pub fn pockets() -> Vec<Pocket> {
    let hw = TABLE_WIDTH / 2.0;
    let hl = TABLE_LENGTH / 2.0;
    vec![
        Pocket { position: Vector3::new(-hw, 0.0, -hl), radius: CORNER_POCKET_RADIUS },
        Pocket { position: Vector3::new(hw, 0.0, -hl), radius: CORNER_POCKET_RADIUS },
        Pocket { position: Vector3::new(-hw, 0.0, hl), radius: CORNER_POCKET_RADIUS },
        Pocket { position: Vector3::new(hw, 0.0, hl), radius: CORNER_POCKET_RADIUS },
        Pocket { position: Vector3::new(-hw, 0.0, 0.0), radius: MIDDLE_POCKET_RADIUS },
        Pocket { position: Vector3::new(hw, 0.0, 0.0), radius: MIDDLE_POCKET_RADIUS },
    ]
}

/// Picks a random point on the playing surface that stays clear of the
/// cushions and every pocket mouth, so balls never spawn half-sunk.
pub fn random_ball_position(pockets: &[Pocket], taken: &[Vector3]) -> Vector3 {
    // Rough outer sampling box (cheap to draw from); the real boundary
    // check below (safe_half_width, against the model's measured
    // geometry) is what actually decides whether a candidate is accepted.
    let margin = BALL_RADIUS * 2.0;
    let hw = TABLE_WIDTH / 2.0 - margin;
    let hl = TABLE_LENGTH / 2.0 - margin;

    loop {
        let x = rand::random_range(-hw..hw);
        let z = rand::random_range(-hl..hl);
        let candidate = Vector3::new(x, BALL_RADIUS, z);

        // Extra breathing room beyond bare wall clearance, same idea as the
        // old flat `margin` above, but checked against the real cushion.
        let clear_of_cushion = x.abs() < safe_half_width(z.abs()) - BALL_RADIUS
            && z.abs() < safe_half_length(x.abs()) - BALL_RADIUS;
        let clear_of_pockets = pockets
            .iter()
            .all(|p| candidate.distance(p.position) > p.radius + BALL_RADIUS * 2.0);
        let clear_of_balls = taken.iter().all(|b| candidate.distance(*b) > MIN_BALL_SEPARATION);

        if clear_of_cushion && clear_of_pockets && clear_of_balls {
            return candidate;
        }
    }
}

/// Centers of the overhead light panel segments, evenly spanning the
/// table's length. Doubles as both the visual panel positions and the
/// point-light positions used for shading.
pub fn light_panel_centers() -> [Vector3; LIGHT_PANEL_COUNT] {
    let segment_len = TABLE_LENGTH / LIGHT_PANEL_COUNT as f32;
    let mut centers = [Vector3::zero(); LIGHT_PANEL_COUNT];
    for (i, center) in centers.iter_mut().enumerate() {
        let z = -TABLE_LENGTH / 2.0 + segment_len * (i as f32 + 0.5);
        *center = Vector3::new(0.0, LIGHT_HEIGHT, z);
    }
    centers
}

pub fn draw_light_fixture(d: &mut impl RaylibDraw3D, centers: &[Vector3]) {
    let segment_len = TABLE_LENGTH / LIGHT_PANEL_COUNT as f32 - LIGHT_PANEL_GAP;
    for &center in centers {
        d.draw_cube(
            center,
            LIGHT_PANEL_WIDTH,
            LIGHT_PANEL_THICKNESS,
            segment_len,
            LIGHT_PANEL_COLOR,
        );
    }
}

pub fn draw_table(d: &mut impl RaylibDraw3D, pockets: &[Pocket]) {
    d.draw_plane(
        Vector3::new(0.0, 0.0, 0.0),
        Vector2::new(TABLE_WIDTH, TABLE_LENGTH),
        CLOTH_COLOR,
    );

    let hw = TABLE_WIDTH / 2.0;
    let hl = TABLE_LENGTH / 2.0;
    let cy = CUSHION_HEIGHT / 2.0;

    // Long rails (run along Z), short rails (run along X), each split at the
    // middle pockets so the pocket mouths stay open.
    d.draw_cube(
        Vector3::new(-hw - CUSHION_THICKNESS / 2.0, cy, 0.0),
        CUSHION_THICKNESS,
        CUSHION_HEIGHT,
        TABLE_LENGTH,
        CUSHION_COLOR,
    );
    d.draw_cube(
        Vector3::new(hw + CUSHION_THICKNESS / 2.0, cy, 0.0),
        CUSHION_THICKNESS,
        CUSHION_HEIGHT,
        TABLE_LENGTH,
        CUSHION_COLOR,
    );
    d.draw_cube(
        Vector3::new(0.0, cy, -hl - CUSHION_THICKNESS / 2.0),
        TABLE_WIDTH,
        CUSHION_HEIGHT,
        CUSHION_THICKNESS,
        CUSHION_COLOR,
    );
    d.draw_cube(
        Vector3::new(0.0, cy, hl + CUSHION_THICKNESS / 2.0),
        TABLE_WIDTH,
        CUSHION_HEIGHT,
        CUSHION_THICKNESS,
        CUSHION_COLOR,
    );

    for pocket in pockets {
        d.draw_cylinder(
            Vector3::new(pocket.position.x, -0.001, pocket.position.z),
            pocket.radius,
            pocket.radius,
            0.01,
            24,
            POCKET_COLOR,
        );
    }
}
