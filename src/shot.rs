use raylib::prelude::*;

use crate::table::{cushion_t, random_ball_position, safe_half_length, safe_half_width, BALL_RADIUS, MAX_PLACEMENT_ATTEMPTS, MAX_REALISTIC_CUT_DEG, Pocket};

pub const GATE_POST_RADIUS: f32 = 0.008;
pub const GATE_POST_HEIGHT: f32 = 0.09;
pub const PATH_HEIGHT: f32 = 0.0015; // path stripes sit just above the cloth

pub const GHOST_BALL_COLOR: Color = Color::new(255, 255, 255, 90);
pub const AIM_LINE_COLOR: Color = Color::new(255, 220, 40, 230);
pub const GHOST_RED_BALL_COLOR: Color = Color::new(230, 60, 60, 110);
pub const GATE_NEUTRAL_COLOR: Color = Color::new(230, 230, 230, 255);
pub const GATE_SUCCESS_COLOR: Color = Color::new(50, 220, 60, 255);
pub const GATE_MISS_COLOR: Color = Color::new(220, 50, 50, 255);
pub const PATH_WHITE_COLOR: Color = Color::new(255, 255, 255, 110);
pub const PATH_RED_COLOR: Color = Color::new(230, 60, 60, 110);

pub fn cross2(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.1 - a.1 * b.0
}

/// `t` along ray `origin + dir*t` where it crosses segment `a`-`b`, if the
/// crossing is ahead of the ray and within the segment's bounds.
pub fn ray_segment_t(origin: (f32, f32), dir: (f32, f32), a: (f32, f32), b: (f32, f32)) -> Option<f32> {
    let ab = (b.0 - a.0, b.1 - a.1);
    let denom = cross2(dir, ab);
    if denom.abs() < 1e-6 {
        return None;
    }
    let ao = (a.0 - origin.0, a.1 - origin.1);
    let t = cross2(ao, ab) / denom;
    let s = cross2(ao, dir) / denom;
    (t > 0.0 && (0.0..=1.0).contains(&s)).then_some(t)
}

/// Where the cue ball's center would be at its first contact — either with
/// the object ball or a cushion — if struck dead straight along the
/// current cue direction, and which of the two it was. Pure geometry, no
/// physics: a straight-line raycast in the table plane.
pub struct CueRaycast {
    pub ghost_pos: Vector3,
    pub hit_object_ball: bool,
}

pub fn cue_raycast(shot_dir: (f32, f32), cue_ball_pos: Vector3, object_ball_pos: Vector3) -> CueRaycast {
    let (dx, dz) = shot_dir;

    // Contact with the object ball: 2D ray-circle intersection, where the
    // circle radius is the sum of both ball radii (centers meet at contact).
    let ocx = cue_ball_pos.x - object_ball_pos.x;
    let ocz = cue_ball_pos.z - object_ball_pos.z;
    let contact_r = BALL_RADIUS * 2.0;
    let b = 2.0 * (ocx * dx + ocz * dz);
    let c = ocx * ocx + ocz * ocz - contact_r * contact_r;
    let discriminant = b * b - 4.0 * c;
    let t_ball = (discriminant >= 0.0)
        .then(|| (-b - discriminant.sqrt()) / 2.0)
        .filter(|t| *t > 0.0);

    let t_cushion = cushion_t(cue_ball_pos.x, cue_ball_pos.z, dx, dz);
    let contact = t_ball.filter(|t| *t < t_cushion);
    let t = contact.unwrap_or(t_cushion);

    CueRaycast {
        ghost_pos: Vector3::new(cue_ball_pos.x + dx * t, BALL_RADIUS, cue_ball_pos.z + dz * t),
        hit_object_ball: contact.is_some(),
    }
}

