/// How a surface looks and reacts to light.
#[derive(Clone, Copy, Debug)]
pub struct Material {
    /// Texture numbers (indexes into `Scene::textures`) for the top face,
    /// the four side faces and the bottom face. Most materials use one
    /// texture everywhere; grass blocks use different ones, like Minecraft.
    pub texture_top: usize,
    pub texture_side: usize,
    pub texture_bottom: usize,
    /// How much light the surface scatters diffusely (0..1). The texture
    /// supplies the color; albedo says how strongly that color is lit.
    pub albedo: f32,
    /// How strong the shiny highlight is. 0 = completely matte.
    pub specular_strength: f32,
    /// How small and sharp the highlight is. ~8 = broad soft shine, ~64 = tight spot.
    pub shininess: f32,
    /// How much light passes THROUGH the surface (0 = opaque). Used in Phase 7.
    #[allow(dead_code)]
    pub transparency: f32,
    /// How much of the surroundings it mirrors: 0 = none, 1 = a perfect mirror.
    pub reflectivity: f32,
    /// How much light bends entering it (air = 1.0, water = 1.33). Used in Phase 7.
    #[allow(dead_code)]
    pub refractive_index: f32,
}

impl Material {
    /// An opaque, non-mirror material with one texture on every face.
    /// Use struct-update syntax to change more: `Material { reflectivity: 0.3, ..base }`
    pub const fn opaque(texture: usize, albedo: f32, specular_strength: f32, shininess: f32) -> Material {
        Material {
            texture_top: texture,
            texture_side: texture,
            texture_bottom: texture,
            albedo,
            specular_strength,
            shininess,
            transparency: 0.0,
            reflectivity: 0.0,
            refractive_index: 1.0,
        }
    }
}
