use crate::vec3::Vec3;

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
    /// How much light passes THROUGH the surface: 0 = opaque, 1 = perfectly clear.
    pub transparency: f32,
    /// How much of the surroundings it mirrors: 0 = none, 1 = a perfect mirror.
    pub reflectivity: f32,
    /// How much light bends entering it (air = 1.0, water = 1.33, glass = 1.5).
    /// Only matters when transparency > 0.
    pub refractive_index: f32,
    /// Light the surface gives off by itself, as a fraction of its texture
    /// color (0 = none). Lit lanterns glow; petals get a little, standing in
    /// for sunlight shining through thin petals.
    pub glow: f32,
    /// How fast the texture slides across the surface, in blocks per second
    /// along X, Y and Z (zero = still). Flowing water and the swirling portal.
    pub texture_scroll: Vec3,
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
            glow: 0.0,
            texture_scroll: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}
