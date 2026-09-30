use crate::color::Color;
use crate::cube::{Cube, Hit};
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
    pub cubes: Vec<Cube>,
    pub materials: Vec<Material>,
    pub sun: Sun,
    /// Light that reaches every surface, even in shadow (a stand-in for sky light).
    pub ambient: Color,
}

impl Scene {
    /// Tests the ray against every cube and keeps the NEAREST hit.
    pub fn closest_hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        let mut closest = None;
        let mut closest_t = t_max;
        for cube in &self.cubes {
            if let Some(hit) = cube.intersect(ray, t_min, closest_t) {
                closest_t = hit.t;
                closest = Some(hit);
            }
        }
        closest
    }

    /// True if ANY cube is hit between t_min and t_max. Faster than closest_hit
    /// for shadow rays, because we can stop at the first blocker we find.
    pub fn is_blocked(&self, ray: &Ray, t_min: f32, t_max: f32) -> bool {
        self.cubes.iter().any(|cube| cube.intersect(ray, t_min, t_max).is_some())
    }

    pub fn add_material(&mut self, material: Material) -> usize {
        self.materials.push(material);
        self.materials.len() - 1
    }

    /// Places one 1×1×1 block. Block (x, y, z) fills the space from
    /// (x, y, z) to (x + 1, y + 1, z + 1), like a Minecraft block.
    pub fn add_block(&mut self, x: i32, y: i32, z: i32, material: usize) {
        let min = Vec3::new(x as f32, y as f32, z as f32);
        let max = min + Vec3::new(1.0, 1.0, 1.0);
        self.cubes.push(Cube::new(min, max, material));
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
}

/// Test scene: a grass base, a stone platform with a wooden block (a tiny
/// shrine stand-in), and a torii gate in front. All 1×1×1 blocks.
pub fn build_test_scene() -> Scene {
    let mut scene = Scene {
        cubes: Vec::new(),
        materials: Vec::new(),
        sun: Sun {
            direction_to_sun: Vec3::new(-0.45, 1.0, 0.55).normalized(),
            color: Color::new(1.0, 0.95, 0.85), // warm sunlight
        },
        ambient: Color::new(0.32, 0.33, 0.42), // cool, slightly blue sky light
    };

    let grass = scene.add_material(Material {
        albedo: Color::new(0.45, 0.72, 0.38),
        specular_strength: 0.0,
        shininess: 1.0,
    });
    let stone = scene.add_material(Material {
        albedo: Color::new(0.62, 0.62, 0.66),
        specular_strength: 0.15,
        shininess: 16.0,
    });
    let vermilion_lacquer = scene.add_material(Material {
        albedo: Color::new(0.85, 0.22, 0.15),
        specular_strength: 0.6,
        shininess: 48.0,
    });
    let wood = scene.add_material(Material {
        albedo: Color::new(0.55, 0.36, 0.2),
        specular_strength: 0.05,
        shininess: 8.0,
    });

    scene.fill((-5, 0, -5), (4, 0, 4), grass); // ground: 10 × 10
    scene.fill((-2, 1, -3), (2, 1, 0), stone); // platform
    scene.fill((-1, 2, -3), (1, 3, -2), wood); // shrine stand-in

    // Torii gate at z = 2
    scene.fill((-2, 1, 2), (-2, 4, 2), vermilion_lacquer); // left pillar
    scene.fill((2, 1, 2), (2, 4, 2), vermilion_lacquer); // right pillar
    scene.fill((-1, 4, 2), (1, 4, 2), vermilion_lacquer); // lower beam (nuki)
    scene.fill((-3, 5, 2), (3, 5, 2), vermilion_lacquer); // top beam (kasagi)

    scene
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_scene() -> Scene {
        Scene {
            cubes: Vec::new(),
            materials: Vec::new(),
            sun: Sun { direction_to_sun: Vec3::UP, color: Color::BLACK },
            ambient: Color::BLACK,
        }
    }

    #[test]
    fn nearer_cube_wins() {
        let mut scene = empty_scene();
        // Far block added FIRST, so list order can't be what decides.
        scene.add_block(0, 0, -6, 1);
        scene.add_block(0, 0, -2, 2);
        let ray = Ray::new(Vec3::new(0.5, 0.5, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = scene.closest_hit(&ray, 0.0, f32::INFINITY).expect("should hit");
        assert_eq!(hit.material, 2);
        assert!((hit.t - 6.0).abs() < 1e-5); // from z = 5 to the block's front at z = -1
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
