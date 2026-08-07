use raylib::prelude::*;

use crate::table::BALL_RADIUS;

pub const CUE_LENGTH: f32 = 1.45;
pub const CUE_TIP_RADIUS: f32 = 0.006;
pub const CUE_BUTT_RADIUS: f32 = 0.014;
pub const CUE_TIP_GAP: f32 = 0.004;
pub const CUE_ELEVATION_DEG: f32 = 8.0;

// cue.glb: a single continuous mesh (1.486m, matching a real cue), rebaked
// by scripts/extract_props.py's flatten_root into a local frame with no
// leftover display-rack rotation -- long axis along local +X. Confirmed
// by sampling actual vertex radii along its length (not just the bounding
// box) that it tapers from ~13mm at -X down to ~4mm at +X, i.e. +X is the
// tip end, -X is the butt. Measured directly from the flattened file.
pub const CUE_MODEL_PATH: &str = "assets/cue.glb";
pub const CUE_MODEL_TIP_X: f32 = 0.71477;
pub const CUE_MODEL_BUTT_X: f32 = -0.77121;

pub const CUE_COLOR: Color = Color::new(160, 110, 60, 255); // wood

/// Draws the cue resting behind the cue ball, parallel to the camera's
/// horizontal look direction — unlike the camera it doesn't pitch up/down
/// with the view, staying near-horizontal, raised by a fixed small angle
/// from tip to butt, like a real stance.
pub fn draw_cue(d: &mut impl RaylibDraw3D, shot_dir: (f32, f32), cue_ball_pos: Vector3) {
    let (sx, sz) = shot_dir;
    let elevation = CUE_ELEVATION_DEG.to_radians();
    let horiz = elevation.cos();
    // Cue points from the ball back toward the camera (butt side), i.e.
    // opposite the shot direction.
    let dir = Vector3::new(-sx * horiz, elevation.sin(), -sz * horiz);
    let tip = cue_ball_pos + dir.scale(BALL_RADIUS + CUE_TIP_GAP);
    let butt = tip + dir.scale(CUE_LENGTH);
    d.draw_cylinder_ex(tip, butt, CUE_TIP_RADIUS, CUE_BUTT_RADIUS, 12, CUE_COLOR);
}

/// Same aim as `draw_cue`, but draws the real cue model instead of a
/// plain cylinder. The model's own local +X axis is its tip-ward long
/// axis (see CUE_MODEL_* comments), centered on the axis with no baked
/// rotation left in it -- so orienting it is just "rotate local +X onto
/// the world direction the tip should point", i.e. onto `-dir` (`dir`
/// itself points tip -> butt, same convention as `draw_cue`).
pub fn draw_cue_model(d: &mut impl RaylibDraw3D, model: &Model, shot_dir: (f32, f32), cue_ball_pos: Vector3, tint: Color) {
    let (sx, sz) = shot_dir;
    let elevation = CUE_ELEVATION_DEG.to_radians();
    let horiz = elevation.cos();
    let dir = Vector3::new(-sx * horiz, elevation.sin(), -sz * horiz);
    let tip = cue_ball_pos + dir.scale(BALL_RADIUS + CUE_TIP_GAP);

    // Stretch only along the cue's own length (local X); keep its natural
    // radius, so scaling up to CUE_LENGTH doesn't also fatten it.
    let scale_x = CUE_LENGTH / (CUE_MODEL_TIP_X - CUE_MODEL_BUTT_X);
    let scale = Vector3::new(scale_x, 1.0, 1.0);

    // Rotate local +X (tip-ward) onto -dir (dir is tip -> butt).
    let target = Vector3::new(-dir.x, -dir.y, -dir.z);
    let axis = Vector3::new(0.0, -target.z, target.y);
    let axis = if axis.length() < 1e-5 {
        Vector3::new(0.0, 1.0, 0.0)
    } else {
        axis.normalize()
    };
    let angle_deg = target.x.clamp(-1.0, 1.0).acos().to_degrees();

    // The model's own local origin isn't the tip (tip sits at local X =
    // CUE_MODEL_TIP_X), so `position` -- which places the local origin,
    // not the tip -- has to be offset back from the desired tip point by
    // however far the (scaled, rotated) tip sits from that origin.
    let position = tip + dir.scale(CUE_MODEL_TIP_X * scale_x);

    d.draw_model_ex(model, position, axis, angle_deg, scale, tint);
}
