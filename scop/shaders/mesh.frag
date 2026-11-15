#version 450

// § 14: set 1, binding 0 — the picture + its sampler (one set per texture).
layout(set = 1, binding = 0) uniform sampler2D picture;

layout(location = 0) in vec3 colour;      // the face's grey (used again in § 15)
layout(location = 1) in vec2 uv;          // this pixel's spot on the picture
layout(location = 0) out vec4 out_colour; // slot 0 -> colour attachment 0

void main() {
    // The lookup: the colour at (u, v), read with the sampler's rules.
    // § 15 will fade between `colour` and this; for now: the picture alone.
    out_colour = texture(picture, uv);
}
