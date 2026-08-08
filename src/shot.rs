use raylib::prelude::*;

use crate::table::{cushion_t, random_ball_position, safe_half_length, safe_half_width, BALL_RADIUS, MAX_PLACEMENT_ATTEMPTS, MAX_REALISTIC_CUT_DEG, Pocket};

pub const GATE_POST_RADIUS: f32 = 0.008;
pub const GATE_POST_HEIGHT: f32 = 0.09;
pub const PATH_HEIGHT: f32 = 0.0015; // path stripes sit just above the cloth

pub const GHOST_BALL_COLOR: Color = Color::new(255, 255, 255, 90);
pub const AIM_LINE_COLOR: Color = Color::new(255, 220, 40, 230);
pub const GHOST_RED_BALL_COLOR: Color = Color::new(230, 60, 60, 110);
pub const GATE_NEUTRAL_COLOR: Color = Color::new(40, 120, 235, 255); // blue -- near-white read poorly against the white gallery room
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

pub struct ShotTest {
    pub white_end: Vector3,
    pub red_path: Option<(Vector3, Vector3)>,
    /// Index into `pockets()` of whichever pocket the object ball actually
    /// fell into, if any -- not necessarily the target pocket. A shot can
    /// physically pot the object ball through *any* pocket's gate, not
    /// just the one the layout was generated to favor (see `best_pocket`);
    /// this is what actually happened, for the caller to compare against
    /// whatever pocket it cares about.
    pub pocketed: Option<usize>,
}

/// The pocket's own fixed "straight in" direction -- for a middle pocket,
/// straight across, perpendicular to the long rail; for a corner pocket,
/// the bisector of the local right angle between the long and short rail
/// meeting there, which is *always* exactly 45° regardless of the table's
/// overall proportions. Deliberately NOT the direction toward the table's
/// center point -- since TABLE_LENGTH != TABLE_WIDTH, that direction is
/// skewed off the true 45° (about 26.5° for this table), biased toward
/// the long-rail side. Fixed per pocket position; does not depend on
/// where the object ball currently is, so the gate built from it (see
/// `pocket_jaws`) never rotates with aim -- only the camera moving around
/// it can make it look different from frame to frame.
fn pocket_mouth_dir(pocket_pos: Vector3) -> (f32, f32) {
    if pocket_pos.z.abs() < 1e-4 {
        (-pocket_pos.x.signum(), 0.0)
    } else {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        (-pocket_pos.x.signum() * s, -pocket_pos.z.signum() * s)
    }
}

// pockets()'s position is the idealized rail-corner coordinate
// (±TABLE_WIDTH/2, ±TABLE_LENGTH/2) -- convenient for placement/collision
// elsewhere, but not where the pocket actually opens. Measured directly
// from the real cushion-nose boundary data already extracted from the
// table mesh (CUSHION_BOUNDARY/SHORT_RAIL_BOUNDARY, see
// scripts/extract_cushion_segments.py): each rail's boundary rises
// through a flare zone near a pocket and then plateaus once "no more
// cushion, this is pocket" -- found where the long-rail boundary reaches
// that plateau (z: 1.6928, vs. the idealized TABLE_LENGTH/2 = 1.7845 --
// inset 0.0917) and where the short-rail boundary does the same (x:
// 0.7988 vs TABLE_WIDTH/2 = 0.889 -- inset 0.0902). Both corner insets
// agree to within 1.5mm, consistent with the extraction script's own
// finding that the cushion cross-section is identical on every rail, so
// one constant (their average) covers all four corners.
//
// The middle pockets' mouth-zone data shows the same thing but much
// smaller: the long-rail boundary is already within a few mm of its peak
// (0.8865) at the smallest along-rail value the data has (z=0.052,
// clamped from z=0), giving inset 0.889-0.8865 = 0.0025 -- negligible
// depth-wise, just a small outward-coordinate correction.
const CORNER_ENTRANCE_INSET: f32 = 0.0909; // (0.0917 + 0.0902) / 2
const MIDDLE_ENTRANCE_INSET: f32 = 0.0025;

