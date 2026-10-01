//! Things that move: falling sakura petals and swimming koi. Both are small
//! cubes that live outside the block grid ("particles"), so they can move
//! smoothly instead of jumping a whole block at a time.
//! (Water ripples are done while shading, in renderer.rs.)
//!
//! Everything is a function of TIME, not of the previous frame: given the
//! same time, a petal is always in the same place. So animation never drifts,
//! and a screenshot at "time = 12.5 s" is always the same picture.

use crate::cube::Cube;
use crate::scene::{ParticleGroup, Scene};
use crate::vec3::Vec3;

/// Edge length of a petal particle (a block is 1.0).
const PETAL_SIZE: f32 = 0.22;
/// Wind pushes petals this far sideways over one full fall.
const WIND: (f32, f32) = (1.6, 0.9);

struct FallingPetal {
    /// Where it appears (inside the canopy) and how far down it falls.
    start: Vec3,
    fall_height: f32,
    fall_speed: f32, // blocks per second
    /// Offsets its timing and sway, so petals don't fall in lockstep.
    phase: f32,
}

struct SwimmingKoi {
    /// The fish swims around an oval centered on (center_x, center_z).
    center_x: f32,
    center_z: f32,
    radius_x: f32,
    radius_z: f32,
    depth: f32, // height of the fish's middle
    speed: f32, // radians per second around the oval; negative = the other way
    phase: f32,
}

pub struct Animation {
    petal_trees: Vec<Vec<FallingPetal>>,
    koi: Vec<SwimmingKoi>,
    petal_material: usize,
    /// Koi are orange with white patches.
    koi_orange: usize,
    koi_white: usize,
    random_state: u32,
}

impl Animation {
    pub fn new(petal_material: usize, koi_orange: usize, koi_white: usize) -> Animation {
        Animation {
            petal_trees: Vec::new(),
            koi: Vec::new(),
            petal_material,
            koi_orange,
            koi_white,
            random_state: 2024,
        }
    }

    /// A small repeatable random number generator (0..1), fine for scattering petals.
    fn random(&mut self) -> f32 {
        self.random_state = self.random_state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.random_state >> 8) as f32 / (1u32 << 24) as f32
    }

    /// `count` petals falling from a canopy of `radius` centered at
    /// `canopy_center`, down to the ground height `ground_y`.
    pub fn add_petal_tree(&mut self, canopy_center: Vec3, radius: f32, ground_y: f32, count: usize) {
        let mut petals = Vec::new();
        for _ in 0..count {
            let angle = self.random() * std::f32::consts::TAU;
            let distance = self.random().sqrt() * radius * 0.9; // spread evenly over the disk
            let start = canopy_center
                + Vec3::new(angle.cos() * distance, -self.random() * radius * 0.5, angle.sin() * distance);
            petals.push(FallingPetal {
                start,
                fall_height: (start.y - ground_y).max(1.0),
                fall_speed: 0.6 + self.random() * 0.5,
                phase: self.random() * 100.0,
            });
        }
        self.petal_trees.push(petals);
    }

    pub fn add_koi(&mut self, center: (f32, f32), radii: (f32, f32), depth: f32, speed: f32, phase: f32) {
        self.koi.push(SwimmingKoi {
            center_x: center.0,
            center_z: center.1,
            radius_x: radii.0,
            radius_z: radii.1,
            depth,
            speed,
            phase,
        });
    }

    /// Moves everything to where it should be at `time` seconds: rebuilds the
    /// scene's list of moving particles (one group per tree, one per fish).
    pub fn update(&mut self, scene: &mut Scene, time: f32) {
        scene.time = time;
        scene.particle_groups.clear();
        for tree in &self.petal_trees {
            let petals = tree.iter().map(|petal| petal_cube(petal, time, self.petal_material)).collect();
            scene.particle_groups.push(group_of(petals));
        }
        for koi in &self.koi {
            scene.particle_groups.push(group_of(self.koi_cubes(koi, time)));
        }
    }

    /// A koi built from small cubes (about mini-block size), laid out along its
    /// swimming direction: a white head, a two-tone body, little side fins, and
    /// a forked tail that wags from side to side.
    ///
    /// The cubes themselves never rotate (they stay lined up with the axes,
    /// like every block), but their POSITIONS follow the heading, so from
    /// above the fish clearly points the way it swims, at any angle.
    fn koi_cubes(&self, koi: &SwimmingKoi, time: f32) -> Vec<Cube> {
        let angle = time * koi.speed + koi.phase;
        let center = Vec3::new(koi.center_x + koi.radius_x * angle.cos(), koi.depth, koi.center_z + koi.radius_z * angle.sin());
        // Swimming direction = how the position changes as the angle grows.
        let heading = Vec3::new(-koi.radius_x * angle.sin(), 0.0, koi.radius_z * angle.cos()) * koi.speed.signum();
        let forward = heading.normalized();
        let side = Vec3::new(forward.z, 0.0, -forward.x); // horizontal, perpendicular to forward
        let up = Vec3::new(0.0, 1.0, 0.0);
        let tail_wag = (time * 5.5 + koi.phase).sin() * 0.12;

        let (orange, white) = (self.koi_orange, self.koi_white);
        // Better fish shape: (distance along, distance to side, distance up, size_x, size_y, size_z, material)
        // More detailed, more fish-like proportions
        let parts = [
            // Head (more prominent)
            (0.35, 0.0, 0.0, 0.16, 0.16, 0.18, white),
            // Upper body (longer, tapered)
            (0.12, 0.0, 0.06, 0.24, 0.14, 0.22, orange),
            // White patch (belly)
            (-0.05, 0.0, -0.06, 0.20, 0.10, 0.20, white),
            // Lower body
            (-0.2, 0.0, -0.04, 0.20, 0.12, 0.18, orange),
            // Dorsal fin (top)
            (-0.08, 0.0, 0.15, 0.08, 0.12, 0.06, orange),
            // Side fins (left/right)
            (0.05, 0.18, -0.02, 0.06, 0.08, 0.10, white),
            (0.05, -0.18, -0.02, 0.06, 0.08, 0.10, white),
            // Tail lobes (left, right) - forked and wagging
            (-0.42, 0.12 + tail_wag * 0.5, 0.04, 0.09, 0.09, 0.14, orange),
            (-0.42, -0.12 + tail_wag * 0.5, 0.04, 0.09, 0.09, 0.14, orange),
            // Tail center base
            (-0.35, 0.0, 0.0, 0.08, 0.08, 0.12, orange),
        ];
        const FISH_SCALE: f32 = 1.5;
        parts
            .iter()
            .map(|&(along, across, up_offset, size_x, size_y, size_z, material)| {
                let position = center
                    + forward * (along * FISH_SCALE)
                    + side * (across * FISH_SCALE)
                    + up * (up_offset * FISH_SCALE);
                let half = Vec3::new(size_x, size_y, size_z) * (0.5 * FISH_SCALE);
                Cube::new(position - half, position + half, material)
            })
            .collect()
    }
}

