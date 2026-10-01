//! Development tool, NOT part of the renderer: paints the skybox images into
//! assets/textures/sky_*.bmp (day, Scene 1) and night_*.bmp (Scene 2).
//! The renderer only reads those BMP files.
//!
//!     cargo run --release --example make_skybox                  (only writes missing files)
//!     cargo run --release --example make_skybox -- --overwrite   (repaints all twelve)
//!
//! How: for every pixel of every face we work out which direction in the world
//! it shows (the same cube-map math the renderer uses, from src/cubemap.rs) and
//! ask `sky_at(direction)` what the sky looks like that way. Since every
//! direction has exactly one answer, the six faces meet without seams.
//!
//! The look: a soft pastel afternoon, pixel-art style: fluffy clouds, the sun
//! (in the SAME direction as the scene's sunlight), misty lavender mountains
//! on the horizon, and a sea of clouds below, since the diorama floats.
//! Colors are snapped to a limited palette with an ordered-dither pattern,
//! like old pixel-art games, so the sky matches the blocky world.

#[path = "../src/cubemap.rs"]
mod cubemap;

use std::fs;
use std::io::Write;
use std::path::Path;

const FACE_SIZE: usize = 256;
/// Must match the sun in src/shrine.rs so the painted sun and the shadows agree.
const SUN_DIRECTION: [f32; 3] = [0.35, 1.0, 0.8];
/// Must match the moon in src/spirit.rs for the same reason.
const MOON_DIRECTION: [f32; 3] = [-0.4, 0.55, -0.75];
/// Levels per color channel after snapping (fewer = more "pixel art").
const COLOR_LEVELS: f32 = 28.0;

type Rgb = [f32; 3];

fn main() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("textures");
    fs::create_dir_all(&folder).expect("could not create assets/textures");
    let overwrite = std::env::args().any(|argument| argument == "--overwrite");
    let sun = normalize(SUN_DIRECTION);
    let moon = normalize(MOON_DIRECTION);

    // Safety check: the painter's mapping and the renderer's must agree, or the
    // sky would appear on the wrong faces.
    for face in 0..6 {
        let (x, y, z) = cubemap::direction_for(face, 0.25, 0.75);
        let (back_face, u, v) = cubemap::face_and_uv(x, y, z);
        assert!(back_face == face && (u - 0.25).abs() < 1e-5 && (v - 0.75).abs() < 1e-5);
    }

    let day = |d: Rgb| sky_at(d, sun);
    let night = |d: Rgb| night_sky_at(d, moon);
    let skies: [(&str, &dyn Fn(Rgb) -> Rgb); 2] = [("sky", &day), ("night", &night)];

    for (sky_name, paint) in skies {
        for (face, suffix) in cubemap::FACE_SUFFIXES.iter().enumerate() {
            let path = folder.join(format!("{sky_name}_{suffix}.bmp"));
            if path.exists() && !overwrite {
                println!("kept  {} (already exists)", path.display());
                continue;
            }
            let mut pixels = Vec::with_capacity(FACE_SIZE * FACE_SIZE);
            for y in 0..FACE_SIZE {
                for x in 0..FACE_SIZE {
                    // Center of this pixel on the face, then the world direction it shows.
                    let u = (x as f32 + 0.5) / FACE_SIZE as f32;
                    let v = (y as f32 + 0.5) / FACE_SIZE as f32;
                    let (dx, dy, dz) = cubemap::direction_for(face, u, v);
                    pixels.push(snap_to_palette(paint(normalize([dx, dy, dz])), x, y));
                }
            }
            write_bmp(&path, FACE_SIZE, &pixels).expect("could not write skybox face");
            println!("wrote {}", path.display());
        }
    }
}

