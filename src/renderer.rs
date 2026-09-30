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
                        // Everything from here on happens in diorama space.
                        let ray = scene.to_diorama_ray(&camera.ray_for_pixel(x, y, width, height));
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
            None => sky(scene, ray.direction),
        };
    }
    trace_ray(ray, scene, 0)
}

/// The color seen looking along `ray`. `depth` counts how many bounces
/// already happened to get here (0 for a ray straight from the camera).
fn trace_ray(ray: &Ray, scene: &Scene, depth: u32) -> Color {
    match scene.closest_hit(ray, 0.0, f32::INFINITY) {
        Some(hit) => shade_hit(ray, &hit, scene, depth),
        // Missed everything: a reflected or refracted ray that flies off shows the sky.
        None => sky(scene, ray.direction),
    }
}

/// The color of a surface point, including what it reflects and what is
/// seen through it. Calls trace_ray again for those new rays: that recursion
/// is how a mirror or a pond shows the rest of the scene.
fn shade_hit(ray: &Ray, hit: &Hit, scene: &Scene, depth: u32) -> Color {
    let material = &scene.materials[hit.material];

    // The surface's own look (texture + sunlight + highlight). Direction
    // from the surface back toward the eye is the reverse of the ray.
    let local_color = shade_surface(scene, hit, -ray.direction);

    let (reflectivity, transparency) = (material.reflectivity, material.transparency);
    if (reflectivity <= 0.0 && transparency <= 0.0) || depth >= MAX_BOUNCES {
        return local_color;
    }

    let reflected_color = if reflectivity > 0.0 {
        // Mirror the incoming direction around the normal, and start the new ray
        // slightly off the surface (same self-hit problem as shadow rays).
        let reflected_ray = Ray::new(hit.point + hit.normal * SURFACE_OFFSET, ray.direction.reflect(hit.normal));
        trace_ray(&reflected_ray, scene, depth + 1)
    } else {
        Color::BLACK
    };

    let transmitted_color = if transparency > 0.0 {
        color_through(ray, hit, scene, depth)
    } else {
        Color::BLACK
    };

    // Energy split: the mirror takes its share first; of what is left, the
    // transparent part passes through and the rest is the surface's own color.
    // Water: 25% mirror, 75% × 80% = 60% seen through, 75% × 20% = 15% own blue.
    // The shares add up to exactly 100%, so nothing gets brighter than its sources.
    reflected_color * reflectivity
        + (transmitted_color * transparency + local_color * (1.0 - transparency)) * (1.0 - reflectivity)
}

/// What you see THROUGH a transparent block: follow the light in, across the
/// whole body of that material, and back out the other side.
///
/// Assumption: one transparent material at a time (no glass inside water).
/// Neighboring blocks of the same material form one body, like a real pond.
fn color_through(ray: &Ray, hit: &Hit, scene: &Scene, depth: u32) -> Color {
    let material = &scene.materials[hit.material];

    // 1. ENTER: bend from air (index 1.0) into the material. The hit normal
    //    points out of the block, toward the incoming ray, as refract() expects.
    //    Going into a denser material can never be total internal reflection.
    let Some(inside_direction) = refract(ray.direction, hit.normal, 1.0 / material.refractive_index) else {
        return Color::BLACK;
    };
    // Start just INSIDE the surface this time, hence minus the normal.
    let mut inside_ray = Ray::new(hit.point - hit.normal * SURFACE_OFFSET, inside_direction);

    // 2. TRAVEL inside until the contents change. A ray can bounce off the
    //    underside of the surface (total internal reflection) a few times.
    for _ in 0..MAX_BOUNCES {
        let Some(boundary) = scene.exit_from(&inside_ray, hit.material) else {
            return sky(scene, inside_ray.direction); // left the world: shouldn't happen
        };

        match boundary.entered {
            // Something solid underwater (the pond floor, a koi, a rock): its own
            // color, seen through the water. boundary.normal points back toward
            // the ray, which is exactly the solid face's outward normal.
            Some(solid) => {
                let underwater_hit = Hit {
                    t: boundary.t,
                    point: boundary.point,
                    normal: boundary.normal,
                    material: solid as usize,
                };
                return shade_hit(&inside_ray, &underwater_hit, scene, depth + 1);
            }
            // 3. EXIT into air: bend back out (material index → 1.0). Here the
            //    normal points back INTO the water, again facing the ray.
            None => match refract(inside_ray.direction, boundary.normal, material.refractive_index) {
                Some(outside_direction) => {
                    let outside_ray = Ray::new(boundary.point - boundary.normal * SURFACE_OFFSET, outside_direction);
                    return trace_ray(&outside_ray, scene, depth + 1);
                }
                None => {
                    // Too shallow an angle to get out: the surface acts as a
                    // perfect mirror from below, and the ray stays inside.
                    inside_ray = Ray::new(
                        boundary.point + boundary.normal * SURFACE_OFFSET,
                        inside_ray.direction.reflect(boundary.normal),
                    );
                }
            },
        }
    }
    Color::BLACK // trapped bouncing inside: give up (rare)
}

