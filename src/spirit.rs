//! Scene 2: the spirit world, a grand yokai pleasure district at night.
//!
//! Layout (X = left/right, Z = back(-)/front(+), Y = up), 96 × 96 blocks:
//!   front  (z 30..47): a large plaza where you arrive; torii; spirit orb
//!   middle (z -30..25): a grand canal with red bridges, narrow wooden streets,
//!                       rows of 3-4 story teahouses, bathhouses, shops with
//!                       red doors, glowing paper windows, red lanterns everywhere,
//!                       lantern strings overhead (Demon Slayer Yoshiwara vibe)
//!   back   (z -40..-30): the great bathhouse, six floors tall
//!
//! The mood: night, deep indigo and purple, lit by warm RED lanterns on every
//! corner. Narrow alleys, wooden architecture, red accents. Atmospheric and
//! otherworldly.

use crate::animation::Animation;
use crate::color::Color;
use crate::controls::Controls;
use crate::material::Material;
use crate::scene::{PointLight, Scene, Sun};
use crate::shrine::{build_gable_roof, noise};
use crate::vec3::Vec3;
use crate::world::World;

const GROUND_TOP: i32 = 4;
const EDGE_MIN: i32 = -48;
const EDGE_MAX: i32 = 47;
/// The canal runs along Z between these depths, 5 blocks wide (x -2..2).
const CANAL_BACK: i32 = -35;
const CANAL_FRONT: i32 = 20;

struct Materials {
    stone: usize,
    wood: usize,
    dark_wood: usize,
    lacquer: usize,
    roof: usize,
    paper_light: usize,
    lantern: usize,
    water: usize,
    glass: usize,
    foxfire: usize,
    portal: usize,
    sakura: usize,
    petal: usize,
}

/// Which side of a building faces the street.
#[derive(Clone, Copy, PartialEq)]
enum Front {
    PlusX,
    MinusX,
    PlusZ,
}

