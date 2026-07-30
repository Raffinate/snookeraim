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

// Table-model shader: same Blinn-Phong/light-panel setup as the balls, but
// also samples the model's own metalness map (texture1 -- raylib's glTF
// loader auto-binds it there, and LoadShaderFromMemory auto-detects
// "texture1" as SHADER_LOC_MAP_SPECULAR for any shader, not just the
// default one) to vary shininess per material: matte cloth/wood stays
// dull, any glossier/metal trim gets a tighter, stronger highlight. Not
// full PBR (no roughness map, no real reflections) -- just enough to make
// the loaded materials feel differentiated instead of uniformly plasticky.

#[cfg(not(target_os = "emscripten"))]
const TABLE_FS: &str = r#"
#version 330

in vec3 fragPosition;
in vec2 fragTexCoord;
in vec4 fragColor;
in vec3 fragNormal;

uniform sampler2D texture0;
uniform sampler2D texture1;
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
    float metalness = texture(texture1, fragTexCoord).r;
    float shininess = mix(8.0, 64.0, metalness);
    float specStrength = mix(0.05, 0.6, metalness);

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
        if (NdotL > 0.0) specCo = pow(max(0.0, dot(viewD, reflect(-lightDir, normal))), shininess);
        specular += specCo * specStrength * lightColor[i].rgb;
    }

    vec4 tint = colDiffuse * fragColor;

    finalColor = texelColor * ((tint + vec4(specular, 1.0)) * vec4(lightDot, 1.0));
    finalColor += texelColor * (ambient / 10.0) * tint;
    finalColor = pow(finalColor, vec4(1.0 / 2.2));
}
"#;

#[cfg(target_os = "emscripten")]
const TABLE_FS: &str = r#"#version 300 es
precision mediump float;

in vec3 fragPosition;
in vec2 fragTexCoord;
in vec4 fragColor;
in vec3 fragNormal;

uniform sampler2D texture0;
uniform sampler2D texture1;
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
    float metalness = texture(texture1, fragTexCoord).r;
    float shininess = mix(8.0, 64.0, metalness);
    float specStrength = mix(0.05, 0.6, metalness);

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
        if (NdotL > 0.0) specCo = pow(max(0.0, dot(viewD, reflect(-lightDir, normal))), shininess);
        specular += specCo * specStrength * lightColor[i].rgb;
    }

    vec4 tint = colDiffuse * fragColor;

    finalColor = texelColor * ((tint + vec4(specular, 1.0)) * vec4(lightDot, 1.0));
    finalColor += texelColor * (ambient / 10.0) * tint;
    finalColor = pow(finalColor, vec4(1.0 / 2.2));
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
const ZOOM_BUTTON_SPEED: f32 = 1.0; // meters per second, for the on-screen +/- buttons
const PINCH_ZOOM_SENSITIVITY: f32 = 0.004; // per pixel of change in two-finger spread
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

// Loaded table model (assets/snooker_table.glb, stripped of balls/lamp/cues
// by scripts/strip_table_model.py -- see that script for provenance and
// licensing). Set to `false` as an escape hatch back to the procedural
// table below, in case the model's alignment needs adjusting.
const USE_TABLE_MODEL: bool = true;
const TABLE_MODEL_PATH: &str = "assets/snooker_table.glb";
// Measured directly from the model's "Baize" (cloth) mesh bounding box:
// the flat bed sits at local (X center 0.1, Z center -1.879, Y top
// 0.8697), not at its own origin. This offset brings that point to our
// world's table center / cloth height (0, 0, 0), matching where the
// procedural table, balls, and gameplay math all assume the cloth is.
const TABLE_MODEL_OFFSET_X: f32 = -0.1;
const TABLE_MODEL_OFFSET_Y: f32 = -0.8697;
const TABLE_MODEL_OFFSET_Z: f32 = 1.879;

// Real cue and ball meshes/textures, extracted from the same source model
// by scripts/extract_props.py (see that script for how/why).
const USE_MODEL_PROPS: bool = true;

