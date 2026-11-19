#version 450

// § 14: set 1, binding 0 — the picture + its sampler (one set per texture).
layout(set = 1, binding = 0) uniform sampler2D picture;

// Sticky note: the same block as mesh.vert. The engine passes `params` through
// untouched; their meaning is scop's (§ 19):
//   x = blend (0 = grey, 1 = picture, § 15)    y = colour mode (0 normal, 1 facing, 2 depth)
//   z, w = the model's nearest and farthest distance from the camera (depth mode)
layout(push_constant) uniform Push {
    mat4 model;
    vec4 params;
} push;

const vec3 FRONT = vec3(0.15, 0.35, 1.0); // facing mode: the side a face points to
const vec3 BACK  = vec3(1.0, 0.15, 0.1);  // facing mode: its back (inside out = red outside)

layout(location = 0) in vec3 colour;      // the face's grey
layout(location = 1) in vec2 uv;          // this pixel's spot on the picture
layout(location = 2) in float distance;   // § 19: linear distance from the camera
layout(location = 0) out vec4 out_colour; // slot 0 -> colour attachment 0

void main() {
    int mode = int(push.params.y + 0.5); // a float carrying a whole number: round it
    if (mode == 1) {
        // § 19: which side of the triangle is on screen, decided by its winding.
        out_colour = vec4(gl_FrontFacing ? FRONT : BACK, 1.0);
        return;
    }
    if (mode == 2) {
        // § 19: nearest point of the model = white, farthest = dark (not black, so it
        // still shows against the background).
        float t = clamp((distance - push.params.z) / (push.params.w - push.params.z), 0.0, 1.0);
        out_colour = vec4(vec3(mix(1.0, 0.06, t)), 1.0);
        return;
    }

    // § 18: keep the picture's squares square. The UVs use one scale for u and v,
    // which only suits a square picture. For any picture, use its shorter side as
    // the unit: wide → u × H/W, tall → v × W/H. Same texels per unit both ways, and
    // both stay within 0..1 (the long side is cropped, never tiled).
    // Done here, not in the mesh, because N hands each object a different picture.
    vec2 size = vec2(textureSize(picture, 0));
    vec2 square_uv = uv * min(size.x, size.y) / size;
    // mix(a, b, t) = a + (b − a) × t: all grey at 0, all picture at 1 (§ 15).
    out_colour = mix(vec4(colour, 1.0), texture(picture, square_uv), push.params.x);
}
