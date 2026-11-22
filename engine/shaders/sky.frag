#version 450

// § 23: each pixel of the sky = the environment cube map, looked up in its direction.

layout(set = 0, binding = 2) uniform samplerCube environment;

layout(location = 0) in vec3 direction;
layout(location = 0) out vec4 out_colour;

void main() {
    out_colour = vec4(texture(environment, direction).rgb, 1.0);
}
