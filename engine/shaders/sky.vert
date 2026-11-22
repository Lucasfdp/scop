#version 450

// § 23: the sky box. A cube centred on the camera (`Sky::draw`'s model card moves it to the
// camera and makes it big), drawn first with depth writing off.

layout(set = 0, binding = 0) uniform Camera {
    mat4 view_proj;
    vec4 light;
    vec4 eye;
} camera;

// Same sticky note as default.vert (the pipeline layout is shared); only `model` is used.
layout(push_constant) uniform Push {
    mat4 model;
    vec4 params;
} push;

layout(location = 0) in vec3 in_position; // a corner of the unit cube (−1..1)
// The other vertex inputs (colour, uv, normal) exist but aren't needed here.

layout(location = 0) out vec3 direction;  // which way this corner is from the camera

void main() {
    // The box is centred on the camera, so a corner's own position *is* the direction
    // from the camera to it. The rasteriser blends it across each face: every pixel
    // gets its own direction.
    direction = in_position;
    gl_Position = camera.view_proj * push.model * vec4(in_position, 1.0);
}
