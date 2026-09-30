mod camera;
mod color;
mod framebuffer;
mod platform;
mod ray;
mod vec3;

use camera::Camera;
use color::Color;
use framebuffer::Framebuffer;
use platform::{Window, key};
use ray::Ray;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use vec3::Vec3;

/// Resolution the raytracer renders at. Small on purpose: fewer rays per frame + scaling gives chunky
const RENDER_WIDTH: usize = 320;
const RENDER_HEIGHT: usize = 180;
/// Each rendered pixel becomes a PIXEL_SCALE × PIXEL_SCALE block on screen.
const PIXEL_SCALE: usize = 4;

const WINDOW_TITLE: &str = "Mikage Shrine Raytracer";

fn main() {
    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);
    let camera = Camera {
        position: Vec3::new(0.0, 2.0, 6.0),
        target: Vec3::new(0.0, 0.5, 0.0),
        vertical_fov_degrees: 60.0,
    };

    // `cargo run --release -- --snapshot` renders one frame to a file and exits,
    // without opening a window. Handy for checking output and for the README.
    if std::env::args().any(|argument| argument == "--snapshot") {
        render_frame(&mut framebuffer, &camera);
        save_screenshot(&framebuffer);
        return;
    }

    let mut window = match Window::open(
        WINDOW_TITLE,
        (RENDER_WIDTH * PIXEL_SCALE) as i32,
        (RENDER_HEIGHT * PIXEL_SCALE) as i32,
    ) {
        Ok(window) => window,
        Err(error) => {
            eprintln!("Could not open the window: {error}");
            std::process::exit(1);
        }
    };

    let mut frames_since_title_update = 0;
    let mut last_title_update = Instant::now();

    while window.process_events() {
        if window.was_key_pressed(key::ESCAPE) {
            break;
        }

        // 1. RENDER: compute every pixel's color into our own memory.
        render_frame(&mut framebuffer, &camera);

        // 2. DISPLAY: hand the finished image to Windows to show.
        window.present(&framebuffer);

        if window.was_key_pressed(key::F12) {
            save_screenshot(&framebuffer);
        }

        // Show frames per second in the title bar
        frames_since_title_update += 1;
        let elapsed = last_title_update.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            let fps = frames_since_title_update as f32 / elapsed;
            window.set_title(&format!("{WINDOW_TITLE} — {fps:.0} FPS"));
            frames_since_title_update = 0;
            last_title_update = Instant::now();
        }
    }
}

/// Shoots one ray per pixel and stores the color it sees.
fn render_frame(framebuffer: &mut Framebuffer, camera: &Camera) {
    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let ray = camera.ray_for_pixel(x, y, framebuffer.width, framebuffer.height);
            framebuffer.set_pixel(x, y, shade_ray(&ray));
        }
    }
}

/// Temp scene 
fn shade_ray(ray: &Ray) -> Color {
    // Ground plane y = 0. Solve origin.y + t * direction.y = 0 for t.
    if ray.direction.y < 0.0 {
        let t = -ray.origin.y / ray.direction.y;
        let hit_point = ray.at(t);
        let is_dark_tile = (hit_point.x.floor() + hit_point.z.floor()) as i32 % 2 == 0;
        return if is_dark_tile {
            Color::new(0.35, 0.55, 0.3)
        } else {
            Color::new(0.55, 0.75, 0.45)
        };
    }

    // Sky: pink at the horizon (direction.y = 0), blue straight up (direction.y = 1).
    let horizon = Color::new(1.0, 0.8, 0.88);
    let zenith = Color::new(0.45, 0.65, 1.0);
    horizon.lerp(zenith, ray.direction.y)
}

/// Saves the framebuffer to screenshots/shot_<milliseconds>.bmp.
fn save_screenshot(framebuffer: &Framebuffer) {
    let folder = PathBuf::from("screenshots");
    if let Err(error) = std::fs::create_dir_all(&folder) {
        eprintln!("Could not create the screenshots folder: {error}");
        return;
    }
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let path = folder.join(format!("shot_{milliseconds}.bmp"));

    match framebuffer.save_bmp(&path) {
        Ok(()) => println!("Saved {}", path.display()),
        Err(error) => eprintln!("Could not save {}: {error}", path.display()),
    }
}
