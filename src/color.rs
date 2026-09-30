
/// 0.0 means "none" and 1.0 means "full intensity".
/// While computing lighting, values may go above 1.0 (a very bright)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b }
    }

    /// Blends from `self` (t = 0) to `other` (t = 1). t = 0.5 gives the halfway color.
    pub fn lerp(self, other: Color, t: f32) -> Color {
        Color::new(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
        )
    }

    /// Converts the color to three bytes (0–255) in R, G, B order.
    pub fn to_bytes(self) -> [u8; 3] {
        [
            channel_to_byte(self.r),
            channel_to_byte(self.g),
            channel_to_byte(self.b),
        ]
    }
}

/// Maps 0.0..=1.0 to 0..=255, clamping anything outside that range.
/// The + 0.5 rounds to the nearest byte instead of always rounding down.
fn channel_to_byte(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}
