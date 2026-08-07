use raylib::prelude::*;

pub const PAN_SPEED: f32 = 1.2; // meters per second
pub const KEY_ROTATE_SPEED_DEG: f32 = 90.0; // degrees per second, for Q/E
pub const ZOOM_BUTTON_SPEED: f32 = 1.0; // meters per second, for the on-screen +/- buttons
pub const PINCH_ZOOM_SENSITIVITY: f32 = 0.004; // per pixel of change in two-finger spread
pub const CAMERA_ELEVATION_DEG: f32 = 15.0; // above the cue ball, as seen when aiming
pub const CAMERA_BACK_DISTANCE: f32 = 0.7; // behind the cue ball, away from the object ball

// Camera presets (1/2/3): quick jumps to fixed vantage points, distinct
// from free orbiting. 1 and 2 stay in aiming mode and only change stance
// relative to the cue ball, preserving whatever direction is currently
// aimed (see `apply_aim_stance`); 3 switches to view mode and looks down
// the potting line itself, so it deliberately ignores the current aim.
pub const CAMERA_CLOSE_BACK_DISTANCE: f32 = 0.42; // preset 1: right above/close behind the cue ball
pub const CAMERA_CLOSE_ELEVATION_DEG: f32 = 12.0;
pub const CAMERA_STANCE_BACK_DISTANCE: f32 = 1.1; // preset 2: standing back, sizing up the shot
pub const CAMERA_STANCE_ELEVATION_DEG: f32 = 30.0;
pub const CAMERA_STANCE_LATERAL_OFFSET: f32 = 0.28; // shifted left of the aim line
// Preset 3 stands well back from the *object* ball rather than the cue
// ball -- reusing preset 2's distance still read as too close, likely
// because the object ball sits close to cushions/pockets far more often
// than the cue ball does, so this needs more clearance to actually read
// as "standing back", not just matching preset 2's number. It reuses
// preset 2's *height* though (see `pot_line_camera`), not its elevation
// angle -- height = distance * tan(angle), so pairing a bigger distance
// with the same angle would scale height up too and give a "giant"
// vantage instead of the same person standing further away.
pub const CAMERA_POT_LINE_BACK_DISTANCE: f32 = 1.7;

// Rotation sensitivity is proximity-based: players reach for "put the
// cursor near something important" when they want fine, precise control,
// so sensitivity ramps from a slow minimum right on top of the nearest
// reference point's on-screen position up to full speed by
// ROTATE_PRECISION_RADIUS_PX pixels away, and stays at full speed beyond
// that. Proximity is vertical-only (screen Y), not full 2D screen
// distance -- horizontal cursor movement is what actually drives yaw (the
// primary aim adjustment), so keying off full radial distance would mean
// the act of adjusting aim itself carries the cursor out of the precision
// zone; vertical position isn't tied to that motion the same way.
pub const ROTATE_SENSITIVITY: f32 = 0.005; // radians per pixel, at full speed
pub const ROTATE_PRECISION_RADIUS_PX: f32 = 160.0;
pub const ROTATE_MIN_SENSITIVITY_SCALE: f32 = 0.15; // fraction of full speed right on the ball

/// Horizontal (table-plane) unit vector along the camera's own look
/// direction (target − position) — i.e. the direction the cue ball travels
/// on a straight shot from the current viewing angle. This tracks the
/// camera's *orientation*, not its position relative to the ball, so
/// panning the camera (which shifts position and target together) doesn't
/// swing the aim — only actually rotating the view does. `None` when the
/// camera looks straight up/down (undefined horizontal bearing).
pub fn shot_direction_xz(camera: Camera3D) -> Option<(f32, f32)> {
    let dx = camera.target.x - camera.position.x;
    let dz = camera.target.z - camera.position.z;
    let len = (dx * dx + dz * dz).sqrt();
    if len < 1e-4 {
        return None;
    }
    Some((dx / len, dz / len))
}

/// Starting view: camera sits behind the cue ball, away from the object
/// ball, raised to ~15° above it — the vantage a player sights down the cue
/// from when addressing a shot.
pub fn aiming_camera(cue_ball_pos: Vector3, object_ball_pos: Vector3) -> Camera3D {
    let dx = cue_ball_pos.x - object_ball_pos.x;
    let dz = cue_ball_pos.z - object_ball_pos.z;
    let len = (dx * dx + dz * dz).sqrt().max(1e-4);
    let (bx, bz) = (dx / len, dz / len); // horizontal dir: object ball -> cue ball, i.e. "behind"

    let height =
        cue_ball_pos.y + CAMERA_BACK_DISTANCE * CAMERA_ELEVATION_DEG.to_radians().tan();
    let position = Vector3::new(
        cue_ball_pos.x + bx * CAMERA_BACK_DISTANCE,
        height,
        cue_ball_pos.z + bz * CAMERA_BACK_DISTANCE,
    );

    Camera3D::perspective(position, cue_ball_pos, Vector3::new(0.0, 1.0, 0.0), 45.0)
}