/// Picks the pocket that gives the easiest ("straightest") pot for the
/// object ball: for each pocket, the cue ball's required contact point
/// (ghost-ball position) must actually lie on the table, and among those,
/// prefer the smallest cut angle — the angle between the cue ball's
/// approach direction and the object ball's required departure direction.
/// 0° is a straight in-line pot; beyond ~90° a cut is physically
/// impossible. Falls back to the nearest pocket if every cut is too thin.
/// Returns the pocket index, its departure direction (object ball → pocket,
/// in the table plane), and the cut angle actually achieved (radians;
/// `f32::INFINITY` if no pocket had a reachable contact point at all).
pub fn best_pocket(
    pockets: &[Pocket],
    cue_ball_pos: Vector3,
    object_ball_pos: Vector3,
) -> (usize, (f32, f32), f32) {
    let mut best: Option<(usize, f32, (f32, f32))> = None;
    let mut nearest: Option<(usize, f32, (f32, f32))> = None;

    for (i, pocket) in pockets.iter().enumerate() {
        let pdx = object_ball_pos.x - pocket.position.x;
        let pdz = object_ball_pos.z - pocket.position.z;
        let plen = (pdx * pdx + pdz * pdz).sqrt();
        if plen < 1e-4 {
            continue;
        }
        let departure = (-pdx / plen, -pdz / plen); // object ball -> pocket
        let approach_from = (pdx / plen, pdz / plen); // pocket -> object ball

        if nearest.is_none_or(|(_, best_len, _)| plen < best_len) {
            nearest = Some((i, plen, departure));
        }

        let ghost_x = object_ball_pos.x + approach_from.0 * BALL_RADIUS * 2.0;
        let ghost_z = object_ball_pos.z + approach_from.1 * BALL_RADIUS * 2.0;
        if ghost_x.abs() > safe_half_width(ghost_z.abs()) || ghost_z.abs() > safe_half_length(ghost_x.abs()) {
            continue; // cue ball couldn't physically sit here
        }

        let adx = ghost_x - cue_ball_pos.x;
        let adz = ghost_z - cue_ball_pos.z;
        let alen = (adx * adx + adz * adz).sqrt();
        if alen < 1e-4 {
            continue;
        }
        let cos_cut = (adx / alen) * departure.0 + (adz / alen) * departure.1;
        let cut_angle = cos_cut.clamp(-1.0, 1.0).acos();

        if best.is_none_or(|(_, best_angle, _)| cut_angle < best_angle) {
            best = Some((i, cut_angle, departure));
        }
    }

    match best {
        Some((i, angle, dir)) if angle <= 80f32.to_radians() => (i, dir, angle),
        _ => {
            let (i, _, dir) = nearest.unwrap_or((0, 0.0, (0.0, 1.0)));
            let angle = best.map_or(f32::INFINITY, |(_, a, _)| a);
            (i, dir, angle)
        }
    }
}

/// Rerolls cue/object ball positions until the layout is realistic: the
/// balls aren't nearly touching, and at least one pocket offers a pot
/// within a makeable cut angle (not a near-90° sliver).
pub fn random_shot_setup(pockets: &[Pocket]) -> (Vector3, Vector3) {
    for _ in 0..MAX_PLACEMENT_ATTEMPTS {
        let cue_ball_pos = random_ball_position(pockets, &[]);
        let object_ball_pos = random_ball_position(pockets, &[cue_ball_pos]);
        let (_, _, cut_angle) = best_pocket(pockets, cue_ball_pos, object_ball_pos);
        if cut_angle <= MAX_REALISTIC_CUT_DEG.to_radians() {
            return (cue_ball_pos, object_ball_pos);
        }
    }
    let cue_ball_pos = random_ball_position(pockets, &[]);
    let object_ball_pos = random_ball_position(pockets, &[cue_ball_pos]);
    (cue_ball_pos, object_ball_pos)
}

pub enum GateState {
    Success,
    Miss,
}

pub struct ShotTest {
    pub white_end: Vector3,
    pub red_path: Option<(Vector3, Vector3)>,
    pub gate_state: GateState,
}

