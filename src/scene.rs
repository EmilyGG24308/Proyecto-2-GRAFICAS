use crate::color::Color;
use crate::cube::Hit;
use crate::grid::BlockGrid;
use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::Vec3;

/// A light infinitely far away, so its rays all arrive parallel (like the real sun).
pub struct Sun {
    /// Unit vector from any surface point TOWARD the sun.
    pub direction_to_sun: Vec3,
    pub color: Color,
}

pub struct Scene {
    pub blocks: BlockGrid,
    pub materials: Vec<Material>,
    pub sun: Sun,
    /// Light that reaches every surface, even in shadow (a stand-in for sky light).
    pub ambient: Color,
}

impl Scene {
    /// An empty world with room for blocks from `min_corner` to
    /// `min_corner + size - 1` on each axis.
    pub fn new(min_corner: [i32; 3], size: [i32; 3], sun: Sun, ambient: Color) -> Scene {
        Scene {
            blocks: BlockGrid::new(min_corner, size),
            materials: Vec::new(),
            sun,
            ambient,
        }
    }

    /// The nearest block the ray hits between t_min and t_max.
    pub fn closest_hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        self.blocks.trace(ray, t_min, t_max)
    }

    /// True if anything blocks the ray between t_min and t_max (for shadows).
    pub fn is_blocked(&self, ray: &Ray, t_min: f32, t_max: f32) -> bool {
        self.blocks.trace(ray, t_min, t_max).is_some()
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

#[cfg(test)]
mod tests {
    use super::*;

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
        scene.add_block(0, 3, 0, 0);
        let upward = Ray::new(Vec3::new(0.5, 1.0, 0.5), Vec3::UP);
        assert!(scene.is_blocked(&upward, 0.0, f32::INFINITY));
        // Distance limit: a light 1 unit away is reached before the block at y = 3.
        assert!(!scene.is_blocked(&upward, 0.0, 1.0));
    }
}
