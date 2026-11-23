//! The engine's default shaders (§ 24, moved from scop): grey ↔ picture fade, one
//! directional light, chrome and debug colours for models; the sky for backgrounds.
//!
//! Use them through `Gfx::create_default_pipeline` / `Gfx::create_default_sky`, and
//! fill their four push numbers with [`DefaultParams`]. An app that wants something
//! else writes its own shaders (an override) and draws with raw `draw_with` numbers:
//! each pipeline owns the meaning of its params (Q-24.3).

use std::path::PathBuf;

use crate::pipeline::Shaders;

/// Model shaders: lighting, chrome, debug colours (`shaders/default.*`).
pub const DEFAULT_SHADERS: Shaders<'static> = Shaders {
    vertex: include_bytes!(concat!(env!("OUT_DIR"), "/default.vert.spv")),
    fragment: include_bytes!(concat!(env!("OUT_DIR"), "/default.frag.spv")),
};

/// Sky shaders: the environment looked up by direction (`shaders/sky.*`).
pub const SKY_SHADERS: Shaders<'static> = Shaders {
    vertex: include_bytes!(concat!(env!("OUT_DIR"), "/sky.vert.spv")),
    fragment: include_bytes!(concat!(env!("OUT_DIR"), "/sky.frag.spv")),
};

/// § 25 extra: the text shaders (`shaders/text.*`) — glyphs from the built-in font.
pub(crate) const TEXT_SHADERS: Shaders<'static> = Shaders {
    vertex: include_bytes!(concat!(env!("OUT_DIR"), "/text.vert.spv")),
    fragment: include_bytes!(concat!(env!("OUT_DIR"), "/text.frag.spv")),
};

/// The default shaders' sources, for hot reload: (vertex, fragment) of the model
/// shaders, then of the sky. They exist on the machine that built the engine; on
/// another one `ShaderWatch::start` says so and hot reload stays off (§ 21).
pub fn default_shader_files() -> [PathBuf; 4] {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("shaders");
    ["default.vert", "default.frag", "sky.vert", "sky.frag"].map(|f| folder.join(f))
}

/// How the default shader colours a model (§ 19, § 23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColourMode {
    /// Vertex colours ↔ picture (the fade), lit if the light is on.
    #[default]
    Normal,
    /// Blue where a triangle's front shows, red where its back does.
    Facing,
    /// Distance from the camera: near white, far dark (over `DefaultParams::depth_range`).
    Depth,
    /// A mirror reflecting the environment (unlit).
    Chrome,
}

impl ColourMode {
    /// The next mode, wrapping round: Normal → Facing → Depth → Chrome → Normal.
    pub fn next(self) -> Self {
        match self {
            ColourMode::Normal => ColourMode::Facing,
            ColourMode::Facing => ColourMode::Depth,
            ColourMode::Depth => ColourMode::Chrome,
            ColourMode::Chrome => ColourMode::Normal,
        }
    }

    /// The number the shader reads.
    fn index(self) -> f32 {
        match self {
            ColourMode::Normal => 0.0,
            ColourMode::Facing => 1.0,
            ColourMode::Depth => 2.0,
            ColourMode::Chrome => 3.0,
        }
    }
}

/// The default shader's four push numbers, by name (§ 24).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DefaultParams {
    /// 0 = vertex colours, 1 = the picture, in between = a mix (§ 15).
    pub blend: f32,
    pub colour_mode: ColourMode,
    /// The model's nearest and farthest distance from the camera, for depth mode
    /// (`camera::depth_range`).
    pub depth_range: (f32, f32),
}

impl DefaultParams {
    /// The four numbers for `draw_with` / `draw_instances`, in the order the default
    /// fragment shader reads them.
    pub fn params(&self) -> [f32; 4] {
        [
            self.blend.clamp(0.0, 1.0),
            self.colour_mode.index(),
            self.depth_range.0,
            self.depth_range.1,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_modes_cycle_back_to_normal() {
        let mut mode = ColourMode::default();
        for _ in 0..4 {
            mode = mode.next();
        }
        assert_eq!(mode, ColourMode::Normal);
    }

    #[test]
    fn params_in_the_shaders_order() {
        let p = DefaultParams {
            blend: 7.0,
            colour_mode: ColourMode::Chrome,
            depth_range: (2.0, 5.0),
        };
        assert_eq!(p.params(), [1.0, 3.0, 2.0, 5.0]); // blend clamped, chrome = 3
    }

    #[test]
    fn the_default_shader_sources_are_in_the_engine() {
        for file in default_shader_files() {
            assert!(file.is_file(), "{}", file.display());
        }
    }
}
