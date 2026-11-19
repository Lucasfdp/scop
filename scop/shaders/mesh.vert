#version 450

// § 7: corners come from the vertex buffer, described by the pipeline's
// vertex input (binding 0: position at location 0, colour at 1, uv at 2).
// § 10: two more inputs, declared by the engine's shared pipeline layout.

// Noticeboard: set 0, binding 0 — this frame slot's camera buffer.
layout(set = 0, binding = 0) uniform Camera {
    mat4 view_proj;   // projection × view, once per frame
    vec4 light;       // § 20: used by mesh.frag (same block there)
} camera;

// Sticky note: pushed before each draw. Same block in mesh.frag (§ 15, § 19).
layout(push_constant) uniform Push {
    mat4 model;       // this object's place and turn
    vec4 params;      // scop's four numbers, used by the fragment shader (§ 19)
} push;

layout(location = 0) in vec3 in_position; // from the vertex buffer (3D since § 10)
layout(location = 1) in vec3 in_colour;   // from the vertex buffer
layout(location = 2) in vec2 in_uv;       // § 14: where this corner sits on the texture
layout(location = 3) in vec3 in_normal;   // § 20: which way the face points (length 1)

layout(location = 0) out vec3 colour;     // to the fragment shader (separate numbering)
layout(location = 1) out vec2 uv;         // blended across the triangle by the rasteriser
layout(location = 2) out float distance;  // § 19: how far from the camera (depth mode)
layout(location = 3) out vec3 normal;     // § 20: the normal, turned with the model

void main() {
    // Right to left (§ 9): place the corner (w = 1: a point), then camera, then projection.
    gl_Position = camera.view_proj * push.model * vec4(in_position, 1.0);
    colour = in_colour;
    uv = in_uv;
    // § 19: the perspective card's last row copied the distance into w (§ 9), so w
    // *is* the linear depth. (gl_FragCoord.z, the stored depth, isn't linear: Q-19.5.)
    distance = gl_Position.w;
    // § 20: normals turn with the model — the model card's turn, without its move
    // (mat3 keeps only the turn part). Enough because scop never stretches models.
    normal = mat3(push.model) * in_normal;
    // § 18: dot size in pixels when drawn as points (ignored for triangles). The GPU
    // clamps it to its range (GpuInfo::point_size_range).
    gl_PointSize = 4.0;
}
