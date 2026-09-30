//! Scene 1: the Mikage Shrine on its hill, with the town below.
//!
//! ROUGH DRAFT ("blockout"): plain colors, simple shapes, to check scale and
//! layout. Textures come in Phase 5 and detail in Phase 10.
//!
//! Layout (X = left/right, Z = back(-)/front(+), Y = up):
//!   back  (z -24..-3): the hill top with the shrine, torii, pond, trees
//!   middle (z -2..11): the slope with the long stone staircase
//!   front (z 12..23): the town: streets, houses, utility poles

use crate::color::Color;
use crate::material::Material;
use crate::scene::{Scene, Sun};
use crate::vec3::Vec3;

/// Top block of the town ground and of the hill.
const GROUND_TOP: i32 = 3;
const HILL_TOP: i32 = 15;
/// Diorama footprint: x and z both go from -24 to 23 (48 × 48 blocks).
const EDGE_MIN: i32 = -24;
const EDGE_MAX: i32 = 23;

/// Material numbers, so the layout code can say `m.grass` instead of `3`.
struct Materials {
    grass: usize,
    dirt: usize,
    rock: usize,
    path_stone: usize,
    lacquer: usize,
    shrine_wood: usize,
    bark: usize,
    roof_tile: usize,
    sakura_light: usize,
    sakura_dark: usize,
    water: usize,
    sand: usize,
    plaster: usize,
    house_roof: usize,
    road: usize,
    paper: usize,
}

pub fn build_shrine_scene() -> Scene {
    let sun = Sun {
        direction_to_sun: Vec3::new(0.35, 1.0, 0.8).normalized(),
        color: Color::new(1.0, 0.95, 0.85), // warm afternoon sunlight
    };
    let ambient = Color::new(0.32, 0.33, 0.42); // cool, slightly blue sky light
    let mut scene = Scene::new([EDGE_MIN, 0, EDGE_MIN], [48, 30, 48], sun, ambient);
    let m = add_materials(&mut scene);

    build_terrain(&mut scene, &m);
    build_staircase(&mut scene, &m);
    build_walkway_and_lanterns(&mut scene, &m);
    build_torii(&mut scene, &m, -5);
    build_shrine_hall(&mut scene, &m);
    build_pond_and_bridge(&mut scene, &m);
    build_cave_rocks(&mut scene, &m);
    build_town(&mut scene, &m);

    // Sakura trees: (x, z, ground top, trunk height, canopy radius)
    for (x, z, ground, trunk, radius) in [
        (-11, -7, HILL_TOP, 6, 5),
        (-15, -17, HILL_TOP, 5, 4),
        (15, -20, HILL_TOP, 5, 4),
        (18, -3, HILL_TOP, 5, 4),
        (-7, -13, HILL_TOP, 4, 3),
        (-20, 19, GROUND_TOP, 3, 3),
    ] {
        build_sakura_tree(&mut scene, &m, x, z, ground, trunk, radius);
    }
    scene
}

fn add_materials(scene: &mut Scene) -> Materials {
    // ================= THE FIVE GRADED MATERIALS =================
    // Each has its own texture and its own albedo, specular, transparency
    // and reflectivity. Zero is a real, deliberate value (grass is not a mirror).
    //
    //  material         | albedo | specular (strength, shininess) | transparency | reflectivity | refractive index
    //  vermilion lacquer |  0.95  | 0.60, 48  glossy paint         |     0.0      |    0.10      |   –
    //  path stone        |  0.85  | 0.10, 16  worn, slightly waxy  |     0.0      |    0.0       |   –
    //  grass block       |  0.90  | 0.00      completely matte     |     0.0      |    0.0       |   –
    //  sakura blossom    |  0.95  | 0.05, 4   soft petals          |     0.0      |    0.0       |   –
    //  pond water        |  0.35  | 0.90, 96  sharp sun glint      |     0.80     |    0.25      |  1.33
    let lacquer_texture = scene.load_texture("lacquer");
    let lacquer = scene.add_material(Material {
        reflectivity: 0.10,
        ..Material::opaque(lacquer_texture, 0.95, 0.6, 48.0)
    });

    let path_stone_texture = scene.load_texture("path_stone");
    let path_stone = scene.add_material(Material::opaque(path_stone_texture, 0.85, 0.1, 16.0));

    let grass_top = scene.load_texture("grass_top");
    let grass_side = scene.load_texture("grass_side");
    let dirt_texture = scene.load_texture("dirt");
    let grass = scene.add_material(Material {
        texture_top: grass_top,
        texture_bottom: dirt_texture,
        ..Material::opaque(grass_side, 0.9, 0.0, 1.0)
    });

    let sakura_texture = scene.load_texture("sakura");
    let sakura_light = scene.add_material(Material::opaque(sakura_texture, 0.95, 0.05, 4.0));

    let water_texture = scene.load_texture("water");
    let water = scene.add_material(Material {
        transparency: 0.80,
        reflectivity: 0.25,
        refractive_index: 1.33,
        ..Material::opaque(water_texture, 0.35, 0.9, 96.0)
    });

    // ================= Supporting materials =================
    let mut textured = |name: &str, albedo: f32, specular: f32, shininess: f32| {
        let texture = scene.load_texture(name);
        scene.add_material(Material::opaque(texture, albedo, specular, shininess))
    };
    Materials {
        grass,
        dirt: textured("dirt", 0.9, 0.0, 1.0),
        rock: textured("rock", 0.9, 0.05, 8.0),
        path_stone,
        lacquer,
        shrine_wood: textured("shrine_wood", 0.9, 0.05, 8.0),
        bark: textured("bark", 0.9, 0.0, 1.0),
        roof_tile: textured("roof_tile", 0.9, 0.3, 24.0),
        sakura_light,
        // Same blossom texture, lit a little less, so canopies have two pinks.
        sakura_dark: textured("sakura", 0.75, 0.05, 4.0),
        water,
        sand: textured("sand", 0.9, 0.0, 1.0),
        plaster: textured("plaster", 0.9, 0.02, 4.0),
        house_roof: textured("house_roof", 0.9, 0.2, 16.0),
        road: textured("road", 0.9, 0.0, 1.0),
        paper: textured("paper", 0.95, 0.0, 1.0),
    }
}

