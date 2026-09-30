use crate::color::Color;

/// How a surface reacts to light.
/// (Phase 5 adds a texture, transparency and reflectivity to this.)
#[derive(Clone, Copy, Debug)]
pub struct Material {
    /// Base color: the fraction of each light color the surface scatters back (0..1).
    pub albedo: Color,
    /// How strong the shiny highlight is. 0 = completely matte.
    pub specular_strength: f32,
    /// How small and sharp the highlight is. ~8 = broad soft shine, ~64 = tight spot.
    pub shininess: f32,
}