/// Snell's law: bends a unit `direction` as it crosses into another material.
///
/// `normal` must face the incoming ray (normal · direction < 0).
/// `eta` = (index of the material we come FROM) / (index we go INTO),
/// e.g. 1.0 / 1.33 from air into water.
///
/// Returns None for total internal reflection: when leaving a denser material
/// at a shallow angle, sin(angle out) would have to exceed 1, which is
/// impossible, so no light gets out.
fn refract(direction: Vec3, normal: Vec3, eta: f32) -> Option<Vec3> {
    let cos_in = -normal.dot(direction);
    // Snell: sin(out) = eta × sin(in), and sin² = 1 − cos²
    let sin_out_squared = eta * eta * (1.0 - cos_in * cos_in);
    if sin_out_squared > 1.0 {
        return None;
    }
    let cos_out = (1.0 - sin_out_squared).sqrt();
    // Keep the part along the surface (scaled by eta) and rebuild the part
    // along the normal so the result has length 1 and the new angle.
    Some(direction * eta + normal * (eta * cos_in - cos_out))
}

/// Phong lighting: ambient + diffuse + specular.
fn shade_surface(scene: &Scene, hit: &Hit, view_direction: Vec3) -> Color {
    let material = &scene.materials[hit.material];
    let normal = hit.normal;
    let to_sun = scene.sun_direction(); // already turned into diorama space

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
/// The sky seen along a diorama-space direction. The sky belongs to the world,
/// not the diorama, so turn the direction back into world space first:
/// spinning the diorama must not spin the clouds.
fn sky(scene: &Scene, diorama_direction: Vec3) -> Color {
    sky_color(scene.to_world_direction(diorama_direction))
}

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

    const WATER: f32 = 1.33;

    #[test]
    fn looking_straight_down_is_not_bent() {
        let down = Vec3::new(0.0, -1.0, 0.0);
        let bent = refract(down, Vec3::UP, 1.0 / WATER).unwrap();
        assert!((bent - down).length() < 1e-6);
    }

    #[test]
    fn entering_water_at_45_degrees_bends_toward_the_normal() {
        let incoming = Vec3::new(1.0, -1.0, 0.0).normalized(); // 45° from vertical
        let bent = refract(incoming, Vec3::UP, 1.0 / WATER).unwrap();
        // Snell: sin(out) = sin(45°) / 1.33 = 0.5317, i.e. about 32.1° from vertical.
        assert!((bent.x - 0.5317).abs() < 1e-3);
        assert!(bent.y < 0.0);
        assert!((bent.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn leaving_water_at_a_shallow_angle_is_total_internal_reflection() {
        // From inside, going up at 60° from vertical: sin(60°) × 1.33 = 1.15 > 1.
        // (The critical angle for water is about 48.8°.) The normal faces the ray: down.
        let upward = Vec3::new(60f32.to_radians().sin(), 60f32.to_radians().cos(), 0.0);
        assert!(refract(upward, Vec3::new(0.0, -1.0, 0.0), WATER).is_none());
        // At 30° it does get out.
        let steep = Vec3::new(30f32.to_radians().sin(), 30f32.to_radians().cos(), 0.0);
        assert!(refract(steep, Vec3::new(0.0, -1.0, 0.0), WATER).is_some());
    }

    #[test]
    fn a_flat_slab_shifts_the_ray_but_keeps_its_direction() {
        // Enter through the top face and leave through the bottom face. Both
        // normals face the ray (point up), so the two bends exactly cancel.
        let incoming = Vec3::new(0.6, -0.8, 0.0);
        let inside = refract(incoming, Vec3::UP, 1.0 / WATER).unwrap();
        let outgoing = refract(inside, Vec3::UP, WATER).unwrap();
        assert!((outgoing - incoming).length() < 1e-5);
    }

    #[test]
    fn perfectly_clear_blocks_are_invisible() {
        // Three clear blocks in a row that don't bend light (index 1.0) and
        // don't reflect: looking through them must show exactly the sky behind.
        let (mut scene, _) = mirror_scene();
        let clear = scene.add_material(Material {
            transparency: 1.0,
            refractive_index: 1.0,
            ..Material::opaque(0, 1.0, 0.0, 1.0)
        });
        scene.fill((0, 0, 0), (0, 0, 2), clear);
        let direction = Vec3::new(0.1, 0.2, 1.0).normalized();
        let ray = Ray::new(Vec3::new(0.5, 0.3, -5.0), direction);
        let seen = trace_ray(&ray, &scene, 0);
        let expected = sky_color(direction);
        assert!((seen.r - expected.r).abs() < 1e-4 && (seen.b - expected.b).abs() < 1e-4);
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