/// Height of the land's top block at depth z: the flat hill top at the back,
/// a slope in the middle, the flat town at the front.
fn terrain_top(z: i32) -> i32 {
    if z <= -3 {
        HILL_TOP
    } else {
        let drop = ((z + 3) as f32 * 0.8).round() as i32;
        (HILL_TOP - drop).max(GROUND_TOP)
    }
}

/// Every column: grass on top, 2 layers of dirt, rock below. Seen from the
/// side this gives the "cut-out slice of land" diorama look.
fn build_terrain(scene: &mut Scene, m: &Materials) {
    for x in EDGE_MIN..=EDGE_MAX {
        for z in EDGE_MIN..=EDGE_MAX {
            let top = terrain_top(z);
            for y in 0..=top {
                let material = if y == top {
                    m.grass
                } else if y >= top - 2 {
                    m.dirt
                } else {
                    m.rock
                };
                scene.add_block(x, y, z, material);
            }
        }
    }
}

/// The long staircase from the town up to the shrine: 12 steps, each one block
/// down and one block forward, cut slightly into the slope.
fn build_staircase(scene: &mut Scene, m: &Materials) {
    for step in 1..=12 {
        let z = -3 + step;
        let step_top = HILL_TOP - step;
        for x in -2..=1 {
            scene.clear((x, step_top + 1, z), (x, HILL_TOP, z));
            scene.add_block(x, step_top, z, m.path_stone);
        }
    }
}

fn build_walkway_and_lanterns(scene: &mut Scene, m: &Materials) {
    // Stone path from the top of the stairs to the shrine hall.
    scene.fill((-1, HILL_TOP, -14), (0, HILL_TOP, -3), m.path_stone);

    // Stone lanterns (toro) on both sides of the path.
    for x in [-4, 3] {
        for z in [-12, -8] {
            scene.fill((x, HILL_TOP + 1, z), (x, HILL_TOP + 2, z), m.rock);
            scene.add_block(x, HILL_TOP + 3, z, m.paper);
            scene.add_block(x, HILL_TOP + 4, z, m.rock);
        }
    }
}

/// Torii gate at depth z, standing across the path.
fn build_torii(scene: &mut Scene, m: &Materials, z: i32) {
    let base = HILL_TOP + 1;
    scene.fill((-3, base, z), (-3, base + 4, z), m.lacquer); // left pillar
    scene.fill((2, base, z), (2, base + 4, z), m.lacquer); // right pillar
    scene.fill((-2, base + 3, z), (1, base + 3, z), m.lacquer); // lower beam (nuki)
    scene.fill((-1, base + 4, z), (0, base + 4, z), m.lacquer); // center strut
    scene.fill((-5, base + 5, z), (4, base + 5, z), m.lacquer); // top beam (kasagi)
    scene.add_block(-5, base + 6, z, m.lacquer); // upturned ends
    scene.add_block(4, base + 6, z, m.lacquer);
}

/// A gable roof over the box x0..=x1, z0..=z1, starting at height `y`.
/// Each layer is one block higher and one block narrower front-to-back,
/// until it meets at the ridge (which runs left-right along X).
fn build_gable_roof(scene: &mut Scene, material: usize, x0: i32, x1: i32, z0: i32, z1: i32, y: i32) {
    let mut layer = 0;
    while z0 - 1 + layer <= z1 + 1 - layer {
        scene.fill((x0 - 1, y + layer, z0 - 1 + layer), (x1 + 1, y + layer, z1 + 1 - layer), material);
        layer += 1;
    }
}

