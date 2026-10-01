//! Scene 1: the Mikage Shrine on its hill, with the town below.
//!
//! Layout (X = left/right, Z = back(-)/front(+), Y = up), 64 × 64 blocks:
//!   back   (z -32..-8): the hilltop: shrine hall, torii, lanterns, guardian
//!                       statues, water basin, red bridge over the koi pond,
//!                       sakura and pine trees, and a trail to a mossy cave
//!   middle (z -7..7):   the steep hillside with the long stone staircase
//!   front  (z 8..31):   the town: two streets, houses and shops, poles
//!
//! Every part is built by its own small function below, from plain
//! "fill this box with that material" steps. Reading them top to bottom
//! is reading the scene.

use crate::animation::Animation;
use crate::color::Color;
use crate::material::Material;
use crate::controls::Controls;
use crate::scene::{PointLight, Scene, Sun};
use crate::world::World;
use crate::vec3::Vec3;

/// Top block of the town ground and of the hill.
const GROUND_TOP: i32 = 3;
const HILL_TOP: i32 = 18;
/// Diorama footprint: x and z both go from -32 to 31 (64 × 64 blocks).
const EDGE_MIN: i32 = -32;
const EDGE_MAX: i32 = 31;
/// Where the hillside starts (last row of the flat hilltop) and ends.
const SLOPE_START: i32 = -8;
const SLOPE_END: i32 = 7;

/// Material numbers, so the layout code can say `m.grass` instead of `3`.
struct Materials {
    grass: usize,
    petal_grass: usize,
    petal: usize,
    dirt: usize,
    rock: usize,
    moss_rock: usize,
    path_stone: usize,
    lacquer: usize,
    shrine_wood: usize,
    dark_wood: usize,
    roof_tile: usize,
    sakura_light: usize,
    sakura_dark: usize,
    leaves: usize,
    water: usize,
    sand: usize,
    plaster: usize,
    house_roof: usize,
    road: usize,
    paper: usize,
    koi: usize,
    portal: usize,
    foxfire: usize,
    window: usize,
    awning: usize,
    concrete: usize,
}

/// Builds the scene, plus the Animation that moves its petals and koi.
pub fn build_shrine_world() -> World {
    let sun = Sun {
        direction_to_sun: Vec3::new(0.35, 1.0, 0.8).normalized(),
        color: Color::new(1.0, 0.95, 0.85), // warm afternoon sunlight
    };
    let ambient = Color::new(0.32, 0.33, 0.42); // cool, slightly blue sky light
    let mut scene = Scene::new([EDGE_MIN, 0, EDGE_MIN], [64, 44, 64], sun, ambient);
    let m = add_materials(&mut scene);
    // Pastel afternoon sky. Its painted sun matches `direction_to_sun` above
    // (examples/make_skybox.rs uses the same direction).
    scene.load_skybox("sky");

    // Ground first, then things standing on it, then trees last: trees only
    // fill empty space, so they never cut into buildings.
    build_terrain(&mut scene, &m);
    build_staircase(&mut scene, &m);
    build_hilltop_paths(&mut scene, &m);
    build_torii(&mut scene, &m, 0, SLOPE_START - 2, HILL_TOP + 1, 3, 6); // top of the stairs
    build_torii(&mut scene, &m, 0, SLOPE_END + 2, GROUND_TOP + 1, 4, 6); // bottom, in town
    build_shrine_hall(&mut scene, &m);
    build_water_basin(&mut scene, &m, -8, -13);
    for x in [-3, 3] {
        build_guardian_statue(&mut scene, &m, x, -13);
    }
    for x in [-4, 4] {
        for z in [-16, -20] {
            build_stone_lantern(&mut scene, &m, x, z);
        }
    }
    build_edge_wall(&mut scene, &m);
    build_pond_and_bridge(&mut scene, &m);
    build_cave_hill(&mut scene, &m);
    build_town(&mut scene, &m);

    // Sakura: (x, z, trunk height, canopy radius)
    let mut animation = Animation::new(m.petal, m.koi, m.water);
    for (x, z, trunk, radius) in [(-15, -17, 6, 5), (13, -27, 7, 5), (24, -29, 5, 4), (27, -10, 5, 4), (-24, -3, 4, 4), (-29, 16, 3, 3), (29, 18, 3, 3)] {
        let ground = surface_top(&scene, x, z);
        build_sakura_tree(&mut scene, &m, x, z, ground, trunk, radius);
        // Petals fall from the canopy (centered one block above the trunk top)
        // down to the grass surface (the top of block `ground`).
        let canopy_center = Vec3::new(x as f32 + 0.5, (ground + trunk + 1) as f32 + 0.5, z as f32 + 0.5);
        animation.add_petal_tree(canopy_center, radius as f32, (ground + 1) as f32, (radius * 3) as usize);
    }
    add_swimming_koi(&mut animation);
    // Pines, for deep green contrast next to all the pink.
    for (x, z, height) in [(-29, -11, 9), (28, -31, 8), (26, 2, 7), (-28, 6, 6)] {
        let ground = surface_top(&scene, x, z);
        build_pine_tree(&mut scene, &m, x, z, ground, height);
    }
    World { scene, animation, portal_material: m.portal, arrival }
}