/// The night sky over the spirit world: deep indigo fading to a magenta glow
/// at the horizon, stars, a big pale moon (in the SAME direction as the
/// scene's moonlight), thin purple clouds, dark hills dotted with tiny warm
/// lantern lights, and a dark sea of clouds below.
fn night_sky_at(d: Rgb, moon: Rgb) -> Rgb {
    let [x, y, z] = d;

    let horizon = [0.46, 0.18, 0.42];
    let middle = [0.17, 0.08, 0.3];
    let zenith = [0.05, 0.04, 0.15];
    let mut color = if y >= 0.0 {
        let t = y.powf(0.45);
        if t < 0.5 { mix(horizon, middle, t * 2.0) } else { mix(middle, zenith, t * 2.0 - 1.0) }
    } else {
        let bumps = fbm(x / (0.2 - y) * 3.0, z / (0.2 - y) * 3.0, 17);
        mix(mix([0.3, 0.13, 0.32], [0.1, 0.06, 0.18], (-y).powf(0.5)), [0.36, 0.2, 0.4], smoothstep(0.55, 0.75, bumps) * 0.5)
    };

    // Stars: chop the sky into tiny cells by direction, and light up the rare
    // cell whose random number is high. Fewer near the bright horizon.
    if y > 0.03 {
        let cell = ((x * 180.0).floor() as i32, (y * 180.0).floor() as i32);
        let star = hash(cell, (z * 180.0).floor() as i32 as u32 ^ 77);
        if star > 0.994 {
            let brightness = 0.5 + (star - 0.994) / 0.006 * 0.5;
            color = mix(color, [1.0, 0.97, 0.9], brightness * smoothstep(0.03, 0.25, y));
        }
    }

    // Thin clouds, faintly lit by the moon.
    if y > 0.0 {
        let ceiling = y + 0.15;
        let density = fbm(x / ceiling * 1.1 + 40.0, z / ceiling * 2.2, 11);
        let amount = smoothstep(0.55, 0.7, density) * smoothstep(0.0, 0.15, y) * 0.55;
        let near_moon = dot(d, moon).max(0.0).powf(6.0);
        color = mix(color, mix([0.32, 0.18, 0.42], [0.75, 0.62, 0.85], near_moon), amount);
    }

    // The moon: a soft halo, then the disc with a few darker patches.
    let toward_moon = dot(d, moon).max(0.0);
    color = add(color, scale([0.55, 0.45, 0.75], toward_moon.powf(40.0) * 0.4));
    if toward_moon > 0.9975 {
        let patches = fbm(x * 90.0, y * 90.0, 23);
        color = mix([1.0, 0.96, 0.86], [0.82, 0.78, 0.74], smoothstep(0.5, 0.7, patches));
    }

    // Dark hills on the horizon, with tiny warm lights from far-off lanterns.
    let angle = x.atan2(-z);
    let (ring_x, ring_z) = (angle.cos() * 2.5 + 50.0, angle.sin() * 2.5 + 50.0);
    let ridge = 0.02 + 0.16 * smoothstep(0.3, 0.72, fbm(ring_x, ring_z, 13));
    if y < ridge && y > -0.06 {
        color = mix([0.07, 0.04, 0.12], [0.14, 0.07, 0.2], smoothstep(-0.06, ridge, y));
        if hash(((angle * 260.0) as i32, (y * 260.0) as i32), 31) > 0.988 {
            color = [1.0, 0.62, 0.3];
        }
    }
    color
}

/// The color of the sky looking in unit direction `d`.
fn sky_at(d: Rgb, sun: Rgb) -> Rgb {
    let [x, y, z] = d;

    // 1. Base: pink at the horizon, through lavender-blue, to blue overhead.
    //    Below the horizon: a soft sea of clouds.
    let horizon = [1.0, 0.84, 0.88];
    let middle = [0.74, 0.8, 0.97];
    let zenith = [0.47, 0.65, 0.94];
    let mut color = if y >= 0.0 {
        let t = y.powf(0.55);
        if t < 0.5 { mix(horizon, middle, t * 2.0) } else { mix(middle, zenith, t * 2.0 - 1.0) }
    } else {
        let bumps = fbm(x / (0.2 - y) * 3.0, z / (0.2 - y) * 3.0, 7);
        let depth = (-y).powf(0.5);
        mix(mix([1.0, 0.9, 0.93], [0.82, 0.78, 0.93], depth), [1.0, 0.97, 0.98], smoothstep(0.55, 0.75, bumps) * 0.6)
    };

    // 2. The sun: a wide warm glow plus a small bright disc.
    let toward_sun = dot(d, sun).max(0.0);
    color = add(color, scale([1.0, 0.85, 0.7], toward_sun.powf(24.0) * 0.35));
    if toward_sun > 0.9985 {
        color = [1.0, 0.98, 0.9];
    }

    // 3. Clouds, above the horizon. Project the direction onto a flat "cloud
    //    ceiling" (divide by height) so clouds shrink toward the horizon.
    if y > 0.0 {
        let ceiling = y + 0.12;
        let (cx, cz) = (x / ceiling * 1.3, z / ceiling * 1.3);
        let density = fbm(cx, cz, 1);
        let amount = smoothstep(0.5, 0.64, density) * smoothstep(0.0, 0.1, y);
        // Brighter tops, lavender undersides, a pink tint near the horizon.
        let lit = smoothstep(0.5, 0.8, density);
        let cloud = mix(mix([0.86, 0.82, 0.94], [1.0, 0.98, 0.99], lit), [1.0, 0.88, 0.92], (1.0 - y).powf(4.0) * 0.5);
        color = mix(color, cloud, amount);
    }

    // 4. Distant mountains around the horizon, drawn in front of the sky:
    //    a far, pale range and a nearer, darker one dotted with pink trees.
    //    The angle around the horizon is fed to the noise as a point on a
    //    circle, so the ridge line joins up seamlessly all the way round.
    let angle = x.atan2(-z);
    let (ring_x, ring_z) = (angle.cos() * 2.5 + 20.0, angle.sin() * 2.5 + 20.0);
    // Heights are in "direction y" units: 0.1 is about 6° above the horizon.
    let far_ridge = 0.04 + 0.26 * smoothstep(0.3, 0.72, fbm(ring_x, ring_z, 3));
    let near_ridge = 0.01 + 0.13 * smoothstep(0.35, 0.72, fbm(ring_x * 1.7, ring_z * 1.7, 5));
    if y < near_ridge && y > -0.06 {
        let haze = smoothstep(-0.06, near_ridge, y);
        color = mix([0.58, 0.54, 0.74], [0.7, 0.66, 0.84], haze);
        if hash(((angle * 300.0) as i32, (y * 300.0) as i32), 9) > 0.93 {
            color = [0.95, 0.66, 0.8]; // sakura on the far hills
        }
    } else if y < far_ridge && y > -0.06 {
        color = mix([0.76, 0.72, 0.9], [0.86, 0.82, 0.95], smoothstep(-0.06, far_ridge, y));
    }
    color
}

