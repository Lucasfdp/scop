#version 450

// § 14: set 1, binding 0 — the picture + its sampler (one set per texture).
layout(set = 1, binding = 0) uniform sampler2D picture;

// Sticky note: the same block as mesh.vert; this shader uses the blend factor (§ 15).
layout(push_constant) uniform Push {
    mat4 model;
    float blend;      // 0 = grey, 1 = picture, eased by scop
} push;

layout(location = 0) in vec3 colour;      // the face's grey
layout(location = 1) in vec2 uv;          // this pixel's spot on the picture
layout(location = 0) out vec4 out_colour; // slot 0 -> colour attachment 0

void main() {
    // § 18: keep the picture's squares square. The UVs use one scale for u and v,
    // which only suits a square picture. For any picture, use its shorter side as
    // the unit: wide → u × H/W, tall → v × W/H. Same texels per unit both ways, and
    // both stay within 0..1 (the long side is cropped, never tiled).
    // Done here, not in the mesh, because N hands each object a different picture.
    vec2 size = vec2(textureSize(picture, 0));
    vec2 square_uv = uv * min(size.x, size.y) / size;
    // mix(a, b, t) = a + (b − a) × t: all grey at 0, all picture at 1 (§ 15).
    out_colour = mix(vec4(colour, 1.0), texture(picture, square_uv), push.blend);
}
