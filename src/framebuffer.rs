use crate::color::Color;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// A rectangular grid of colors: the image the raytracer draws into.
///
/// Pixels are stored row by row in one flat Vec, starting at the top-left.
/// Pixel (x, y) lives at index `y * width + x`.
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pixels: Vec<Color>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Framebuffer {
        Framebuffer {
            width,
            height,
            pixels: vec![Color::BLACK; width * height],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let index = y * self.width + x;
        self.pixels[index] = color;
    }

    /// 32-bit format Windows expects when we copy the image to the window.
    /// `output` is reused between frames to avoid allocating every frame.
    pub fn to_display_pixels(&self, output: &mut Vec<u32>) {
        output.clear();
        output.extend(self.pixels.iter().map(|color| {
            let [r, g, b] = color.to_bytes();
            (r as u32) << 16 | (g as u32) << 8 | b as u32
        }));
    }

    /// Writes the framebuffer as an uncompressed BMP file
    pub fn save_bmp(&self, path: &Path) -> io::Result<()> {
        const HEADERS_SIZE: u32 = 14 + 40;
        let row_size = (self.width * 3 + 3) & !3; // round up to a multiple of 4
        let padding = row_size - self.width * 3;
        let pixel_data_size = (row_size * self.height) as u32;

        let mut file = BufWriter::new(File::create(path)?);

        // File header
        file.write_all(b"BM")?;
        file.write_all(&(HEADERS_SIZE + pixel_data_size).to_le_bytes())?; // whole file size
        file.write_all(&0u32.to_le_bytes())?; // reserved
        file.write_all(&HEADERS_SIZE.to_le_bytes())?; // where the pixels start

        // Info header (BITMAPINFOHEADER)
        file.write_all(&40u32.to_le_bytes())?; // size of this header
        file.write_all(&(self.width as i32).to_le_bytes())?;
        file.write_all(&(self.height as i32).to_le_bytes())?; // positive = bottom-up rows
        file.write_all(&1u16.to_le_bytes())?; // color planes, always 1
        file.write_all(&24u16.to_le_bytes())?; // bits per pixel
        file.write_all(&0u32.to_le_bytes())?; // compression: none
        file.write_all(&pixel_data_size.to_le_bytes())?;
        file.write_all(&2835i32.to_le_bytes())?; // horizontal resolution (72 DPI)
        file.write_all(&2835i32.to_le_bytes())?; // vertical resolution
        file.write_all(&0u32.to_le_bytes())?; // palette colors used: none
        file.write_all(&0u32.to_le_bytes())?; // important colors: all

        // Pixel rows, bottom row first
        for y in (0..self.height).rev() {
            for x in 0..self.width {
                let [r, g, b] = self.pixels[y * self.width + x].to_bytes();
                file.write_all(&[b, g, r])?;
            }
            file.write_all(&[0u8; 3][..padding])?;
        }
        file.flush()
    }
}
