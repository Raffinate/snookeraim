use raylib::prelude::*;

// Desktop targets compile against desktop OpenGL 3.3 core (GLSL 330); the
// Emscripten/web target negotiates a WebGL2 context (GLSL ES 300, requested
// via the opengl_es_30 feature — see Cargo.toml). The two dialects are close
// enough (both use in/out and an explicit output var) that only the version
// line and a precision qualifier differ; desktop GLSL doesn't need the
// qualifier but WebGL2's compiler requires it in fragment shaders.

#[cfg(not(target_os = "emscripten"))]
const BALL_VS: &str = r#"
#version 330

in vec3 vertexPosition;
in vec2 vertexTexCoord;
in vec3 vertexNormal;
in vec4 vertexColor;

uniform mat4 mvp;
uniform mat4 matModel;
uniform mat4 matNormal;

out vec3 fragPosition;
out vec2 fragTexCoord;
out vec4 fragColor;
out vec3 fragNormal;

void main()
{
    fragPosition = vec3(matModel * vec4(vertexPosition, 1.0));
    fragTexCoord = vertexTexCoord;
    fragColor = vertexColor;
    fragNormal = normalize(vec3(matNormal * vec4(vertexNormal, 1.0)));
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
"#;

#[cfg(target_os = "emscripten")]
const BALL_VS: &str = r#"#version 300 es
precision mediump float;

in vec3 vertexPosition;
in vec2 vertexTexCoord;
in vec3 vertexNormal;
in vec4 vertexColor;

uniform mat4 mvp;
uniform mat4 matModel;
uniform mat4 matNormal;

out vec3 fragPosition;
out vec2 fragTexCoord;
out vec4 fragColor;
out vec3 fragNormal;

void main()
{
    fragPosition = vec3(matModel * vec4(vertexPosition, 1.0));
    fragTexCoord = vertexTexCoord;
    fragColor = vertexColor;
    fragNormal = normalize(vec3(matNormal * vec4(vertexNormal, 1.0)));
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
"#;

#[cfg(not(target_os = "emscripten"))]
const BALL_FS: &str = r#"
#version 330

in vec3 fragPosition;
in vec2 fragTexCoord;
in vec4 fragColor;
in vec3 fragNormal;

uniform sampler2D texture0;
uniform vec4 colDiffuse;
uniform vec4 ambient;
uniform vec3 viewPos;

#define NUM_LIGHTS 3
uniform vec3 lightPos[NUM_LIGHTS];
uniform vec4 lightColor[NUM_LIGHTS];

out vec4 finalColor;

void main()
{
    vec4 texelColor = texture(texture0, fragTexCoord);
    vec3 normal = normalize(fragNormal);
    vec3 viewD = normalize(viewPos - fragPosition);

    vec3 lightDot = vec3(0.0);
    vec3 specular = vec3(0.0);

    for (int i = 0; i < NUM_LIGHTS; i++)
    {
        vec3 lightDir = normalize(lightPos[i] - fragPosition);
        float NdotL = max(dot(normal, lightDir), 0.0);
        lightDot += lightColor[i].rgb * NdotL;

        float specCo = 0.0;
        if (NdotL > 0.0) specCo = pow(max(0.0, dot(viewD, reflect(-lightDir, normal))), 24.0);
        specular += specCo * lightColor[i].rgb;
    }

    vec4 tint = colDiffuse * fragColor;

    finalColor = texelColor * ((tint + vec4(specular, 1.0)) * vec4(lightDot, 1.0));
    finalColor += texelColor * (ambient / 10.0) * tint;
    finalColor = pow(finalColor, vec4(1.0 / 2.2));
}
"#;

#[cfg(target_os = "emscripten")]
const BALL_FS: &str = r#"#version 300 es
precision mediump float;

in vec3 fragPosition;
in vec2 fragTexCoord;
in vec4 fragColor;
in vec3 fragNormal;

uniform sampler2D texture0;
uniform vec4 colDiffuse;
uniform vec4 ambient;
uniform vec3 viewPos;

#define NUM_LIGHTS 3
uniform vec3 lightPos[NUM_LIGHTS];
uniform vec4 lightColor[NUM_LIGHTS];

out vec4 finalColor;

void main()
{
    vec4 texelColor = texture(texture0, fragTexCoord);
    vec3 normal = normalize(fragNormal);
    vec3 viewD = normalize(viewPos - fragPosition);

    vec3 lightDot = vec3(0.0);
    vec3 specular = vec3(0.0);

    for (int i = 0; i < NUM_LIGHTS; i++)
    {
        vec3 lightDir = normalize(lightPos[i] - fragPosition);
        float NdotL = max(dot(normal, lightDir), 0.0);
        lightDot += lightColor[i].rgb * NdotL;

        float specCo = 0.0;
        if (NdotL > 0.0) specCo = pow(max(0.0, dot(viewD, reflect(-lightDir, normal))), 24.0);
        specular += specCo * lightColor[i].rgb;
    }

    vec4 tint = colDiffuse * fragColor;

    finalColor = texelColor * ((tint + vec4(specular, 1.0)) * vec4(lightDot, 1.0));
    finalColor += texelColor * (ambient / 10.0) * tint;
    finalColor = pow(finalColor, vec4(1.0 / 2.2));
}
"#;

// Ghost-ball shader: same Blinn-Phong shading as the real balls (so ghosts
// get a highlight and a shaded side — visual volume, not a flat disc), but
// with alpha taken directly from colDiffuse.a instead of the ball
// shader's formula, which pushes alpha above 1 via its specular/ambient
// terms and so can't be used for anything translucent.

#[cfg(not(target_os = "emscripten"))]
const GHOST_FS: &str = r#"
#version 330

in vec3 fragPosition;
in vec3 fragNormal;

uniform vec4 colDiffuse;
uniform vec4 ambient;
uniform vec3 viewPos;

#define NUM_LIGHTS 3
uniform vec3 lightPos[NUM_LIGHTS];
uniform vec4 lightColor[NUM_LIGHTS];

out vec4 finalColor;

void main()
{
    vec3 normal = normalize(fragNormal);
    vec3 viewD = normalize(viewPos - fragPosition);

    vec3 lightDot = vec3(0.0);
    vec3 specular = vec3(0.0);

    for (int i = 0; i < NUM_LIGHTS; i++)
    {
        vec3 lightDir = normalize(lightPos[i] - fragPosition);
        float NdotL = max(dot(normal, lightDir), 0.0);
        lightDot += lightColor[i].rgb * NdotL;

        float specCo = 0.0;
        if (NdotL > 0.0) specCo = pow(max(0.0, dot(viewD, reflect(-lightDir, normal))), 24.0);
        specular += specCo * lightColor[i].rgb;
    }

    vec3 shaded = colDiffuse.rgb * (ambient.rgb + lightDot) + specular;
    finalColor = vec4(shaded, colDiffuse.a);
}
"#;

#[cfg(target_os = "emscripten")]
const GHOST_FS: &str = r#"#version 300 es
precision mediump float;

in vec3 fragPosition;
in vec3 fragNormal;

uniform vec4 colDiffuse;
uniform vec4 ambient;
uniform vec3 viewPos;

#define NUM_LIGHTS 3
uniform vec3 lightPos[NUM_LIGHTS];
uniform vec4 lightColor[NUM_LIGHTS];

out vec4 finalColor;

void main()
{
    vec3 normal = normalize(fragNormal);
    vec3 viewD = normalize(viewPos - fragPosition);

    vec3 lightDot = vec3(0.0);
    vec3 specular = vec3(0.0);

    for (int i = 0; i < NUM_LIGHTS; i++)
    {
        vec3 lightDir = normalize(lightPos[i] - fragPosition);
        float NdotL = max(dot(normal, lightDir), 0.0);
        lightDot += lightColor[i].rgb * NdotL;

        float specCo = 0.0;
        if (NdotL > 0.0) specCo = pow(max(0.0, dot(viewD, reflect(-lightDir, normal))), 24.0);
        specular += specCo * lightColor[i].rgb;
    }

    vec3 shaded = colDiffuse.rgb * (ambient.rgb + lightDot) + specular;
    finalColor = vec4(shaded, colDiffuse.a);
}
"#;

// Real-world snooker dimensions, in meters.
const TABLE_LENGTH: f32 = 3.569; // long axis (Z)
const TABLE_WIDTH: f32 = 1.778; // short axis (X)
const CUSHION_HEIGHT: f32 = 0.05;
const CUSHION_THICKNESS: f32 = 0.06;
const BALL_RADIUS: f32 = 0.02625;
const CORNER_POCKET_RADIUS: f32 = 0.045;
const MIDDLE_POCKET_RADIUS: f32 = 0.05;

const CUE_LENGTH: f32 = 1.45;
const CUE_TIP_RADIUS: f32 = 0.006;
const CUE_BUTT_RADIUS: f32 = 0.014;
const CUE_TIP_GAP: f32 = 0.004;
const CUE_ELEVATION_DEG: f32 = 8.0;
const PAN_SPEED: f32 = 1.2; // meters per second
const KEY_ROTATE_SPEED_DEG: f32 = 90.0; // degrees per second, for Q/E
const CAMERA_ELEVATION_DEG: f32 = 15.0; // above the cue ball, as seen when aiming
const CAMERA_BACK_DISTANCE: f32 = 0.7; // behind the cue ball, away from the object ball

const ROTATE_SENSITIVITY: f32 = 0.005; // radians per pixel, at ROTATE_REFERENCE_DISTANCE
const ROTATE_REFERENCE_DISTANCE: f32 = 1.5; // meters
const ROTATE_MIN_DIST: f32 = 0.3;
// Off-table virtual-cone parameters (see cursor_cone_distance):
// ROTATE_CONE_HEIGHT is tuned so typical on-table-adjacent angles land near
// ROTATE_REFERENCE_DISTANCE; ROTATE_VIRTUAL_MAX_DIST is deliberately large
// so shallow angles ("aiming at nothing") end up much slower, not just a
// bit slower. Also doubles as the clamp ceiling for the on-table branch.
const ROTATE_CONE_HEIGHT: f32 = 0.6;
const ROTATE_VIRTUAL_MAX_DIST: f32 = 20.0;

// A random layout is only "realistic" if the balls have some breathing room
// and at least one pocket offers a pot that isn't a near-impossible sliver
// of a cut.
const MIN_BALL_SEPARATION: f32 = 0.18;
const MAX_REALISTIC_CUT_DEG: f32 = 65.0;
const MAX_PLACEMENT_ATTEMPTS: u32 = 300;

const GATE_WIDTH_FACTOR: f32 = 1.4; // gate width, in ball diameters
const GATE_POST_RADIUS: f32 = 0.008;
const GATE_POST_HEIGHT: f32 = 0.09;
const PATH_HEIGHT: f32 = 0.0015; // path stripes sit just above the cloth

const CLOTH_COLOR: Color = Color::new(20, 110, 60, 255);
const CUSHION_COLOR: Color = Color::new(15, 80, 45, 255);
const POCKET_COLOR: Color = Color::BLACK;
const CUE_BALL_COLOR: Color = Color::WHITE;
const OBJECT_BALL_COLOR: Color = Color::new(200, 30, 30, 255); // red ball
const CUE_COLOR: Color = Color::new(160, 110, 60, 255); // wood

// Overhead LED light bank: a row of wide rectangular panels, like the
// segmented shade units over a real snooker table, rather than one point.
const LIGHT_PANEL_COUNT: usize = 3;
const LIGHT_HEIGHT: f32 = 1.0;
const LIGHT_PANEL_WIDTH: f32 = TABLE_WIDTH * 0.65;
const LIGHT_PANEL_THICKNESS: f32 = 0.04;
const LIGHT_PANEL_GAP: f32 = 0.08;
const LIGHT_PANEL_COLOR: Color = Color::new(255, 250, 235, 255);
const LIGHT_COLOR_INTENSITY: f32 = 0.42; // per light, so 3 lights don't overblow brightness
const GHOST_BALL_COLOR: Color = Color::new(255, 255, 255, 90);
const AIM_LINE_COLOR: Color = Color::new(255, 220, 40, 230);
const GHOST_RED_BALL_COLOR: Color = Color::new(230, 60, 60, 110);
const GATE_NEUTRAL_COLOR: Color = Color::new(230, 230, 230, 255);
const GATE_SUCCESS_COLOR: Color = Color::new(50, 220, 60, 255);
const GATE_MISS_COLOR: Color = Color::new(220, 50, 50, 255);
const PATH_WHITE_COLOR: Color = Color::new(255, 255, 255, 110);
const PATH_RED_COLOR: Color = Color::new(230, 60, 60, 110);

struct Pocket {
    position: Vector3,
    radius: f32,
}

fn pockets() -> Vec<Pocket> {
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
fn random_ball_position(pockets: &[Pocket], taken: &[Vector3]) -> Vector3 {
    let margin = BALL_RADIUS * 2.0;
    let hw = TABLE_WIDTH / 2.0 - margin;
    let hl = TABLE_LENGTH / 2.0 - margin;

    loop {
        let x = rand::random_range(-hw..hw);
        let z = rand::random_range(-hl..hl);
        let candidate = Vector3::new(x, BALL_RADIUS, z);

        let clear_of_pockets = pockets
            .iter()
            .all(|p| candidate.distance(p.position) > p.radius + BALL_RADIUS * 2.0);
        let clear_of_balls = taken.iter().all(|b| candidate.distance(*b) > MIN_BALL_SEPARATION);

        if clear_of_pockets && clear_of_balls {
            return candidate;
        }
    }
}

/// Centers of the overhead light panel segments, evenly spanning the
/// table's length. Doubles as both the visual panel positions and the
/// point-light positions used for shading.
fn light_panel_centers() -> [Vector3; LIGHT_PANEL_COUNT] {
    let segment_len = TABLE_LENGTH / LIGHT_PANEL_COUNT as f32;
    let mut centers = [Vector3::zero(); LIGHT_PANEL_COUNT];
    for (i, center) in centers.iter_mut().enumerate() {
        let z = -TABLE_LENGTH / 2.0 + segment_len * (i as f32 + 0.5);
        *center = Vector3::new(0.0, LIGHT_HEIGHT, z);
    }
    centers
}

fn draw_light_fixture(d: &mut impl RaylibDraw3D, centers: &[Vector3]) {
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

fn draw_table(d: &mut impl RaylibDraw3D, pockets: &[Pocket]) {
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

/// `draw_mesh` consumes its material by value; this hands it a throwaway
/// non-owning copy so the real material survives to the next frame.
fn weak_copy(material: &WeakMaterial) -> WeakMaterial {
    unsafe { WeakMaterial::from_raw(*material.as_ref()) }
}

/// Horizontal (table-plane) unit vector along the camera's own look
/// direction (target − position) — i.e. the direction the cue ball travels
/// on a straight shot from the current viewing angle. This tracks the
/// camera's *orientation*, not its position relative to the ball, so
/// panning the camera (which shifts position and target together) doesn't
/// swing the aim — only actually rotating the view does. `None` when the
/// camera looks straight up/down (undefined horizontal bearing).
fn shot_direction_xz(camera: Camera3D) -> Option<(f32, f32)> {
    let dx = camera.target.x - camera.position.x;
    let dz = camera.target.z - camera.position.z;
    let len = (dx * dx + dz * dz).sqrt();
    if len < 1e-4 {
        return None;
    }
    Some((dx / len, dz / len))
}

/// Draws the cue resting behind the cue ball, parallel to the camera's
/// horizontal look direction — unlike the camera it doesn't pitch up/down
/// with the view, staying near-horizontal, raised by a fixed small angle
/// from tip to butt, like a real stance.
fn draw_cue(d: &mut impl RaylibDraw3D, shot_dir: (f32, f32), cue_ball_pos: Vector3) {
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

/// Distance along a ray from `(x, z)` in direction `(dx, dz)` (need not be
/// unit length, but must be a real direction) to the first cushion — i.e.
/// where a ball's center would leave the playing rectangle inset by one
/// ball radius.
fn cushion_t(x: f32, z: f32, dx: f32, dz: f32) -> f32 {
    let x_max = TABLE_WIDTH / 2.0 - BALL_RADIUS;
    let z_max = TABLE_LENGTH / 2.0 - BALL_RADIUS;
    let t_x = if dx > 1e-6 {
        (x_max - x) / dx
    } else if dx < -1e-6 {
        (-x_max - x) / dx
    } else {
        f32::INFINITY
    };
    let t_z = if dz > 1e-6 {
        (z_max - z) / dz
    } else if dz < -1e-6 {
        (-z_max - z) / dz
    } else {
        f32::INFINITY
    };
    t_x.min(t_z)
}

fn cross2(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.1 - a.1 * b.0
}

/// `t` along ray `origin + dir*t` where it crosses segment `a`-`b`, if the
/// crossing is ahead of the ray and within the segment's bounds.
fn ray_segment_t(origin: (f32, f32), dir: (f32, f32), a: (f32, f32), b: (f32, f32)) -> Option<f32> {
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
struct CueRaycast {
    ghost_pos: Vector3,
    hit_object_ball: bool,
}

fn cue_raycast(shot_dir: (f32, f32), cue_ball_pos: Vector3, object_ball_pos: Vector3) -> CueRaycast {
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
fn best_pocket(
    pockets: &[Pocket],
    cue_ball_pos: Vector3,
    object_ball_pos: Vector3,
) -> (usize, (f32, f32), f32) {
    let x_max = TABLE_WIDTH / 2.0 - BALL_RADIUS;
    let z_max = TABLE_LENGTH / 2.0 - BALL_RADIUS;

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
        if ghost_x.abs() > x_max || ghost_z.abs() > z_max {
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
fn random_shot_setup(pockets: &[Pocket]) -> (Vector3, Vector3) {
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

enum GateState {
    Success,
    Miss,
}

struct ShotTest {
    white_end: Vector3,
    red_path: Option<(Vector3, Vector3)>,
    gate_state: GateState,
}

/// Simulates a dead-straight shot from the current cue direction: traces
/// the cue ball to its first contact (object ball or cushion), then — if it
/// hit the object ball — traces the object ball's resulting path (straight
/// through its center, no spin) to its own first event: passing through the
/// target gate (potted) or hitting a cushion (missed).
fn test_shot(
    shot_dir: (f32, f32),
    cue_ball_pos: Vector3,
    object_ball_pos: Vector3,
    pocket_pos: Vector3,
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

    let half = BALL_RADIUS * GATE_WIDTH_FACTOR;
    let (px, pz) = (-gate_dir.1 * half, gate_dir.0 * half);
    let gate_a = (pocket_pos.x + px, pocket_pos.z + pz);
    let gate_b = (pocket_pos.x - px, pocket_pos.z - pz);
    let t_gate = ray_segment_t(
        (object_ball_pos.x, object_ball_pos.z),
        (rdx, rdz),
        gate_a,
        gate_b,
    );

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

/// Draws the potting "gate" at a pocket: two posts spanning a bit wider
/// than a ball's diameter, perpendicular to the ball's intended path into
/// the pocket.
fn draw_gate(d: &mut impl RaylibDraw3D, pocket_pos: Vector3, gate_dir: (f32, f32), color: Color) {
    let half = BALL_RADIUS * GATE_WIDTH_FACTOR;
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
fn draw_path_stripe(d: &mut impl RaylibDraw3D, start: Vector3, end: Vector3, color: Color) {
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
fn draw_object_ball_aim_line(d: &mut impl RaylibDraw3D, ghost_pos: Vector3, object_ball_pos: Vector3) {
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

/// Starting view: camera sits behind the cue ball, away from the object
/// ball, raised to ~15° above it — the vantage a player sights down the cue
/// from when addressing a shot.
fn aiming_camera(cue_ball_pos: Vector3, object_ball_pos: Vector3) -> Camera3D {
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

/// Where the mouse cursor's ray meets the table plane (y = 0), if it does
/// so in front of the camera. `None` when the cursor points away from the
/// table entirely (e.g. up toward the sky).
fn cursor_table_point(rl: &RaylibHandle, camera: Camera3D) -> Option<Vector3> {
    let ray = rl.get_screen_to_world_ray(rl.get_mouse_position(), camera);
    if ray.direction.y.abs() <= 1e-4 {
        return None;
    }
    let t = -ray.position.y / ray.direction.y;
    (t > 0.0).then(|| ray.position + ray.direction.scale(t))
}

/// Same as `cursor_table_point`, but only counts as a hit within the
/// table's actual rendered extent (cloth + cushions). The y = 0 plane
/// itself is infinite, so without this a cursor pointing at the empty
/// background just past the table's edge would still math out to a nearby
/// point and be treated as "aiming at something."
fn cursor_on_table_point(rl: &RaylibHandle, camera: Camera3D) -> Option<Vector3> {
    let hit = cursor_table_point(rl, camera)?;
    let hw = TABLE_WIDTH / 2.0 + CUSHION_THICKNESS;
    let hl = TABLE_LENGTH / 2.0 + CUSHION_THICKNESS;
    (hit.x.abs() <= hw && hit.z.abs() <= hl).then_some(hit)
}

/// Virtual-cone distance for rotation sensitivity when the cursor is *not*
/// on the table: depends only on how steeply the cursor ray points
/// downward, not on what it actually hits. Shallow angles (pointing toward
/// the horizon or background — "aiming at nothing") give a large distance
/// and slow rotation; steeper angles give a smaller one.
fn cursor_cone_distance(rl: &RaylibHandle, camera: Camera3D) -> f32 {
    let ray = rl.get_screen_to_world_ray(rl.get_mouse_position(), camera);
    let downness = (-ray.direction.y).max(0.01);
    (ROTATE_CONE_HEIGHT / downness).clamp(ROTATE_MIN_DIST, ROTATE_VIRTUAL_MAX_DIST)
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 800)
        .title("Snooker Aim Trainer")
        .msaa_4x()
        .build();

    rl.set_target_fps(60);

    let pockets = pockets();
    let (mut cue_ball_pos, mut object_ball_pos) = random_shot_setup(&pockets);
    let mut camera = aiming_camera(cue_ball_pos, object_ball_pos);
    let (pocket_idx, gate_dir, _) = best_pocket(&pockets, cue_ball_pos, object_ball_pos);
    let mut target_pocket = (pocket_idx, gate_dir);
    let mut shot_test: Option<ShotTest> = None;
    let mut show_ghost_ball = true;
    let mut show_aim_line = false;
    let mut view_mode = false;
    let mut saved_camera: Option<Camera3D> = None;
    let mut last_view_camera: Option<Camera3D> = None;
    let mut frozen_aim_camera = camera;

    let light_panels = light_panel_centers();

    let ball_mesh = Mesh::gen_mesh_sphere(&thread, BALL_RADIUS, 24, 24);
    let mut ball_shader = rl.load_shader_from_memory(&thread, Some(BALL_VS), Some(BALL_FS));
    let ambient_loc = ball_shader.get_shader_location("ambient");
    let view_pos_loc = ball_shader.get_shader_location("viewPos");
    let light_pos_loc = ball_shader.get_shader_location("lightPos");
    let light_color_loc = ball_shader.get_shader_location("lightColor");

    ball_shader.set_shader_value(ambient_loc, Vector4::new(0.35, 0.35, 0.35, 1.0));
    ball_shader.set_shader_value_v(light_pos_loc, &light_panels);
    let light_colors = [Vector4::new(
        LIGHT_COLOR_INTENSITY,
        LIGHT_COLOR_INTENSITY,
        LIGHT_COLOR_INTENSITY,
        1.0,
    ); LIGHT_PANEL_COUNT];
    ball_shader.set_shader_value_v(light_color_loc, &light_colors);

    let mut ball_material = rl.load_material_default(&thread);
    ball_material.set_shader(&ball_shader);

    let mut ghost_shader = rl.load_shader_from_memory(&thread, Some(BALL_VS), Some(GHOST_FS));
    let ghost_ambient_loc = ghost_shader.get_shader_location("ambient");
    let ghost_view_pos_loc = ghost_shader.get_shader_location("viewPos");
    let ghost_light_pos_loc = ghost_shader.get_shader_location("lightPos");
    let ghost_light_color_loc = ghost_shader.get_shader_location("lightColor");
    ghost_shader.set_shader_value(ghost_ambient_loc, Vector4::new(0.35, 0.35, 0.35, 1.0));
    ghost_shader.set_shader_value_v(ghost_light_pos_loc, &light_panels);
    ghost_shader.set_shader_value_v(ghost_light_color_loc, &light_colors);

    let mut ghost_material = rl.load_material_default(&thread);
    ghost_material.set_shader(&ghost_shader);

    while !rl.window_should_close() {
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            if shot_test.is_some() {
                // A tested shot is on screen (paths/gate result) — first R
                // just clears that, rather than jumping straight to a new
                // layout.
                shot_test = None;
            } else {
                (cue_ball_pos, object_ball_pos) = random_shot_setup(&pockets);
                let (pocket_idx, gate_dir, _) =
                    best_pocket(&pockets, cue_ball_pos, object_ball_pos);
                target_pocket = (pocket_idx, gate_dir);
                camera = aiming_camera(cue_ball_pos, object_ball_pos);
                view_mode = false;
                saved_camera = None;
                last_view_camera = None;
            }
        }

        if rl.is_key_pressed(KeyboardKey::KEY_C) {
            // Re-center the orbit pivot on the cue ball without touching
            // zoom or viewing angle: shift both position and target by the
            // same offset, so their relative vector is unchanged.
            let offset = cue_ball_pos - camera.target;
            camera.target = cue_ball_pos;
            camera.position = camera.position + offset;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_V) {
            if view_mode {
                // Exiting: remember exactly where we were so re-entering
                // resumes here, then snap back to the pre-view-mode camera.
                last_view_camera = Some(camera);
                if let Some(saved) = saved_camera.take() {
                    camera = saved;
                }
            } else {
                // The frozen aim must come from the normal-mode camera (the
                // one the player actually aims with) — captured here,
                // before any swap to a resumed free-roam view below.
                // Otherwise, re-entering after having zoomed/orbited around
                // while inspecting would redefine the "frozen" aim from
                // that free-roam drift instead of the real aim.
                if last_view_camera.is_none() {
                    // First time entering this session: re-center the orbit
                    // pivot on the cue ball (instead of whatever the target
                    // happened to be) so rotating while inspecting orbits
                    // around the ball, not some unrelated leftover point.
                    let offset = cue_ball_pos - camera.target;
                    camera.target = cue_ball_pos;
                    camera.position = camera.position + offset;
                }
                frozen_aim_camera = camera;
                saved_camera = Some(camera);

                if let Some(resume) = last_view_camera {
                    camera = resume;
                }
            }
            view_mode = !view_mode;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_G) {
            show_ghost_ball = !show_ghost_ball;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_H) {
            show_aim_line = !show_aim_line;
        }

        // While in view mode, the cue's aim stays frozen at whatever it was
        // when view mode was entered, even as the camera keeps moving.
        let aim_camera = if view_mode { frozen_aim_camera } else { camera };
        let shot_dir = shot_direction_xz(aim_camera);

        if let (true, Some(dir)) = (rl.is_key_pressed(KeyboardKey::KEY_SPACE), shot_dir) {
            let (pocket_idx, gate_dir) = target_pocket;
            shot_test = Some(test_shot(
                dir,
                cue_ball_pos,
                object_ball_pos,
                pockets[pocket_idx].position,
                gate_dir,
            ));
        }

        let pan_dist = PAN_SPEED * rl.get_frame_time();
        if rl.is_key_down(KeyboardKey::KEY_W) || rl.is_key_down(KeyboardKey::KEY_UP) {
            camera.move_forward(pan_dist, true);
        }
        if rl.is_key_down(KeyboardKey::KEY_S) || rl.is_key_down(KeyboardKey::KEY_DOWN) {
            camera.move_forward(-pan_dist, true);
        }
        if rl.is_key_down(KeyboardKey::KEY_A) || rl.is_key_down(KeyboardKey::KEY_LEFT) {
            camera.move_right(-pan_dist, true);
        }
        if rl.is_key_down(KeyboardKey::KEY_D) || rl.is_key_down(KeyboardKey::KEY_RIGHT) {
            camera.move_right(pan_dist, true);
        }

        let key_rotate = KEY_ROTATE_SPEED_DEG.to_radians() * rl.get_frame_time();
        if rl.is_key_down(KeyboardKey::KEY_Q) {
            camera.yaw(key_rotate, true);
        }
        if rl.is_key_down(KeyboardKey::KEY_E) {
            camera.yaw(-key_rotate, true);
        }

        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            let delta = rl.get_mouse_delta();
            // On the table: real distance to the point under the cursor.
            // Off the table: the virtual cone (angle-only, continuous —
            // no fixed number, no discontinuity at the table's edge).
            let cursor_dist = cursor_on_table_point(&rl, camera)
                .map(|hit| {
                    camera
                        .position
                        .distance(hit)
                        .clamp(ROTATE_MIN_DIST, ROTATE_VIRTUAL_MAX_DIST)
                })
                .unwrap_or_else(|| cursor_cone_distance(&rl, camera));
            let sensitivity = ROTATE_SENSITIVITY * (ROTATE_REFERENCE_DISTANCE / cursor_dist);
            camera.yaw(-delta.x * sensitivity, true);
            camera.pitch(-delta.y * sensitivity, true, true, false);
        }
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            // Zoom toward whatever world point is under the cursor so that
            // point stays fixed on screen as we zoom, instead of always
            // zooming to the orbit target.
            if let Some(hit) = cursor_table_point(&rl, camera) {
                let factor = (wheel * 0.15).clamp(-0.9, 0.9);
                camera.position = camera.position.lerp(hit, factor);
                camera.target = camera.target.lerp(hit, factor);

                let dist = camera.position.distance(camera.target).clamp(0.15, 6.0);
                let dir = (camera.position - camera.target).normalize();
                camera.position = camera.target + dir.scale(dist);
            }
        }

        ball_shader.set_shader_value(view_pos_loc, camera.position);
        ghost_shader.set_shader_value(ghost_view_pos_loc, camera.position);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(30, 30, 30, 255));

        {
            let mut d3 = d.begin_mode3D(camera);
            draw_table(&mut d3, &pockets);
            draw_light_fixture(&mut d3, &light_panels);

            ball_material.set_map_color(MaterialMapIndex::MATERIAL_MAP_ALBEDO, CUE_BALL_COLOR);
            d3.draw_mesh(
                &ball_mesh,
                weak_copy(&ball_material),
                Matrix::translate(cue_ball_pos.x, cue_ball_pos.y, cue_ball_pos.z),
            );

            ball_material.set_map_color(MaterialMapIndex::MATERIAL_MAP_ALBEDO, OBJECT_BALL_COLOR);
            d3.draw_mesh(
                &ball_mesh,
                weak_copy(&ball_material),
                Matrix::translate(object_ball_pos.x, object_ball_pos.y, object_ball_pos.z),
            );

            if let Some(dir) = shot_dir {
                draw_cue(&mut d3, dir, cue_ball_pos);

                if show_ghost_ball {
                    let raycast = cue_raycast(dir, cue_ball_pos, object_ball_pos);
                    ghost_material.set_map_color(MaterialMapIndex::MATERIAL_MAP_ALBEDO, GHOST_BALL_COLOR);
                    d3.draw_mesh(
                        &ball_mesh,
                        weak_copy(&ghost_material),
                        Matrix::translate(raycast.ghost_pos.x, raycast.ghost_pos.y, raycast.ghost_pos.z),
                    );
                    if show_aim_line && raycast.hit_object_ball {
                        draw_object_ball_aim_line(&mut d3, raycast.ghost_pos, object_ball_pos);
                    }
                }
            }

            let (pocket_idx, gate_dir) = target_pocket;
            let gate_color = match shot_test.as_ref().map(|s| &s.gate_state) {
                Some(GateState::Success) => GATE_SUCCESS_COLOR,
                Some(GateState::Miss) => GATE_MISS_COLOR,
                None => GATE_NEUTRAL_COLOR,
            };
            draw_gate(&mut d3, pockets[pocket_idx].position, gate_dir, gate_color);

            if let Some(test) = &shot_test {
                draw_path_stripe(&mut d3, cue_ball_pos, test.white_end, PATH_WHITE_COLOR);
                if let Some((start, end)) = test.red_path {
                    draw_path_stripe(&mut d3, start, end, PATH_RED_COLOR);
                    ghost_material.set_map_color(MaterialMapIndex::MATERIAL_MAP_ALBEDO, GHOST_RED_BALL_COLOR);
                    d3.draw_mesh(
                        &ball_mesh,
                        weak_copy(&ghost_material),
                        Matrix::translate(end.x, end.y, end.z),
                    );
                }
            }
        }

        d.draw_fps(10, 10);
        d.draw_text(
            "Drag/scroll: orbit  |  WASD/arrows: pan  |  Q/E: rotate  |  C: center on cue ball  |  R: reposition  |  Space: test shot  |  G: ghost ball  |  H: aim line  |  V: view mode",
            10,
            36,
            18,
            Color::LIGHTGRAY,
        );
        if view_mode {
            d.draw_text("VIEW MODE (cue aim frozen)", 10, 58, 18, Color::YELLOW);
        }
    }
}
