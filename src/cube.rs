use crate::ray::Ray;
use crate::vec3::Vec3;

/// An axis-aligned box: every face is parallel to the X, Y or Z axis.
/// It is described by its lowest corner `min` and its highest corner `max`.
pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    /// Index into `Scene::materials`.
    pub material: usize,
}

/// Hit record: everything we need to know about where a ray hit a surface.
pub struct Hit {
    /// Distance along the ray.
    pub t: f32,
    pub point: Vec3,
    /// Unit vector pointing straight out of the face that was hit.
    pub normal: Vec3,
    pub material: usize,
}

impl Cube {
    pub fn new(min: Vec3, max: Vec3, material: usize) -> Cube {
        Cube { min, max, material }
    }

    /// True if `point` is inside the box (or on its surface).
    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x
            && point.y >= self.min.y && point.y <= self.max.y
            && point.z >= self.min.z && point.z <= self.max.z
    }

    /// Slab method. A box is the overlap of three "slabs" (the space between two
    /// parallel planes) along X, Y and Z. For each slab we find the t where the
    /// ray enters and leaves it. The ray is inside the box only while it is inside
    /// all three slabs at once: from the LATEST entry to the EARLIEST exit.
    ///
    /// Returns the hit with the smallest t between `t_min` and `t_max`, or None.
    pub fn intersect(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        let origin = ray.origin.to_array();
        let direction = ray.direction.to_array();
        let min = self.min.to_array();
        let max = self.max.to_array();

        let mut t_enter = f32::NEG_INFINITY;
        let mut t_exit = f32::INFINITY;
        let mut enter_axis = 0;
        let mut exit_axis = 0;

        for axis in 0..3 {
            if direction[axis] == 0.0 {
                // Ray is parallel to this slab: it is either always inside it or never.
                if origin[axis] < min[axis] || origin[axis] > max[axis] {
                    return None;
                }
                continue;
            }

            // Solve origin + t * direction = plane, for both planes of this slab.
            let t_at_min_plane = (min[axis] - origin[axis]) / direction[axis];
            let t_at_max_plane = (max[axis] - origin[axis]) / direction[axis];
            // Moving in the negative direction reaches the max plane first, so sort.
            let t_near = t_at_min_plane.min(t_at_max_plane);
            let t_far = t_at_min_plane.max(t_at_max_plane);

            if t_near > t_enter {
                t_enter = t_near;
                enter_axis = axis;
            }
            if t_far < t_exit {
                t_exit = t_far;
                exit_axis = axis;
            }
        }

        // Left some slab before entering another one: the ray misses the box.
        if t_enter > t_exit {
            return None;
        }

        // Normally the visible hit is where the ray enters. If the entry is behind
        // the ray's start, the ray started INSIDE the box, so the hit is the exit.
        let (t, axis) = if t_enter > t_min {
            (t_enter, enter_axis)
        } else {
            (t_exit, exit_axis)
        };
        if t <= t_min || t >= t_max {
            return None;
        }

        let point = ray.at(t);
        Some(Hit {
            t,
            point,
            normal: self.outward_normal(point, axis),
            material: self.material,
        })
    }

    /// The face on `axis` that `point` lies on points +1 if it is the max side
    /// of the box and -1 if it is the min side. Always points OUT of the box.
    fn outward_normal(&self, point: Vec3, axis: usize) -> Vec3 {
        let center = ((self.min + self.max) * 0.5).to_array();
        let mut normal = [0.0; 3];
        normal[axis] = if point.to_array()[axis] > center[axis] { 1.0 } else { -1.0 };
        Vec3::from_array(normal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2×2×2 cube centered at the origin.
    fn unit_cube() -> Cube {
        Cube::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0), 0)
    }

    #[test]
    fn hits_front_face_from_outside() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = unit_cube().intersect(&ray, 0.0, f32::INFINITY).expect("should hit");
        assert!((hit.t - 4.0).abs() < 1e-5); // from z = 5 to z = 1
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn hits_top_face_from_above() {
        let ray = Ray::new(Vec3::new(0.2, 3.0, 0.3), Vec3::new(0.0, -1.0, 0.0));
        let hit = unit_cube().intersect(&ray, 0.0, f32::INFINITY).expect("should hit");
        assert!((hit.t - 2.0).abs() < 1e-5);
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn parallel_ray_outside_misses() {
        // Travels along X at height y = 2, above the cube's top (y = 1).
        let ray = Ray::new(Vec3::new(-5.0, 2.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        assert!(unit_cube().intersect(&ray, 0.0, f32::INFINITY).is_none());
    }

    #[test]
    fn cube_behind_the_ray_misses() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(unit_cube().intersect(&ray, 0.0, f32::INFINITY).is_none());
    }

    #[test]
    fn ray_starting_inside_hits_exit_face() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = unit_cube().intersect(&ray, 0.0, f32::INFINITY).expect("should hit");
        assert!((hit.t - 1.0).abs() < 1e-5);
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, -1.0)); // still points OUT of the box
    }
}
