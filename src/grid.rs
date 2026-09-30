use crate::cube::{Cube, Hit};
use crate::ray::Ray;
use crate::vec3::Vec3;

/// The world stored like Minecraft: a 3D grid of 1×1×1 cells, each either
/// empty (air) or filled with one material.
///
/// Why: instead of testing a ray against every block, we walk it through the
/// grid one cell at a time and only look at the cells it actually passes
/// through. The cost depends on how FAR the ray travels, not on how many
/// blocks exist, so 20,000 blocks cost about the same as 200.
pub struct BlockGrid {
    /// World position of the grid's lowest corner cell.
    min_corner: [i32; 3],
    /// Number of cells along X, Y and Z.
    size: [i32; 3],
    /// `None` = air, `Some(m)` = a block with material number m.
    cells: Vec<Option<u8>>,
    /// The grid's outer box, used to find where an outside ray enters the grid.
    bounds: Cube,
}

impl BlockGrid {
    pub fn new(min_corner: [i32; 3], size: [i32; 3]) -> BlockGrid {
        let cell_count = (size[0] * size[1] * size[2]) as usize;
        let min = Vec3::new(min_corner[0] as f32, min_corner[1] as f32, min_corner[2] as f32);
        let max = min + Vec3::new(size[0] as f32, size[1] as f32, size[2] as f32);
        BlockGrid {
            min_corner,
            size,
            cells: vec![None; cell_count],
            bounds: Cube::new(min, max, 0),
        }
    }

    /// Converts a world block position to a cell position inside the grid.
    fn world_to_cell(&self, world: [i32; 3]) -> [i32; 3] {
        [
            world[0] - self.min_corner[0],
            world[1] - self.min_corner[1],
            world[2] - self.min_corner[2],
        ]
    }

    /// Where a cell lives in `cells` (X changes fastest), or None if outside the grid.
    fn index(&self, cell: [i32; 3]) -> Option<usize> {
        for axis in 0..3 {
            if cell[axis] < 0 || cell[axis] >= self.size[axis] {
                return None;
            }
        }
        Some(((cell[1] * self.size[2] + cell[2]) * self.size[0] + cell[0]) as usize)
    }

    fn cell(&self, cell: [i32; 3]) -> Option<u8> {
        self.index(cell).and_then(|index| self.cells[index])
    }

    pub fn contains(&self, world: [i32; 3]) -> bool {
        self.index(self.world_to_cell(world)).is_some()
    }

    pub fn get(&self, world: [i32; 3]) -> Option<u8> {
        self.cell(self.world_to_cell(world))
    }

    /// Puts a block (Some) or air (None) at a world position.
    pub fn set(&mut self, world: [i32; 3], material: Option<u8>) {
        let index = self
            .index(self.world_to_cell(world))
            .unwrap_or_else(|| panic!("block {world:?} is outside the grid"));
        self.cells[index] = material;
    }

    pub fn block_count(&self) -> usize {
        self.cells.iter().filter(|cell| cell.is_some()).count()
    }

    /// Grid walk ("DDA", Amanatides & Woo 1987). Returns the first solid block
    /// the ray ENTERS with t between t_min and t_max.
    ///
    /// Idea: along each axis, the ray crosses a cell wall every `t_delta` units
    /// of t. We keep, per axis, the t of the NEXT wall (`t_next`). Each step we
    /// cross whichever wall comes first, move one cell that way, and check it.
    pub fn trace(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        // 1. Where does the walk start? Inside the grid: at the ray's start.
        //    Outside: where the ray enters the grid's outer box (slab method).
        let (t_start, entry_normal) = if self.bounds.contains(ray.at(t_min)) {
            (t_min, None)
        } else {
            let entry = self.bounds.intersect(ray, t_min, t_max)?;
            (entry.t, Some(entry.normal))
        };

        // Work in grid coordinates, where cell (0,0,0) spans 0..1 on every axis.
        let origin = (ray.origin - self.bounds.min).to_array();
        let start = (ray.at(t_start) - self.bounds.min).to_array();
        let direction = ray.direction.to_array();

        let mut cell = [0i32; 3];
        let mut step = [0i32; 3];
        let mut t_next = [f32::INFINITY; 3];
        let mut t_delta = [f32::INFINITY; 3];
        for axis in 0..3 {
            // clamp: a start exactly on the far wall would otherwise floor to one past the end
            cell[axis] = (start[axis].floor() as i32).clamp(0, self.size[axis] - 1);
            if direction[axis] > 0.0 {
                step[axis] = 1;
                t_delta[axis] = 1.0 / direction[axis];
                t_next[axis] = (cell[axis] as f32 + 1.0 - origin[axis]) / direction[axis];
            } else if direction[axis] < 0.0 {
                step[axis] = -1;
                t_delta[axis] = -1.0 / direction[axis];
                t_next[axis] = (cell[axis] as f32 - origin[axis]) / direction[axis];
            }
            // direction 0: never crosses a wall on this axis, t_next stays infinite
        }

        // 2. Coming from outside, the very first cell may already be solid.
        //    (Starting inside, we skip the start cell: shadow rays start in air.)
        if let Some(normal) = entry_normal {
            if let Some(material) = self.cell(cell) {
                return Some(self.make_hit(ray, t_start, normal, material));
            }
        }

        // 3. Step from cell to cell until we find a block or leave the grid.
        loop {
            let axis = if t_next[0] < t_next[1] {
                if t_next[0] < t_next[2] { 0 } else { 2 }
            } else if t_next[1] < t_next[2] {
                1
            } else {
                2
            };
            let t = t_next[axis];
            if t >= t_max {
                return None;
            }

            cell[axis] += step[axis];
            if cell[axis] < 0 || cell[axis] >= self.size[axis] {
                return None; // walked out of the grid without hitting anything
            }
            t_next[axis] += t_delta[axis];

            if let Some(material) = self.cell(cell) {
                if t >= t_min {
                    // We entered this block through the wall on `axis`,
                    // so its face normal points back against our step.
                    let mut normal = [0.0; 3];
                    normal[axis] = -step[axis] as f32;
                    return Some(self.make_hit(ray, t, Vec3::from_array(normal), material));
                }
            }
        }
    }

