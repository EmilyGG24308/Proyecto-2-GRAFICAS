use crate::ray::Ray;
use crate::vec3::Vec3;

/// A perspective camera
pub struct Camera {
    pub position: Vec3,
    /// The point it looks at.
    pub target: Vec3,
    /// Vertical FOV in degrees. Smaller = zoomed in.
    pub vertical_fov_degrees: f32,
}

impl Camera {
    /// Builds the ray that goes from the camera through the center of pixel (x, y).
    /// (0, 0) is the top-left pixel of an image `width` × `height` pixels.
    pub fn ray_for_pixel(&self, x: usize, y: usize, width: usize, height: usize) -> Ray {
        
        let forward = (self.target - self.position).normalized();
        let right = forward.cross(Vec3::UP).normalized();
        let up = right.cross(forward);

        // Pixel center -> screen coordinates from -1 to +1. The y line is flipped: pixel rows grow downward, but +Y is up.
        let screen_x = (x as f32 + 0.5) / width as f32 * 2.0 - 1.0;
        let screen_y = 1.0 - (y as f32 + 0.5) / height as f32 * 2.0;

        // How wide the view is at distance 1
        let aspect_ratio = width as f32 / height as f32;
        let half_height = (self.vertical_fov_degrees.to_radians() / 2.0).tan();
        let half_width = half_height * aspect_ratio;

        // Step forward 1 unit, then sideways and up to reach the pixel.
        let direction =
            forward + right * (screen_x * half_width) + up * (screen_y * half_height);
        Ray::new(self.position, direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_camera() -> Camera {
        Camera {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::new(0.0, 0.0, 0.0),
            vertical_fov_degrees: 60.0,
        }
    }

    #[test]
    fn center_pixel_looks_straight_ahead() {
        // With an even size there is no single center pixel, so use 3×3.
        let ray = test_camera().ray_for_pixel(1, 1, 3, 3);
        assert!((ray.direction.z - -1.0).abs() < 1e-6);
    }

    #[test]
    fn top_left_pixel_points_up_and_left() {
        let ray = test_camera().ray_for_pixel(0, 0, 320, 180);
        println!("top-left ray direction = {:?}", ray.direction);
        assert!(ray.direction.x < 0.0, "should point left");
        assert!(ray.direction.y > 0.0, "should point up");
        assert!(ray.direction.z < 0.0, "should point toward the target");
        assert!((ray.direction.length() - 1.0).abs() < 1e-6, "should be unit length");
    }
}
