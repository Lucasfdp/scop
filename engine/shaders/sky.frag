#version 450

// § 23: each pixel of the sky = the sky's cube map, looked up in its direction.
// § 25: read from set 1 (the sky's own cube, passed as its picture), not the
// environment (set 0, binding 2): with live reflections on, the environment is the
// probe's capture — the sky would then paint the probe's view (props and all) around
// the camera.

layout(set = 1, binding = 0) uniform samplerCube environment;

layout(location = 0) in vec3 direction;
layout(location = 0) out vec4 out_colour;

void main() {
    out_colour = vec4(texture(environment, direction).rgb, 1.0);
}
