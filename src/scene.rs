use crate::color::Color;
use crate::cube::Hit;
use crate::cubemap;
use crate::grid::{BlockGrid, Boundary};
use crate::material::Material;
use crate::ray::Ray;
use crate::texture::Texture;
use crate::vec3::Vec3;
use std::path::PathBuf;

/// A light infinitely far away, so its rays all arrive parallel (like the real sun).
pub struct Sun {
    /// Unit vector from any surface point TOWARD the sun.
    pub direction_to_sun: Vec3,
    pub color: Color,
}

/// Two coordinate systems are used:
/// - WORLD space: the camera, the sun and the sky live here and never turn.
/// - DIORAMA space: the blocks live here. The whole diorama can spin like it's
///   on a turntable, around a vertical axis through `pivot`, by `turn_angle`.
///
/// Instead of moving 28,000 blocks, we move each RAY the opposite way into
/// diorama space, and do all the hit-testing and shading there. The blocks
/// stay axis-aligned, so the fast grid walk keeps working. Only the things that
/// live in world space (the sun direction, sky lookups) need converting.
pub struct Scene {
    pub blocks: BlockGrid,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub sun: Sun,
    /// Light that reaches every surface, even in shadow (a stand-in for sky light).
    pub ambient: Color,
    /// The diorama's center at ground level; it turns around this point.
    pivot: Vec3,
    /// Cosine and sine of the turn angle, computed once when the angle changes.
    turn_cos: f32,
    turn_sin: f32,
    /// The sun direction as seen from inside the turned diorama.
    local_sun_direction: Vec3,
    /// Texture numbers of the six skybox faces (+X, -X, +Y, -Y, +Z, -Z),
    /// or None to fall back to a plain gradient.
    skybox_faces: Option<[usize; 6]>,
}

impl Scene {
    /// An empty world with room for blocks from `min_corner` to
    /// `min_corner + size - 1` on each axis.
    pub fn new(min_corner: [i32; 3], size: [i32; 3], sun: Sun, ambient: Color) -> Scene {
        let pivot = Vec3::new(
            min_corner[0] as f32 + size[0] as f32 / 2.0,
            0.0,
            min_corner[2] as f32 + size[2] as f32 / 2.0,
        );
        let local_sun_direction = sun.direction_to_sun;
        Scene {
            blocks: BlockGrid::new(min_corner, size),
            materials: Vec::new(),
            textures: Vec::new(),
            sun,
            ambient,
            pivot,
            turn_cos: 1.0,
            turn_sin: 0.0,
            local_sun_direction,
            skybox_faces: None,
        }
    }

    /// Loads the six skybox faces (assets/textures/sky_px.bmp, sky_nx.bmp, ...).
    pub fn load_skybox(&mut self) {
        self.skybox_faces = Some(cubemap::FACE_NAMES.map(|name| self.load_texture(name)));
    }

    /// The sky seen looking in WORLD direction `direction` (any length).
    /// Returns None if this scene has no skybox loaded.
    pub fn skybox_color(&self, direction: Vec3) -> Option<Color> {
        let faces = self.skybox_faces?;
        let (face, u, v) = cubemap::face_and_uv(direction.x, direction.y, direction.z);
        Some(self.textures[faces[face]].sample(u, v))
    }

    /// Turns the diorama to `angle` radians (positive = counterclockwise seen from above).
    pub fn set_turn_angle(&mut self, angle: f32) {
        self.turn_cos = angle.cos();
        self.turn_sin = angle.sin();
        self.local_sun_direction = self.to_diorama_direction(self.sun.direction_to_sun);
    }

    /// World → diorama: undo the diorama's turn (rotate by -angle around Y).
    pub fn to_diorama_direction(&self, v: Vec3) -> Vec3 {
        rotate_around_y(v, self.turn_cos, -self.turn_sin)
    }

    /// Diorama → world: apply the diorama's turn (rotate by +angle around Y).
    pub fn to_world_direction(&self, v: Vec3) -> Vec3 {
        rotate_around_y(v, self.turn_cos, self.turn_sin)
    }

    /// A camera ray, re-expressed in diorama space. Positions turn around the
    /// pivot; directions just turn. Lengths don't change, so t values stay valid.
    pub fn to_diorama_ray(&self, ray: &Ray) -> Ray {
        let origin = self.pivot + self.to_diorama_direction(ray.origin - self.pivot);
        Ray::new(origin, self.to_diorama_direction(ray.direction))
    }

    /// Direction toward the sun, in diorama space (use this for all lighting).
    pub fn sun_direction(&self) -> Vec3 {
        self.local_sun_direction
    }

    /// Loads assets/textures/<name>.bmp and returns its texture number.
    /// A missing or broken file prints a warning and shows up magenta.
    pub fn load_texture(&mut self, name: &str) -> usize {
        let file_name = format!("{name}.bmp");
        // Look next to where the program was started first, then in the project folder.
        let local = PathBuf::from("assets").join("textures").join(&file_name);
        let path = if local.exists() {
            local
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join("textures").join(&file_name)
        };
        let texture = Texture::load_bmp(&path).unwrap_or_else(|error| {
            eprintln!("Warning: could not load texture ({error}); using a magenta placeholder");
            Texture::missing()
        });
        self.textures.push(texture);
        self.textures.len() - 1
    }

