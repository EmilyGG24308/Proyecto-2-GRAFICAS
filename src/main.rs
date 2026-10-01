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

const RENDER_WIDTH: usize = 640;
const RENDER_HEIGHT: usize = 360;
const PIXEL_SCALE: usize = 2;
const HD_WIDTH: usize = 1280;
const HD_HEIGHT: usize = 720;

const WINDOW_TITLE: &str = "Mikage Shrine Raytracer";
const WORLD_NAMES: [&str; 2] = ["Mikage Shrine", "Spirit World"];

const FADE_SECONDS: f32 = 0.9;
const FADE_COLOR: Color = Color::new(0.85, 0.7, 1.0);

enum Fade {
    None,
    Out { since: Instant },
    In { since: Instant },
}

enum GameState {
    Homepage,
    Playing,
}

fn main() {
    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);
    let mut worlds = [shrine::build_shrine_world(), spirit::build_spirit_world()];
    let block_counts = worlds.each_ref().map(|world| world.scene.blocks.block_count());
    let mut current = if std::env::args().any(|argument| argument == "--spirit") { 1 } else { 0 };
    let mut show_normals = std::env::args().any(|argument| argument == "--normals");

    if std::env::args().any(|argument| argument == "--snapshot") {
        let world = &mut worlds[current];
        let default_view = if current == 0 { starting_controls() } else { (world.arrival)() };
        let camera = camera_from_arguments().unwrap_or_else(|| default_view.camera());
        if let Some(degrees) = std::env::args().find_map(|a| a.strip_prefix("--turn=").and_then(|d| d.parse::<f32>().ok())) {
            world.scene.set_turn_angle(degrees.to_radians());
        }
        if std::env::args().any(|argument| argument == "--hd") {
            framebuffer = Framebuffer::new(HD_WIDTH, HD_HEIGHT);
        }
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

    let mut game_state = GameState::Homepage;
    let mut controls = if current == 0 { starting_controls() } else { (worlds[1].arrival)() };
    let mut fade = Fade::None;
    let start_time = Instant::now();
    let mut last_frame = Instant::now();
    let mut frames_since_title_update = 0;
    let mut last_title_update = Instant::now();

    while window.process_events() {
        if window.was_key_pressed(key::ESCAPE) {
            if matches!(game_state, GameState::Playing) && window.is_mouse_captured() {
                window.set_mouse_captured(false);
            } else if !matches!(game_state, GameState::Playing) || !window.is_mouse_captured() {
                break;
            }
        }
        if window.was_key_pressed(key::N) {
            show_normals = !show_normals;
        }

        if matches!(game_state, GameState::Homepage) {
            if window.left_click().is_some() {
                game_state = GameState::Playing;
                current = 0;
                controls = starting_controls();
            }
            draw_homepage(&mut framebuffer);
            window.present(&framebuffer);
            frames_since_title_update += 1;
            let elapsed = last_title_update.elapsed().as_secs_f32();
            if elapsed >= 0.5 {
                window.set_title(&format!("{WINDOW_TITLE} — Haz clic para explorar 🏯"));
                frames_since_title_update = 0;
                last_title_update = Instant::now();
            }
            continue;
        }

        let seconds = last_frame.elapsed().as_secs_f32().min(0.1);
        last_frame = Instant::now();
        let world = &mut worlds[current];
        world.animation.update(&mut world.scene, start_time.elapsed().as_secs_f32());
        controls.update(&window, seconds);
        world.scene.set_turn_angle(controls.diorama_angle);
        let camera = controls.camera();

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

        render_frame(&mut framebuffer, &camera, &world.scene, show_normals);

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
            framebuffer.fade_toward(FADE_COLOR, fade_amount * fade_amount * (3.0 - 2.0 * fade_amount));
        } else if window.is_mouse_captured() {
            framebuffer.draw_crosshair();
        }

        window.present(&framebuffer);

        if window.was_key_pressed(key::F12) {
            let mut screenshot = Framebuffer::new(HD_WIDTH, HD_HEIGHT);
            render_frame(&mut screenshot, &camera, &worlds[current].scene, show_normals);
            save_screenshot(&screenshot);
        }

        frames_since_title_update += 1;
        let elapsed = last_title_update.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            let fps = frames_since_title_update as f32 / elapsed;
            let hint = if window.is_mouse_captured() {
                "WASD mover · Espacio/Mayús arriba/abajo · rueda zoom · Q/E girar · R reestablec · clic portal · Esc liberar"
            } else {
                "clic para mirar"
            };
            let (name, blocks) = (WORLD_NAMES[current], block_counts[current]);
            window.set_title(&format!("{WINDOW_TITLE} — {name} — {fps:.0} FPS — {blocks} bloques — {hint}"));
            frames_since_title_update = 0;
            last_title_update = Instant::now();
        }
    }
}

