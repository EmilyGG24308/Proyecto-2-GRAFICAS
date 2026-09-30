use crate::camera::Camera;
use crate::color::Color;
use crate::cube::Hit;
use crate::framebuffer::Framebuffer;
use crate::ray::Ray;
use crate::scene::Scene;
use crate::vec3::Vec3;
use std::sync::Mutex;

/// How far to push a new ray's start off the surface, so it does not
/// immediately re-hit the surface it starts on (see `is_in_shadow`).
const SURFACE_OFFSET: f32 = 1e-3;

/// Most bounces a single camera ray may make. Two mirrors facing each other
/// would otherwise bounce forever; after this many we stop and use the
/// surface's own color. 3 is plenty: each bounce's share gets multiplied by
/// the reflectivity again, so a 4th bounce would barely change the pixel.
const MAX_BOUNCES: u32 = 3;

/// Shoots one ray per pixel and stores the color it sees, using every CPU core.
///
/// Rows are handed out one at a time from a shared queue (the Mutex): each
/// thread grabs the next unfinished row, renders it, and comes back for more.
/// Rows full of sky are fast and rows full of scene are slow, so this keeps
/// every core busy until the very end instead of some finishing early.
pub fn render_frame(framebuffer: &mut Framebuffer, camera: &Camera, scene: &Scene, show_normals: bool) {
    let (width, height) = (framebuffer.width, framebuffer.height);
    let rows = Mutex::new(framebuffer.rows_mut());
    let thread_count = std::thread::available_parallelism().map_or(1, |count| count.get());

    // `scope` guarantees all threads finish before render_frame returns,
    // which is what lets them safely borrow the camera, scene and rows.
    std::thread::scope(|scope| {
        for _ in 0..thread_count {
            scope.spawn(|| {
                loop {
                    let Some((y, row)) = rows.lock().unwrap().next() else {
                        break; // no rows left
                    };
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let ray = camera.ray_for_pixel(x, y, width, height);
                        *pixel = shade_ray(&ray, scene, show_normals);
                    }
                }
            });
        }
    });
}

fn shade_ray(ray: &Ray, scene: &Scene, show_normals: bool) -> Color {
    if show_normals {
        return match scene.closest_hit(ray, 0.0, f32::INFINITY) {
            Some(hit) => normal_as_color(&hit),
            None => sky_color(ray.direction),
        };
    }
    trace_ray(ray, scene, 0)
}

/// The color seen looking along `ray`. `depth` counts how many bounces
/// already happened to get here (0 for a ray straight from the camera).
///
/// This calls itself for the reflected ray: that is the recursion that makes
/// a mirror show the scene, including other mirrors, up to MAX_BOUNCES deep.
fn trace_ray(ray: &Ray, scene: &Scene, depth: u32) -> Color {
    let Some(hit) = scene.closest_hit(ray, 0.0, f32::INFINITY) else {
        // Missed everything: a reflected ray that flies off shows the sky.
        return sky_color(ray.direction);
    };
    let material = &scene.materials[hit.material];

    // The surface's own look (texture + sunlight + highlight). Direction
    // from the surface back toward the eye is the reverse of the ray.
    let local_color = shade_surface(scene, &hit, -ray.direction);

    if material.reflectivity <= 0.0 || depth >= MAX_BOUNCES {
        return local_color;
    }

    // Mirror the incoming direction around the normal, and start the new ray
    // slightly off the surface (same self-hit problem as shadow rays).
    let reflected_ray = Ray::new(
        hit.point + hit.normal * SURFACE_OFFSET,
        ray.direction.reflect(hit.normal),
    );
    let reflected_color = trace_ray(&reflected_ray, scene, depth + 1);

    // Blend: reflectivity 0.25 means 25% mirror image, 75% the surface itself.
    // The two shares always add up to 100%, so the result is never brighter
    // than its ingredients.
    local_color * (1.0 - material.reflectivity) + reflected_color * material.reflectivity
}

/// Phong lighting: ambient + diffuse + specular.
fn shade_surface(scene: &Scene, hit: &Hit, view_direction: Vec3) -> Color {
    let material = &scene.materials[hit.material];
    let normal = hit.normal;
    let to_sun = scene.sun.direction_to_sun;

    // The surface's own color comes from its texture at this exact point.
    let texture_number = if normal.y > 0.5 {
        material.texture_top
    } else if normal.y < -0.5 {
        material.texture_bottom
    } else {
        material.texture_side
    };
    let (u, v) = face_uv(hit.point, normal);
    let surface_color = scene.textures[texture_number].sample(u, v) * material.albedo;

    // Ambient: a little light everywhere, so shadows are not pitch black.
    let ambient = surface_color * scene.ambient;

    // How directly the face points at the sun: 1 = straight at it, 0 or less = away.
    let facing_sun = normal.dot(to_sun);
    if facing_sun <= 0.0 || is_in_shadow(scene, hit, to_sun) {
        return ambient;
    }

    // Diffuse: matte light, brightest when the face points at the sun.
    let diffuse = surface_color * scene.sun.color * facing_sun;

    // Specular: the shiny highlight. Mirror the incoming sunlight around the
    // normal; the closer that mirrored ray points at the eye, the brighter.
    let reflected_sunlight = (-to_sun).reflect(normal);
    let toward_eye = reflected_sunlight.dot(view_direction).max(0.0);
    let specular = scene.sun.color * (material.specular_strength * toward_eye.powf(material.shininess));

    ambient + diffuse + specular
}

