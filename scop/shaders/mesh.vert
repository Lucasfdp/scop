#version 450

// § 7: corners come from the vertex buffer, described by the pipeline's
// vertex input (binding 0: position at location 0, colour at location 1).
// § 10: two more inputs, declared by the engine's shared pipeline layout.

// Noticeboard: set 0, binding 0 — this frame slot's camera buffer.
layout(set = 0, binding = 0) uniform Camera {
    mat4 view_proj;   // projection × view, once per frame
} camera;

// Sticky note: pushed before each draw.
layout(push_constant) uniform Push {
    mat4 model;       // this object's place and turn
} push;

layout(location = 0) in vec3 in_position; // from the vertex buffer (3D since § 10)
layout(location = 1) in vec3 in_colour;   // from the vertex buffer

layout(location = 0) out vec3 colour;     // to the fragment shader (separate numbering)

void main() {
    // Right to left (§ 9): place the corner (w = 1: a point), then camera, then projection.
    gl_Position = camera.view_proj * push.model * vec4(in_position, 1.0);
    colour = in_colour;
}