pub fn build_spirit_world() -> World {
    // The "sun" here is the moon: dim and cool. Its direction matches the moon
    // painted in the night skybox (examples/make_skybox.rs).
    let moon = Sun {
        direction_to_sun: Vec3::new(-0.4, 0.55, -0.75).normalized(),
        color: Color::new(0.5, 0.5, 0.78),
    };
    let ambient = Color::new(0.2, 0.15, 0.32); // deep indigo night, but readable
    let mut scene = Scene::new([EDGE_MIN, 0, EDGE_MIN], [96, 40, 96], moon, ambient);
    let m = add_materials(&mut scene);
    scene.load_skybox("night");

    scene.fill((EDGE_MIN, 0, EDGE_MIN), (EDGE_MAX, GROUND_TOP, EDGE_MAX), m.stone);
    build_canal_and_bridges(&mut scene, &m);

    // Expanded: many more buildings, 3-4 rows on each side for that dense red-light district feel
    // Left side (+X direction)
    for (z0, z1, floors) in [(-32, -26, 3), (-24, -18, 4), (-16, -10, 3), (-8, -2, 4), (0, 6, 3), (8, 14, 4), (16, 22, 3)] {
        build_teahouse(&mut scene, &m, (-26, z0, -16, z1), floors, Front::PlusX);
    }
    // Right side (-X direction)
    for (z0, z1, floors) in [(-32, -26, 4), (-24, -18, 3), (-16, -10, 4), (-8, -2, 3), (0, 6, 4), (8, 14, 3), (16, 22, 4)] {
        build_teahouse(&mut scene, &m, (16, z0, 26, z1), floors, Front::MinusX);
    }
    // The great bathhouse closing off the far end of the street.
    build_teahouse(&mut scene, &m, (-13, -40, 13, -32), 6, Front::PlusZ);

    // Additional small shops/buildings in narrow alleys
    for (z0, z1) in [(-28, -24), (-12, -8), (4, 8)] {
        build_teahouse(&mut scene, &m, (-12, z0, -6, z1), 2, Front::PlusX);
        build_teahouse(&mut scene, &m, (6, z0, 12, z1), 2, Front::MinusX);
    }

    // Many lantern strings across the street: every 6-7 blocks for atmosphere
    for z in (-30..=20).step_by(7) {
        for x in (-20..=20).step_by(2) {
            scene.add_block(x, GROUND_TOP + 12, z, m.lantern);
        }
        for x in [-18, -10, -2, 6, 14] {
            add_lantern_light(&mut scene, x, GROUND_TOP + 12, z);
        }
    }

    // Red lanterns on building corners for more ambient light
    for z in (-35..=18).step_by(8) {
        for x in [-24, -18, 18, 24] {
            scene.add_block(x, GROUND_TOP + 3, z, m.lantern);
            add_lantern_light(&mut scene, x, GROUND_TOP + 4, z);
        }
    }

    // Extra lantern poles on street corners for dramatic red-light district vibe
    for (x, z) in [(-28, -15), (28, -15), (-28, 8), (28, 8)] {
        for dy in 0..3 {
            scene.add_block(x, GROUND_TOP + 2 + dy, z, m.dark_wood);
        }
        scene.add_block(x, GROUND_TOP + 5, z, m.lantern);
        scene.add_block(x + 1, GROUND_TOP + 5, z, m.lantern);
        scene.add_block(x, GROUND_TOP + 5, z + 1, m.lantern);
        add_lantern_light(&mut scene, x, GROUND_TOP + 6, z);
        add_lantern_light(&mut scene, x + 1, GROUND_TOP + 6, z);
    }

    // Small paper lanterns on alley posts (10 more) for deep atmospheric detail
    for (x, z) in [(-20, -20), (-20, 0), (-20, 15), (20, -20), (20, 0), (20, 15),
                    (-8, -30), (8, -30), (-8, 25), (8, 25)] {
        for dy in 1..3 {
            scene.add_block(x, GROUND_TOP + dy, z, m.dark_wood);
        }
        scene.add_block(x, GROUND_TOP + 3, z, m.lantern);
        if (x + z).abs() % 2 == 0 {
            add_lantern_light(&mut scene, x, GROUND_TOP + 4, z);
        }
    }

    build_small_stalls(&mut scene, &m);
    build_central_stage(&mut scene, &m);
    build_spirit_orb(&mut scene, &m);
    build_portal_gate(&mut scene, &m);
    for (x, z) in [(-9, 19), (7, 16)] {
        build_glowing_sakura(&mut scene, &m, x, z);
    }

    // Petals drifting through the warm haze: from the two plaza trees, and
    // from high above the street (as if blown in from somewhere unseen).
    let mut animation = Animation::new(m.petal, m.lantern, m.water);
    animation.add_petal_tree(Vec3::new(-8.5, 9.5, 19.5), 3.0, (GROUND_TOP + 1) as f32, 8);
    animation.add_petal_tree(Vec3::new(7.5, 9.5, 16.5), 3.0, (GROUND_TOP + 1) as f32, 8);
    animation.add_petal_tree(Vec3::new(0.0, 22.0, -6.0), 7.0, (GROUND_TOP + 1) as f32, 14);

    World { scene, animation, portal_material: m.portal, arrival }
}

/// Arriving through the portal: standing in the plaza in front of the gate,
/// looking down the lantern-lit street toward the great bathhouse.
pub fn arrival() -> Controls {
    Controls::new(Vec3::new(0.5, 8.5, 18.5), Vec3::new(0.5, 7.0, -10.0), 60.0)
}

fn add_materials(scene: &mut Scene) -> Materials {
    let mut add = |texture_name: &str, base: Material| {
        let texture = scene.load_texture(texture_name);
        scene.add_material(Material { texture_top: texture, texture_side: texture, texture_bottom: texture, ..base })
    };
    let opaque = |albedo, specular, shininess| Material::opaque(0, albedo, specular, shininess);
    Materials {
        // Wet paving stones: a little shine catches the lantern light.
        stone: add("spirit_stone", opaque(0.85, 0.25, 24.0)),
        wood: add("spirit_wood", opaque(0.9, 0.05, 8.0)),
        dark_wood: add("bark", opaque(0.9, 0.0, 1.0)),
        lacquer: add("lacquer", Material { reflectivity: 0.1, ..opaque(0.95, 0.6, 48.0) }),
        roof: add("roof_tile", opaque(0.8, 0.3, 24.0)),
        // Paper windows lit from inside: they glow warmly.
        paper_light: add("paper", Material { glow: 0.9, ..opaque(0.95, 0.0, 1.0) }),
        lantern: add("red_lantern", Material { glow: 1.1, ..opaque(0.95, 0.1, 8.0) }),
        // Dark canal water: more mirror than the pond, to reflect the lanterns.
        water: add("water", Material {
            transparency: 0.6,
            reflectivity: 0.4,
            refractive_index: 1.33,
            texture_scroll: Vec3::new(0.0, 0.0, 0.2),
            ..opaque(0.25, 0.9, 96.0)
        }),
        // Glass of the spirit orb: almost clear, and bends light more than water.
        glass: add("glass", Material {
            transparency: 0.9,
            reflectivity: 0.1,
            refractive_index: 1.5,
            ..opaque(0.9, 0.9, 128.0)
        }),
        foxfire: add("foxfire", Material { glow: 1.5, ..opaque(0.9, 0.2, 16.0) }),
        portal: add("portal", Material { glow: 1.4, texture_scroll: Vec3::new(0.0, 0.5, 0.0), ..opaque(0.9, 0.3, 32.0) }),
        sakura: add("sakura", Material { glow: 0.25, ..opaque(0.95, 0.05, 4.0) }),
        petal: add("sakura", Material { glow: 0.6, ..opaque(0.95, 0.05, 4.0) }),
    }
}