/// Shadow ray: from the hit point toward the sun. If anything is in the way,
/// the point is in shadow.
///
/// The start is pushed slightly off the surface along the normal. Without this,
/// rounding errors can put the start a hair INSIDE the block, the ray "hits" its
/// own face, and the surface gets covered in random dark speckles ("shadow acne").
fn is_in_shadow(scene: &Scene, hit: &Hit, to_sun: Vec3) -> bool {
    let shadow_ray = Ray::new(hit.point + hit.normal * SURFACE_OFFSET, to_sun);
    // The sun is infinitely far away, so a blocker at ANY distance counts.
    // (For a lamp at distance d, we would use t_max = d instead.)
    scene.is_blocked(&shadow_ray, 0.0, f32::INFINITY)
}

/// Texture coordinates (u, v), each 0..1, for a point on a block face.
///
/// Every block is 1×1×1 with corners on whole numbers, so the fractional part
/// of the hit point says where on the face we are. Each face uses the two axes
/// that run ALONG it (a face facing ±X spans Z and Y). The flips below make
/// u run left→right and v top→bottom as seen from OUTSIDE the face, with +Y up
/// on the sides, so textures are upright and never mirrored.
fn face_uv(point: Vec3, normal: Vec3) -> (f32, f32) {
    let (fx, fy, fz) = (fract(point.x), fract(point.y), fract(point.z));
    if normal.x > 0.5 {
        (1.0 - fz, 1.0 - fy) // looking at it from +X, "right" is -Z
    } else if normal.x < -0.5 {
        (fz, 1.0 - fy) // from -X, "right" is +Z
    } else if normal.z > 0.5 {
        (fx, 1.0 - fy) // from +Z, "right" is +X
    } else if normal.z < -0.5 {
        (1.0 - fx, 1.0 - fy) // from -Z, "right" is -X
    } else if normal.y > 0.5 {
        (fx, fz) // top
    } else {
        (fx, 1.0 - fz) // bottom
    }
}

fn fract(value: f32) -> f32 {
    value - value.floor()
}

/// Debug view: maps the normal's -1..1 range to a 0..1 color.
/// +X face = reddish, +Y (top) = greenish, +Z (front) = bluish.
fn normal_as_color(hit: &Hit) -> Color {
    let n = hit.normal;
    Color::new(n.x * 0.5 + 0.5, n.y * 0.5 + 0.5, n.z * 0.5 + 0.5)
}

/// Temporary sky gradient (the real skybox comes in Phase 8):
/// pink at the horizon, blue straight up.
fn sky_color(direction: Vec3) -> Color {
    let horizon = Color::new(1.0, 0.8, 0.88);
    let zenith = Color::new(0.45, 0.65, 1.0);
    horizon.lerp(zenith, direction.y.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Material;
    use crate::scene::Sun;
    use crate::texture::Texture;

    /// An empty world containing one texture and one PERFECT mirror material.
    fn mirror_scene() -> (Scene, usize) {
        let sun = Sun { direction_to_sun: Vec3::UP, color: Color::new(1.0, 1.0, 1.0) };
        let mut scene = Scene::new([-10, -10, -10], [20, 20, 20], sun, Color::BLACK);
        scene.textures.push(Texture::missing());
        let mirror = scene.add_material(Material {
            reflectivity: 1.0,
            ..Material::opaque(0, 1.0, 0.0, 1.0)
        });
        (scene, mirror)
    }

    #[test]
    fn perfect_mirror_shows_the_sky_in_the_mirrored_direction() {
        let (mut scene, mirror) = mirror_scene();
        scene.add_block(0, -1, 0, mirror); // its top face is the plane y = 0

        // Coming down at 45°, it must bounce up at 45° and see exactly that sky.
        let ray = Ray::new(Vec3::new(0.5, 5.0, -4.5), Vec3::new(0.0, -1.0, 1.0));
        let seen = trace_ray(&ray, &scene, 0);
        let expected = sky_color(Vec3::new(0.0, 1.0, 1.0).normalized());
        assert!((seen.r - expected.r).abs() < 1e-5);
        assert!((seen.g - expected.g).abs() < 1e-5);
        assert!((seen.b - expected.b).abs() < 1e-5);
    }

    #[test]
    fn two_facing_mirrors_stop_after_max_bounces() {
        let (mut scene, mirror) = mirror_scene();
        scene.add_block(0, 0, -3, mirror);
        scene.add_block(0, 0, 3, mirror);
        // Straight at a mirror: it bounces straight back to the other one, forever
        // without a limit. This test finishing at all proves the limit works.
        let ray = Ray::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(0.0, 0.0, -1.0));
        let _ = trace_ray(&ray, &scene, 0);
    }

    #[test]
    fn side_face_uv_is_upright_and_not_mirrored() {
        // Front (+Z) face of the block at (2, 3, 4): a point 1/4 from its left
        // edge and 1/4 down from its top edge.
        let (u, v) = face_uv(Vec3::new(2.25, 3.75, 5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!((u - 0.25).abs() < 1e-5 && (v - 0.25).abs() < 1e-5);

        // Same idea on the right (+X) face: seen from +X, "left" is the +Z side.
        let (u, v) = face_uv(Vec3::new(3.0, 3.75, 4.75), Vec3::new(1.0, 0.0, 0.0));
        assert!((u - 0.25).abs() < 1e-5 && (v - 0.25).abs() < 1e-5);
    }
}