/// Where the pocket actually opens, inset from `pockets()`'s idealized
/// rail-corner coordinate by `CORNER_ENTRANCE_INSET`/`MIDDLE_ENTRANCE_INSET`
/// -- see the comment above. Used only for gate placement (`pocket_jaws`);
/// everything else (ball-clearance checks, pocket rendering, cut-angle
/// math) keeps using the plain idealized position, since that's a separate
/// concern from where the gate itself should sit.
fn pocket_entrance(pocket_pos: Vector3) -> Vector3 {
    if pocket_pos.z.abs() < 1e-4 {
        Vector3::new(pocket_pos.x - pocket_pos.x.signum() * MIDDLE_ENTRANCE_INSET, pocket_pos.y, pocket_pos.z)
    } else {
        Vector3::new(
            pocket_pos.x - pocket_pos.x.signum() * CORNER_ENTRANCE_INSET,
            pocket_pos.y,
            pocket_pos.z - pocket_pos.z.signum() * CORNER_ENTRANCE_INSET,
        )
    }
}

/// The pocket's two fixed jaw points, `pocket_radius` out from the
/// pocket's real entrance (`pocket_entrance`, not the idealized
/// `pocket_pos`) on either side of `pocket_mouth_dir`, approximating the
/// real physical entrance a ball must pass between to be potted.
fn pocket_jaws(pocket_pos: Vector3, pocket_radius: f32) -> ((f32, f32), (f32, f32)) {
    let (mx, mz) = pocket_mouth_dir(pocket_pos);
    let entrance = pocket_entrance(pocket_pos);
    let (px, pz) = (-mz * pocket_radius, mx * pocket_radius);
    (
        (entrance.x + px, entrance.z + pz),
        (entrance.x - px, entrance.z - pz),
    )
}

/// Simulates a dead-straight shot from the current cue direction: traces
/// the cue ball to its first contact (object ball or cushion), then — if it
/// hit the object ball — traces the object ball's resulting path (straight
/// through its center, no spin) to its own first event: passing through
/// *any* pocket's gate (potted -- not necessarily the target pocket, since
/// a ball headed into a mouth pots regardless of which pocket the layout
/// happened to favor) or hitting a cushion (missed). Potted means: the
/// ball's straight-line path, given its own radius, passes between a
/// pocket's two fixed jaw points (`pocket_jaws`) without touching either.
/// Deliberately *not* compared against `cushion_t` -- that boundary is
/// measured off the cushion nose's own flare geometry, which (like a real
/// table) rises to meet the cloth right at the pocket mouth with no gap,
/// so it's always reached at or before the gate line; requiring the gate
/// to win that race would make potting impossible everywhere, not just
/// non-target pockets.
pub fn test_shot(
    shot_dir: (f32, f32),
    cue_ball_pos: Vector3,
    object_ball_pos: Vector3,
    pockets: &[Pocket],
) -> ShotTest {
    let raycast = cue_raycast(shot_dir, cue_ball_pos, object_ball_pos);
    let white_end = raycast.ghost_pos;

    if !raycast.hit_object_ball {
        return ShotTest { white_end, red_path: None, pocketed: None };
    }

    // Object ball departs along the line from the contact point through its
    // own center — the standard no-spin "ghost ball" approximation.
    let rdx = object_ball_pos.x - white_end.x;
    let rdz = object_ball_pos.z - white_end.z;
    let rlen = (rdx * rdx + rdz * rdz).sqrt();
    if rlen < 1e-5 {
        return ShotTest { white_end, red_path: None, pocketed: None };
    }
    let (rdx, rdz) = (rdx / rlen, rdz / rlen);

    let t_red_cushion = cushion_t(object_ball_pos.x, object_ball_pos.z, rdx, rdz);

    // Fits through a gate only if the ball's edge clears *both* fixed jaw
    // points by BALL_RADIUS -- checked directly as the perpendicular
    // distance from each jaw to the ball's actual path line -- and its
    // center's path actually crosses between them (ray_segment_t). The
    // drawn post (GATE_POST_RADIUS) is just a visual marker for the jaw's
    // location, not a real obstacle -- real pockets have no post there --
    // so it plays no part in this check. Checked against every pocket
    // (not just the target), taking whichever gate the path reaches first
    // -- a straight line can plausibly clear more than one pocket's jaws
    // at once (e.g. skimming past a middle pocket on the way to a corner),
    // so "first" is what decides which one the ball actually falls into.
    let origin = (object_ball_pos.x, object_ball_pos.z);
    let mut earliest_gate: Option<(usize, f32)> = None;
    for (i, pocket) in pockets.iter().enumerate() {
        let (jaw_a, jaw_b) = pocket_jaws(pocket.position, pocket.radius);
        let clears_jaw = |jaw: (f32, f32)| {
            let to_jaw = (jaw.0 - origin.0, jaw.1 - origin.1);
            cross2((rdx, rdz), to_jaw).abs() >= BALL_RADIUS
        };
        if !(clears_jaw(jaw_a) && clears_jaw(jaw_b)) {
            continue;
        }
        if let Some(t) = ray_segment_t(origin, (rdx, rdz), jaw_a, jaw_b) {
            if earliest_gate.is_none_or(|(_, best_t)| t < best_t) {
                earliest_gate = Some((i, t));
            }
        }
    }

    let (red_end_t, pocketed) = match earliest_gate {
        Some((i, t)) => (t, Some(i)),
        None => (t_red_cushion, None),
    };
    let red_end = Vector3::new(
        object_ball_pos.x + rdx * red_end_t,
        BALL_RADIUS,
        object_ball_pos.z + rdz * red_end_t,
    );

    ShotTest {
        white_end,
        red_path: Some((object_ball_pos, red_end)),
        pocketed,
    }
}