    fn make_hit(&self, ray: &Ray, t: f32, normal: Vec3, material: u8) -> Hit {
        Hit { t, point: ray.at(t), normal, material: material as usize }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_with_block_at_origin() -> BlockGrid {
        let mut grid = BlockGrid::new([-10, -10, -10], [20, 20, 20]);
        grid.set([0, 0, 0], Some(3));
        grid
    }

    #[test]
    fn ray_from_outside_hits_front_face() {
        let grid = grid_with_block_at_origin();
        let ray = Ray::new(Vec3::new(0.5, 0.5, 30.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = grid.trace(&ray, 0.0, f32::INFINITY).expect("should hit");
        assert!((hit.t - 29.0).abs() < 1e-4); // block's front face is at z = 1
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(hit.material, 3);
    }

    #[test]
    fn ray_from_inside_grid_hits_back_face() {
        let grid = grid_with_block_at_origin();
        let ray = Ray::new(Vec3::new(0.5, 0.5, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let hit = grid.trace(&ray, 0.0, f32::INFINITY).expect("should hit");
        assert!((hit.t - 5.0).abs() < 1e-4); // block's back face is at z = 0
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn distance_limit_is_respected() {
        let grid = grid_with_block_at_origin();
        let ray = Ray::new(Vec3::new(0.5, 0.5, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(grid.trace(&ray, 0.0, 4.0).is_none());
    }

    /// The strongest check: for hundreds of random rays, the grid walk must find
    /// the same distance as the slow "test every block" method from Phase 3.
    #[test]
    fn grid_walk_matches_brute_force() {
        let mut seed: u32 = 12345;
        let mut random = move || {
            // Small linear congruential generator: fine for test data.
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 // 0..1
        };

        let mut grid = BlockGrid::new([-5, -5, -5], [10, 10, 10]);
        let mut cubes = Vec::new();
        for _ in 0..60 {
            let position = [
                (random() * 10.0) as i32 - 5,
                (random() * 10.0) as i32 - 5,
                (random() * 10.0) as i32 - 5,
            ];
            grid.set(position, Some(1));
            let min = Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32);
            cubes.push(Cube::new(min, min + Vec3::new(1.0, 1.0, 1.0), 1));
        }

        for _ in 0..500 {
            // Start outside the grid, aim at a random point inside it.
            let origin = Vec3::new(random() - 0.5, random() - 0.5, random() - 0.5).normalized() * 20.0;
            let target = Vec3::new(random() * 10.0 - 5.0, random() * 10.0 - 5.0, random() * 10.0 - 5.0);
            let ray = Ray::new(origin, target - origin);

            let brute_force = cubes
                .iter()
                .filter_map(|cube| cube.intersect(&ray, 0.0, f32::INFINITY))
                .map(|hit| hit.t)
                .fold(f32::INFINITY, f32::min);
            let walked = grid.trace(&ray, 0.0, f32::INFINITY).map_or(f32::INFINITY, |hit| hit.t);

            assert!(
                (brute_force.is_infinite() && walked.is_infinite()) || (brute_force - walked).abs() < 1e-3,
                "mismatch: brute force {brute_force}, grid {walked}"
            );
        }
    }
}
