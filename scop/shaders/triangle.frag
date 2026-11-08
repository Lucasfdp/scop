#version 450

layout(location = 0) in vec3 colour;      // slot 0 <- already interpolated by the rasteriser
layout(location = 0) out vec4 out_colour; // slot 0 -> colour attachment 0

void main() {
    out_colour = vec4(colour, 1.0);
}
