use crate::vec3::Vec3;

/// A half-line starting at `origin` and going forever along `direction`.
/// `direction` is always kept at length 1, so `t` is a real distance.
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Ray {
        Ray { origin, direction: direction.normalized() }
    }

    /// The point reached after travelling distance `t`: P(t) = origin + t * direction.
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}
