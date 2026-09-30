use crate::color::Color;
use std::path::Path;

/// An image used to color a surface. Pixel (0, 0) is the TOP-LEFT.
pub struct Texture {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

impl Texture {
    /// Reads an uncompressed 24- or 32-bit BMP file (what MS Paint saves).
    /// This is the reverse of Framebuffer::save_bmp.
    pub fn load_bmp(path: &Path) -> Result<Texture, String> {
        let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        if bytes.len() < 54 || &bytes[0..2] != b"BM" {
            return Err(format!("{}: not a BMP file", path.display()));
        }
        let read_u16 = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let read_u32 = |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);

        let pixel_start = read_u32(10) as usize;
        let width = read_u32(18) as i32;
        let signed_height = read_u32(22) as i32; // positive = rows stored bottom-up
        let bits_per_pixel = read_u16(28);
        let compression = read_u32(30);

        // 0 = no compression. 3 ("bitfields") is how 32-bit files mark standard BGRA order.
        let supported = (bits_per_pixel == 24 && compression == 0)
            || (bits_per_pixel == 32 && (compression == 0 || compression == 3));
        if !supported || width <= 0 || signed_height == 0 {
            return Err(format!(
                "{}: only uncompressed 24/32-bit BMPs are supported (got {bits_per_pixel}-bit, compression {compression})",
                path.display()
            ));
        }

        let (width, height) = (width as usize, signed_height.unsigned_abs() as usize);
        let bytes_per_pixel = bits_per_pixel as usize / 8;
        let row_size = (width * bytes_per_pixel + 3) & !3; // rows are padded to 4 bytes
        if bytes.len() < pixel_start + row_size * height {
            return Err(format!("{}: file is cut short", path.display()));
        }

        let mut pixels = vec![Color::BLACK; width * height];
        for stored_row in 0..height {
            let y = if signed_height > 0 { height - 1 - stored_row } else { stored_row };
            let row_start = pixel_start + stored_row * row_size;
            for x in 0..width {
                let at = row_start + x * bytes_per_pixel;
                let [b, g, r] = [bytes[at], bytes[at + 1], bytes[at + 2]]; // BMP stores B, G, R
                pixels[y * width + x] = Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
            }
        }
        Ok(Texture { width, height, pixels })
    }

    /// Stand-in used when a file is missing: a loud magenta/black checkerboard,
    /// so the mistake is obvious on screen instead of crashing the program.
    pub fn missing() -> Texture {
        let pixels = (0..4)
            .map(|i| if (i % 2) ^ (i / 2) == 0 { Color::new(1.0, 0.0, 1.0) } else { Color::BLACK })
            .collect();
        Texture { width: 2, height: 2, pixels }
    }

    /// Color at texture coordinates (u, v): u = 0 left → 1 right, v = 0 top → 1 bottom.
    /// "Nearest" sampling: we take the single texel the point falls in, with no
    /// blending, which keeps pixel art crisp. Clamping keeps u or v of exactly
    /// 1.0 (the far edge) inside the image.
    pub fn sample(&self, u: f32, v: f32) -> Color {
        let x = ((u * self.width as f32) as usize).min(self.width - 1);
        let y = ((v * self.height as f32) as usize).min(self.height - 1);
        self.pixels[y * self.width + x]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn bmp_round_trip_keeps_pixels_and_orientation() {
        // Save a 3×2 image with our Phase 1 writer, then read it back.
        let mut framebuffer = Framebuffer::new(3, 2);
        let red = Color::new(1.0, 0.0, 0.0);
        let blue = Color::new(0.0, 0.0, 1.0);
        for (y, row) in framebuffer.rows_mut() {
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = if (x, y) == (0, 0) { red } else { blue };
            }
        }
        let path = std::env::temp_dir().join("mikage_round_trip_test.bmp");
        framebuffer.save_bmp(&path).unwrap();
        let texture = Texture::load_bmp(&path).unwrap();

        assert_eq!((texture.width, texture.height), (3, 2));
        assert_eq!(texture.sample(0.1, 0.1), red); // top-left stayed top-left
        assert_eq!(texture.sample(0.9, 0.9), blue);
    }

    #[test]
    fn sampling_picks_the_right_texel_and_clamps_edges() {
        let texture = Texture::missing(); // 2×2: magenta, black / black, magenta
        assert_eq!(texture.sample(0.25, 0.25), Color::new(1.0, 0.0, 1.0));
        assert_eq!(texture.sample(0.75, 0.25), Color::BLACK);
        assert_eq!(texture.sample(1.0, 1.0), Color::new(1.0, 0.0, 1.0)); // clamped to the last texel
    }
}