fn build_shrine_hall(scene: &mut Scene, m: &Materials) {
    let floor = HILL_TOP + 1;
    scene.fill((-6, floor, -22), (5, floor, -15), m.path_stone); // raised stone base
    scene.fill((-5, floor + 1, -21), (4, floor + 3, -16), m.shrine_wood); // hall body
    // Red lacquered posts at the front corners and entrance.
    for x in [-5, -2, 1, 4] {
        scene.fill((x, floor + 1, -16), (x, floor + 3, -16), m.lacquer);
    }
    build_gable_roof(scene, m.roof_tile, -6, 5, -22, -15, floor + 4);
}

fn build_pond_and_bridge(scene: &mut Scene, m: &Materials) {
    let (center_x, center_z, radius) = (11.0, -10.0, 4.5);
    for x in 6..=16 {
        for z in -15..=-5 {
            let dx = (x as f32 + 0.5 - center_x) / radius;
            let dz = (z as f32 + 0.5 - center_z) / radius;
            if dx * dx + dz * dz <= 1.0 {
                scene.remove_block(x, HILL_TOP, z); // pond surface sits 1 below the grass
                scene.fill((x, HILL_TOP - 2, z), (x, HILL_TOP - 1, z), m.water);
                scene.add_block(x, HILL_TOP - 3, z, m.sand);
            }
        }
    }
    // Rocks poking out of the water.
    scene.add_block(13, HILL_TOP - 1, -7, m.rock);
    scene.add_block(8, HILL_TOP - 1, -12, m.rock);

    // Arched wooden bridge across the pond (along X at z = -10).
    for x in 6..=16 {
        let height = if (8..=14).contains(&x) { HILL_TOP + 1 } else { HILL_TOP };
        scene.add_block(x, height, -10, m.shrine_wood);
    }
}

/// Rock outcrop in the back-left corner, with a low opening (the future cave).
fn build_cave_rocks(scene: &mut Scene, m: &Materials) {
    for x in -24..=-18 {
        for z in -24..=-18 {
            let distance_from_corner = (x + 24).max(z + 24);
            let height = 5 - distance_from_corner / 2;
            scene.fill((x, HILL_TOP + 1, z), (x, HILL_TOP + height, z), m.rock);
        }
    }
    scene.clear((-20, HILL_TOP + 1, -21), (-18, HILL_TOP + 2, -20)); // cave mouth
}

fn build_town(scene: &mut Scene, m: &Materials) {
    // Streets: one running from the stairs to the front, one crossing it.
    // First cut through the bottom of the slope so the stairs meet the street.
    scene.clear((-2, GROUND_TOP + 1, 10), (1, HILL_TOP, 11));
    scene.fill((-2, GROUND_TOP, 10), (1, GROUND_TOP, EDGE_MAX), m.road);
    scene.fill((EDGE_MIN, GROUND_TOP, 17), (EDGE_MAX, GROUND_TOP, 18), m.road);

    // Houses and shops: (x0, z0, x1, z1)
    for (x0, z0, x1, z1) in [
        (-10, 12, -5, 15),
        (-20, 12, -13, 15),
        (4, 12, 9, 15),
        (12, 12, 19, 15),
        (-12, 20, -5, 22),
        (5, 20, 12, 22),
        (-21, 20, -15, 22),
    ] {
        scene.fill((x0, GROUND_TOP + 1, z0), (x1, GROUND_TOP + 3, z1), m.plaster);
        build_gable_roof(scene, m.house_roof, x0, x1, z0, z1, GROUND_TOP + 4);
    }

    // Utility poles along the main street.
    for (x, z) in [(2, 13), (2, 21), (-3, 19)] {
        scene.fill((x, GROUND_TOP + 1, z), (x, GROUND_TOP + 7, z), m.bark);
    }
}

fn build_sakura_tree(scene: &mut Scene, m: &Materials, x: i32, z: i32, ground: i32, trunk_height: i32, radius: i32) {
    let trunk_top = ground + trunk_height;
    scene.fill((x, ground + 1, z), (x, trunk_top, z), m.bark);

    // Canopy: a squashed ball of blossoms, with random gaps and two pinks.
    let r = radius as f32;
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                let squashed_dy = dy as f32 * 1.4; // wider than tall
                let distance = ((dx * dx + dz * dz) as f32 + squashed_dy * squashed_dy).sqrt();
                let (bx, by, bz) = (x + dx, trunk_top + dy, z + dz);
                let roll = noise(bx, by, bz);
                // Near the edge, leave random gaps so the outline looks fluffy.
                if distance > r || (distance > r - 1.2 && roll < 35) {
                    continue;
                }
                let pink = if roll % 3 == 0 { m.sakura_dark } else { m.sakura_light };
                scene.add_block_if_empty(bx, by, bz, pink);
            }
        }
    }
}

/// A repeatable "random" number from 0 to 99 for a block position.
/// Same position = same number every run, so the scene never changes.
fn noise(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(73_856_093)
        ^ (y as u32).wrapping_mul(19_349_663)
        ^ (z as u32).wrapping_mul(83_492_791);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    h % 100
}