fn starting_controls() -> Controls {
    Controls::new(Vec3::new(0.0, 18.0, 20.0), Vec3::new(0.0, 16.0, -10.0), 50.0)
}

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

fn draw_homepage(framebuffer: &mut Framebuffer) {
    let h = framebuffer.height;
    let w = framebuffer.width;

    for (y, (_, row)) in framebuffer.rows_mut().enumerate() {
        let t = y as f32 / h as f32;
        let color = Color::new(0.88 + t * 0.08, 0.82 + t * 0.10, 0.96 - t * 0.06);
        for pixel in row {
            *pixel = color;
        }
    }

    let title = Color::new(0.75, 0.4, 0.88);
    let text = Color::new(0.5, 0.2, 0.7);
    let accent = Color::new(1.0, 0.7, 0.85);

    print_centered(framebuffer, "SANTUARIO MIKAGE", 30, 2, title);
    line_h(framebuffer, 50, 65, w - 50, accent);

    print_centered(framebuffer, "BIENVENIDO", 95, 1, text);
    print_centered(framebuffer, "EXPLORA DOS MUNDOS", 120, 1, title);

    print_centered(framebuffer, "CONTROLES", 160, 1, title);
    print_centered(framebuffer, "WASD mover  Espacio/Mayús arriba/abajo", 185, 1, text);
    print_centered(framebuffer, "Rueda zoom  Q/E girar  Esc menú", 200, 1, text);

    line_h(framebuffer, 50, 225, w - 50, accent);
    print_centered(framebuffer, "HAZ CLIC PARA EXPLORAR", 250, 2, accent);
}

fn print_centered(fb: &mut Framebuffer, text: &str, y: usize, scale: usize, color: Color) {
    let char_w = 12 * scale;
    let total_w = text.len() * char_w;
    let start_x = (fb.width - total_w) / 2;
    let mut x = start_x;
    for ch in text.chars() {
        print_char(fb, ch, x, y, scale, color);
        x += char_w;
    }
}

