#version 450

// § 25 extra: a glyph's pixel. texelFetch reads exactly one texel — no blending
// between neighbours — so letters stay sharp at any whole scale. The font's alpha
// says "ink or not": no ink, no pixel (discard), so no blending is needed either.

layout(set = 1, binding = 0) uniform sampler2D font;

layout(location = 0) in vec2 texel;
layout(location = 1) flat in vec3 colour;

layout(location = 0) out vec4 out_colour;

void main() {
    if (texelFetch(font, ivec2(floor(texel)), 0).a < 0.5) {
        discard;
    }
    out_colour = vec4(colour, 1.0);
}