/// Half-width of the potting gate for an object ball crossing at angle ε
/// from the ideal straight-on line to the pocket (`cos_eps` = cos ε; 1.0 is
/// a dead-straight approach). `pocket_radius` approximates half the
/// pocket's true mouth width.
///
/// Derived by treating each jaw as a fixed point the ball's center must
/// clear by BALL_RADIUS at all times along its path (not just at the
/// crossing point): for a path crossing the gate line at offset x0 from
/// the pocket center, the perpendicular distance from a jaw at ±pocket_radius
/// to that path is (pocket_radius ∓ x0)·cos(ε), which must be ≥ BALL_RADIUS
/// on both sides. Solving both inequalities for x0 gives the valid range
/// [-h, h] where h = pocket_radius − BALL_RADIUS/cos(ε) -- this function.
/// At ε=0 that's just pocket_radius − BALL_RADIUS (full ball clearance on
/// each side); it shrinks as ε grows and goes negative once potting is
/// geometrically impossible at that angle regardless of aim (the point
/// where the pocket's angle-foreshortened projected width drops below the
/// ball's own diameter).
fn gate_half_width(pocket_radius: f32, cos_eps: f32) -> f32 {
    pocket_radius - BALL_RADIUS / cos_eps
}

/// Simulates a dead-straight shot from the current cue direction: traces
/// the cue ball to its first contact (object ball or cushion), then — if it
/// hit the object ball — traces the object ball's resulting path (straight
/// through its center, no spin) to its own first event: passing through the
/// target gate (potted) or hitting a cushion (missed). The gate narrows as
/// the resulting path deviates from the ideal straight-on line to the
/// pocket -- see `gate_half_width`.
pub fn test_shot(
    shot_dir: (f32, f32),
    cue_ball_pos: Vector3,
    object_ball_pos: Vector3,
    pocket_pos: Vector3,
    pocket_radius: f32,
    gate_dir: (f32, f32),
) -> ShotTest {
    let raycast = cue_raycast(shot_dir, cue_ball_pos, object_ball_pos);
    let white_end = raycast.ghost_pos;

    if !raycast.hit_object_ball {
        return ShotTest {
            white_end,
            red_path: None,
            gate_state: GateState::Miss,
        };
    }

    // Object ball departs along the line from the contact point through its
    // own center — the standard no-spin "ghost ball" approximation.
    let rdx = object_ball_pos.x - white_end.x;
    let rdz = object_ball_pos.z - white_end.z;
    let rlen = (rdx * rdx + rdz * rdz).sqrt();
    if rlen < 1e-5 {
        return ShotTest {
            white_end,
            red_path: None,
            gate_state: GateState::Miss,
        };
    }
    let (rdx, rdz) = (rdx / rlen, rdz / rlen);

    let t_red_cushion = cushion_t(object_ball_pos.x, object_ball_pos.z, rdx, rdz);

    // cos(ε): how far this shot's actual departure direction deviates from
    // the ideal straight-on line to the pocket. cos_eps <= 0 means the ball
    // isn't even heading toward the pocket's side of that line -- no gate
    // is possible.
    let cos_eps = rdx * gate_dir.0 + rdz * gate_dir.1;
    let t_gate = (cos_eps > 0.0)
        .then(|| gate_half_width(pocket_radius, cos_eps))
        .filter(|half| *half > 0.0)
        .and_then(|half| {
            let (px, pz) = (-gate_dir.1 * half, gate_dir.0 * half);
            let gate_a = (pocket_pos.x + px, pocket_pos.z + pz);
            let gate_b = (pocket_pos.x - px, pocket_pos.z - pz);
            ray_segment_t((object_ball_pos.x, object_ball_pos.z), (rdx, rdz), gate_a, gate_b)
        });

    // The gate sits at the true rail, at the pocket's opening — a bit
    // further out than the ball-radius-inset boundary `cushion_t` treats as
    // a solid wall. So a shot heading into the pocket always reaches the
    // (inset) "cushion" a hair before the gate; a valid gate crossing must
    // take priority, since that inset boundary isn't a real wall there.
    let (red_end_t, gate_state) = match t_gate {
        Some(t_gate) => (t_gate, GateState::Success),
        None => (t_red_cushion, GateState::Miss),
    };
    let red_end = Vector3::new(
        object_ball_pos.x + rdx * red_end_t,
        BALL_RADIUS,
        object_ball_pos.z + rdz * red_end_t,
    );

    ShotTest {
        white_end,
        red_path: Some((object_ball_pos, red_end)),
        gate_state,
    }
}

