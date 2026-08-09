// Generated from the model-measured half-tables (see git history for the
// original scripts/extract_cushion_segments.py extraction) by reflecting
// each about its own rail's center and merging -- both rails are symmetric
// about their own center, so the reflected half is exactly the other real
// corner/end, not a guess. This gives the full signed span of each rail
// directly, so a straight run bounded by real data on only one side (e.g.
// the short rail's corner-to-corner run, which has no middle pocket to
// anchor a point near its own center) still gets a real point at the far
// end once its mirror twin is merged in -- the two nearest points either
// side of 0 bracket the flat run correctly instead of relying on
// boundary_lookup's clamp to paper over the missing half.
//
// safe_half_width()/safe_half_length() take a *signed* z/x now (not
// abs_z/abs_x) -- see their doc comments in table.rs.

// Long rails (left/right): [z, abs_x].
pub const CUSHION_BOUNDARY: &[[f32; 2]] = &[
    [-1.784, 0.8892],
    [-1.712, 0.8881],
    [-1.704, 0.8881],
    [-1.703, 0.8888],
    [-1.694, 0.8881],
    [-1.693, 0.8892],
    [-1.689, 0.8631],
    [-1.682, 0.8568],
    [-1.675, 0.8508],
    [-1.667, 0.8454],
    [-1.659, 0.8406],
    [-1.65, 0.8366],
    [-1.649, 0.8368],
    [-1.64, 0.8336],
    [-1.639, 0.8338],
    [-1.629, 0.8317],
    [-1.617, 0.831],
    [-0.132, 0.831],
    [-0.118, 0.8322],
    [-0.117, 0.832],
    [-0.105, 0.835],
    [-0.104, 0.8348],
    [-0.093, 0.8391],
    [-0.083, 0.8445],
    [-0.075, 0.8507],
    [-0.069, 0.8576],
    [-0.068, 0.8574],
    [-0.063, 0.8642],
    [-0.059, 0.8709],
    [-0.056, 0.877],
    [-0.054, 0.8823],
    [-0.053, 0.8867],
    [-0.052, 0.8865],
    [0.052, 0.8865],
    [0.053, 0.8867],
    [0.054, 0.8823],
    [0.056, 0.877],
    [0.059, 0.8709],
    [0.063, 0.8642],
    [0.068, 0.8574],
    [0.069, 0.8576],
    [0.075, 0.8507],
    [0.083, 0.8445],
    [0.093, 0.8391],
    [0.104, 0.8348],
    [0.105, 0.835],
    [0.117, 0.832],
    [0.118, 0.8322],
    [0.132, 0.831],
    [1.617, 0.831],
    [1.629, 0.8317],
    [1.639, 0.8338],
    [1.64, 0.8336],
    [1.649, 0.8368],
    [1.65, 0.8366],
    [1.659, 0.8406],
    [1.667, 0.8454],
    [1.675, 0.8508],
    [1.682, 0.8568],
    [1.689, 0.8631],
    [1.693, 0.8892],
    [1.694, 0.8881],
    [1.703, 0.8888],
    [1.704, 0.8881],
    [1.712, 0.8881],
    [1.784, 0.8892],
];

// Short rails (baulk/top): [x, abs_z].
pub const SHORT_RAIL_BOUNDARY: &[[f32; 2]] = &[
    [-0.889, 1.784],
    [-0.799, 1.784],
    [-0.794, 1.758],
    [-0.787, 1.752],
    [-0.78, 1.746],
    [-0.772, 1.741],
    [-0.763, 1.736],
    [-0.754, 1.732],
    [-0.744, 1.729],
    [-0.733, 1.727],
    [-0.722, 1.726],
    [0.722, 1.726],
    [0.733, 1.727],
    [0.744, 1.729],
    [0.754, 1.732],
    [0.763, 1.736],
    [0.772, 1.741],
    [0.78, 1.746],
    [0.787, 1.752],
    [0.794, 1.758],
    [0.799, 1.784],
    [0.889, 1.784],
];