/// Wraps particles in a group with the smallest box that contains them all.
fn group_of(particles: Vec<Cube>) -> ParticleGroup {
    let mut min = particles[0].min;
    let mut max = particles[0].max;
    for particle in &particles {
        min = Vec3::new(min.x.min(particle.min.x), min.y.min(particle.min.y), min.z.min(particle.min.z));
        max = Vec3::new(max.x.max(particle.max.x), max.y.max(particle.max.y), max.z.max(particle.max.z));
    }
    ParticleGroup { bounds: Cube::new(min, max, 0), particles }
}

/// Where a petal is at `time`. It falls at a steady speed, sways in a little
/// loop, and drifts with the wind; when it reaches the ground, the `fract`
/// wraps it back to the top, as if a new petal fell from the tree.
fn petal_cube(petal: &FallingPetal, time: f32, material: usize) -> Cube {
    let progress = ((time * petal.fall_speed + petal.phase) / petal.fall_height).fract(); // 0 = top, 1 = ground
    let sway_x = (time * 1.3 + petal.phase * 5.0).sin() * 0.4;
    let sway_z = (time * 1.1 + petal.phase * 3.0).cos() * 0.4;
    let position = petal.start
        + Vec3::new(sway_x + progress * WIND.0, -progress * petal.fall_height, sway_z + progress * WIND.1);
    let half = PETAL_SIZE / 2.0;
    Cube::new(position - Vec3::new(half, half, half), position + Vec3::new(half, half, half), material)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn petal_stays_between_its_tree_and_the_ground() {
        let petal = FallingPetal { start: Vec3::new(0.0, 10.0, 0.0), fall_height: 6.0, fall_speed: 1.0, phase: 0.3 };
        for step in 0..200 {
            let cube = petal_cube(&petal, step as f32 * 0.1, 0);
            let center_y = (cube.min.y + cube.max.y) / 2.0;
            assert!(center_y <= 10.0 + 1e-4 && center_y >= 4.0 - 1e-4);
        }
    }

    #[test]
    fn same_time_gives_the_same_position() {
        let petal = FallingPetal { start: Vec3::new(1.0, 8.0, 2.0), fall_height: 5.0, fall_speed: 0.8, phase: 7.0 };
        assert_eq!(petal_cube(&petal, 12.5, 0).min, petal_cube(&petal, 12.5, 0).min);
    }
}
