//! Game-style controls: fly around freely, zoom, and spin the diorama.
//!
//! Four different things, on purpose kept separate:
//! - MOVING the camera (W A S D, Space, Shift): walking/flying through the world.
//! - LOOKING around (mouse): turning the camera's head, it stays where it is.
//! - ZOOMING (mouse wheel, + / -): changing the field of view, like a camera
//!   lens. The camera doesn't move; the picture gets narrower and bigger.
//! - TURNING THE DIORAMA (Q / E): the scene itself spins on its turntable,
//!   while the camera, sun and sky stay put. (See Scene for how.)

use crate::camera::Camera;
use crate::platform::{Window, key, WHEEL_NOTCH};
use crate::vec3::Vec3;

const MOVE_SPEED: f32 = 12.0; // blocks per second
const LOOK_SENSITIVITY: f32 = 0.0025; // radians per mouse pixel
const TURN_SPEED: f32 = 1.0; // diorama turn, radians per second (~57°/s)
/// Zoom limits: 10° is a strong telephoto, 90° a wide angle.
const MIN_FOV: f32 = 10.0;
const MAX_FOV: f32 = 90.0;
/// Each wheel notch narrows (or widens) the view by 10%.
const ZOOM_PER_NOTCH: f32 = 0.9;
/// The camera may not fly further than this from the diorama's center,
/// nor below its base, so you can't get lost.
const MAX_DISTANCE: f32 = 120.0;
const MIN_HEIGHT: f32 = -5.0;

pub struct Controls {
    pub position: Vec3,
    /// Left/right look angle. 0 = looking toward -Z; positive = turned right.
    yaw: f32,
    /// Up/down look angle. 0 = level; positive = looking up.
    pitch: f32,
    pub fov_degrees: f32,
    pub diorama_angle: f32,
    /// The starting view, for the reset key.
    start: (Vec3, f32, f32, f32),
}

impl Controls {
    /// Starts at `position` looking toward `target`.
    pub fn new(position: Vec3, target: Vec3, fov_degrees: f32) -> Controls {
        let direction = (target - position).normalized();
        let pitch = direction.y.asin();
        let yaw = direction.x.atan2(-direction.z);
        Controls {
            position,
            yaw,
            pitch,
            fov_degrees,
            diorama_angle: 0.0,
            start: (position, yaw, pitch, fov_degrees),
        }
    }

    /// Reads the input for one frame. `seconds` = time since the last frame,
    /// so movement speed is the same no matter how many FPS we get.
    pub fn update(&mut self, window: &Window, seconds: f32) {
        // Look around with the mouse (only while it's captured).
        let (mouse_x, mouse_y) = window.mouse_delta();
        self.yaw += mouse_x as f32 * LOOK_SENSITIVITY;
        self.pitch -= mouse_y as f32 * LOOK_SENSITIVITY; // mouse down = look down
        // Stop just short of straight up/down, where "right" would be undefined.
        self.pitch = self.pitch.clamp(-1.55, 1.55);

        // Move along the ground plane: forward follows yaw but ignores pitch,
        // so looking down while pressing W doesn't dive into the grass.
        let forward = Vec3::new(self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin());
        let mut movement = Vec3::new(0.0, 0.0, 0.0);
        let mut add_if_down = |key_code, direction: Vec3| {
            if window.is_key_down(key_code) {
                movement = movement + direction;
            }
        };
        add_if_down(key::W, forward);
        add_if_down(key::S, -forward);
        add_if_down(key::D, right);
        add_if_down(key::A, -right);
        add_if_down(key::SPACE, Vec3::UP);
        add_if_down(key::SHIFT, -Vec3::UP);
        if movement.length() > 0.0 {
            // normalized: moving diagonally isn't faster than moving straight
            self.position = self.position + movement.normalized() * (MOVE_SPEED * seconds);
        }
        self.keep_camera_near_the_diorama();

        // Zoom: the wheel changes the field of view; + and - do it smoothly.
        let notches = window.wheel_delta() as f32 / WHEEL_NOTCH as f32;
        self.fov_degrees *= ZOOM_PER_NOTCH.powf(notches);
        if window.is_key_down(key::PLUS) || window.is_key_down(key::NUMPAD_PLUS) {
            self.fov_degrees *= ZOOM_PER_NOTCH.powf(3.0 * seconds);
        }
        if window.is_key_down(key::MINUS) || window.is_key_down(key::NUMPAD_MINUS) {
            self.fov_degrees /= ZOOM_PER_NOTCH.powf(3.0 * seconds);
        }
        self.fov_degrees = self.fov_degrees.clamp(MIN_FOV, MAX_FOV);

        // Turn the diorama itself.
        if window.is_key_down(key::Q) {
            self.diorama_angle += TURN_SPEED * seconds;
        }
        if window.is_key_down(key::E) {
            self.diorama_angle -= TURN_SPEED * seconds;
        }

        if window.was_key_pressed(key::R) {
            let (position, yaw, pitch, fov) = self.start;
            (self.position, self.yaw, self.pitch, self.fov_degrees) = (position, yaw, pitch, fov);
            self.diorama_angle = 0.0;
        }
    }

    fn keep_camera_near_the_diorama(&mut self) {
        let from_center = self.position; // the diorama is centered on the origin
        if from_center.length() > MAX_DISTANCE {
            self.position = from_center.normalized() * MAX_DISTANCE;
        }
        self.position.y = self.position.y.max(MIN_HEIGHT);
    }

    /// The camera for this frame: where we are, and a point 1 unit ahead.
    pub fn camera(&self) -> Camera {
        let looking = Vec3::new(
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            -self.pitch.cos() * self.yaw.cos(),
        );
        Camera { position: self.position, target: self.position + looking, vertical_fov_degrees: self.fov_degrees }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_view_looks_at_the_requested_target() {
        let target = Vec3::new(0.0, 9.0, -2.0);
        let controls = Controls::new(Vec3::new(44.0, 40.0, 50.0), target, 40.0);
        let camera = controls.camera();
        let wanted = (target - camera.position).normalized();
        let looking = (camera.target - camera.position).normalized();
        assert!((looking - wanted).length() < 1e-4);
    }
}