/// A warm light hanging just BELOW a lantern block at (x, y, z), so the
/// lantern itself doesn't block the light's shadow rays.
fn add_lantern_light(scene: &mut Scene, x: i32, y: i32, z: i32) {
    scene.point_lights.push(PointLight {
        position: Vec3::new(x as f32 + 0.5, y as f32 - 0.3, z as f32 + 0.5),
        color: Color::new(1.3, 0.6, 0.3),
        radius: 7.0,
    });
}

fn build_canal_and_bridges(scene: &mut Scene, m: &Materials) {
    // Two blocks of water, one block below the street.
    scene.clear((-2, GROUND_TOP, CANAL_BACK), (2, GROUND_TOP, CANAL_FRONT));
    scene.fill((-2, GROUND_TOP - 2, CANAL_BACK), (2, GROUND_TOP - 1, CANAL_FRONT), m.water);

    // Red arched bridges where the alleys meet the canal.
    for z in [-7, 1] {
        for x in -3i32..=3 {
            let deck = GROUND_TOP + 1 + if x.abs() <= 1 { 1 } else { 0 };
            scene.add_block(x, deck, z, m.wood);
            for side in [z - 1, z + 1] {
                scene.add_block(x, deck, side, m.wood);
                scene.add_block(x, deck + 1, side, m.lacquer);
            }
        }
    }
}

/// A multi-story wooden teahouse or bathhouse. Each floor is 4 blocks: three
/// of wall with glowing paper windows, then a ring of roof tiles sticking out
/// (the tiered eaves). Upper floors get red balconies; red lanterns hang under
/// the eaves; the top gets a gable roof with upturned corners.
fn build_teahouse(scene: &mut Scene, m: &Materials, (x0, z0, x1, z1): (i32, i32, i32, i32), floors: i32, front: Front) {
    const FLOOR_HEIGHT: i32 = 4;
    let bottom = GROUND_TOP + 1;
    let top = GROUND_TOP + FLOOR_HEIGHT * floors;
    scene.fill((x0, bottom, z0), (x1, top, z1), m.wood);

    // A position on the front: `along` runs across the front face, `out` is
    // 0 for the wall itself and 1 for the space just in front of it.
    let (along_range, at): (std::ops::RangeInclusive<i32>, Box<dyn Fn(i32, i32, i32) -> (i32, i32, i32)>) = match front {
        Front::PlusX => (z0..=z1, Box::new(move |along, y, out| (x1 + out, y, along))),
        Front::MinusX => (z0..=z1, Box::new(move |along, y, out| (x0 - out, y, along))),
        Front::PlusZ => (x0..=x1, Box::new(move |along, y, out| (along, y, z1 + out))),
    };
    let start = *along_range.start();
    let middle = (along_range.start() + along_range.end()) / 2;

    for floor in 0..floors {
        let y0 = bottom + floor * FLOOR_HEIGHT;
        let is_top_floor = floor == floors - 1;
        for along in along_range.clone() {
            let column = along - start;
            // Paper windows in pairs, with a wooden post every third block.
            if column % 3 != 0 {
                for y in y0 + 1..=y0 + 2 {
                    let (x, y, z) = at(along, y, 0);
                    scene.add_block(x, y, z, m.paper_light);
                }
            }
            // Balcony floor and posts on upper floors.
            if floor > 0 {
                let (x, y, z) = at(along, y0, 1);
                scene.add_block(x, y, z, m.lacquer);
                if column % 3 == 0 {
                    let (x, y, z) = at(along, y0 + 1, 1);
                    scene.add_block(x, y, z, m.lacquer);
                }
            }
            // Lanterns hanging under the eave above this floor.
            if !is_top_floor && column % 3 == 1 {
                let (x, y, z) = at(along, y0 + 2, 1);
                scene.add_block_if_empty(x, y, z, m.lantern);
                // Only the street-level lanterns cast real light (keeps it fast).
                if floor == 0 && column % 6 == 1 {
                    add_lantern_light(scene, x, y, z);
                }
            }
        }
        // Tiered eave: a ring of roof tiles one block out, above each floor.
        if !is_top_floor {
            for x in x0 - 1..=x1 + 1 {
                for z in z0 - 1..=z1 + 1 {
                    if x < x0 || x > x1 || z < z0 || z > z1 {
                        scene.add_block(x, y0 + 3, z, m.roof);
                    }
                }
            }
        }
    }

    // Glowing entrance on the ground floor, in the middle of the front.
    for along in middle..=middle + 1 {
        for y in bottom..=bottom + 1 {
            let (x, y, z) = at(along, y, 0);
            scene.add_block(x, y, z, m.paper_light);
        }
    }

    // Gable roof, plus corners that curl up and a dark ridge cap.
    let ridge = build_gable_roof(scene, m.roof, x0, x1, z0, z1, top + 1);
    for (x, z) in [(x0 - 1, z0 - 1), (x1 + 1, z0 - 1), (x0 - 1, z1 + 1), (x1 + 1, z1 + 1)] {
        scene.add_block(x, top + 2, z, m.roof);
    }
    scene.fill((x0 - 1, ridge + 1, (z0 + z1) / 2), (x1 + 1, ridge + 1, (z0 + z1 + 1) / 2), m.dark_wood);
}