/// Draws the potting "gate" at a pocket: two posts at its fixed jaw points
/// (see `pocket_jaws`) -- the same points `test_shot` checks a shot's path
/// against, so the drawn gate and the pass/fail check always agree. Fixed
/// per pocket, not per aim, so this doesn't rotate as the object ball
/// moves; any apparent rotation on screen is just the camera.
pub fn draw_gate(d: &mut impl RaylibDraw3D, pocket_pos: Vector3, pocket_radius: f32, color: Color) {
    let (jaw_a, jaw_b) = pocket_jaws(pocket_pos, pocket_radius);
    let a = Vector3::new(jaw_a.0, 0.0, jaw_a.1);
    let b = Vector3::new(jaw_b.0, 0.0, jaw_b.1);
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

#[cfg(test)]
mod pot_tests {
    use super::*;
    use crate::table::pockets;

    /// Builds a cue/object ball pair such that a dead-straight shot sends
    /// the object ball directly at `pockets[target_idx]`, then runs
    /// `test_shot` and asserts it's recognized as potted there. Regression
    /// coverage for a bug where comparing the gate crossing's `t` against
    /// `cushion_t` made every pot impossible (see `test_shot`'s doc
    /// comment) -- caught because it broke potting into every pocket, not
    /// just non-target ones, once gates were checked for all of them.
    fn assert_pots(object_ball_pos: Vector3, target_idx: usize) {
        let pockets = pockets();
        let target = pockets[target_idx].position;
        let dx = target.x - object_ball_pos.x;
        let dz = target.z - object_ball_pos.z;
        let len = (dx * dx + dz * dz).sqrt();
        let dir = (dx / len, dz / len);

        // Cue ball placed so its contact point with the object ball sends
        // the object ball exactly along `dir` (standard ghost-ball offset,
        // approaching from further back along the same line).
        let contact = Vector3::new(
            object_ball_pos.x - dir.0 * BALL_RADIUS * 2.0,
            BALL_RADIUS,
            object_ball_pos.z - dir.1 * BALL_RADIUS * 2.0,
        );
        let cue_ball_pos = Vector3::new(contact.x - dir.0 * 0.3, BALL_RADIUS, contact.z - dir.1 * 0.3);

        let result = test_shot(dir, cue_ball_pos, object_ball_pos, &pockets);
        assert_eq!(result.pocketed, Some(target_idx));
    }

    #[test]
    fn pots_into_middle_pocket() {
        assert_pots(Vector3::new(0.0, BALL_RADIUS, 0.0), 4);
    }

    #[test]
    fn pots_into_corner_pocket() {
        assert_pots(Vector3::new(0.0, BALL_RADIUS, 1.0), 2);
    }
}