/// Snaps a color to COLOR_LEVELS steps per channel. The 4×4 "Bayer" pattern
/// nudges each pixel's rounding a little differently, so smooth gradients
/// become a fine checker of two neighboring colors instead of hard bands.
fn snap_to_palette(color: Rgb, x: usize, y: usize) -> [u8; 3] {
    const BAYER: [[f32; 4]; 4] = [[0.0, 8.0, 2.0, 10.0], [12.0, 4.0, 14.0, 6.0], [3.0, 11.0, 1.0, 9.0], [15.0, 7.0, 13.0, 5.0]];
    let nudge = BAYER[y % 4][x % 4] / 16.0;
    color.map(|channel| {
        let snapped = (channel.clamp(0.0, 1.0) * COLOR_LEVELS + nudge).floor() / COLOR_LEVELS;
        (snapped.min(1.0) * 255.0) as u8
    })
}

// ---------- Noise: smooth "random" patterns for clouds and mountains ----------

/// A repeatable pseudo-random number 0..1 for a grid point.
fn hash((ix, iy): (i32, i32), seed: u32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(0x8da6_b343) ^ (iy as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Value noise: random numbers on a grid, smoothly blended in between.
fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy)); // smooth, no creases
    let top = lerp(hash((ix, iy), seed), hash((ix + 1, iy), seed), sx);
    let bottom = lerp(hash((ix, iy + 1), seed), hash((ix + 1, iy + 1), seed), sx);
    lerp(top, bottom, sy)
}

/// "Fractal" noise: big soft shapes plus smaller and smaller details on top.
fn fbm(x: f32, y: f32, seed: u32) -> f32 {
    let (mut total, mut amplitude, mut frequency, mut weight) = (0.0, 0.5, 1.0, 0.0);
    for octave in 0..5 {
        total += value_noise(x * frequency, y * frequency, seed + octave) * amplitude;
        weight += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    total / weight
}

// ---------- Small helpers ----------

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}
fn add(a: Rgb, b: Rgb) -> Rgb {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: Rgb, s: f32) -> Rgb {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: Rgb, b: Rgb) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn normalize(a: Rgb) -> Rgb {
    scale(a, 1.0 / dot(a, a).sqrt())
}
/// 0 below `edge0`, 1 above `edge1`, a smooth S-curve in between.
fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 24-bit BMP, same layout as Framebuffer::save_bmp (square image, rows bottom-up).
fn write_bmp(path: &Path, size: usize, pixels: &[[u8; 3]]) -> std::io::Result<()> {
    let row_bytes = size * 3; // 768 for 256 pixels: already a multiple of 4
    let pixel_bytes = (row_bytes * size) as u32;
    let mut bytes = Vec::with_capacity(54 + pixel_bytes as usize);
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&(54 + pixel_bytes).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&54u32.to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&(size as i32).to_le_bytes());
    bytes.extend_from_slice(&(size as i32).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&24u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&pixel_bytes.to_le_bytes());
    bytes.extend_from_slice(&2835i32.to_le_bytes());
    bytes.extend_from_slice(&2835i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    for y in (0..size).rev() {
        for x in 0..size {
            let [r, g, b] = pixels[y * size + x];
            bytes.extend_from_slice(&[b, g, r]);
        }
    }
    fs::File::create(path)?.write_all(&bytes)
}
