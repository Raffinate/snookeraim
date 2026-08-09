use raylib::prelude::*;

use crate::table::{cushion_t, random_ball_position, safe_half_length, safe_half_width, BALL_RADIUS, MAX_PLACEMENT_ATTEMPTS, MAX_REALISTIC_CUT_DEG, Pocket};

pub const PATH_HEIGHT: f32 = 0.0015; // path stripes sit just above the cloth

pub const GHOST_BALL_COLOR: Color = Color::new(255, 255, 255, 90);
pub const AIM_LINE_COLOR: Color = Color::new(255, 220, 40, 230);
pub const GHOST_RED_BALL_COLOR: Color = Color::new(230, 60, 60, 110);
pub const POCKET_NEUTRAL_COLOR: Color = Color::new(40, 120, 235, 255); // blue -- near-white read poorly against the white gallery room
pub const POCKET_SUCCESS_COLOR: Color = Color::new(50, 220, 60, 255);
pub const POCKET_MISS_COLOR: Color = Color::new(220, 50, 50, 255);
pub const PATH_WHITE_COLOR: Color = Color::new(255, 255, 255, 110);
pub const PATH_RED_COLOR: Color = Color::new(230, 60, 60, 110);

/// Beyond this cut angle a pot is treated as physically impossible, not
/// just difficult -- `best_pocket`'s own ceiling for "give up and fall back
/// to the nearest pocket", also reused by puzzle.rs when deciding whether
/// a grid-sampled placement is even a legal shot.
pub const MAX_REACHABLE_CUT_DEG: f32 = 80.0;