// cue.glb: a single continuous mesh (1.486m, matching a real cue), rebaked
// by scripts/extract_props.py's flatten_root into a local frame with no
// leftover display-rack rotation -- long axis along local +X. Confirmed
// by sampling actual vertex radii along its length (not just the bounding
// box) that it tapers from ~13mm at -X down to ~4mm at +X, i.e. +X is the
// tip end, -X is the butt. Measured directly from the flattened file.
const CUE_MODEL_PATH: &str = "assets/cue.glb";
const CUE_MODEL_TIP_X: f32 = 0.71477;
const CUE_MODEL_BUTT_X: f32 = -0.77121;

// balls.glb: mesh indices and baked centers for the one cue-ball mesh and
// one (of 15 identical) red-ball mesh we actually use, measured the same
// way as the table's Baize offset. Node transforms were left untouched by
// extract_props.py, so these centers come straight from the original
// rack layout in the source file.
const BALLS_MODEL_PATH: &str = "assets/balls.glb";
const CUE_BALL_MESH_INDEX: usize = 5;
const RED_BALL_MESH_INDEX: usize = 7;
const CUE_BALL_MODEL_CENTER: Vector3 = Vector3 { x: 0.23129013, y: 0.89566159, z: -0.64475494 };
const RED_BALL_MODEL_CENTER: Vector3 = Vector3 { x: 0.10001251, y: 0.89566159, z: -3.01039052 };

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

// On-screen touch controls + help popup: sized for fingers, not just
// mouse pointers, and laid out from the current screen size each frame so
// they hold up across window resizes and phone aspect ratios. These are
// the *base* (desktop-scale) sizes; `touch_ui` scales them down together
// -- down to `UI_MIN_SCALE` -- so the whole control set still fits (and
// doesn't overlap itself) on small phone screens.
const BTN_BASE: i32 = 52; // button edge length, px, at scale 1.0
const BTN_GAP_BASE: i32 = 8;
const BTN_MARGIN_BASE: i32 = 14;
const BTN_FONT_BASE: i32 = 16;
const BTN_FONT_MIN: i32 = 10;
const UI_MIN_SCALE: f32 = 0.55;
const BTN_FILL: Color = Color::new(255, 255, 255, 40);
const BTN_FILL_ACTIVE: Color = Color::new(255, 220, 40, 90);
const BTN_BORDER: Color = Color::new(255, 255, 255, 160);
const BTN_TEXT: Color = Color::WHITE;
const HELP_BG: Color = Color::new(0, 0, 0, 190);

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

