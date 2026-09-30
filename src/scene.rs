use crate::color::Color;
use crate::cube::{Cube, Hit};
use crate::ray::Ray;
use crate::vec3::Vec3;

pub struct Scene {
    pub cubes: Vec<Cube>,
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
}

/// A tiny test scene: a grass base, a stone platform, a small torii gate,
/// and a wooden block behind it (partly hidden by the gate).
pub fn build_test_scene() -> Scene {
    let grass = Color::new(0.45, 0.72, 0.38);
    let stone = Color::new(0.62, 0.62, 0.66);
    let vermilion = Color::new(0.85, 0.22, 0.15);
    let wood = Color::new(0.55, 0.36, 0.2);

    Scene {
        cubes: vec![
            Cube::new(Vec3::new(-4.0, -0.5, -4.0), Vec3::new(4.0, 0.0, 4.0), grass),
            Cube::new(Vec3::new(-1.5, 0.0, -1.5), Vec3::new(1.5, 0.4, 1.5), stone),
            // Torii: two pillars and a crossbeam
            Cube::new(Vec3::new(-1.2, 0.4, 0.8), Vec3::new(-0.9, 2.4, 1.1), vermilion),
            Cube::new(Vec3::new(0.9, 0.4, 0.8), Vec3::new(1.2, 2.4, 1.1), vermilion),
            Cube::new(Vec3::new(-1.6, 2.4, 0.7), Vec3::new(1.6, 2.7, 1.2), vermilion),
            // Block behind the gate
            Cube::new(Vec3::new(-0.5, 0.4, -1.2), Vec3::new(0.5, 1.4, -0.2), wood),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearer_cube_wins() {
        let red = Color::new(1.0, 0.0, 0.0);
        let blue = Color::new(0.0, 0.0, 1.0);
        // Far cube listed FIRST, so the order in the list can't be what decides.
        let scene = Scene {
            cubes: vec![
                Cube::new(Vec3::new(-1.0, -1.0, -6.0), Vec3::new(1.0, 1.0, -4.0), blue),
                Cube::new(Vec3::new(-1.0, -1.0, -2.0), Vec3::new(1.0, 1.0, 0.0), red),
            ],
        };
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = scene.closest_hit(&ray, 0.0, f32::INFINITY).expect("should hit");
        assert_eq!(hit.color, red);
        assert!((hit.t - 5.0).abs() < 1e-5);
    }
}