/// `t` along ray `origin + dir*t` where it first enters the circle of
/// `radius` centered at `center`, if that happens ahead of the ray at all
/// -- same 2D ray-circle math `cue_raycast` already uses for ball-ball
/// contact, reused here for "does this ball's center cross into the
/// pocket's real boundary".
pub fn ray_circle_t(origin: (f32, f32), dir: (f32, f32), center: (f32, f32), radius: f32) -> Option<f32> {
    let ocx = origin.0 - center.0;
    let ocz = origin.1 - center.1;
    let b = 2.0 * (ocx * dir.0 + ocz * dir.1);
    let c = ocx * ocx + ocz * ocz - radius * radius;
    let discriminant = b * b - 4.0 * c;
    (discriminant >= 0.0)
        .then(|| (-b - discriminant.sqrt()) / 2.0)
        .filter(|t| *t > 0.0)
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
        if ghost_x.abs() > safe_half_width(ghost_z) || ghost_z.abs() > safe_half_length(ghost_x) {
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
        Some((i, angle, dir)) if angle <= MAX_REACHABLE_CUT_DEG.to_radians() => (i, dir, angle),
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
    /// physically pot the object ball through *any* pocket's boundary, not
    /// just the one the layout was generated to favor (see `best_pocket`);
    /// this is what actually happened, for the caller to compare against
    /// whatever pocket it cares about.
    pub pocketed: Option<usize>,
}

/// Simulates a dead-straight shot from the current cue direction: traces
/// the cue ball to its first contact (object ball or cushion), then — if it
/// hit the object ball — traces the object ball's resulting path (straight
/// through its center, no spin) to its own first event: its *center*
/// crossing into any pocket's boundary circle (potted -- not necessarily
/// the target pocket, since a ball headed into a mouth pots regardless of
/// which pocket the layout happened to favor) or hitting a cushion
/// (missed). Pockets are treated as plain cylinders: a circle of
/// `pocket.radius` centered on `pocket.position` -- the idealized rail-
/// corner coordinate (±TABLE_WIDTH/2, ±TABLE_LENGTH/2 for a corner,
/// ±TABLE_WIDTH/2 at mid-length for a middle pocket), i.e. exactly where
/// the table's two rail lines intersect. No inset correction (an earlier
/// attempt to find the "real" mesh-measured pocket mouth, via the cushion
/// boundary's flare plateau and then via the mesh's own throat-wall
/// vertices, either didn't match how it actually looked in the game or
/// wasn't reliably measurable) -- instead `pocket.radius` itself
/// (table.rs) is sized to make this off-center circle work: see that
/// constant's own comment for why corner and middle scale differently.
/// A pocket only counts if its circle is reached *before* `cushion_t`
/// (checked explicitly, not just assumed) -- with the old, small, inset
/// circles the cushion boundary's own flare geometry guaranteed this on
/// its own (no cushion left at the pocket mouth to hit first), but the
/// enlarged, non-inset circles now in use can extend into table area a
/// real cushion still covers.
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

    // Potted the moment the ball's *center* crosses into a pocket's
    // boundary circle (pocket.position, pocket.radius) -- a plain ray-
    // circle test, checked against every pocket (not just the target),
    // taking whichever one the path reaches first -- a straight line can
    // plausibly cross more than one pocket's circle (e.g. skimming past a
    // middle pocket on the way to a corner), so "first" is what decides
    // which one the ball actually falls into. Explicitly required to be
    // reached *before* the cushion (t < t_red_cushion): with the old,
    // small, inset circles the cushion boundary's own flare geometry
    // guaranteed this automatically (no cushion at the pocket mouth to
    // hit first), but the enlarged, non-inset circles now used (see
    // table.rs's *_POCKET_RADIUS comments) can extend into table area a
    // real cushion still covers, so a ball skimming past on the cushion
    // line -- never actually heading into the mouth -- must still bounce
    // off that cushion, not get credited with a pot it never reached.
    let origin = (object_ball_pos.x, object_ball_pos.z);
    let mut earliest_pocket: Option<(usize, f32)> = None;
    for (i, pocket) in pockets.iter().enumerate() {
        let center = (pocket.position.x, pocket.position.z);
        if let Some(t) = ray_circle_t(origin, (rdx, rdz), center, pocket.radius) {
            if t < t_red_cushion && earliest_pocket.is_none_or(|(_, best_t)| t < best_t) {
                earliest_pocket = Some((i, t));
            }
        }
    }

    let (red_end_t, pocketed) = match earliest_pocket {
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

/// Draws a pocket's boundary circle (`pocket_pos`, `pocket_radius`) as a
/// ring at cloth height -- the same circle `test_shot` checks a shot's
/// path against, so the drawn boundary and the pass/fail check always
/// agree. Same technique as `draw_ball_collision_ring` in table.rs.
pub fn draw_pocket_boundary(d: &mut impl RaylibDraw3D, pocket_pos: Vector3, pocket_radius: f32, color: Color) {
    const SEGMENTS: usize = 32;
    let y = 0.02;
    let mut prev = Vector3::new(pocket_pos.x + pocket_radius, y, pocket_pos.z);
    for i in 1..=SEGMENTS {
        let a = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
        let next = Vector3::new(pocket_pos.x + pocket_radius * a.cos(), y, pocket_pos.z + pocket_radius * a.sin());
        d.draw_line3D(prev, next, color);
        prev = next;
    }
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
    /// coverage for a bug where comparing the pocket-circle crossing's `t`
    /// against `cushion_t` made every pot impossible (see `test_shot`'s doc
    /// comment) -- caught because it broke potting into every pocket, not
    /// just non-target ones, once every pocket's circle was checked.
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

    /// Same as `pots_into_corner_pocket`, but from a position off both
    /// axes -- a genuine angled cut, not just the axis-aligned degenerate
    /// case `assert_pots`'s other callers happen to use. Guards against a
    /// regression where the pocket-boundary circle only lines up right
    /// along the rail directions.
    #[test]
    fn pots_into_corner_pocket_at_an_angle() {
        assert_pots(Vector3::new(0.3, BALL_RADIUS, 1.2), 3);
    }

    /// A shot aimed well wide of every pocket's real boundary circle
    /// should miss (hit a cushion), even though it starts from roughly
    /// the same neighborhood as `pots_into_corner_pocket_at_an_angle`'s
    /// object ball -- guards against the boundary circle being so large
    /// it swallows shots that were never actually heading into the mouth.
    #[test]
    fn misses_when_aimed_well_wide_of_every_pocket() {
        let pockets = pockets();
        let object_ball_pos = Vector3::new(0.3, BALL_RADIUS, 1.2);
        // Straight down the table (+X), parallel to the short rail --
        // nowhere near any pocket's boundary circle.
        let dir = (1.0, 0.0);
        let contact = Vector3::new(object_ball_pos.x - dir.0 * BALL_RADIUS * 2.0, BALL_RADIUS, object_ball_pos.z);
        let cue_ball_pos = Vector3::new(contact.x - 0.3, BALL_RADIUS, contact.z);

        let result = test_shot(dir, cue_ball_pos, object_ball_pos, &pockets);
        assert_eq!(result.pocketed, None);
    }

    /// A pocket circle that the path would only cross *after* the real
    /// cushion is reached must not count as potted -- guards against the
    /// regression where `test_shot` trusted "some pocket circle was
    /// crossed at all" instead of actually comparing that crossing's `t`
    /// against `cushion_t`. Uses a synthetic pocket placed just beyond
    /// the real long-rail cushion (rather than today's real, now-
    /// enlarged pockets) so this checks the ordering logic itself, not
    /// whatever the current tuned radii happen to allow.
    #[test]
    fn hits_the_cushion_before_a_pocket_circle_reached_only_after_it() {
        let fake_pocket = Pocket { position: Vector3::new(1.5, 0.0, 0.0), radius: 0.3 };
        let object_ball_pos = Vector3::new(0.5, BALL_RADIUS, 0.0);
        let dir = (1.0, 0.0); // straight toward the long-rail cushion, and beyond it, the fake pocket
        let contact = Vector3::new(object_ball_pos.x - dir.0 * BALL_RADIUS * 2.0, BALL_RADIUS, object_ball_pos.z);
        let cue_ball_pos = Vector3::new(contact.x - 0.3, BALL_RADIUS, contact.z);

        let result = test_shot(dir, cue_ball_pos, object_ball_pos, &[fake_pocket]);
        assert_eq!(result.pocketed, None, "should hit the real cushion before ever reaching the pocket circle beyond it");
    }
}
