mod color;
mod framebuffer;
mod platform;

use color::Color;
use framebuffer::Framebuffer;
use platform::{Window, key};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Resolution the raytracer renders at. Small on purpose: fewer rays per frame + scaling gives chunky
const RENDER_WIDTH: usize = 320;
const RENDER_HEIGHT: usize = 180;
/// Each rendered pixel becomes a PIXEL_SCALE × PIXEL_SCALE block on screen.
const PIXEL_SCALE: usize = 4;

const WINDOW_TITLE: &str = "Mikage Shrine Raytracer";

fn main() {
    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);

    // `cargo run --release -- --snapshot` renders one frame to a file and exits,
    // without opening a window. Handy for checking output and for the README.
    if std::env::args().any(|argument| argument == "--snapshot") {
        draw_test_pattern(&mut framebuffer, 0.0);
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

    let start_time = Instant::now();
    let mut frames_since_title_update = 0;
    let mut last_title_update = Instant::now();

    while window.process_events() {
        if window.was_key_pressed(key::ESCAPE) {
            break;
        }

        // 1. RENDER: compute every pixel's color into our own memory.
        let seconds = start_time.elapsed().as_secs_f32();
        draw_test_pattern(&mut framebuffer, seconds);

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

/// Fills the framebuffer with an image that makes orientation and color easy to check:
fn draw_test_pattern(framebuffer: &mut Framebuffer, seconds: f32) {
    let blue = 0.5 + 0.5 * (seconds * 2.0).sin(); // oscillates between 0 and 1

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let red = x as f32 / (framebuffer.width - 1) as f32;
            let green = y as f32 / (framebuffer.height - 1) as f32;

            let is_dark_cell = (x / 16 + y / 16) % 2 == 1;
            let shade = if is_dark_cell { 0.6 } else { 1.0 };

            let mut color = Color::new(red * shade, green * shade, blue * shade);
            if x < 8 && y < 8 {
                color = Color::WHITE;
            }
            framebuffer.set_pixel(x, y, color);
        }
    }
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
