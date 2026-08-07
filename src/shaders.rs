// Desktop targets compile against desktop OpenGL 3.3 core (GLSL 330); the
// Emscripten/web target negotiates a WebGL2 context (GLSL ES 300, requested
// via the opengl_es_30 feature — see Cargo.toml). The two dialects are close
// enough (both use in/out and an explicit output var) that only the version
// line and a precision qualifier differ; desktop GLSL doesn't need the
// qualifier but WebGL2's compiler requires it in fragment shaders.

#[cfg(not(target_os = "emscripten"))]
pub const BALL_VS: &str = r#"
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
pub const BALL_VS: &str = r#"#version 300 es
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
pub const BALL_FS: &str = r#"
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
pub const BALL_FS: &str = r#"#version 300 es
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
pub const GHOST_FS: &str = r#"
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
pub const GHOST_FS: &str = r#"#version 300 es
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
pub const TABLE_FS: &str = r#"
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
    // The lighting math above pushes alpha well past tint.a (and gamma
    // then distorts it further) so it's useless for transparency as-is;
    // override it directly from the tint's own alpha, post-gamma, so
    // callers can make this opaque-by-default shader translucent just by
    // lowering their tint color's alpha.
    finalColor.a = texelColor.a * tint.a;
}
"#;

#[cfg(target_os = "emscripten")]
pub const TABLE_FS: &str = r#"#version 300 es
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
    finalColor.a = texelColor.a * tint.a;
}
"#;
