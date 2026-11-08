#version 450

// § 7: corners come from the vertex buffer, described by the pipeline's
// vertex input (binding 0: position at location 0, colour at location 1).

layout(location = 0) in vec2 in_position; // from the vertex buffer
layout(location = 1) in vec3 in_colour;   // from the vertex buffer

layout(location = 0) out vec3 colour;     // to the fragment shader (separate numbering)

void main() {
    gl_Position = vec4(in_position, 0.0, 1.0);
    colour = in_colour;
}
