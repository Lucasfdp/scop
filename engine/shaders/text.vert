#version 450

// § 25 extra: text on screen. One unit quad (corners 0..1), drawn once per glyph with
// instancing (§ 22). Each instance's "matrix" carries the glyph, not a transform:
//   column 0 = rectangle on screen (x, y, width, height), pixels from the top-left
//   column 1 = its cell in the font picture (x, y, width, height), in texels
//   column 2 = colour (rgb)

layout(set = 0, binding = 1) readonly buffer Instances {
    mat4 instance[];
} instances;

// Same sticky note as the other shaders (shared layout); params.xy = screen size.
layout(push_constant) uniform Push {
    mat4 model;
    vec4 params;
} push;

layout(location = 0) in vec3 in_position; // a corner of the unit quad

layout(location = 0) out vec2 texel;          // where in the font picture
layout(location = 1) flat out vec3 colour;

void main() {
    mat4 glyph = instances.instance[gl_InstanceIndex];
    vec2 pixel = glyph[0].xy + in_position.xy * glyph[0].zw;
    // Pixels → Vulkan's −1..+1 (y already points down, like pixels). Depth 0: in
    // front of everything drawn before (the pipeline tests depth but never writes it).
    gl_Position = vec4(pixel / push.params.xy * 2.0 - 1.0, 0.0, 1.0);
    texel = glyph[1].xy + in_position.xy * glyph[1].zw;
    colour = glyph[2].rgb;
}