/// Draws the potting "gate" at a pocket: two posts at the pocket's real
/// physical jaw positions (`pocket_radius` apart, unlike the ball's
/// *center*-tolerance zone `gate_half_width` computes for the pass/fail
/// check in `test_shot`, which is narrower and deliberately not what's
/// drawn here). The jaws don't move with aim, so this is the same
/// regardless of the current shot's angle -- what actually determines
/// success is where a shot's ball-width path stripe (see
/// `draw_path_stripe`) falls relative to these posts: the stripe's own
/// edge reaches a post exactly at the true pass/fail boundary, so a near
/// miss visually clips a post instead of the gate itself silently
/// shrinking to an abstraction the player never sees.
pub fn draw_gate(d: &mut impl RaylibDraw3D, pocket_pos: Vector3, pocket_radius: f32, gate_dir: (f32, f32), color: Color) {
    let half = pocket_radius;
    let (px, pz) = (-gate_dir.1 * half, gate_dir.0 * half);
    let a = Vector3::new(pocket_pos.x + px, 0.0, pocket_pos.z + pz);
    let b = Vector3::new(pocket_pos.x - px, 0.0, pocket_pos.z - pz);
    d.draw_cylinder(a, GATE_POST_RADIUS, GATE_POST_RADIUS, GATE_POST_HEIGHT, 10, color);
    d.draw_cylinder(b, GATE_POST_RADIUS, GATE_POST_RADIUS, GATE_POST_HEIGHT, 10, color);
    d.draw_line3D(
        Vector3::new(a.x, GATE_POST_HEIGHT, a.z),
        Vector3::new(b.x, GATE_POST_HEIGHT, b.z),
        color,
    );
}

/// Draws a ball's swept path as a flat "stadium" (rectangle + round caps)
/// stripe lying on the cloth, one ball-width wide.
pub fn draw_path_stripe(d: &mut impl RaylibDraw3D, start: Vector3, end: Vector3, color: Color) {
    let dx = end.x - start.x;
    let dz = end.z - start.z;
    let len = (dx * dx + dz * dz).sqrt();
    if len < 1e-4 {
        return;
    }
    let (ux, uz) = (dx / len, dz / len);
    let (px, pz) = (-uz * BALL_RADIUS, ux * BALL_RADIUS);

    let p0 = Vector3::new(start.x - px, PATH_HEIGHT, start.z - pz);
    let p1 = Vector3::new(start.x + px, PATH_HEIGHT, start.z + pz);
    let p2 = Vector3::new(end.x - px, PATH_HEIGHT, end.z - pz);
    let p3 = Vector3::new(end.x + px, PATH_HEIGHT, end.z + pz);
    d.draw_triangle_strip3D(&[p0, p1, p2, p3], color);

    let cap_y = PATH_HEIGHT - 0.001;
    d.draw_cylinder(
        Vector3::new(start.x, cap_y, start.z),
        BALL_RADIUS,
        BALL_RADIUS,
        0.002,
        20,
        color,
    );
    d.draw_cylinder(
        Vector3::new(end.x, cap_y, end.z),
        BALL_RADIUS,
        BALL_RADIUS,
        0.002,
        20,
        color,
    );
}

/// Draws a line from the top of the ghost cue ball through the top of the
/// object ball, continuing on to the cushion — a live preview of the
/// object ball's resulting travel direction for the current aim.
pub fn draw_object_ball_aim_line(d: &mut impl RaylibDraw3D, ghost_pos: Vector3, object_ball_pos: Vector3) {
    let dx = object_ball_pos.x - ghost_pos.x;
    let dz = object_ball_pos.z - ghost_pos.z;
    let len = (dx * dx + dz * dz).sqrt();
    if len < 1e-5 {
        return;
    }
    let (ux, uz) = (dx / len, dz / len);
    let t_cushion = cushion_t(object_ball_pos.x, object_ball_pos.z, ux, uz);
    let top = BALL_RADIUS * 2.0;
    let start = Vector3::new(ghost_pos.x, top, ghost_pos.z);
    let end = Vector3::new(
        object_ball_pos.x + ux * t_cushion,
        top,
        object_ball_pos.z + uz * t_cushion,
    );
    d.draw_line3D(start, end, AIM_LINE_COLOR);
}