    /// The nearest block the ray hits between t_min and t_max.
    pub fn closest_hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        self.blocks.trace(ray, t_min, t_max)
    }

    /// True if an OPAQUE block is in the way between t_min and t_max (for shadows).
    /// See-through blocks (water, glass) let sunlight pass, so the pond floor
    /// is lit. Simplification: they don't dim or focus the light passing through.
    pub fn is_blocked(&self, ray: &Ray, t_min: f32, t_max: f32) -> bool {
        self.blocks
            .walk_until(ray, t_min, t_max, |cell| {
                cell.is_some_and(|material| self.materials[material as usize].transparency == 0.0)
            })
            .is_some()
    }

    /// For a ray travelling INSIDE a see-through material: the first wall where
    /// the contents change, i.e. where it reaches air or hits something solid.
    /// Neighboring blocks of the same material count as one continuous body,
    /// so the walls between two water blocks are invisible, as they should be.
    pub fn exit_from(&self, ray: &Ray, medium: usize) -> Option<Boundary> {
        self.blocks.walk_until(ray, 0.0, f32::INFINITY, |cell| cell != Some(medium as u8))
    }

    pub fn add_material(&mut self, material: Material) -> usize {
        assert!(self.materials.len() < 256, "a block stores its material in one byte");
        self.materials.push(material);
        self.materials.len() - 1
    }

    // ---- Scene-building helpers. Block (x, y, z) fills the space from
    // ---- (x, y, z) to (x + 1, y + 1, z + 1), like a Minecraft block.

    /// Places one block, replacing whatever was there.
    pub fn add_block(&mut self, x: i32, y: i32, z: i32, material: usize) {
        self.blocks.set([x, y, z], Some(material as u8));
    }

    /// Places a block only where there is air inside the world, so decorations
    /// (like tree canopies) never overwrite buildings or run off the edge.
    pub fn add_block_if_empty(&mut self, x: i32, y: i32, z: i32, material: usize) {
        if self.blocks.contains([x, y, z]) && self.blocks.get([x, y, z]).is_none() {
            self.add_block(x, y, z, material);
        }
    }

    pub fn remove_block(&mut self, x: i32, y: i32, z: i32) {
        self.blocks.set([x, y, z], None);
    }

    /// Fills every block from `from` to `to`, both corners included.
    pub fn fill(&mut self, from: (i32, i32, i32), to: (i32, i32, i32), material: usize) {
        for x in from.0..=to.0 {
            for y in from.1..=to.1 {
                for z in from.2..=to.2 {
                    self.add_block(x, y, z, material);
                }
            }
        }
    }

    /// Turns every block from `from` to `to` (inclusive) into air.
    pub fn clear(&mut self, from: (i32, i32, i32), to: (i32, i32, i32)) {
        for x in from.0..=to.0 {
            for y in from.1..=to.1 {
                for z in from.2..=to.2 {
                    self.remove_block(x, y, z);
                }
            }
        }
    }
}

/// Rotation around the vertical (Y) axis, given the angle's cosine and sine.
/// Y is unchanged; X and Z turn like the hands of a clock seen from above.
fn rotate_around_y(v: Vec3, cos: f32, sin: f32) -> Vec3 {
    Vec3::new(v.x * cos + v.z * sin, v.y, -v.x * sin + v.z * cos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_and_turning_back_gives_the_original_direction() {
        let mut scene = empty_scene();
        scene.set_turn_angle(0.7);
        let v = Vec3::new(0.3, -0.5, 0.8);
        let round_trip = scene.to_world_direction(scene.to_diorama_direction(v));
        assert!((round_trip - v).length() < 1e-6);
    }

    #[test]
    fn a_quarter_turn_moves_the_front_to_the_side() {
        let mut scene = empty_scene();
        scene.set_turn_angle(std::f32::consts::FRAC_PI_2);
        // What faced +Z (the front) now faces +X.
        let front = scene.to_world_direction(Vec3::new(0.0, 0.0, 1.0));
        assert!((front - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6);
    }

    fn empty_scene() -> Scene {
        let sun = Sun { direction_to_sun: Vec3::UP, color: Color::BLACK };
        Scene::new([-10, -10, -10], [20, 20, 20], sun, Color::BLACK)
    }

    #[test]
    fn nearer_block_wins() {
        let mut scene = empty_scene();
        scene.add_block(0, 0, -6, 1);
        scene.add_block(0, 0, -2, 2);
        let ray = Ray::new(Vec3::new(0.5, 0.5, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = scene.closest_hit(&ray, 0.0, f32::INFINITY).expect("should hit");
        assert_eq!(hit.material, 2);
        assert!((hit.t - 6.0).abs() < 1e-4); // from z = 5 to the block's front at z = -1
    }

    #[test]
    fn shadow_ray_is_blocked_by_block_above() {
        let mut scene = empty_scene();
        // Shadows check the block's material (see-through or not), so it needs one.
        let stone = scene.add_material(Material::opaque(0, 1.0, 0.0, 1.0));
        scene.add_block(0, 3, 0, stone);
        let upward = Ray::new(Vec3::new(0.5, 1.0, 0.5), Vec3::UP);
        assert!(scene.is_blocked(&upward, 0.0, f32::INFINITY));
        // Distance limit: a light 1 unit away is reached before the block at y = 3.
        assert!(!scene.is_blocked(&upward, 0.0, 1.0));
    }
}