/// The spirit orb: a 3×3×3 cube of glass on a stone pedestal, with a block of
/// blue fox-fire floating in its center. Looking through the glass bends the
/// lanterns and the street behind it (refraction, index 1.5).
fn build_spirit_orb(scene: &mut Scene, m: &Materials) {
    // On the left side of the plaza, so it doesn't block the view down the street.
    let (x, z, base) = (-5, 14, GROUND_TOP + 1);
    scene.fill((x - 1, base, z - 1), (x + 1, base, z + 1), m.stone);
    scene.fill((x - 1, base + 1, z - 1), (x + 1, base + 3, z + 1), m.glass);
    scene.add_block(x, base + 2, z, m.foxfire);
    scene.point_lights.push(PointLight {
        position: Vec3::new(x as f32 + 0.5, (base + 4) as f32 + 0.6, z as f32 + 0.5), // just above the glass
        color: Color::new(0.4, 0.8, 1.6),
        radius: 9.0,
    });
}

/// The way home: a lacquered torii with a glowing portal filling its opening.
fn build_portal_gate(scene: &mut Scene, m: &Materials) {
    let (base, z) = (GROUND_TOP + 1, 21);
    scene.fill((-3, base, z), (-3, base + 6, z), m.lacquer);
    scene.fill((3, base, z), (3, base + 6, z), m.lacquer);
    scene.fill((-2, base + 5, z), (2, base + 5, z), m.lacquer);
    scene.fill((-5, base + 7, z), (5, base + 7, z), m.dark_wood);
    scene.add_block(-5, base + 8, z, m.dark_wood);
    scene.add_block(5, base + 8, z, m.dark_wood);
    scene.fill((-2, base, z), (2, base + 4, z), m.portal);
    scene.point_lights.push(PointLight {
        position: Vec3::new(0.5, (base + 2) as f32 + 0.5, z as f32 - 1.5),
        color: Color::new(0.9, 0.5, 1.3),
        radius: 8.0,
    });
}

/// A small sakura whose blossoms glow faintly in the dark.
fn build_glowing_sakura(scene: &mut Scene, m: &Materials, x: i32, z: i32) {
    let trunk_top = GROUND_TOP + 4;
    scene.fill((x, GROUND_TOP + 1, z), (x, trunk_top, z), m.dark_wood);
    for dx in -3i32..=3 {
        for dy in -2i32..=2 {
            for dz in -3i32..=3 {
                let distance = ((dx * dx + dz * dz) as f32 + (dy as f32 * 1.4).powi(2)).sqrt();
                let (bx, by, bz) = (x + dx, trunk_top + 1 + dy, z + dz);
                if distance > 3.0 || (distance > 1.8 && noise(bx, by, bz) < 35) {
                    continue;
                }
                scene.add_block_if_empty(bx, by, bz, m.sakura);
            }
        }
    }
}

