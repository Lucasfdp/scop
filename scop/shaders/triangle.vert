#version 450

// § 6: a hard-coded triangle — no vertex data, positions live in the shader.
// Vulkan's y points DOWN: y = -0.5 is the top corner.

layout(location = 0) out vec3 colour; // slot 0 -> the fragment shader

vec2 positions[3] = vec2[](
    vec2( 0.0, -0.5), // top
    vec2( 0.5,  0.5), // bottom right
    vec2(-0.5,  0.5)  // bottom left
);

vec3 colours[3] = vec3[](
    vec3(1.0, 0.0, 0.0), // red
    vec3(0.0, 1.0, 0.0), // green
    vec3(0.0, 0.0, 1.0)  // blue
);

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
    colour = colours[gl_VertexIndex];
}