fn print_char(fb: &mut Framebuffer, ch: char, x: usize, y: usize, s: usize, color: Color) {
    let mut cell = |dx: usize, dy: usize| {
        let px = x + dx * s;
        let py = y + dy * s;
        if px < fb.width && py < fb.height {
            for sy in 0..s {
                for sx in 0..s {
                    if let Some((_, row)) = fb.rows_mut().nth(py + sy) {
                        if px + sx < row.len() {
                            row[px + sx] = color;
                        }
                    }
                }
            }
        }
    };

    match ch.to_ascii_uppercase() {
        'A' => { cell(1,0); cell(2,0); cell(3,0); cell(0,1); cell(4,1); cell(0,2); cell(4,2); cell(0,3); cell(1,3); cell(2,3); cell(3,3); cell(4,3); cell(0,4); cell(4,4); },
        'B' => { cell(0,0); cell(1,0); cell(2,0); cell(3,0); cell(0,1); cell(4,1); cell(0,2); cell(1,2); cell(2,2); cell(3,2); cell(0,3); cell(4,3); cell(0,4); cell(1,4); cell(2,4); cell(3,4); },
        'C' => { cell(1,0); cell(2,0); cell(3,0); cell(4,0); cell(0,1); cell(0,2); cell(0,3); cell(1,4); cell(2,4); cell(3,4); cell(4,4); },
        'D' => { cell(0,0); cell(1,0); cell(2,0); cell(3,0); cell(0,1); cell(4,1); cell(0,2); cell(4,2); cell(0,3); cell(4,3); cell(0,4); cell(1,4); cell(2,4); cell(3,4); },
        'E' => { cell(0,0); cell(1,0); cell(2,0); cell(3,0); cell(4,0); cell(0,1); cell(0,2); cell(1,2); cell(2,2); cell(3,2); cell(0,3); cell(0,4); cell(1,4); cell(2,4); cell(3,4); cell(4,4); },
        'F' => { cell(0,0); cell(1,0); cell(2,0); cell(3,0); cell(4,0); cell(0,1); cell(0,2); cell(1,2); cell(2,2); cell(3,2); cell(0,3); cell(0,4); },
        'G' => { cell(1,0); cell(2,0); cell(3,0); cell(4,0); cell(0,1); cell(0,2); cell(0,3); cell(3,3); cell(4,3); cell(1,4); cell(2,4); cell(3,4); cell(4,4); },
        'H' => { cell(0,0); cell(4,0); cell(0,1); cell(4,1); cell(0,2); cell(1,2); cell(2,2); cell(3,2); cell(4,2); cell(0,3); cell(4,3); cell(0,4); cell(4,4); },
        'I' => { cell(1,0); cell(2,0); cell(3,0); cell(2,1); cell(2,2); cell(2,3); cell(1,4); cell(2,4); cell(3,4); },
        'J' => { cell(0,0); cell(1,0); cell(2,0); cell(3,0); cell(4,0); cell(4,1); cell(4,2); cell(4,3); cell(1,4); cell(2,4); },
        'K' => { cell(0,0); cell(4,0); cell(0,1); cell(3,1); cell(0,2); cell(1,2); cell(2,2); cell(0,3); cell(3,3); cell(0,4); cell(4,4); },
        'L' => { cell(0,0); cell(0,1); cell(0,2); cell(0,3); cell(0,4); cell(1,4); cell(2,4); cell(3,4); cell(4,4); },
        'M' => { cell(0,0); cell(4,0); cell(0,1); cell(1,1); cell(3,1); cell(4,1); cell(0,2); cell(2,2); cell(4,2); cell(0,3); cell(4,3); cell(0,4); cell(4,4); },
        'N' => { cell(0,0); cell(4,0); cell(0,1); cell(1,1); cell(4,1); cell(0,2); cell(2,2); cell(4,2); cell(0,3); cell(3,3); cell(4,3); cell(0,4); cell(4,4); },
        'O' => { cell(1,0); cell(2,0); cell(3,0); cell(0,1); cell(4,1); cell(0,2); cell(4,2); cell(0,3); cell(4,3); cell(1,4); cell(2,4); cell(3,4); },
        'P' => { cell(0,0); cell(1,0); cell(2,0); cell(3,0); cell(0,1); cell(4,1); cell(0,2); cell(1,2); cell(2,2); cell(3,2); cell(0,3); cell(0,4); },
        'Q' => { cell(1,0); cell(2,0); cell(3,0); cell(0,1); cell(4,1); cell(0,2); cell(4,2); cell(0,3); cell(3,3); cell(4,3); cell(1,4); cell(2,4); cell(4,4); },
        'R' => { cell(0,0); cell(1,0); cell(2,0); cell(3,0); cell(0,1); cell(4,1); cell(0,2); cell(1,2); cell(2,2); cell(3,2); cell(0,3); cell(3,3); cell(0,4); cell(4,4); },
        'S' => { cell(1,0); cell(2,0); cell(3,0); cell(4,0); cell(0,1); cell(1,2); cell(2,2); cell(3,2); cell(4,3); cell(1,4); cell(2,4); cell(3,4); },
        'T' => { cell(0,0); cell(1,0); cell(2,0); cell(3,0); cell(4,0); cell(2,1); cell(2,2); cell(2,3); cell(2,4); },
        'U' => { cell(0,0); cell(4,0); cell(0,1); cell(4,1); cell(0,2); cell(4,2); cell(0,3); cell(4,3); cell(1,4); cell(2,4); cell(3,4); },
        'V' => { cell(0,0); cell(4,0); cell(0,1); cell(4,1); cell(0,2); cell(4,2); cell(1,3); cell(3,3); cell(2,4); },
        'W' => { cell(0,0); cell(4,0); cell(0,1); cell(4,1); cell(0,2); cell(2,2); cell(4,2); cell(0,3); cell(4,3); cell(1,4); cell(3,4); },
        'X' => { cell(0,0); cell(4,0); cell(0,1); cell(3,1); cell(1,2); cell(2,2); cell(3,3); cell(0,4); cell(4,4); },
        'Y' => { cell(0,0); cell(4,0); cell(1,1); cell(3,1); cell(2,2); cell(2,3); cell(2,4); },
        'Z' => { cell(0,0); cell(1,0); cell(2,0); cell(3,0); cell(4,0); cell(3,1); cell(2,2); cell(1,3); cell(0,4); cell(1,4); cell(2,4); cell(3,4); cell(4,4); },
        ' ' => {},
        '&' => { cell(1,0); cell(2,0); cell(0,1); cell(3,1); cell(0,2); cell(2,2); cell(3,2); cell(4,2); cell(3,3); cell(1,4); cell(4,4); },
        '/' => { cell(3,0); cell(3,1); cell(2,2); cell(1,3); cell(1,4); },
        '.' => { cell(2,4); },
        '-' => { cell(1,2); cell(2,2); cell(3,2); },
        _ => {},
    }
}

fn line_h(fb: &mut Framebuffer, x1: usize, y: usize, x2: usize, color: Color) {
    if y < fb.height {
        if let Some((_, row)) = fb.rows_mut().nth(y) {
            for x in x1..x2.min(row.len()) {
                row[x] = color;
            }
        }
    }
}
