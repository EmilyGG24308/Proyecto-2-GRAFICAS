mod camera;
mod color;
mod cube;
mod framebuffer;
mod grid;
mod material;
mod platform;
mod ray;
mod renderer;
mod scene;
mod shrine;
mod texture;
mod vec3;

use camera::Camera;
use framebuffer::Framebuffer;
use platform::{Window, key};
use renderer::render_frame;
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
    let camera = camera_from_arguments().unwrap_or(Camera {
        position: Vec3::new(44.0, 40.0, 50.0),
        target: Vec3::new(0.0, 9.0, -2.0),
        vertical_fov_degrees: 40.0,
    });
    let scene = shrine::build_shrine_scene();
    let block_count = scene.blocks.block_count();
    // N key (or --normals) shows each face's normal as a color, to check the faces.
    let mut show_normals = std::env::args().any(|argument| argument == "--normals");

    // `cargo run --release -- --snapshot` renders one frame to a file and exits,
    // without opening a window. Handy for checking output and for the README.
    // Add `--hd` to render that frame at 1280×720 instead.
    if std::env::args().any(|argument| argument == "--snapshot") {
        if std::env::args().any(|argument| argument == "--hd") {
            framebuffer = Framebuffer::new(1280, 720);
        }
        render_frame(&mut framebuffer, &camera, &scene, show_normals);
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

        if window.was_key_pressed(key::N) {
            show_normals = !show_normals;
        }

        // 1. RENDER: compute every pixel's color into our own memory.
        render_frame(&mut framebuffer, &camera, &scene, show_normals);

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
            window.set_title(&format!("{WINDOW_TITLE} — {fps:.0} FPS — {block_count} blocks"));
            frames_since_title_update = 0;
            last_title_update = Instant::now();
        }
    }
}


/// `--camera=px,py,pz,tx,ty,tz[,fov]` places the camera at p looking at t.
/// Used to take test snapshots from specific spots.
fn camera_from_arguments() -> Option<Camera> {
    let argument = std::env::args().find_map(|a| a.strip_prefix("--camera=").map(String::from))?;
    let numbers: Vec<f32> = argument.split(',').filter_map(|part| part.trim().parse().ok()).collect();
    if numbers.len() < 6 {
        eprintln!("--camera needs at least 6 numbers: px,py,pz,tx,ty,tz[,fov]");
        return None;
    }
    Some(Camera {
        position: Vec3::new(numbers[0], numbers[1], numbers[2]),
        target: Vec3::new(numbers[3], numbers[4], numbers[5]),
        vertical_fov_degrees: numbers.get(6).copied().unwrap_or(50.0),
    })
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
