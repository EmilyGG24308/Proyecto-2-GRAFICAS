mod animation;
mod camera;
mod color;
mod controls;
mod cube;
mod cubemap;
mod framebuffer;
mod grid;
mod material;
mod platform;
mod ray;
mod renderer;
mod scene;
mod shrine;
mod spirit;
mod texture;
mod vec3;
mod world;

use camera::Camera;
use color::Color;
use controls::Controls;
use framebuffer::Framebuffer;
use platform::{Window, key};
use renderer::{pick, render_frame};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use vec3::Vec3;

/// Live (interactive) resolution. Each rendered pixel becomes a
/// PIXEL_SCALE × PIXEL_SCALE block in the 1280×720 window.
const RENDER_WIDTH: usize = 640;
const RENDER_HEIGHT: usize = 360;
const PIXEL_SCALE: usize = 2;
/// Resolution for screenshots (F12 and --snapshot --hd): full window size.
const HD_WIDTH: usize = 1280;
const HD_HEIGHT: usize = 720;

const WINDOW_TITLE: &str = "Mikage Shrine Raytracer";
const WORLD_NAMES: [&str; 2] = ["Mikage Shrine", "Spirit World"];

/// Traveling through a portal: fade out to a soft violet, swap worlds, fade in.
const FADE_SECONDS: f32 = 0.9;
const FADE_COLOR: Color = Color::new(0.85, 0.7, 1.0);

enum Fade {
    None,
    Out { since: Instant },
    In { since: Instant },
}

fn main() {
    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);
    // Both worlds are built once at startup; the portal just switches between them.
    let mut worlds = [shrine::build_shrine_world(), spirit::build_spirit_world()];
    let block_counts = worlds.each_ref().map(|world| world.scene.blocks.block_count());
    // `--spirit` starts in the spirit world.
    let mut current = if std::env::args().any(|argument| argument == "--spirit") { 1 } else { 0 };
    // N key (or --normals) shows each face's normal as a color, to check the faces.
    let mut show_normals = std::env::args().any(|argument| argument == "--normals");

    // `cargo run --release -- --snapshot` renders one frame to a file and exits,
    // without opening a window. Handy for checking output and for the README.
    // Add `--hd` to render that frame at 1280×720 instead.
    if std::env::args().any(|argument| argument == "--snapshot") {
        let world = &mut worlds[current];
        let default_view = if current == 0 { starting_controls() } else { (world.arrival)() };
        let camera = camera_from_arguments().unwrap_or_else(|| default_view.camera());
        // `--turn=45` turns the diorama 45° first, like holding Q.
        if let Some(degrees) = std::env::args().find_map(|a| a.strip_prefix("--turn=").and_then(|d| d.parse::<f32>().ok())) {
            world.scene.set_turn_angle(degrees.to_radians());
        }
        if std::env::args().any(|argument| argument == "--hd") {
            framebuffer = Framebuffer::new(HD_WIDTH, HD_HEIGHT);
        }
        // `--time=12.5` freezes the animation at that moment (default: 0).
        let time = std::env::args().find_map(|a| a.strip_prefix("--time=").and_then(|t| t.parse::<f32>().ok()));
        world.animation.update(&mut world.scene, time.unwrap_or(0.0));
        render_frame(&mut framebuffer, &camera, &world.scene, show_normals);
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

    let mut controls = if current == 0 { starting_controls() } else { (worlds[1].arrival)() };
    let mut fade = Fade::None;
    let start_time = Instant::now();
    let mut last_frame = Instant::now();
    let mut frames_since_title_update = 0;
    let mut last_title_update = Instant::now();

    while window.process_events() {
        // Esc gives the mouse back first; pressing it again quits.
        if window.was_key_pressed(key::ESCAPE) {
            if window.is_mouse_captured() {
                window.set_mouse_captured(false);
            } else {
                break;
            }
        }
        if window.was_key_pressed(key::N) {
            show_normals = !show_normals;
        }

        // Time since the last frame, capped so a hiccup doesn't teleport you.
        let seconds = last_frame.elapsed().as_secs_f32().min(0.1);
        last_frame = Instant::now();
        let world = &mut worlds[current];
        world.animation.update(&mut world.scene, start_time.elapsed().as_secs_f32());
        controls.update(&window, seconds);
        world.scene.set_turn_angle(controls.diorama_angle);
        let camera = controls.camera();

        // Clicking: the first click captures the mouse. After that, clicking
        // "shoots" a ray through the crosshair (the center of the screen);
        // if the first thing it hits is the portal, start the journey.
        if window.left_click().is_some() {
            if !window.is_mouse_captured() {
                window.set_mouse_captured(true);
            } else if matches!(fade, Fade::None) {
                let (width, height) = (framebuffer.width, framebuffer.height);
                let clicked = pick(&camera, &world.scene, width / 2, height / 2, width, height);
                if clicked.is_some_and(|hit| hit.material == world.portal_material) {
                    fade = Fade::Out { since: Instant::now() };
                }
            }
        }

        // 1. RENDER: compute every pixel's color into our own memory.
        render_frame(&mut framebuffer, &camera, &world.scene, show_normals);

        // The portal journey: fade out, switch worlds while the screen is
        // fully violet, then fade back in at the other world's arrival point.
        let fade_amount = match fade {
            Fade::None => 0.0,
            Fade::Out { since } => {
                let progress = since.elapsed().as_secs_f32() / FADE_SECONDS;
                if progress >= 1.0 {
                    current = 1 - current;
                    controls = (worlds[current].arrival)();
                    fade = Fade::In { since: Instant::now() };
                }
                progress.min(1.0)
            }
            Fade::In { since } => {
                let progress = since.elapsed().as_secs_f32() / FADE_SECONDS;
                if progress >= 1.0 {
                    fade = Fade::None;
                }
                1.0 - progress.min(1.0)
            }
        };
        if fade_amount > 0.0 {
            // Smoothstep: starts and ends gently instead of abruptly.
            framebuffer.fade_toward(FADE_COLOR, fade_amount * fade_amount * (3.0 - 2.0 * fade_amount));
        } else if window.is_mouse_captured() {
            framebuffer.draw_crosshair();
        }

        // 2. DISPLAY: hand the finished image to Windows to show.
        window.present(&framebuffer);

        if window.was_key_pressed(key::F12) {
            // Screenshots are rendered again at full resolution: sharper than the live view.
            let mut screenshot = Framebuffer::new(HD_WIDTH, HD_HEIGHT);
            render_frame(&mut screenshot, &camera, &worlds[current].scene, show_normals);
            save_screenshot(&screenshot);
        }

        // Title bar: FPS, plus a hint while the mouse isn't captured.
        frames_since_title_update += 1;
        let elapsed = last_title_update.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            let fps = frames_since_title_update as f32 / elapsed;
            let hint = if window.is_mouse_captured() {
                "WASD move · Space/Shift up/down · wheel zoom · Q/E turn diorama · R reset · click the portal · Esc release mouse"
            } else {
                "click to look around"
            };
            let (name, blocks) = (WORLD_NAMES[current], block_counts[current]);
            window.set_title(&format!("{WINDOW_TITLE} — {name} — {fps:.0} FPS — {blocks} blocks — {hint}"));
            frames_since_title_update = 0;
            last_title_update = Instant::now();
        }
    }
}

/// The overview the program starts with (and R returns to).
fn starting_controls() -> Controls {
    // First-person POV in front of the temple, looking back toward the shrine and town
    Controls::new(Vec3::new(0.0, 18.0, 20.0), Vec3::new(0.0, 16.0, -10.0), 50.0)
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
