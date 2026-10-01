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
use crate::grid::MINI;
use crate::vec3::Vec3;

/// Top block of the town ground and of the hill.
const GROUND_TOP: i32 = 3;
const HILL_TOP: i32 = 18;
/// Diorama footprint: x and z both go from -48 to 47 (96 × 96 blocks) - expanded for more space.
const EDGE_MIN: i32 = -48;
const EDGE_MAX: i32 = 47;
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
    koi_white: usize,
    hydrangea: usize,
    azalea: usize,
    banner: usize,
    red_lantern: usize,
}

/// Builds the scene, plus the Animation that moves its petals and koi.
pub fn build_shrine_world() -> World {
    // Golden hour: a low sun off to the right, so light is warm and shadows long.
    let sun = Sun {
        direction_to_sun: Vec3::new(0.8, 0.45, 0.35).normalized(),
        color: Color::new(1.05, 0.84, 0.62),
    };
    let ambient = Color::new(0.38, 0.33, 0.45); // soft lavender sky light
    let mut scene = Scene::new([EDGE_MIN, 0, EDGE_MIN], [96, 44, 96], sun, ambient);
    let m = add_materials(&mut scene);
    // Golden-hour sky over the sea. Its painted sun matches `direction_to_sun`
    // above (examples/make_skybox.rs uses the same direction).
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
    // Stone lanterns: original 4 corner positions plus 6 more for better lighting
    for x in [-4, 4] {
        for z in [-16, -20] {
            build_stone_lantern(&mut scene, &m, x, HILL_TOP + 1, z);
        }
    }
    // Additional lanterns along the pathways (reduced from 6 to 3)
    for (x, z) in [(0, -25), (-6, -18), (6, -18)] {
        build_stone_lantern(&mut scene, &m, x, HILL_TOP + 1, z);
    }
    build_edge_wall(&mut scene, &m);
    build_pond_and_bridge(&mut scene, &m);
    build_cave_hill(&mut scene, &m);
    build_guide_lights(&mut scene, &m);
    build_town(&mut scene, &m);
    build_stair_banners(&mut scene, &m);

    // Sakura: (x, z, trunk height, canopy radius)
    let mut animation = Animation::new(m.petal, m.koi, m.koi_white);
    // Sakura trees - optimized count (reduced from 25 to 12)
    for (x, z, trunk, radius) in [
        // Large statement trees (reduced)
        (-15, -17, 6, 4), (24, -29, 5, 3), (27, -10, 5, 3),
        // Medium trees (reduced)
        (18, -20, 4, 3), (-10, 5, 3, 2), (15, 8, 4, 3),
        // Small accent trees (reduced)
        (0, -30, 3, 2), (10, -10, 3, 2), (-20, 15, 2, 2), (25, 20, 3, 2),
        // Two more for balance
        (-30, -25, 4, 3), (30, -20, 4, 3),
    ] {
        let ground = surface_top(&scene, x, z);
        build_sakura_tree(&mut scene, &m, x, z, ground, trunk, radius);
        let canopy_center = Vec3::new(x as f32 + 0.5, (ground + trunk + 1) as f32 + 0.5, z as f32 + 0.5);
        animation.add_petal_tree(canopy_center, radius as f32, (ground + 1) as f32, (radius * 2) as usize);
    }
    add_swimming_koi(&mut animation);
    // Pine trees - optimized (reduced from 12 to 6)
    for (x, z, height) in [
        (-29, -11, 7), (28, -31, 7), (26, 2, 6),
        (-35, 8, 6), (-12, -35, 6), (30, 15, 6),
    ] {
        let ground = surface_top(&scene, x, z);
        build_pine_tree(&mut scene, &m, x, z, ground, height);
    }
    // Flowering bushes last: they only fill empty space, around everything else.
    build_flower_bushes(&mut scene, &m);

    // Decorative structures: tea stalls, small shrines, lantern poles
    build_decorative_elements(&mut scene, &m);

    World { scene, animation, portal_material: m.portal, arrival }
}