/// Central performance stage with tall lantern tower and spectator platforms
fn build_central_stage(scene: &mut Scene, m: &Materials) {
    let center_x = 0;
    let center_z = 0;

    // Raised stage platform
    scene.fill((center_x - 3, GROUND_TOP + 1, center_z - 3), (center_x + 3, GROUND_TOP + 1, center_z + 3), m.lacquer);

    // Decorative posts at corners
    for (dx, dz) in [(-3, -3), (3, -3), (-3, 3), (3, 3)] {
        for dy in 0..3 {
            scene.add_block(center_x + dx, GROUND_TOP + 1 + dy, center_z + dz, m.dark_wood);
        }
    }

    // Grand central lantern tower (8 blocks tall with multiple lanterns)
    for dy in 0..8 {
        scene.add_block(center_x, GROUND_TOP + 2 + dy, center_z, m.dark_wood);
        if dy % 2 == 0 && dy > 0 {
            // Lanterns jutting out at intervals
            for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                scene.add_block(center_x + dx, GROUND_TOP + 2 + dy, center_z + dz, m.lantern);
                scene.point_lights.push(PointLight {
                    position: Vec3::new((center_x + dx) as f32 + 0.5, (GROUND_TOP + 3 + dy) as f32, (center_z + dz) as f32 + 0.5),
                    color: Color::new(1.4, 0.7, 0.4),
                    radius: 12.0,
                });
            }
        }
    }

    // Top lantern crown
    scene.add_block(center_x, GROUND_TOP + 9, center_z, m.lantern);
    scene.point_lights.push(PointLight {
        position: Vec3::new(center_x as f32 + 0.5, (GROUND_TOP + 10) as f32, center_z as f32 + 0.5),
        color: Color::new(1.5, 0.8, 0.5),
        radius: 15.0,
    });
}

/// Small vendor stalls and food shops scattered in the alley streets
fn build_small_stalls(scene: &mut Scene, m: &Materials) {
    // Noodle vendor stall: small wooden structure with red roof
    let stalls = [(-15, -22), (15, -22), (-12, 10), (12, 10), (-10, -8), (10, 5)];
    for &(x, z) in &stalls {
        // Counter: 3×2 blocks
        scene.fill((x, GROUND_TOP + 1, z), (x + 1, GROUND_TOP + 1, z + 1), m.dark_wood);
        // Small roof/awning: 2 blocks tall
        for dy in 0..2 {
            scene.add_block(x, GROUND_TOP + 2 + dy, z, m.roof);
            scene.add_block(x + 1, GROUND_TOP + 2 + dy, z, m.roof);
        }
        // Red lantern on corner
        scene.add_block(x + 1, GROUND_TOP + 4, z + 1, m.lantern);
    }
}

#[cfg(test)]
mod tests {
    use crate::camera::Camera;
    use crate::renderer::pick;
    use crate::vec3::Vec3;

    /// Clicking means: shoot a ray through the crosshair (the screen center)
    /// and look at what it hits first. Aiming at each portal must hit the portal.
    fn crosshair_hits_portal(world: &crate::world::World, position: Vec3, target: Vec3) -> bool {
        let camera = Camera { position, target, vertical_fov_degrees: 60.0 };
        pick(&camera, &world.scene, 320, 180, 640, 360).is_some_and(|hit| hit.material == world.portal_material)
    }

    #[test]
    fn aiming_at_the_spirit_world_gate_hits_the_portal() {
        let world = super::build_spirit_world();
        // From the arrival spot, turn around and look at the gate.
        assert!(crosshair_hits_portal(&world, Vec3::new(0.5, 7.5, 18.5), Vec3::new(0.5, 7.5, 25.0)));
        // Looking down the street instead must NOT count as clicking the portal.
        assert!(!crosshair_hits_portal(&world, Vec3::new(0.5, 7.5, 18.5), Vec3::new(0.5, 7.5, -10.0)));
    }

    #[test]
    fn aiming_at_the_cave_portal_hits_the_portal() {
        let world = crate::shrine::build_shrine_world();
        // Standing just outside the cave tunnel entrance, aiming at the portal.
        // After shrine expansion, adjusted to account for new shrine layout.
        assert!(crosshair_hits_portal(&world, Vec3::new(-23.0, 19.5, -13.0), Vec3::new(-23.0, 19.5, -25.0)));
    }
}