/// Repositions `camera` into a fixed "sighting stance" relative to the cue
/// ball, without touching where it's currently aimed: the camera's existing
/// horizontal bearing (`shot_direction_xz`) is kept and only the distance
/// behind the ball, elevation, and sideways offset change. The lateral
/// offset shifts *both* position and target together (the same trick
/// panning uses) rather than sliding position sideways while target stays
/// pinned to the ball -- pinning the target would rotate the
/// position-to-target vector itself, which is exactly the aim direction,
/// so standing "to the side" would silently re-aim the shot. Falls back to
/// looking toward the object ball when the camera has no defined bearing
/// yet (looking straight up/down). Used by camera presets 1 and 2.
pub fn apply_aim_stance(
    camera: &mut Camera3D,
    cue_ball_pos: Vector3,
    object_ball_pos: Vector3,
    back_distance: f32,
    elevation_deg: f32,
    lateral_offset: f32,
) {
    let (fx, fz) = shot_direction_xz(*camera).unwrap_or_else(|| {
        let dx = object_ball_pos.x - cue_ball_pos.x;
        let dz = object_ball_pos.z - cue_ball_pos.z;
        let len = (dx * dx + dz * dz).sqrt().max(1e-4);
        (dx / len, dz / len)
    });
    let (lx, lz) = (fz, -fx); // left of the forward direction, in the table plane

    let pivot = Vector3::new(
        cue_ball_pos.x + lx * lateral_offset,
        cue_ball_pos.y,
        cue_ball_pos.z + lz * lateral_offset,
    );
    let height = pivot.y + back_distance * elevation_deg.to_radians().tan();
    camera.position = Vector3::new(pivot.x - fx * back_distance, height, pivot.z - fz * back_distance);
    camera.target = pivot;
}

/// Camera preset 3: stands at the object ball, on the line away from the
/// target pocket, and looks straight down that line toward the pocket --
/// the vantage a player uses to check a potting angle by eye. Unlike
/// `apply_aim_stance` this ignores the current aim entirely, since the
/// point is to sight a specific, fixed line. Stands at the same eye
/// height as preset 2's standing stance -- only the back distance is
/// bigger, so this reads as the same person standing further away, not
/// someone taller looking down at a steeper angle.
pub fn pot_line_camera(object_ball_pos: Vector3, pocket_pos: Vector3) -> Camera3D {
    let dx = object_ball_pos.x - pocket_pos.x;
    let dz = object_ball_pos.z - pocket_pos.z;
    let len = (dx * dx + dz * dz).sqrt().max(1e-4);
    let (bx, bz) = (dx / len, dz / len); // pocket -> object ball, i.e. "behind", away from the pocket

    let height = object_ball_pos.y
        + CAMERA_STANCE_BACK_DISTANCE * CAMERA_STANCE_ELEVATION_DEG.to_radians().tan();
    let position = Vector3::new(
        object_ball_pos.x + bx * CAMERA_POT_LINE_BACK_DISTANCE,
        height,
        object_ball_pos.z + bz * CAMERA_POT_LINE_BACK_DISTANCE,
    );
    Camera3D::perspective(position, pocket_pos, Vector3::new(0.0, 1.0, 0.0), 45.0)
}

/// Where a screen-space ray through `screen_pos` meets the table plane
/// (y = 0), if it does so in front of the camera. `None` when it points
/// away from the table entirely (e.g. up toward the sky).
pub fn screen_ray_table_point(rl: &RaylibHandle, camera: Camera3D, screen_pos: Vector2) -> Option<Vector3> {
    let ray = rl.get_screen_to_world_ray(screen_pos, camera);
    if ray.direction.y.abs() <= 1e-4 {
        return None;
    }
    let t = -ray.position.y / ray.direction.y;
    (t > 0.0).then(|| ray.position + ray.direction.scale(t))
}

/// Same as `screen_ray_table_point`, but through the mouse cursor.
pub fn cursor_table_point(rl: &RaylibHandle, camera: Camera3D) -> Option<Vector3> {
    screen_ray_table_point(rl, camera, rl.get_mouse_position())
}

/// Zooms the camera toward `hit` by `factor` (a fraction of the remaining
/// distance, same convention as a lerp) while keeping the camera-to-target
/// distance clamped to a sane range -- shared by wheel-zoom and pinch-zoom,
/// which both zoom toward a specific world point rather than along the
/// current view axis (see the on-screen zoom buttons for that variant).
pub fn zoom_toward(camera: &mut Camera3D, hit: Vector3, factor: f32) {
    camera.position = camera.position.lerp(hit, factor);
    camera.target = camera.target.lerp(hit, factor);

    let dist = camera.position.distance(camera.target).clamp(0.15, 6.0);
    let dir = (camera.position - camera.target).normalize();
    camera.position = camera.target + dir.scale(dist);
}

/// Orbit-drag sensitivity, scaled by how close the cursor is (on screen) to
/// the object ball: right on top of it gives `ROTATE_MIN_SENSITIVITY_SCALE`
/// of full speed for precise aiming, ramping linearly up to full speed by
/// `ROTATE_PRECISION_RADIUS_PX` pixels away and staying there beyond that.
pub fn rotate_sensitivity(rl: &RaylibHandle, camera: Camera3D, reference_points: &[Vector3]) -> f32 {
    let mouse_y = rl.get_mouse_position().y;
    let screen_dist = reference_points
        .iter()
        .map(|&p| (mouse_y - rl.get_world_to_screen(p, camera).y).abs())
        .fold(f32::INFINITY, f32::min);
    let t = (screen_dist / ROTATE_PRECISION_RADIUS_PX).clamp(0.0, 1.0);
    let scale = ROTATE_MIN_SENSITIVITY_SCALE + (1.0 - ROTATE_MIN_SENSITIVITY_SCALE) * t;
    ROTATE_SENSITIVITY * scale
}