/// Same aim as `draw_cue`, but draws the real cue model instead of a
/// plain cylinder. The model's own local +X axis is its tip-ward long
/// axis (see CUE_MODEL_* comments), centered on the axis with no baked
/// rotation left in it -- so orienting it is just "rotate local +X onto
/// the world direction the tip should point", i.e. onto `-dir` (`dir`
/// itself points tip -> butt, same convention as `draw_cue`).
fn draw_cue_model(d: &mut impl RaylibDraw3D, model: &Model, shot_dir: (f32, f32), cue_ball_pos: Vector3) {
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

    d.draw_model_ex(model, position, axis, angle_deg, scale, Color::WHITE);
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

/// Where a screen-space ray through `screen_pos` meets the table plane
/// (y = 0), if it does so in front of the camera. `None` when it points
/// away from the table entirely (e.g. up toward the sky).
fn screen_ray_table_point(rl: &RaylibHandle, camera: Camera3D, screen_pos: Vector2) -> Option<Vector3> {
    let ray = rl.get_screen_to_world_ray(screen_pos, camera);
    if ray.direction.y.abs() <= 1e-4 {
        return None;
    }
    let t = -ray.position.y / ray.direction.y;
    (t > 0.0).then(|| ray.position + ray.direction.scale(t))
}

/// Same as `screen_ray_table_point`, but through the mouse cursor.
fn cursor_table_point(rl: &RaylibHandle, camera: Camera3D) -> Option<Vector3> {
    screen_ray_table_point(rl, camera, rl.get_mouse_position())
}

/// Zooms the camera toward `hit` by `factor` (a fraction of the remaining
/// distance, same convention as a lerp) while keeping the camera-to-target
/// distance clamped to a sane range -- shared by wheel-zoom and pinch-zoom,
/// which both zoom toward a specific world point rather than along the
/// current view axis (see the on-screen zoom buttons for that variant).
fn zoom_toward(camera: &mut Camera3D, hit: Vector3, factor: f32) {
    camera.position = camera.position.lerp(hit, factor);
    camera.target = camera.target.lerp(hit, factor);

    let dist = camera.position.distance(camera.target).clamp(0.15, 6.0);
    let dir = (camera.position - camera.target).normalize();
    camera.position = camera.target + dir.scale(dist);
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

fn point_in_rect(px: f32, py: f32, x: i32, y: i32, w: i32, h: i32) -> bool {
    px >= x as f32 && px <= (x + w) as f32 && py >= y as f32 && py <= (y + h) as f32
}

/// A single on-screen touch/click button. Positions are recomputed from
/// the current screen size every frame (see `touch_ui`) rather than
/// stored, so the layout holds up across window resizes.
struct Btn {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &'static str,
    font: i32,
}

impl Btn {
    fn new(x: i32, y: i32, w: i32, h: i32, label: &'static str, font: i32) -> Self {
        Btn { x, y, w, h, label, font }
    }

    fn hit(&self, mouse: Vector2) -> bool {
        point_in_rect(mouse.x, mouse.y, self.x, self.y, self.w, self.h)
    }

    fn draw(&self, d: &mut RaylibDrawHandle, mouse: Vector2, active: bool) {
        let hovered = self.hit(mouse);
        let fill = if active {
            BTN_FILL_ACTIVE
        } else if hovered {
            Color::new(255, 255, 255, 70)
        } else {
            BTN_FILL
        };
        d.draw_rectangle(self.x, self.y, self.w, self.h, fill);
        d.draw_rectangle_lines(self.x, self.y, self.w, self.h, BTN_BORDER);
        let text_w = d.measure_text(self.label, self.font);
        d.draw_text(
            self.label,
            self.x + (self.w - text_w) / 2,
            self.y + (self.h - self.font) / 2,
            self.font,
            BTN_TEXT,
        );
    }
}

/// All on-screen controls: a camera cluster (pan/rotate/zoom) bottom-left,
/// action buttons (mirroring the keyboard hotkeys) bottom-right, and a
/// help toggle top-right. Rebuilt fresh from the current screen size every
/// frame -- cheap (a couple dozen integer adds), and the only way the
/// layout stays correct as the window/canvas is resized.
struct TouchUi {
    pan_up: Btn,
    pan_down: Btn,
    pan_left: Btn,
    pan_right: Btn,
    rot_left: Btn,
    rot_right: Btn,
    zoom_in: Btn,
    zoom_out: Btn,
    reset: Btn,
    test: Btn,
    // `None` while collapsed via `expand_toggle` -- not drawn, not
    // hit-tested, no space reserved for them.
    ghost: Option<Btn>,
    aim: Option<Btn>,
    expand_toggle: Btn,
    view: Btn,
    center: Btn,
    help: Btn,
}

fn opt_hit(btn: &Option<Btn>, mouse: Vector2) -> bool {
    btn.as_ref().is_some_and(|b| b.hit(mouse))
}

impl TouchUi {
    fn hit_any(&self, mouse: Vector2) -> bool {
        [
            &self.pan_up,
            &self.pan_down,
            &self.pan_left,
            &self.pan_right,
            &self.rot_left,
            &self.rot_right,
            &self.zoom_in,
            &self.zoom_out,
            &self.reset,
            &self.test,
            &self.expand_toggle,
            &self.view,
            &self.center,
            &self.help,
        ]
        .iter()
        .any(|b| b.hit(mouse))
            || opt_hit(&self.ghost, mouse)
            || opt_hit(&self.aim, mouse)
    }
}

fn touch_ui(
    screen_w: i32,
    screen_h: i32,
    reset_label: &'static str,
    ghost_aim_visible: bool,
) -> TouchUi {
    // Scale the whole control set down together so it always fits the
    // current screen without overlapping itself, with a floor so buttons
    // never shrink below tappable. The bottom-left camera cluster and the
    // bottom-right HIT square are the two widest bottom-anchored things
    // (each `6*BTN+5*GAP` / `3*BTN+2*GAP` wide); the top-right column and
    // that same HIT square are the two tallest. Solving for the scale that
    // keeps each pair's combined footprint within the screen (with a
    // little slack for margins) gives a hard upper bound; below that we
    // just use the screen as-is.
    let bl_w_base = 6 * BTN_BASE + 5 * BTN_GAP_BASE;
    let hit_w_base = 3 * BTN_BASE + 2 * BTN_GAP_BASE;
    let top_h_base = 3 * BTN_BASE + 2 * BTN_GAP_BASE;
    let width_scale = (screen_w - 3 * BTN_MARGIN_BASE) as f32 / (bl_w_base + hit_w_base) as f32;
    let height_scale = (screen_h - 3 * BTN_MARGIN_BASE) as f32 / (top_h_base + hit_w_base) as f32;
    let scale = width_scale.min(height_scale).min(1.0).max(UI_MIN_SCALE);

    let btn = (BTN_BASE as f32 * scale).round() as i32;
    let gap = ((BTN_GAP_BASE as f32 * scale).round() as i32).max(2);
    let margin = (BTN_MARGIN_BASE as f32 * scale).round() as i32;
    let font = ((BTN_FONT_BASE as f32 * scale).round() as i32).max(BTN_FONT_MIN);
    let step = btn + gap;

    // Bottom-left: a 4x2 grid (Q ^ E + / < v > -), a square 2x2 CENTER
    // button to its right, and a LOOK header bar spanning both on top.
    let grid_x = margin;
    let row_bottom_y = screen_h - margin - btn;
    let row_top_y = row_bottom_y - step;
    let look_y = row_top_y - step;
    let col_x = |col: i32| grid_x + col * step;

    let rot_left = Btn::new(col_x(0), row_top_y, btn, btn, "Q", font);
    let pan_up = Btn::new(col_x(1), row_top_y, btn, btn, "^", font);
    let rot_right = Btn::new(col_x(2), row_top_y, btn, btn, "E", font);
    let zoom_in = Btn::new(col_x(3), row_top_y, btn, btn, "+", font);

    let pan_left = Btn::new(col_x(0), row_bottom_y, btn, btn, "<", font);
    let pan_down = Btn::new(col_x(1), row_bottom_y, btn, btn, "v", font);
    let pan_right = Btn::new(col_x(2), row_bottom_y, btn, btn, ">", font);
    let zoom_out = Btn::new(col_x(3), row_bottom_y, btn, btn, "-", font);

    let center_w = 2 * btn + gap;
    let center_x = col_x(3) + step;
    let center = Btn::new(center_x, row_top_y, center_w, center_w, "CENTER", font);

    let look_w = (center_x + center_w) - grid_x;
    let view = Btn::new(grid_x, look_y, look_w, btn, "LOOK", font);

    // Bottom-right: HIT stands alone, mirroring the Space hotkey -- a big
    // 3x3 square, since it's the main "do the thing" action.
    let hit_w = 3 * btn + 2 * gap;
    let test = Btn::new(
        screen_w - margin - hit_w,
        screen_h - margin - hit_w,
        hit_w,
        hit_w,
        "HIT",
        font,
    );

    // Top-right: help, with an expand/collapse toggle directly below it.
    // Reset sits to help's left, two squares wide so its "CLEAR"/"NEXT"
    // label has room to breathe, and its label reflects what pressing it
    // will actually do right now. Ghost/Aim stack below reset, sharing its
    // width, but only exist (and only take up space) while expanded.
    let help = Btn::new(screen_w - margin - btn, margin, btn, btn, "?", font);
    let expand_toggle = Btn::new(
        help.x,
        margin + step,
        btn,
        btn,
        if ghost_aim_visible { ">>" } else { "<<" },
        font,
    );
    let reset_w = 2 * btn + gap;
    let reset_x = help.x - gap - reset_w;
    let reset = Btn::new(reset_x, margin, reset_w, btn, reset_label, font);
    let (ghost, aim) = if ghost_aim_visible {
        (
            Some(Btn::new(reset_x, margin + step, reset_w, btn, "GHOST", font)),
            Some(Btn::new(reset_x, margin + 2 * step, reset_w, btn, "AIM", font)),
        )
    } else {
        (None, None)
    };

    TouchUi {
        pan_up,
        pan_down,
        pan_left,
        pan_right,
        rot_left,
        rot_right,
        zoom_in,
        zoom_out,
        reset,
        test,
        ghost,
        aim,
        expand_toggle,
        view,
        center,
        help,
    }
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 800)
        .title("Snooker Aim Trainer")
        .msaa_4x()
        .resizable()
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
    let mut help_visible = false;
    let mut ghost_aim_visible = true;
    // Distance between the two touch points on the previous frame, so
    // pinch-zoom can look at the *change* in spread rather than its
    // absolute value. `None` whenever fewer than two fingers are down, so
    // a fresh pinch never starts from a stale distance left over from a
    // previous one.
    let mut pinch_prev_dist: Option<f32> = None;

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

    let mut table_shader = rl.load_shader_from_memory(&thread, Some(BALL_VS), Some(TABLE_FS));
    let table_ambient_loc = table_shader.get_shader_location("ambient");
    let table_view_pos_loc = table_shader.get_shader_location("viewPos");
    let table_light_pos_loc = table_shader.get_shader_location("lightPos");
    let table_light_color_loc = table_shader.get_shader_location("lightColor");
    table_shader.set_shader_value(table_ambient_loc, Vector4::new(0.35, 0.35, 0.35, 1.0));
    table_shader.set_shader_value_v(table_light_pos_loc, &light_panels);
    table_shader.set_shader_value_v(table_light_color_loc, &light_colors);

    let mut table_model = rl
        .load_model(&thread, TABLE_MODEL_PATH)
        .expect("failed to load assets/snooker_table.glb");
    for material in table_model.materials_mut() {
        material.set_shader(&table_shader);
    }

    let mut cue_model = rl
        .load_model(&thread, CUE_MODEL_PATH)
        .expect("failed to load assets/cue.glb");
    for material in cue_model.materials_mut() {
        material.set_shader(&table_shader);
    }

    let mut balls_model = rl
        .load_model(&thread, BALLS_MODEL_PATH)
        .expect("failed to load assets/balls.glb");
    for material in balls_model.materials_mut() {
        material.set_shader(&table_shader);
    }

    while !rl.window_should_close() {
        let mouse = rl.get_mouse_position();
        let screen_w = rl.get_screen_width();
        let screen_h = rl.get_screen_height();
        let reset_label = if shot_test.is_some() { "CLEAR" } else { "NEXT" };
        let ui = touch_ui(screen_w, screen_h, reset_label, ghost_aim_visible);
        let tap = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let held = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let over_ui = ui.hit_any(mouse);

        if rl.is_key_pressed(KeyboardKey::KEY_SLASH) || (tap && ui.help.hit(mouse)) {
            help_visible = !help_visible;
        } else if help_visible && tap {
            // The help button itself is excluded by the branch above, so any
            // other tap/click while the popup is open just dismisses it.
            help_visible = false;
        }

        if !help_visible {
            if tap && ui.expand_toggle.hit(mouse) {
                ghost_aim_visible = !ghost_aim_visible;
            }

            if rl.is_key_pressed(KeyboardKey::KEY_R) || (tap && ui.reset.hit(mouse)) {
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

            if rl.is_key_pressed(KeyboardKey::KEY_C) || (tap && ui.center.hit(mouse)) {
                // Re-center the orbit pivot on the cue ball without touching
                // zoom or viewing angle: shift both position and target by the
                // same offset, so their relative vector is unchanged.
                let offset = cue_ball_pos - camera.target;
                camera.target = cue_ball_pos;
                camera.position = camera.position + offset;
            }

            if rl.is_key_pressed(KeyboardKey::KEY_V) || (tap && ui.view.hit(mouse)) {
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

            if rl.is_key_pressed(KeyboardKey::KEY_G) || (tap && opt_hit(&ui.ghost, mouse)) {
                show_ghost_ball = !show_ghost_ball;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_H) || (tap && opt_hit(&ui.aim, mouse)) {
                show_aim_line = !show_aim_line;
            }
        }

        // While in view mode, the cue's aim stays frozen at whatever it was
        // when view mode was entered, even as the camera keeps moving. This
        // is derived from state, not input, so it's still computed while the
        // help popup is open (the frozen 3D scene keeps rendering behind it).
        let aim_camera = if view_mode { frozen_aim_camera } else { camera };
        let shot_dir = shot_direction_xz(aim_camera);

        if !help_visible {
            if let (true, Some(dir)) = (
                rl.is_key_pressed(KeyboardKey::KEY_SPACE) || (tap && ui.test.hit(mouse)),
                shot_dir,
            ) {
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
            if rl.is_key_down(KeyboardKey::KEY_W)
                || rl.is_key_down(KeyboardKey::KEY_UP)
                || (held && ui.pan_up.hit(mouse))
            {
                camera.move_forward(pan_dist, true);
            }
            if rl.is_key_down(KeyboardKey::KEY_S)
                || rl.is_key_down(KeyboardKey::KEY_DOWN)
                || (held && ui.pan_down.hit(mouse))
            {
                camera.move_forward(-pan_dist, true);
            }
            if rl.is_key_down(KeyboardKey::KEY_A)
                || rl.is_key_down(KeyboardKey::KEY_LEFT)
                || (held && ui.pan_left.hit(mouse))
            {
                camera.move_right(-pan_dist, true);
            }
            if rl.is_key_down(KeyboardKey::KEY_D)
                || rl.is_key_down(KeyboardKey::KEY_RIGHT)
                || (held && ui.pan_right.hit(mouse))
            {
                camera.move_right(pan_dist, true);
            }

            let key_rotate = KEY_ROTATE_SPEED_DEG.to_radians() * rl.get_frame_time();
            if rl.is_key_down(KeyboardKey::KEY_Q) || (held && ui.rot_left.hit(mouse)) {
                camera.yaw(key_rotate, true);
            }
            if rl.is_key_down(KeyboardKey::KEY_E) || (held && ui.rot_right.hit(mouse)) {
                camera.yaw(-key_rotate, true);
            }

            let touch_count = rl.get_touch_point_count();

            if held && !over_ui && !tap && touch_count < 2 {
                // Skip the exact press frame: on touch devices there's no
                // cursor easing into position beforehand, so the browser's
                // synthesized mouse position jumps straight from wherever it
                // last was to the tap point on this same frame. Reading that
                // as a drag delta would snap the camera toward the tap
                // instead of orbiting from it. Also skip while a second
                // finger is down -- that's a pinch, not a one-finger orbit,
                // and the browser's synthesized mouse position (tracking
                // just the first finger) would otherwise fight the pinch.
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
            if wheel != 0.0 && !over_ui {
                // Zoom toward whatever world point is under the cursor so that
                // point stays fixed on screen as we zoom, instead of always
                // zooming to the orbit target.
                if let Some(hit) = cursor_table_point(&rl, camera) {
                    let factor = (wheel * 0.15).clamp(-0.9, 0.9);
                    zoom_toward(&mut camera, hit, factor);
                }
            }

            // Pinch-zoom: two fingers down, zoom by how their spread
            // changes frame to frame (spreading apart = zoom in), toward
            // the point under their midpoint.
            if touch_count >= 2 {
                let t0 = rl.get_touch_position(0);
                let t1 = rl.get_touch_position(1);
                let dist = t0.distance(t1);
                let midpoint = t0.lerp(t1, 0.5);
                if let Some(prev) = pinch_prev_dist {
                    let hit = screen_ray_table_point(&rl, camera, midpoint)
                        .or_else(|| cursor_table_point(&rl, camera));
                    if let Some(hit) = hit {
                        let factor = ((dist - prev) * PINCH_ZOOM_SENSITIVITY).clamp(-0.9, 0.9);
                        zoom_toward(&mut camera, hit, factor);
                    }
                }
                pinch_prev_dist = Some(dist);
            } else {
                pinch_prev_dist = None;
            }

            // On-screen zoom buttons: there's no cursor-hit point to zoom
            // toward (the cursor is on the button, not over the table), so
            // these just zoom straight along the existing camera->target axis.
            let mut button_zoom = 0.0;
            if held && ui.zoom_in.hit(mouse) {
                button_zoom -= ZOOM_BUTTON_SPEED * rl.get_frame_time();
            }
            if held && ui.zoom_out.hit(mouse) {
                button_zoom += ZOOM_BUTTON_SPEED * rl.get_frame_time();
            }
            if button_zoom != 0.0 {
                let dist =
                    (camera.position.distance(camera.target) + button_zoom).clamp(0.15, 6.0);
                let dir = (camera.position - camera.target).normalize();
                camera.position = camera.target + dir.scale(dist);
            }
        }

        ball_shader.set_shader_value(view_pos_loc, camera.position);
        ghost_shader.set_shader_value(ghost_view_pos_loc, camera.position);
        table_shader.set_shader_value(table_view_pos_loc, camera.position);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(30, 30, 30, 255));

        {
            let mut d3 = d.begin_mode3D(camera);
            if USE_TABLE_MODEL {
                let offset = Vector3::new(TABLE_MODEL_OFFSET_X, TABLE_MODEL_OFFSET_Y, TABLE_MODEL_OFFSET_Z);
                d3.draw_model(&table_model, offset, 1.0, Color::WHITE);
            } else {
                draw_table(&mut d3, &pockets);
            }
            draw_light_fixture(&mut d3, &light_panels);

            if USE_MODEL_PROPS {
                // raylib always reserves materials()[0] for its own
                // auto-inserted default material (a blank white texture) and
                // appends the glTF file's real materials starting at index 1
                // -- balls.glb has exactly one ("Balls"), so index 1 is it.
                let balls_material = &balls_model.materials()[1];
                let cue_ball_offset = cue_ball_pos - CUE_BALL_MODEL_CENTER;
                d3.draw_mesh(
                    &balls_model.meshes()[CUE_BALL_MESH_INDEX],
                    weak_copy(balls_material),
                    Matrix::translate(cue_ball_offset.x, cue_ball_offset.y, cue_ball_offset.z),
                );

                let red_ball_offset = object_ball_pos - RED_BALL_MODEL_CENTER;
                d3.draw_mesh(
                    &balls_model.meshes()[RED_BALL_MESH_INDEX],
                    weak_copy(balls_material),
                    Matrix::translate(red_ball_offset.x, red_ball_offset.y, red_ball_offset.z),
                );
            } else {
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
            }

            if let Some(dir) = shot_dir {
                if USE_MODEL_PROPS {
                    draw_cue_model(&mut d3, &cue_model, dir, cue_ball_pos);
                } else {
                    draw_cue(&mut d3, dir, cue_ball_pos);
                }

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
        if view_mode {
            d.draw_text("VIEW MODE (cue aim frozen)", 10, 36, 18, Color::YELLOW);
        }

        // On-screen touch controls -- drawn every frame, on every platform
        // (desktop mouse clicks work identically to touch taps, since
        // browsers synthesize mouse events from touch input).
        ui.pan_up.draw(&mut d, mouse, false);
        ui.pan_down.draw(&mut d, mouse, false);
        ui.pan_left.draw(&mut d, mouse, false);
        ui.pan_right.draw(&mut d, mouse, false);
        ui.rot_left.draw(&mut d, mouse, false);
        ui.rot_right.draw(&mut d, mouse, false);
        ui.zoom_in.draw(&mut d, mouse, false);
        ui.zoom_out.draw(&mut d, mouse, false);
        ui.reset.draw(&mut d, mouse, false);
        ui.test.draw(&mut d, mouse, false);
        if let Some(b) = &ui.ghost {
            b.draw(&mut d, mouse, show_ghost_ball);
        }
        if let Some(b) = &ui.aim {
            b.draw(&mut d, mouse, show_aim_line);
        }
        ui.expand_toggle.draw(&mut d, mouse, false);
        ui.view.draw(&mut d, mouse, view_mode);
        ui.center.draw(&mut d, mouse, false);
        ui.help.draw(&mut d, mouse, help_visible);

        if help_visible {
            d.draw_rectangle(0, 0, screen_w, screen_h, HELP_BG);

            let lines = [
                "Controls   (tap anywhere, or ? / the ? button, to close)",
                "",
                "Drag / D-pad     orbit the camera",
                "Scroll / +  -    zoom toward the cursor / camera target",
                "Pinch            zoom toward the point between your fingers",
                "WASD or arrows   pan the camera",
                "Q / E            rotate (yaw) the camera",
                "C / CENTER       re-center the orbit pivot on the cue ball",
                "V / LOOK         toggle view mode (inspect freely, aim stays frozen)",
                "G / GHOST        toggle the ghost cue ball",
                "H / AIM          toggle the object-ball aim line (needs ghost ball on)",
                ">> / <<          show or hide the GHOST/AIM buttons",
                "Space / HIT      test the current aim: trace both balls' paths",
                "R / CLEAR-NEXT   clear a tested shot, or reposition both balls",
                "? / help button  toggle this popup",
            ];
            let start_y = 90;
            let line_h = 26;
            for (i, line) in lines.iter().enumerate() {
                d.draw_text(line, 24, start_y + i as i32 * line_h, 20, Color::RAYWHITE);
            }
        }
    }
}