/// Four koi circling the pond (center 15, -16) on ovals of different sizes,
/// at two depths, some clockwise and some counterclockwise.
fn add_swimming_koi(animation: &mut Animation) {
    animation.add_koi((15.0, -15.5), (5.0, 3.0), HILL_TOP - 2, 0.35, 0.0);
    animation.add_koi((15.0, -15.5), (3.5, 2.2), HILL_TOP - 3, -0.45, 2.0);
    animation.add_koi((13.0, -14.0), (2.5, 1.5), HILL_TOP - 2, 0.6, 4.0);
    animation.add_koi((17.0, -16.5), (3.0, 2.0), HILL_TOP - 3, -0.3, 1.0);
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
    // (Plus a slight glow: blossoms are thin, so their shaded undersides stay pink.)
    let sakura_light = scene.add_material(Material { glow: 0.15, ..Material::opaque(sakura_texture, 0.95, 0.05, 4.0) });
    let sakura_dark = scene.add_material(Material { glow: 0.12, ..Material::opaque(sakura_texture, 0.75, 0.05, 4.0) });

    let water_texture = scene.load_texture("water");
    let water = scene.add_material(Material {
        transparency: 0.80,
        reflectivity: 0.25,
        refractive_index: 1.33,
        texture_scroll: Vec3::new(0.15, 0.0, 0.08), // slowly flowing
        ..Material::opaque(water_texture, 0.35, 0.9, 96.0)
    });

    // ================= Supporting materials =================
    // Falling petals: the blossom texture with a soft glow, so they stay pink
    // in the shade (standing in for sunlight shining through thin petals).
    let petal = scene.add_material(Material { glow: 0.5, ..Material::opaque(sakura_texture, 0.95, 0.05, 4.0) });
    // Lantern paper glows a little, even by day.
    let paper_texture = scene.load_texture("paper");
    let paper = scene.add_material(Material { glow: 0.25, ..Material::opaque(paper_texture, 0.95, 0.0, 1.0) });
    // Grass with fallen petals: same sides as grass, petals on top.
    let petal_top = scene.load_texture("petal_grass");
    let petal_grass = scene.add_material(Material {
        texture_top: petal_top,
        texture_bottom: dirt_texture,
        ..Material::opaque(grass_side, 0.9, 0.0, 1.0)
    });
    // Mossy rock: moss on top, moss-edged rock on the sides.
    let moss_side = scene.load_texture("moss_rock");
    let rock_texture = scene.load_texture("rock");
    let moss_rock = scene.add_material(Material {
        texture_top: grass_top,
        texture_bottom: rock_texture,
        ..Material::opaque(moss_side, 0.85, 0.02, 4.0)
    });
    // The portal: bright glowing violet whose swirls drift upward, and the
    // blue fox-fire crystals that light the way to it.
    let portal_texture = scene.load_texture("portal");
    let portal = scene.add_material(Material {
        glow: 1.4,
        texture_scroll: Vec3::new(0.0, 0.5, 0.0),
        ..Material::opaque(portal_texture, 0.9, 0.3, 32.0)
    });
    let foxfire_texture = scene.load_texture("foxfire");
    let foxfire = scene.add_material(Material { glow: 1.2, ..Material::opaque(foxfire_texture, 0.9, 0.2, 16.0) });
    // Window glass: opaque (the houses are solid), but it mirrors the sky.
    let window_texture = scene.load_texture("window");
    let window = scene.add_material(Material {
        reflectivity: 0.3,
        ..Material::opaque(window_texture, 0.8, 0.8, 64.0)
    });

    let mut textured = |name: &str, albedo: f32, specular: f32, shininess: f32| {
        let texture = scene.load_texture(name);
        scene.add_material(Material::opaque(texture, albedo, specular, shininess))
    };
    Materials {
        grass,
        petal_grass,
        dirt: textured("dirt", 0.9, 0.0, 1.0),
        rock: textured("rock", 0.9, 0.05, 8.0),
        moss_rock,
        path_stone,
        lacquer,
        shrine_wood: textured("shrine_wood", 0.9, 0.05, 8.0),
        dark_wood: textured("bark", 0.9, 0.0, 1.0),
        roof_tile: textured("roof_tile", 0.9, 0.3, 24.0),
        sakura_light,
        // Same blossom texture, lit a little less, so canopies have two pinks.
        sakura_dark,
        leaves: textured("leaves", 0.9, 0.05, 8.0),
        water,
        sand: textured("sand", 0.9, 0.0, 1.0),
        plaster: textured("plaster", 0.9, 0.02, 4.0),
        house_roof: textured("house_roof", 0.9, 0.2, 16.0),
        road: textured("road", 0.9, 0.0, 1.0),
        paper,
        petal,
        koi: textured("koi", 0.95, 0.3, 32.0), // wet, slightly shiny scales
        portal,
        foxfire,
        window,
        awning: textured("awning", 0.9, 0.05, 8.0),
        concrete: textured("concrete", 0.9, 0.05, 8.0),
    }
}