/// Nine koi circling the pond (center 15, -16) on ovals of different sizes,
/// at two depths, some clockwise and some counterclockwise. The water's
/// surface is at height HILL_TOP; the floor at HILL_TOP - 3.
fn add_swimming_koi(animation: &mut Animation) {
    let (shallow, deep) = (HILL_TOP as f32 - 0.6, HILL_TOP as f32 - 1.6);
    // Outer ring: large, slower
    animation.add_koi((15.0, -15.5), (5.0, 3.0), shallow, 0.35, 0.0);
    animation.add_koi((15.0, -15.5), (5.2, 3.2), deep, -0.28, 1.5);
    // Middle ring: medium, moderate speed
    animation.add_koi((15.0, -15.5), (3.5, 2.2), deep, -0.45, 2.0);
    animation.add_koi((13.0, -14.0), (2.8, 1.8), shallow, 0.6, 4.0);
    animation.add_koi((17.0, -16.5), (3.2, 2.1), deep, -0.38, 3.5);
    // Inner ring: smaller, faster
    animation.add_koi((14.0, -15.0), (4.5, 2.5), deep, 0.25, 5.5);
    animation.add_koi((16.0, -15.0), (2.0, 1.2), shallow, 0.55, 0.8);
    animation.add_koi((15.0, -15.5), (1.5, 1.0), deep, -0.65, 3.0);
    animation.add_koi((14.5, -16.5), (3.0, 2.0), shallow, 0.42, 2.5);
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
    // Koi: wet, slightly shiny scales, orange with white patches. A little glow
    // keeps them bright and readable under the water, even in the trees' shade.
    let koi_texture = scene.load_texture("koi");
    let koi = scene.add_material(Material { glow: 0.45, ..Material::opaque(koi_texture, 0.95, 0.3, 32.0) });
    let white_texture = scene.load_texture("plaster");
    let koi_white = scene.add_material(Material { glow: 0.35, ..Material::opaque(white_texture, 1.0, 0.3, 32.0) });
    // Red paper lanterns hanging from the hall's eaves.
    let red_lantern_texture = scene.load_texture("red_lantern");
    let red_lantern = scene.add_material(Material { glow: 0.5, ..Material::opaque(red_lantern_texture, 0.95, 0.1, 8.0) });
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
        koi,
        portal,
        foxfire,
        red_lantern,
        window,
        awning: textured("awning", 0.9, 0.05, 8.0),
        concrete: textured("concrete", 0.9, 0.05, 8.0),
        koi_white,
        hydrangea: textured("hydrangea", 0.95, 0.02, 4.0),
        azalea: textured("azalea", 0.95, 0.02, 4.0),
        banner: textured("banner", 0.95, 0.0, 1.0),
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
                build_small_lantern(scene, m, x, step_top + 2, z); // sitting on the wall
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

    // Decorative viewing platform on the hilltop with railings
    let viewing_x = (8, 10);
    let viewing_z = (-23, -20);
    scene.fill((viewing_x.0, HILL_TOP, viewing_z.0), (viewing_x.1, HILL_TOP, viewing_z.1), m.path_stone);

    // Characters in front of the shrine
    build_tomoe(scene, &m, -4, 0, -20);      // White fox character on left
    build_nanami(scene, &m, 4, 0, -20);      // Dark-haired character on right
    // Railing: mini-block posts at corners
    for (x, z) in [(viewing_x.0, viewing_z.0), (viewing_x.0, viewing_z.1), (viewing_x.1, viewing_z.0), (viewing_x.1, viewing_z.1)] {
        let (bx, by, bz) = (x * MINI, (HILL_TOP + 1) * MINI, z * MINI);
        scene.fill_mini((bx, by, bz), (bx + 1, by + 3, bz + 1), m.lacquer);
    }

    // The cave trail: 2-wide stepping stones through a list of turning points,
    // ending at the cave's torii.
    let turns: [(i32, i32); 5] = [(-3, -10), (-11, -9), (-17, -11), (-23, -12), (-23, -15)];
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
/// Bigger and more impressive version of the original.
fn build_shrine_hall(scene: &mut Scene, m: &Materials) {
    let base = HILL_TOP + 1;
    // Bigger stone platform
    scene.fill((-10, base, -32), (10, base, -21), m.path_stone); // stone base
    scene.fill((-2, base, -20), (2, base, -20), m.path_stone); // front steps
    scene.fill((-9, base + 1, -31), (9, base + 1, -22), m.shrine_wood); // veranda floor
    scene.fill((-2, base + 1, -21), (2, base + 1, -21), m.shrine_wood); // entrance step

    // Walls (5 high for grander appearance), then the front facade drawn over them.
    let (wall_bottom, wall_top) = (base + 2, base + 6);
    scene.fill((-8, wall_bottom, -30), (8, wall_top, -24), m.shrine_wood); // bigger wall
    for x in -8..=8 {
        if [-8, -4, 4, 8].contains(&x) {
            scene.fill((x, wall_bottom, -24), (x, wall_top, -24), m.lacquer); // posts
        } else if x.abs() <= 2 {
            scene.fill((x, wall_bottom, -24), (x, wall_top - 1, -24), m.paper); // sliding doors
        } else {
            scene.fill((x, wall_bottom + 1, -24), (x, wall_top - 1, -24), m.paper); // windows
        }
    }
    for x in [-8, 8] {
        scene.fill((x, wall_bottom + 1, -28), (x, wall_top - 1, -25), m.paper); // side windows
    }
    // Veranda posts holding up the roof (more posts for bigger structure).
    for (x, z) in [(-9, -22), (9, -22), (-4, -22), (4, -22), (-9, -31), (9, -31), (0, -22)] {
        scene.fill((x, wall_bottom, z), (x, wall_top, z), m.lacquer);
    }
    // Offering box in front of the doors (larger, more prominent)
    scene.fill((-2, wall_bottom, -22), (2, wall_bottom, -22), m.dark_wood);
    scene.fill((-1, wall_bottom + 1, -22), (1, wall_bottom + 1, -22), m.lacquer);

    let ridge = build_gable_roof(scene, m.roof_tile, -9, 9, -31, -22, wall_top + 1);
    // Ridge ornaments at both ends.
    for x in [-10, 10] {
        scene.fill((x, ridge + 1, -27), (x, ridge + 1, -26), m.dark_wood);
    }

    // Red paper lanterns hanging under the front eave (more of them for grander look).
    for x in [-8, -4, 0, 4, 8] {
        let (mx, my, mz) = (x * MINI + 1, wall_top * MINI, -21 * MINI + 1);
        scene.fill_mini((mx, my, mz), (mx + 1, my + 2, mz + 1), m.red_lantern);
        scene.fill_mini((mx, my + 3, mz), (mx + 1, my + 3, mz + 1), m.dark_wood);
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

// ============================ Mini-block details ============================
// Positions inside these functions are in mini-blocks (MINI = 4 per block):
// block (x, y, z) starts at mini (4x, 4y, 4z).

/// Stone lantern (toro), standing in block (x, y, z) and the one above it:
/// a wide base, a thin post, a glowing paper fire-box with stone corners, a
/// roof that sticks out on every side, and a small knob on top.
/// Supports three sizes: 0 = tiny, 1 = normal (default), 2 = large
fn build_stone_lantern(scene: &mut Scene, m: &Materials, x: i32, y: i32, z: i32) {
    build_stone_lantern_size(scene, m, x, y, z, 1);
}

fn build_stone_lantern_size(scene: &mut Scene, m: &Materials, x: i32, y: i32, z: i32, size: i32) {
    let (bx, by, bz) = (x * MINI, y * MINI, z * MINI);
    match size {
        0 => {
            // Tiny lantern: minimal size
            scene.fill_mini((bx + 1, by, bz + 1), (bx + 2, by, bz + 2), m.rock);
            scene.fill_mini((bx + 1, by + 1, bz + 1), (bx + 2, by + 1, bz + 2), m.rock);
            scene.fill_mini((bx + 1, by + 2, bz + 1), (bx + 2, by + 2, bz + 2), m.paper);
            scene.fill_mini((bx + 1, by + 3, bz + 1), (bx + 2, by + 3, bz + 2), m.path_stone);
        }
        2 => {
            // Large lantern: 1.5x size
            scene.fill_mini((bx, by, bz), (bx + 3, by, bz + 3), m.rock);
            scene.fill_mini((bx + 1, by + 1, bz + 1), (bx + 2, by + 3, bz + 2), m.rock);
            scene.fill_mini((bx, by + 4, bz), (bx + 3, by + 5, bz + 3), m.paper);
            for (cx, cz) in [(0, 0), (3, 0), (0, 3), (3, 3)] {
                scene.fill_mini((bx + cx, by + 4, bz + cz), (bx + cx, by + 5, bz + cz), m.rock);
            }
            for mx in bx - 1..=bx + 4 {
                for mz in bz - 1..=bz + 4 {
                    scene.add_mini_if_empty(mx, by + 6, mz, m.path_stone);
                }
            }
            scene.fill_mini((bx, by + 7, bz), (bx + 3, by + 7, bz + 3), m.path_stone);
            scene.fill_mini((bx + 1, by + 8, bz + 1), (bx + 2, by + 8, bz + 2), m.rock);
        }
        _ => {
            // Normal size (default)
            scene.fill_mini((bx, by, bz), (bx + 3, by, bz + 3), m.rock);
            scene.fill_mini((bx + 1, by + 1, bz + 1), (bx + 2, by + 3, bz + 2), m.rock);
            scene.fill_mini((bx, by + 4, bz), (bx + 3, by + 5, bz + 3), m.paper);
            for (cx, cz) in [(0, 0), (3, 0), (0, 3), (3, 3)] {
                scene.fill_mini((bx + cx, by + 4, bz + cz), (bx + cx, by + 5, bz + cz), m.rock);
            }
            for mx in bx - 1..=bx + 4 {
                for mz in bz - 1..=bz + 4 {
                    scene.add_mini_if_empty(mx, by + 6, mz, m.path_stone);
                }
            }
            scene.fill_mini((bx, by + 7, bz), (bx + 3, by + 7, bz + 3), m.path_stone);
            scene.fill_mini((bx + 1, by + 8, bz + 1), (bx + 2, by + 8, bz + 2), m.rock);
        }
    }
}

/// A small lantern for the top of a wall: stone foot, paper light, stone cap.
fn build_small_lantern(scene: &mut Scene, m: &Materials, x: i32, y: i32, z: i32) {
    let (bx, by, bz) = (x * MINI, y * MINI, z * MINI);
    scene.fill_mini((bx + 1, by, bz + 1), (bx + 2, by, bz + 2), m.rock);
    scene.fill_mini((bx + 1, by + 1, bz + 1), (bx + 2, by + 2, bz + 2), m.paper);
    scene.fill_mini((bx, by + 3, bz), (bx + 3, by + 3, bz + 3), m.path_stone);
}

/// A nobori banner standing in block (x, y, z): a thin dark pole, a top bar,
/// and a tall strip of cloth hanging from it, facing along X (toward the stairs).
fn build_banner(scene: &mut Scene, m: &Materials, x: i32, y: i32, z: i32) {
    let (bx, by, bz) = (x * MINI, y * MINI, z * MINI);
    scene.fill_mini((bx + 2, by, bz + 1), (bx + 2, by + 15, bz + 1), m.dark_wood); // pole, 4 blocks tall
    scene.fill_mini((bx + 2, by + 14, bz + 1), (bx + 2, by + 14, bz + 3), m.dark_wood); // top bar
    scene.fill_mini((bx + 2, by + 4, bz + 2), (bx + 2, by + 13, bz + 3), m.banner); // cloth
}

/// Banners along both sides of the staircase, between the wall lanterns.
fn build_stair_banners(scene: &mut Scene, m: &Materials) {
    for z in (SLOPE_START + 3..=SLOPE_END).step_by(4) {
        for x in [-4, 4] {
            let ground = surface_top(scene, x, z);
            build_banner(scene, m, x, ground + 1, z);
        }
    }
}

/// A rounded flowering bush about 2 blocks wide, made of mini-blocks: leaves
/// inside and low down, flowers on the upper outside, with a ragged edge.
fn build_bush(scene: &mut Scene, m: &Materials, x: i32, z: i32, flower: usize) {
    let ground = surface_top(scene, x, z);
    let (cx, base, cz) = (x * MINI + 2, (ground + 1) * MINI, z * MINI + 2);
    for dx in -4i32..=4 {
        for dy in 0i32..=4 {
            for dz in -4i32..=4 {
                // Inside a squashed half-ball: (dx/4.5)² + (dy/4.5)² + (dz/4.5)² ≤ 1
                let distance = ((dx * dx + dy * dy + dz * dz) as f32).sqrt() / 4.5;
                let (mx, my, mz) = (cx + dx, base + dy, cz + dz);
                let roll = noise(mx, my, mz);
                if distance > 1.0 || (distance > 0.75 && roll < 35) {
                    continue;
                }
                let material = if dy >= 2 && roll % 3 != 0 { flower } else { m.leaves };
                scene.add_mini_if_empty(mx, my, mz, material);
            }
        }
    }
}

fn build_flower_bushes(scene: &mut Scene, m: &Materials) {
    let (blue, pink) = (m.hydrangea, m.azalea);
    for (x, z, flower) in [
        // Along the approach and in front of the hall.
        (-7, -11, blue), (6, -11, pink), (-7, -21, pink), (7, -21, blue), (-11, -26, blue), (11, -23, pink),
        // Around the pond.
        (8, -12, blue), (12, -9, pink), (18, -9, blue), (24, -15, pink),
        // Beside the stairs on the hillside.
        (-6, -4, pink), (6, -3, blue), (-6, 2, blue), (6, 3, pink),
        // At the cave entrance.
        (-19, -14, pink), (-27, -14, blue),
        // Front gardens in town.
        (-12, 18, pink), (8, 18, blue), (21, 18, pink), (-22, 18, blue), (-8, 23, blue), (10, 23, pink),
    ] {
        build_bush(scene, m, x, z, flower);
    }
}

/// Lantern poles and tea stalls scattered throughout
fn build_decorative_elements(scene: &mut Scene, m: &Materials) {
    // Lantern poles at key spots
    for (x, z) in [(0, 10), (-35, 15), (35, -10), (-20, 0), (25, 25), (-25, -20)] {
        let ground = surface_top(scene, x, z);
        for dy in 0..4 {
            scene.add_block(x, ground + 1 + dy, z, m.shrine_wood);
        }
        scene.add_block(x, ground + 5, z, m.path_stone);
        let (bx, by, bz) = (x * MINI, (ground + 5) * MINI, z * MINI);
        scene.fill_mini((bx, by, bz), (bx + 3, by + 2, bz + 3), m.red_lantern);
        scene.point_lights.push(PointLight {
            position: Vec3::new(x as f32 + 0.5, (ground + 7) as f32, z as f32 + 0.5),
            color: Color::new(1.1, 0.8, 0.5),
            radius: 8.0,
        });
    }
}

// ============================ Pond and cave ============================

/// Koi pond (an oval 3 blocks deep) with rocks, koi, lily pads, and an
/// arched red bridge with railings across the middle.
fn build_pond_and_bridge(scene: &mut Scene, m: &Materials) {
    // MUCH BIGGER koi pond: expanded to 24×18 blocks
    let (center_x, center_z, radius_x, radius_z) = (16.0, -15.0, 12.0, 9.0);
    for x in 4..=28 {
        for z in -24..=-6 {
            let dx = (x as f32 + 0.5 - center_x) / radius_x;
            let dz = (z as f32 + 0.5 - center_z) / radius_z;
            if dx * dx + dz * dz <= 1.0 {
                scene.remove_block(x, HILL_TOP, z); // water surface
                scene.fill((x, HILL_TOP - 4, z), (x, HILL_TOP - 1, z), m.water); // deeper pond
                scene.add_block(x, HILL_TOP - 5, z, m.sand);
            }
        }
    }

    // Rocks and stones on the pond floor for visual interest
    for (x, z) in [(8, -20), (24, -18), (12, -10), (20, -12), (6, -14)] {
        scene.fill((x, HILL_TOP - 4, z), (x, HILL_TOP - 2, z), m.rock);
    }

    // More lily pads scattered across the pond (increased from 6 to 10)
    for (x, z) in [(7, -14), (10, -18), (23, -20), (15, -10), (18, -16), (11, -8), (9, -22), (25, -12), (13, -24), (19, -20)] {
        scene.add_block(x, HILL_TOP - 1, z, m.leaves);
    }

    // Stepping stones across the pond for a traditional garden aesthetic (now 8)
    for (x, z) in [(8, -17), (14, -17), (20, -17), (16, -12), (16, -22), (22, -20)] {
        scene.add_block(x, HILL_TOP, z, m.path_stone);
    }

    // Rocks on the pond floor (optimized)
    for (x, z) in [(8, -20), (24, -18), (12, -10), (20, -12), (6, -14)] {
        scene.fill((x, HILL_TOP - 4, z), (x, HILL_TOP - 2, z), m.rock);
    }

    // (The koi are added by Animation - 9 of them swimming through this bigger space)

    // Bigger arched bridge (z = -19), now with wider approach
    for x in 3..=29 {
        let from_end = (x - 3).min(29 - x);
        let deck = HILL_TOP + 1 + (from_end / 3).min(2); // gentler arch
        scene.fill((x, deck, -21), (x, deck, -17), m.shrine_wood);

        // Red railings on both outer edges
        let top = (deck + 1) * MINI;
        for rail_z in [-21 * MINI, -17 * MINI + 3] {
            if x % 2 == 0 {
                scene.fill_mini((x * MINI + 1, top, rail_z), (x * MINI + 2, top + 2, rail_z), m.lacquer);
            }
            scene.fill_mini((x * MINI, top + 3, rail_z), (x * MINI + 3, top + 3, rail_z), m.lacquer);
        }
    }
}

/// Subtle guide lights marking the path to the cave - lanterns creating a subtle trail
fn build_guide_lights(scene: &mut Scene, m: &Materials) {
    // Only mark the cave entrance itself, don't clutter the path
    let cave_entrance_x = -23;
    let cave_entrance_z = -13;
    let y = HILL_TOP + 1;

    // Stone lantern right at the cave entrance (large for prominence)
    build_stone_lantern_size(scene, m, cave_entrance_x, y, cave_entrance_z, 2);

    // Two additional guide lanterns forming a subtle path toward the cave (varied sizes)
    build_stone_lantern_size(scene, m, -15, y, -15, 1);
    build_stone_lantern_size(scene, m, -20, y, -12, 0);
}

/// More lanterns and flowers around the shrine area to make it feel grander
/// and more detailed: lanterns lighting the pathways and flower beds.
/// A mossy rock hill in the back-left corner. A straight tunnel runs into it
/// from the front, and the portal glows at its far end, so you can see the
/// light from the shrine grounds. A red torii and lanterns mark the way.
fn build_cave_hill(scene: &mut Scene, m: &Materials) {
    for x in EDGE_MIN..=-11 {
        for z in EDGE_MIN..=-11 {
            // Tallest at the corner, sloping down toward the shrine grounds.
            // The square root rounds the top off, so it's a mound, not a cone.
            let distance = (((x - EDGE_MIN) * (x - EDGE_MIN) + (z - EDGE_MIN) * (z - EDGE_MIN)) as f32).sqrt();
            let rise = (1.0 - distance / 21.0).max(0.0).sqrt() * 11.0;
            let height = (rise + (noise(x, 5, z) % 2) as f32).floor() as i32;
            if height <= 0 {
                continue;
            }
            scene.fill((x, HILL_TOP + 1, z), (x, HILL_TOP + height - 1, z), m.rock);
            scene.add_block(x, HILL_TOP + height, z, m.moss_rock);
        }
    }

    // The tunnel: 3 wide, 3 tall, straight back along -Z from the hill's front.
    let (tunnel_x0, tunnel_x1, portal_z) = (-24, -22, -26);
    scene.clear((tunnel_x0, HILL_TOP + 1, portal_z + 1), (tunnel_x1, HILL_TOP + 3, -14));
    scene.fill((tunnel_x0, HILL_TOP, portal_z + 1), (tunnel_x1, HILL_TOP, -14), m.path_stone); // its floor

    // The portal fills the tunnel's far end. Clicking it travels to the spirit world.
    scene.fill((tunnel_x0, HILL_TOP + 1, portal_z), (tunnel_x1, HILL_TOP + 3, portal_z), m.portal);
    // Its violet light fills the tunnel and spills out of the entrance.
    scene.point_lights.push(PointLight {
        position: Vec3::new(-22.5, HILL_TOP as f32 + 2.5, portal_z as f32 + 2.5),
        color: Color::new(1.0, 0.55, 1.4),
        radius: 12.0,
    });

    // Blue fox-fire crystals in little clusters along the tunnel walls.
    for (x, z) in [(-24, -17), (-22, -20), (-24, -23)] {
        let (bx, by, bz) = (x * MINI, (HILL_TOP + 1) * MINI, z * MINI);
        let inner = if x == tunnel_x0 { bx } else { bx + 2 }; // against the wall
        scene.fill_mini((inner, by, bz + 1), (inner + 1, by + 2, bz + 2), m.foxfire);
        scene.add_mini_if_empty(inner, by, bz + 3, m.foxfire);
    }

    // Hanging moss along the entrance.
    for x in tunnel_x0..=tunnel_x1 {
        let (bx, by, bz) = (x * MINI, (HILL_TOP + 3) * MINI, -15 * MINI);
        for dx in 0..MINI {
            let length = 1 + (noise(bx + dx, 9, bz) % 3) as i32;
            scene.fill_mini((bx + dx, by + MINI - length, bz + 3), (bx + dx, by + MINI - 1, bz + 3), m.leaves);
        }
    }

    // A small red torii in front of the entrance, and lanterns leading to it.
    build_torii(scene, m, -23, -13, HILL_TOP + 1, 2, 4);
    for (x, z) in [(-9, -11), (-15, -9), (-20, -10), (-21, -14), (-27, -11)] {
        build_stone_lantern(scene, m, x, HILL_TOP + 1, z);
    }
}

/// Arriving back from the spirit world: in front of the cave's torii,
/// looking out over the shrine grounds.
pub fn arrival() -> Controls {
    Controls::new(Vec3::new(-22.5, HILL_TOP as f32 + 3.5, -10.0), Vec3::new(-5.0, HILL_TOP as f32 + 1.5, -18.0), 60.0)
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

    // Small shrines and trading posts for town atmosphere
    build_town_shrines_and_markets(scene, m);

    let house = |x0, z0, x1, z1, floors, walls, roof, shop, door_toward_front| House {
        x0, z0, x1, z1, floors, walls, roof, shop, door_toward_front,
    };
    for spec in [
        // Left side: main street (optimized)
        house(-14, 11, -6, 17, 2, m.plaster, m.house_roof, false, true),
        house(-26, 11, -17, 17, 1, m.shrine_wood, m.roof_tile, true, true),
        house(-13, 24, -6, 30, 2, m.plaster, m.roof_tile, false, false),
        house(-30, 16, -22, 22, 2, m.plaster, m.house_roof, true, false),
        // Right side: main street (optimized)
        house(6, 11, 14, 17, 2, m.plaster, m.house_roof, false, true),
        house(17, 11, 27, 17, 2, m.shrine_wood, m.roof_tile, true, true),
        house(17, 24, 26, 30, 2, m.shrine_wood, m.house_roof, false, false),
        house(22, 16, 30, 22, 2, m.plaster, m.house_roof, true, true),
        // Back expansion (reduced)
        house(-25, -2, -15, 3, 2, m.plaster, m.house_roof, false, true),
        house(15, -2, 25, 3, 1, m.plaster, m.roof_tile, true, false),
        // Far corners (reduced)
        house(-40, -15, -32, -8, 1, m.shrine_wood, m.roof_tile, true, false),
        house(32, -15, 40, -8, 1, m.plaster, m.house_roof, true, true),
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

    // Mini-block decorative window frames and details
    for x in h.x0..=h.x1 {
        for floor in 0..h.floors {
            let floor_y = bottom + floor * floor_height;
            for y_offset in 1..=2 {
                if scene.blocks.get([x, floor_y + y_offset, h.z0]) == Some(m.window as u8) {
                    let (bx, by, bz) = (x * MINI, (floor_y + y_offset) * MINI, h.z0 * MINI);
                    // Window frame detail using dark wood
                    scene.fill_mini((bx, by, bz), (bx + 3, by + 3, bz), m.dark_wood);
                    scene.fill_mini((bx + 1, by + 1, bz + 1), (bx + 2, by + 2, bz - 1), m.window);
                }
            }
        }
    }

    build_gable_roof(scene, h.roof, h.x0, h.x1, h.z0, h.z1, top + 1);
}

/// Small decorative shrines and market stalls scattered through town
fn build_town_shrines_and_markets(scene: &mut Scene, m: &Materials) {
    // Small roadside shrines: 2×2 base with a tiny roof
    for (x, z) in [(-20, 6), (22, 6), (-18, -8), (20, -8)] {
        let y = GROUND_TOP + 1;
        // Small base
        scene.fill((x, y, z), (x + 1, y, z + 1), m.path_stone);
        // Shrine structure
        scene.add_block(x, y + 1, z, m.shrine_wood);
        scene.add_block(x, y + 2, z, m.roof_tile);
        scene.add_block(x, y + 1, z + 1, m.dark_wood);
        // Tiny roof
        scene.add_block(x - 1, y + 2, z, m.roof_tile);
        scene.add_block(x + 2, y + 2, z, m.roof_tile);
    }

    // Market stalls: counters with red/white striped awnings (using mini-blocks)
    for (x, z) in [(-12, 0), (12, 0), (-24, 10), (24, 10)] {
        let (bx, by, bz) = (x * MINI, GROUND_TOP * MINI, z * MINI);
        // Counter top
        scene.fill_mini((bx, by, bz), (bx + 3, by + 1, bz + 3), m.shrine_wood);
        // Awning posts
        scene.fill_mini((bx - 1, by + 2, bz - 1), (bx - 1, by + 3, bz - 1), m.dark_wood);
        scene.fill_mini((bx + 4, by + 2, bz - 1), (bx + 4, by + 3, bz - 1), m.dark_wood);
        // Awning fabric (alternating)
        for by_offset in 0..2 {
            scene.fill_mini((bx - 1, by + 4 + by_offset, bz - 1), (bx + 4, by + 4 + by_offset, bz + 4), m.awning);
        }
    }
}

// ============================ Trees ============================

fn build_sakura_tree(scene: &mut Scene, m: &Materials, x: i32, z: i32, ground: i32, trunk_height: i32, radius: i32) {
    // Fallen petals on the grass around the tree (before the trunk goes in).
    for dx in -radius - 1..=radius + 1 {
        for dz in -radius - 1..=radius + 1 {
            let (px, pz) = (x + dx, z + dz);
            if dx * dx + dz * dz > (radius + 1) * (radius + 1) || !scene.blocks.contains([px, 0, pz]) {
                continue;
            }
            let top = surface_top(scene, px, pz);
            if scene.blocks.get([px, top, pz]) == Some(m.grass as u8) && noise(px, top, pz) < 40 {
                scene.add_block(px, top, pz, m.petal_grass);
            }
        }
    }

    let trunk_top = ground + trunk_height;
    // Trunk: wider at base, tapers toward top
    scene.add_block(x, ground + 1, z, m.dark_wood);
    scene.fill((x, ground + 2, z), (x, trunk_top - 2, z), m.dark_wood);

    // Major branches spreading out from the upper trunk: 4 directions, getting shorter
    for (bx, bz, height_down) in [(1, 0, 2), (-1, 0, 2), (0, 1, 2), (0, -1, 2)] {
        let branch_height = trunk_top - height_down;
        scene.add_block(x + bx, branch_height, z + bz, m.dark_wood);
        scene.add_block(x + 2 * bx, branch_height + 1, z + 2 * bz, m.dark_wood);
    }

    // Canopy: dense at center, sparse at edges. Slightly asymmetric for natural look.
    let r = radius as f32;
    for dx in -radius..=radius {
        for dy in 0..=radius {
            for dz in -radius..=radius {
                let distance = ((dx * dx + dz * dz) as f32 + (dy as f32 * 0.8) * (dy as f32 * 0.8)).sqrt();
                let (bx, by, bz) = (x + dx, trunk_top + dy, z + dz);
                let roll = noise(bx, by, bz);

                // Denser core, sparser edges
                if distance > r {
                    continue;
                }
                if distance > r - 0.8 && roll < 45 {
                    continue; // Random gaps at edges
                }

                // Two-tone pinks for visual depth
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

/// Tomoe: white fox character with fluffy appearance
fn build_tomoe(scene: &mut Scene, m: &Materials, x: i32, y: i32, z: i32) {
    let base = y;

    // Head (white, fluffy)
    scene.fill((x - 1, base + 3, z - 1), (x + 1, base + 5, z + 1), m.plaster);

    // Ears (pointed white)
    scene.fill((x - 1, base + 5, z - 1), (x, base + 7, z), m.plaster);
    scene.fill((x, base + 5, z - 1), (x + 1, base + 7, z), m.plaster);

    // Body (upper)
    scene.fill((x - 1, base + 1, z - 1), (x + 1, base + 3, z + 1), m.plaster);

    // Lower body (slightly tapered)
    scene.fill((x - 1, base - 1, z - 1), (x + 1, base + 1, z + 1), m.plaster);

    // Arms (simplified)
    scene.fill((x - 2, base + 1, z - 1), (x - 1, base + 2, z), m.plaster);
    scene.fill((x + 1, base + 1, z - 1), (x + 2, base + 2, z), m.plaster);

    // Tail (fluffy white, curved)
    scene.fill((x + 2, base + 1, z - 2), (x + 3, base + 2, z - 1), m.plaster);
    scene.fill((x + 2, base, z - 3), (x + 3, base + 1, z - 2), m.plaster);
}

/// Nanami: dark-haired human character with brown outfit
fn build_nanami(scene: &mut Scene, m: &Materials, x: i32, y: i32, z: i32) {
    let base = y;
    let dark = m.shrine_wood;  // Dark material for hair
    let skin = m.plaster;      // Light material for face/arms
    let outfit = m.roof_tile;  // Brown material for outfit

    // Head (skin colored)
    scene.fill((x - 1, base + 3, z - 1), (x + 1, base + 5, z + 1), skin);

    // Hair (long, dark, flowing)
    scene.fill((x - 1, base + 4, z - 2), (x + 1, base + 7, z + 2), dark);

    // Hair sides (longer on sides)
    scene.fill((x - 2, base + 2, z - 1), (x - 1, base + 4, z + 1), dark);
    scene.fill((x + 1, base + 2, z - 1), (x + 2, base + 4, z + 1), dark);

    // Upper body (brown outfit)
    scene.fill((x - 1, base + 1, z - 1), (x + 1, base + 3, z + 1), outfit);

    // Lower body (skirt/pants, brown)
    scene.fill((x - 1, base - 1, z - 1), (x + 1, base + 1, z + 1), outfit);

    // Arms (skin colored)
    scene.fill((x - 2, base + 1, z - 1), (x - 1, base + 2, z), skin);
    scene.fill((x + 1, base + 1, z - 1), (x + 2, base + 2, z), skin);

    // Legs (brown)
    scene.fill((x - 1, base - 2, z - 1), (x - 1, base, z + 1), outfit);
    scene.fill((x, base - 2, z - 1), (x, base, z + 1), outfit);
}
