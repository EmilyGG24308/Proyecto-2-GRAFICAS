use std::ops::{Add, Mul, Neg, Sub};

/// A 3D vector. Used for points (positions) and directions.
///   +X = right, +Y = up, -Z = "into the screen" for the default camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    /// How much two vectors point the same way.
    /// For unit vectors: 1 = same direction, 0 = perpendicular, -1 = opposite.
    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// A vector perpendicular to both inputs (right-hand rule).
    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// [x, y, z], so code can loop over the three axes (0 = X, 1 = Y, 2 = Z).
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    pub fn from_array(values: [f32; 3]) -> Vec3 {
        Vec3::new(values[0], values[1], values[2])
    }

    /// Mirrors this direction around a surface with unit normal `normal`,
    /// like a ball bouncing off a wall: r = d - 2 (d·n) n
    pub fn reflect(self, normal: Vec3) -> Vec3 {
        self - normal * (2.0 * self.dot(normal))
    }

    /// Same direction, length 1.
    pub fn normalized(self) -> Vec3 {
        self * (1.0 / self.length())
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, other: Vec3) -> Vec3 {
        Vec3::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Vec3) -> Vec3 {
        Vec3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

/// Vector × number: scales the length (a negative number also flips it).
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, scale: f32) -> Vec3 {
        Vec3::new(self.x * scale, self.y * scale, self.z * scale)
    }
}

/// -v points the opposite way.
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}
