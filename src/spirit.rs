//! Scene 2: the spirit world, a yokai pleasure district at night.
//!
//! Layout (X = left/right, Z = back(-)/front(+), Y = up), 48 × 48 blocks:
//!   front  (z 11..23): a small plaza where you arrive through the portal
//!                      torii; a glass spirit orb with fox-fire inside
//!   middle (z -17..9): a canal down the center with two red bridges,
//!                      streets on both banks, rows of multi-story wooden
//!                      bathhouses and teahouses with glowing paper windows,
//!                      balconies and red lanterns; lantern strings overhead
//!   back   (z -23..-19): the great bathhouse, six floors tall
//!
//! The mood is the opposite of the shrine: night, deep indigo and purple,
//! lit almost only by warm lanterns and the cool moon.

use crate::animation::Animation;
use crate::color::Color;
use crate::controls::Controls;
use crate::material::Material;
use crate::scene::{PointLight, Scene, Sun};
use crate::shrine::{build_gable_roof, noise};
use crate::vec3::Vec3;
use crate::world::World;

const GROUND_TOP: i32 = 4;
const EDGE_MIN: i32 = -24;
const EDGE_MAX: i32 = 23;
/// The canal runs along Z between these depths, 5 blocks wide (x -2..2).
const CANAL_BACK: i32 = -17;
const CANAL_FRONT: i32 = 9;

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
    let mut scene = Scene::new([EDGE_MIN, 0, EDGE_MIN], [48, 40, 48], moon, ambient);
    let m = add_materials(&mut scene);
    scene.load_skybox("night");

    scene.fill((EDGE_MIN, 0, EDGE_MIN), (EDGE_MAX, GROUND_TOP, EDGE_MAX), m.stone);
    build_canal_and_bridges(&mut scene, &m);

    // Two rows of buildings facing the canal: (z0, z1, floors).
    for (z0, z1, floors) in [(-15, -9, 3), (-6, 0, 4), (3, 9, 3)] {
        build_teahouse(&mut scene, &m, (-20, z0, -10, z1), floors, Front::PlusX);
    }
    for (z0, z1, floors) in [(-15, -9, 4), (-6, 0, 3), (3, 9, 4)] {
        build_teahouse(&mut scene, &m, (10, z0, 20, z1), floors, Front::MinusX);
    }
    // The great bathhouse closing off the far end of the street.
    build_teahouse(&mut scene, &m, (-11, -23, 11, -19), 6, Front::PlusZ);

    // Strings of lanterns hung across the street, high above the canal.
    for z in [-11, -4, 5] {
        for x in (-8..=8).step_by(2) {
            scene.add_block(x, GROUND_TOP + 12, z, m.lantern);
        }
        for x in [-6, 6] {
            add_lantern_light(&mut scene, x, GROUND_TOP + 12, z);
        }
    }

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
        // Standing in the cave chamber, facing the back wall.
        assert!(crosshair_hits_portal(&world, Vec3::new(-27.5, 20.5, -27.5), Vec3::new(-31.0, 20.5, -27.5)));
    }
}