// ============================ Land ============================

/// Height of the land's top block at (x, z): the flat hilltop at the back,
/// a steep hillside (one block down per block forward), the flat town.
/// Away from the stairs, the hillside gets gentle lumps so it looks natural
/// instead of like a perfect ramp.
fn terrain_top(x: i32, z: i32) -> i32 {
    if z <= SLOPE_START {
        return HILL_TOP;
    }
    let ramp = HILL_TOP - (z - SLOPE_START);
    // One random bump value per 3×3 patch: -1, 0 or +1 blocks.
    let lump = if x.abs() > 4 { (noise(x.div_euclid(3), 7, z.div_euclid(3)) % 3) as i32 - 1 } else { 0 };
    (ramp + lump).clamp(GROUND_TOP, HILL_TOP)
}

/// Every column: grass on top, 2 layers of dirt, rock below. Seen from the
/// side this gives the "cut-out slice of land" diorama look.
fn build_terrain(scene: &mut Scene, m: &Materials) {
    for x in EDGE_MIN..=EDGE_MAX {
        for z in EDGE_MIN..=EDGE_MAX {
            let top = terrain_top(x, z);
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

/// The top block of the ground (highest solid block) in column (x, z).
fn surface_top(scene: &Scene, x: i32, z: i32) -> i32 {
    (0..44).rev().find(|&y| scene.blocks.get([x, y, z]).is_some()).unwrap_or(0)
}

// ============================ The climb ============================

/// The long staircase from the town up to the shrine: 5 blocks wide, one step
/// per row of the hillside, with low stone walls on both sides topped by
/// small lanterns every few steps.
fn build_staircase(scene: &mut Scene, m: &Materials) {
    for z in SLOPE_START + 1..=SLOPE_END {
        let step_top = HILL_TOP - (z - SLOPE_START);
        for x in -2..=2 {
            scene.clear((x, step_top + 1, z), (x, HILL_TOP + 2, z));
            scene.add_block(x, step_top, z, m.path_stone);
        }
        for x in [-3, 3] {
            scene.clear((x, step_top + 2, z), (x, HILL_TOP + 2, z));
            scene.fill((x, step_top, z), (x, step_top + 1, z), m.path_stone);
            if (z - SLOPE_START) % 4 == 2 {
                scene.add_block(x, step_top + 2, z, m.paper); // a little lantern on the wall
            }
        }
    }
}

/// A low stone wall along the front edge of the hilltop, with red posts,
/// leaving a gap where the stairs arrive.
fn build_edge_wall(scene: &mut Scene, m: &Materials) {
    for x in EDGE_MIN + 2..=EDGE_MAX - 2 {
        if x.abs() <= 3 {
            continue;
        }
        scene.add_block(x, HILL_TOP + 1, SLOPE_START, m.path_stone);
        if x % 4 == 0 {
            scene.add_block(x, HILL_TOP + 2, SLOPE_START, m.lacquer);
        }
    }
}

// ============================ The shrine ============================

/// Stone paths on the hilltop: the main approach to the hall, a plaza in
/// front of it, a side path to the red bridge, and the winding trail to the cave.
fn build_hilltop_paths(scene: &mut Scene, m: &Materials) {
    scene.fill((-1, HILL_TOP, -21), (1, HILL_TOP, SLOPE_START), m.path_stone); // main approach
    scene.fill((-5, HILL_TOP, -22), (5, HILL_TOP, -19), m.path_stone); // plaza
    scene.fill((6, HILL_TOP, -20), (7, HILL_TOP, -18), m.path_stone); // plaza → bridge

    // The cave trail: 2-wide stepping stones through a list of turning points.
    let turns: [(i32, i32); 5] = [(-3, -10), (-11, -9), (-18, -13), (-22, -19), (-23, -23)];
    for pair in turns.windows(2) {
        let ((x0, z0), (x1, z1)) = (pair[0], pair[1]);
        let steps = (x1 - x0).abs().max((z1 - z0).abs());
        for i in 0..=steps {
            let x = x0 + (x1 - x0) * i / steps;
            let z = z0 + (z1 - z0) * i / steps;
            for (dx, dz) in [(0, 0), (1, 0), (0, 1)] {
                // Skip a few stones so it reads as a trail, not a road.
                if noise(x + dx, 3, z + dz) > 15 {
                    scene.add_block(x + dx, HILL_TOP, z + dz, m.path_stone);
                }
            }
        }
    }
}

/// Torii gate centered on x = `center_x` at depth z. `half_width` is the
/// distance from the center to each pillar; `height` is the pillar height.
fn build_torii(scene: &mut Scene, m: &Materials, center_x: i32, z: i32, base: i32, half_width: i32, height: i32) {
    let (left, right) = (center_x - half_width, center_x + half_width);
    let top = base + height - 1;
    scene.fill((left, base, z), (left, top, z), m.lacquer); // pillars
    scene.fill((right, base, z), (right, top, z), m.lacquer);
    scene.fill((left + 1, top - 1, z), (right - 1, top - 1, z), m.lacquer); // lower beam (nuki)
    scene.add_block(center_x, top, z, m.lacquer); // center strut
    scene.fill((left - 2, top + 1, z), (right + 2, top + 1, z), m.dark_wood); // top beam (kasagi), dark cap
    scene.add_block(left - 2, top + 2, z, m.dark_wood); // upturned ends
    scene.add_block(right + 2, top + 2, z, m.dark_wood);
}

/// A gable roof over the box x0..=x1, z0..=z1, starting at height `y`.
/// Each layer is one block higher and one block narrower front-to-back,
/// until it meets at the ridge (which runs left-right along X).
pub(crate) fn build_gable_roof(scene: &mut Scene, material: usize, x0: i32, x1: i32, z0: i32, z1: i32, y: i32) -> i32 {
    let mut layer = 0;
    while z0 - 1 + layer <= z1 + 1 - layer {
        scene.fill((x0 - 1, y + layer, z0 - 1 + layer), (x1 + 1, y + layer, z1 + 1 - layer), material);
        layer += 1;
    }
    y + layer - 1 // height of the ridge
}

/// The main hall: a raised stone base, a wooden veranda all around, walls of
/// wood with paper (shoji) panels between red posts, and a big tiled roof.
fn build_shrine_hall(scene: &mut Scene, m: &Materials) {
    let base = HILL_TOP + 1;
    scene.fill((-9, base, -31), (9, base, -22), m.path_stone); // stone base
    scene.fill((-2, base, -21), (2, base, -21), m.path_stone); // front step
    scene.fill((-8, base + 1, -30), (8, base + 1, -23), m.shrine_wood); // veranda floor
    scene.fill((-2, base + 1, -22), (2, base + 1, -22), m.shrine_wood); // entrance step

    // Walls (4 high), then the front facade drawn over them.
    let (wall_bottom, wall_top) = (base + 2, base + 5);
    scene.fill((-6, wall_bottom, -29), (6, wall_top, -25), m.shrine_wood);
    for x in -6..=6 {
        if [-6, -3, 3, 6].contains(&x) {
            scene.fill((x, wall_bottom, -25), (x, wall_top, -25), m.lacquer); // posts
        } else if x.abs() <= 2 {
            scene.fill((x, wall_bottom, -25), (x, wall_top - 1, -25), m.paper); // sliding doors
        } else {
            scene.fill((x, wall_bottom + 1, -25), (x, wall_top - 1, -25), m.paper); // windows
        }
    }
    for x in [-6, 6] {
        scene.fill((x, wall_bottom + 1, -28), (x, wall_top - 1, -26), m.paper); // side windows
    }
    // Veranda posts holding up the roof.
    for (x, z) in [(-8, -23), (8, -23), (-4, -23), (4, -23), (-8, -30), (8, -30)] {
        scene.fill((x, wall_bottom, z), (x, wall_top, z), m.lacquer);
    }
    // Offering box in front of the doors.
    scene.fill((-1, wall_bottom, -23), (1, wall_bottom, -23), m.dark_wood);

    let ridge = build_gable_roof(scene, m.roof_tile, -8, 8, -30, -23, wall_top + 1);
    // Ridge ornaments at both ends.
    for x in [-9, 9] {
        scene.fill((x, ridge + 1, -27), (x, ridge + 1, -26), m.dark_wood);
    }
}

/// Temizuya: a stone basin of water under a small roof on four red posts.
fn build_water_basin(scene: &mut Scene, m: &Materials, center_x: i32, center_z: i32) {
    let y = HILL_TOP + 1;
    scene.fill((center_x - 1, y, center_z - 1), (center_x + 1, y, center_z + 1), m.rock);
    scene.add_block(center_x, y, center_z, m.water);
    for (dx, dz) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
        scene.fill((center_x + dx, y, center_z + dz), (center_x + dx, y + 2, center_z + dz), m.lacquer);
    }
    // Small pyramid roof: each layer one block smaller on every side.
    for layer in 0..3 {
        let r = 3 - layer;
        scene.fill((center_x - r, y + 3 + layer, center_z - r), (center_x + r, y + 3 + layer, center_z + r), m.roof_tile);
    }
}

/// Komainu (guardian lion-dog) on a pedestal, facing the path.
fn build_guardian_statue(scene: &mut Scene, m: &Materials, x: i32, z: i32) {
    let y = HILL_TOP + 1;
    scene.add_block(x, y, z, m.rock); // pedestal
    scene.fill((x, y + 1, z - 1), (x, y + 1, z), m.path_stone); // body (sitting, 2 long)
    scene.add_block(x, y + 2, z, m.path_stone); // head
}

/// Stone lantern (toro): base, post, glowing paper light box, stone cap.
fn build_stone_lantern(scene: &mut Scene, m: &Materials, x: i32, z: i32) {
    let y = HILL_TOP + 1;
    scene.fill((x, y, z), (x, y + 1, z), m.rock);
    scene.add_block(x, y + 2, z, m.paper);
    scene.add_block(x, y + 3, z, m.path_stone);
}

// ============================ Pond and cave ============================

/// Koi pond (an oval 3 blocks deep) with rocks, koi, lily pads, and an
/// arched red bridge with railings across the middle.
fn build_pond_and_bridge(scene: &mut Scene, m: &Materials) {
    let (center_x, center_z, radius_x, radius_z) = (15.0, -16.0, 7.5, 5.5);
    for x in 7..=23 {
        for z in -22..=-10 {
            let dx = (x as f32 + 0.5 - center_x) / radius_x;
            let dz = (z as f32 + 0.5 - center_z) / radius_z;
            if dx * dx + dz * dz <= 1.0 {
                scene.remove_block(x, HILL_TOP, z); // water surface sits 1 below the grass
                scene.fill((x, HILL_TOP - 3, z), (x, HILL_TOP - 1, z), m.water);
                scene.add_block(x, HILL_TOP - 4, z, m.sand);
            }
        }
    }
    // Rocks on the pond floor, tops just under (or poking out of) the surface.
    scene.fill((11, HILL_TOP - 3, -20), (11, HILL_TOP - 1, -20), m.rock);
    scene.fill((19, HILL_TOP - 3, -13), (20, HILL_TOP - 2, -13), m.rock);
    scene.add_block(18, HILL_TOP - 3, -19, m.rock);
    // Lily pads floating on the surface.
    for (x, z) in [(9, -14), (21, -18), (13, -12)] {
        scene.add_block(x, HILL_TOP - 1, z, m.leaves);
    }
    // (The koi are added by `add_swimming_koi`: they move, so Animation owns them.)

    // Arched bridge along X near the back of the pond (z = -19), so the koi in
    // front stay visible: rises by one block twice toward the middle.
    for x in 6..=24 {
        let from_end = (x - 6).min(24 - x);
        let deck = HILL_TOP + 1 + (from_end / 2).min(2);
        scene.add_block(x, deck, -19, m.shrine_wood);
        for z in [-20, -18] {
            scene.add_block(x, deck, z, m.shrine_wood);
            scene.add_block(x, deck + 1, z, m.lacquer); // red railings
        }
    }
}

/// A mossy rock hill in the back-left corner, with a tunnel leading to a
/// small hidden chamber (the portal goes there later).
fn build_cave_hill(scene: &mut Scene, m: &Materials) {
    for x in EDGE_MIN..=-16 {
        for z in EDGE_MIN..=-16 {
            // Tallest at the corner, sloping down toward the shrine grounds.
            // The square root rounds the top off, so it's a mound, not a cone.
            let distance = (((x - EDGE_MIN) * (x - EDGE_MIN) + (z - EDGE_MIN) * (z - EDGE_MIN)) as f32).sqrt();
            let rise = (1.0 - distance / 17.0).max(0.0).sqrt() * 10.0;
            let height = (rise + (noise(x, 5, z) % 2) as f32).floor() as i32;
            if height <= 0 {
                continue;
            }
            scene.fill((x, HILL_TOP + 1, z), (x, HILL_TOP + height - 1, z), m.rock);
            scene.add_block(x, HILL_TOP + height, z, m.moss_rock);
        }
    }
    // Tunnel along the diagonal from the trail's end toward the corner,
    // 3 wide and 3 tall, opening into a 5×5×4 chamber.
    for i in 0..6 {
        let (x, z) = (-23 - i, -23 - i);
        scene.clear((x - 1, HILL_TOP + 1, z), (x + 1, HILL_TOP + 3, z));
        scene.clear((x, HILL_TOP + 1, z - 1), (x, HILL_TOP + 3, z + 1));
    }
    scene.clear((-30, HILL_TOP + 1, -30), (-26, HILL_TOP + 4, -26));
    // Hanging moss over the entrance.
    for (x, z) in [(-22, -24), (-24, -22), (-23, -23)] {
        scene.add_block_if_empty(x, HILL_TOP + 3, z, m.leaves);
    }

    // The portal: a 3×3 glowing sheet against the chamber's back wall,
    // facing the tunnel. Clicking it travels to the spirit world.
    scene.fill((-30, HILL_TOP + 1, -29), (-30, HILL_TOP + 3, -27), m.portal);
    // Its violet light fills the chamber and spills down the tunnel.
    scene.point_lights.push(PointLight {
        position: Vec3::new(-28.0, HILL_TOP as f32 + 2.5, -27.5),
        color: Color::new(0.9, 0.5, 1.3),
        radius: 10.0,
    });
    // Blue fox-fire crystals on the tunnel floor: a faint trail of glow
    // leading inside, with a soft blue light over them.
    for (x, z) in [(-24, -25), (-25, -24), (-27, -26), (-26, -28)] {
        scene.add_block(x, HILL_TOP + 1, z, m.foxfire);
    }
    scene.point_lights.push(PointLight {
        position: Vec3::new(-24.5, HILL_TOP as f32 + 2.5, -24.5),
        color: Color::new(0.3, 0.6, 1.2),
        radius: 5.0,
    });
}

/// Arriving back from the spirit world: just outside the cave mouth,
/// looking out over the shrine grounds.
pub fn arrival() -> Controls {
    Controls::new(Vec3::new(-18.5, HILL_TOP as f32 + 4.0, -18.5), Vec3::new(-8.0, HILL_TOP as f32 + 1.5, -8.0), 60.0)
}

// ============================ Town ============================

struct House {
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
    floors: i32,
    walls: usize,
    roof: usize,
    /// A shop has a wide front window and a striped awning.
    shop: bool,
    /// True: the door is on the +Z side; false: on the -Z side (facing the street).
    door_toward_front: bool,
}

fn build_town(scene: &mut Scene, m: &Materials) {
    // Streets: one from the bottom of the stairs to the front edge, one crossing it.
    scene.fill((-3, GROUND_TOP, SLOPE_END + 1), (3, GROUND_TOP, EDGE_MAX), m.road);
    scene.fill((EDGE_MIN, GROUND_TOP, 20), (EDGE_MAX, GROUND_TOP, 22), m.road);

    let house = |x0, z0, x1, z1, floors, walls, roof, shop, door_toward_front| House {
        x0, z0, x1, z1, floors, walls, roof, shop, door_toward_front,
    };
    for spec in [
        house(-14, 11, -6, 17, 2, m.plaster, m.house_roof, false, true),
        house(-26, 11, -17, 17, 1, m.shrine_wood, m.roof_tile, true, true),
        house(-13, 24, -6, 30, 2, m.plaster, m.roof_tile, false, false),
        house(-27, 24, -18, 30, 1, m.plaster, m.house_roof, true, false),
        house(6, 11, 14, 17, 2, m.plaster, m.house_roof, false, true),
        house(17, 11, 27, 17, 1, m.shrine_wood, m.roof_tile, true, true),
        house(6, 24, 14, 30, 1, m.plaster, m.house_roof, true, false),
        house(17, 24, 26, 30, 2, m.shrine_wood, m.house_roof, false, false),
    ] {
        build_house(scene, m, &spec);
    }

    // Concrete utility poles along the main street, with a crossarm on top.
    for (x, z) in [(4, 13), (4, 27), (-4, 17), (-4, 25)] {
        scene.fill((x, GROUND_TOP + 1, z), (x, GROUND_TOP + 8, z), m.concrete);
        scene.fill((x - 1, GROUND_TOP + 7, z), (x + 1, GROUND_TOP + 7, z), m.concrete);
    }
}

fn build_house(scene: &mut Scene, m: &Materials, h: &House) {
    let floor_height = 3;
    let bottom = GROUND_TOP + 1;
    let top = GROUND_TOP + floor_height * h.floors;
    scene.fill((h.x0, bottom, h.z0), (h.x1, top, h.z1), h.walls);

    for floor in 0..h.floors {
        let floor_bottom = bottom + floor * floor_height;
        // A wooden band between floors on plaster houses.
        if floor > 0 && h.walls == m.plaster {
            for x in h.x0..=h.x1 {
                scene.add_block(x, floor_bottom, h.z0, m.shrine_wood);
                scene.add_block(x, floor_bottom, h.z1, m.shrine_wood);
            }
        }
        // Windows every other block, all around, in the middle of each floor.
        let y = floor_bottom + 1;
        for x in (h.x0 + 1..h.x1).step_by(2) {
            scene.add_block(x, y, h.z0, m.window);
            scene.add_block(x, y, h.z1, m.window);
        }
        for z in (h.z0 + 1..h.z1).step_by(2) {
            scene.add_block(h.x0, y, z, m.window);
            scene.add_block(h.x1, y, z, m.window);
        }
    }

    // Door (and shop front) on the side facing the street.
    let (front_z, outside_z) = if h.door_toward_front { (h.z1, h.z1 + 1) } else { (h.z0, h.z0 - 1) };
    if h.shop {
        scene.fill((h.x0 + 1, bottom + 1, front_z), (h.x1 - 1, bottom + 1, front_z), m.window);
        scene.fill((h.x0, bottom + 2, outside_z), (h.x1, bottom + 2, outside_z), m.awning);
    }
    let door_x = (h.x0 + h.x1) / 2;
    scene.fill((door_x, bottom, front_z), (door_x, bottom + 1, front_z), m.dark_wood);

    build_gable_roof(scene, h.roof, h.x0, h.x1, h.z0, h.z1, top + 1);
}

// ============================ Trees ============================

fn build_sakura_tree(scene: &mut Scene, m: &Materials, x: i32, z: i32, ground: i32, trunk_height: i32, radius: i32) {
    // Fallen petals on the grass around the tree (before the trunk goes in).
    for dx in -radius - 2..=radius + 2 {
        for dz in -radius - 2..=radius + 2 {
            let (px, pz) = (x + dx, z + dz);
            if dx * dx + dz * dz > (radius + 2) * (radius + 2) || !scene.blocks.contains([px, 0, pz]) {
                continue;
            }
            let top = surface_top(scene, px, pz);
            if scene.blocks.get([px, top, pz]) == Some(m.grass as u8) && noise(px, top, pz) < 55 {
                scene.add_block(px, top, pz, m.petal_grass);
            }
        }
    }

    let trunk_top = ground + trunk_height;
    scene.fill((x, ground + 1, z), (x, trunk_top, z), m.dark_wood);
    // A few branches leaning out from the upper trunk.
    for (bx, bz) in [(1, 0), (-1, 1), (0, -1)] {
        scene.add_block(x + bx, trunk_top - 1, z + bz, m.dark_wood);
        scene.add_block(x + 2 * bx, trunk_top, z + 2 * bz, m.dark_wood);
    }

    // Canopy: a squashed ball of blossoms, with random gaps and two pinks.
    let r = radius as f32;
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                let squashed_dy = dy as f32 * 1.4; // wider than tall
                let distance = ((dx * dx + dz * dz) as f32 + squashed_dy * squashed_dy).sqrt();
                let (bx, by, bz) = (x + dx, trunk_top + 1 + dy, z + dz);
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

/// A pine: a trunk with stacked rings of needles, wide at the bottom and
/// narrowing to a point.
fn build_pine_tree(scene: &mut Scene, m: &Materials, x: i32, z: i32, ground: i32, height: i32) {
    scene.fill((x, ground + 1, z), (x, ground + height - 1, z), m.dark_wood);
    for level in 2..=height {
        let radius = ((height - level) / 2).min(3);
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx * dx + dz * dz <= radius * radius + 1 {
                    scene.add_block_if_empty(x + dx, ground + level, z + dz, m.leaves);
                }
            }
        }
    }
    scene.add_block_if_empty(x, ground + height + 1, z, m.leaves);
}

/// A repeatable "random" number from 0 to 99 for a block position.
/// Same position = same number every run, so the scene never changes.
pub(crate) fn noise(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(73_856_093)
        ^ (y as u32).wrapping_mul(19_349_663)
        ^ (z as u32).wrapping_mul(83_492_791);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    h % 100
}
