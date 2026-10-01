use crate::cube::{Cube, Hit};
use crate::ray::Ray;
use crate::vec3::Vec3;

/// How many mini-blocks fit along each side of one normal block.
/// A mini-block is 1/4 of a block wide: a detail cell holds 4 × 4 × 4 = 64.
pub const MINI: i32 = 4;
const MINI_COUNT: usize = (MINI * MINI * MINI) as usize;

/// What one 1×1×1 cell of the world holds.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Cell {
    Air,
    /// A whole block of one material.
    Block(u8),
    /// A cell split into 4×4×4 mini-blocks, for small detailed things
    /// (fish, flowers, lanterns, railings). The number says which entry of
    /// `BlockGrid::details` holds its 64 mini-blocks.
    Detail(u32),
}

/// The world stored like Minecraft: a 3D grid of 1×1×1 cells. Most cells are
/// air or a whole block; a few are split into mini-blocks for fine detail.
///
/// Why a grid: instead of testing a ray against every block, we walk it
/// through the grid one cell at a time and only look at the cells it actually
/// passes through. The cost depends on how FAR the ray travels, not on how
/// many blocks exist.
///
/// Why two levels: walking everything at mini-block size would cost 4× more
/// steps everywhere. Instead the ray walks normal cells, and only when it
/// enters a detail cell does it walk that cell's 4×4×4 mini-grid, with the
/// very same algorithm. Fine detail only costs where it exists.
pub struct BlockGrid {
    /// World position of the grid's lowest corner cell.
    min_corner: [i32; 3],
    /// Number of cells along X, Y and Z.
    size: [i32; 3],
    cells: Vec<Cell>,
    /// Mini-blocks of every detail cell: `None` = air, `Some(m)` = material m.
    /// Index inside one array: x + 4·y + 16·z.
    details: Vec<[Option<u8>; MINI_COUNT]>,
    /// The grid's outer box, used to find where an outside ray enters the grid.
    bounds: Cube,
}

/// Where a grid walk stopped: the wall it crossed and what is on the other side.
pub struct Boundary {
    pub t: f32,
    pub point: Vec3,
    /// Unit normal of the crossed wall, pointing back toward where the ray came from.
    pub normal: Vec3,
    /// What was entered: `None` = air, `Some(m)` = a block or mini-block of material m.
    pub entered: Option<u8>,
}

impl BlockGrid {
    pub fn new(min_corner: [i32; 3], size: [i32; 3]) -> BlockGrid {
        let cell_count = (size[0] * size[1] * size[2]) as usize;
        let min = Vec3::new(min_corner[0] as f32, min_corner[1] as f32, min_corner[2] as f32);
        let max = min + Vec3::new(size[0] as f32, size[1] as f32, size[2] as f32);
        BlockGrid {
            min_corner,
            size,
            cells: vec![Cell::Air; cell_count],
            details: Vec::new(),
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

    fn cell(&self, cell: [i32; 3]) -> Cell {
        self.index(cell).map_or(Cell::Air, |index| self.cells[index])
    }

    pub fn contains(&self, world: [i32; 3]) -> bool {
        self.index(self.world_to_cell(world)).is_some()
    }

    /// The material of a WHOLE block at a world position (None for air, and
    /// also for a cell split into mini-blocks).
    pub fn get(&self, world: [i32; 3]) -> Option<u8> {
        match self.cell(self.world_to_cell(world)) {
            Cell::Block(material) => Some(material),
            _ => None,
        }
    }

    /// True if nothing at all (no block, no mini-blocks) is at this position.
    pub fn is_empty(&self, world: [i32; 3]) -> bool {
        self.cell(self.world_to_cell(world)) == Cell::Air
    }

    /// Puts a whole block (Some) or air (None) at a world position,
    /// replacing any mini-blocks that were there.
    pub fn set(&mut self, world: [i32; 3], material: Option<u8>) {
        let index = self
            .index(self.world_to_cell(world))
            .unwrap_or_else(|| panic!("block {world:?} is outside the grid"));
        self.cells[index] = material.map_or(Cell::Air, Cell::Block);
    }

    /// Puts one mini-block (Some) or mini-air (None). `mini` is in mini-block
    /// units: mini-block (8, 3, 0) is inside world block (2, 0, 0).
    /// If that block was whole, it is split first, keeping its material in
    /// the other 63 mini-blocks (so you can carve into blocks, too).
    pub fn set_mini(&mut self, mini: [i32; 3], material: Option<u8>) {
        let world = mini.map(|m| m.div_euclid(MINI));
        let [sx, sy, sz] = mini.map(|m| m.rem_euclid(MINI));
        let Some(index) = self.index(self.world_to_cell(world)) else {
            return; // outside the world: silently ignore, like add_block_if_empty
        };
        let detail = match self.cells[index] {
            Cell::Detail(detail) => detail as usize,
            whole => {
                let fill = if let Cell::Block(m) = whole { Some(m) } else { None };
                self.details.push([fill; MINI_COUNT]);
                self.cells[index] = Cell::Detail((self.details.len() - 1) as u32);
                self.details.len() - 1
            }
        };
        self.details[detail][(sx + MINI * sy + MINI * MINI * sz) as usize] = material;
    }

    /// Whole blocks plus mini-blocks counted as fractions (64 minis = 1 block).
    pub fn block_count(&self) -> usize {
        self.cells
            .iter()
            .map(|cell| match cell {
                Cell::Air => 0.0,
                Cell::Block(_) => 1.0,
                Cell::Detail(d) => self.details[*d as usize].iter().filter(|m| m.is_some()).count() as f32 / 64.0,
            })
            .sum::<f32>() as usize
    }

    /// The first solid block or mini-block the ray ENTERS with t between t_min and t_max.
    pub fn trace(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        let boundary = self.walk_until(ray, t_min, t_max, |cell| cell.is_some())?;
        let material = boundary.entered.expect("a solid cell always has a material");
        Some(Hit { t: boundary.t, point: boundary.point, normal: boundary.normal, material: material as usize })
    }

    /// Walks the ray cell by cell (and through mini-blocks inside detail cells)
    /// and stops at the first thing ENTERED (t between t_min and t_max) for
    /// which `is_boundary` says yes. Whatever the ray starts inside is skipped.
    ///
    /// Callers decide what counts as a boundary:
    /// - camera rays: any block at all
    /// - shadow rays: only blocks that are not see-through
    /// - rays travelling inside water: anything that is NOT water (air or solid)
    pub fn walk_until(
        &self,
        ray: &Ray,
        t_min: f32,
        t_max: f32,
        is_boundary: impl Fn(Option<u8>) -> bool,
    ) -> Option<Boundary> {
        // 1. Where does the walk start? Inside the grid: at the ray's start.
        //    Outside: where the ray enters the grid's outer box (slab method).
        let (t_start, entry_normal) = if self.bounds.contains(ray.at(t_min)) {
            (t_min, None)
        } else {
            let entry = self.bounds.intersect(ray, t_min, t_max)?;
            (entry.t, Some(entry.normal))
        };
        let origin = (ray.origin - self.bounds.min).to_array();
        let mut walk = Dda::new(origin, ray.direction.to_array(), t_start, self.size);

        // 2. The first cell. Coming from outside, it may already be the answer.
        //    Starting inside a detail cell, the OTHER mini-blocks in it still count
        //    (e.g. a shadow ray leaving a flower petal must not ignore the stem).
        match self.cell(walk.cell) {
            Cell::Detail(detail) => {
                if let Some(found) = self.walk_detail(detail, walk.cell, ray, t_start, entry_normal, t_min, t_max, &is_boundary) {
                    return Some(found);
                }
            }
            whole => {
                if let Some(normal) = entry_normal {
                    let content = if let Cell::Block(m) = whole { Some(m) } else { None };
                    if is_boundary(content) {
                        return Some(Boundary { t: t_start, point: ray.at(t_start), normal, entered: content });
                    }
                }
            }
        }

        // 3. Step from cell to cell until we find a boundary or leave the grid.
        loop {
            let (t, axis) = walk.step()?; // None = walked out of the grid
            if t >= t_max {
                return None;
            }
            // We crossed the wall on `axis`, so the wall's normal points
            // back against our step: toward the side the ray came from.
            let mut normal = [0.0; 3];
            normal[axis] = -walk.step[axis] as f32;
            let normal = Vec3::from_array(normal);

            match self.cell(walk.cell) {
                Cell::Detail(detail) => {
                    if let Some(found) = self.walk_detail(detail, walk.cell, ray, t, Some(normal), t_min, t_max, &is_boundary) {
                        return Some(found);
                    }
                }
                whole => {
                    let content = if let Cell::Block(m) = whole { Some(m) } else { None };
                    if t >= t_min && is_boundary(content) {
                        return Some(Boundary { t, point: ray.at(t), normal, entered: content });
                    }
                }
            }
        }
    }

    /// The same walk, inside one detail cell's 4×4×4 mini-grid. We scale
    /// positions by 4 so each mini-block is 1 unit wide (then the DDA code is
    /// identical), and divide the resulting t by 4 to get back to world units.
    /// Returns None if the ray crosses the whole cell without a boundary.
    #[allow(clippy::too_many_arguments)]
    fn walk_detail(
        &self,
        detail: u32,
        cell: [i32; 3],
        ray: &Ray,
        t_enter: f32,
        entry_normal: Option<Vec3>,
        t_min: f32,
        t_max: f32,
        is_boundary: &impl Fn(Option<u8>) -> bool,
    ) -> Option<Boundary> {
        let minis = &self.details[detail as usize];
        let mini_at = |m: [i32; 3]| minis[(m[0] + MINI * m[1] + MINI * MINI * m[2]) as usize];
        let scale = MINI as f32;
        let cell_min = self.bounds.min + Vec3::new(cell[0] as f32, cell[1] as f32, cell[2] as f32);
        let local_origin = ((ray.origin - cell_min) * scale).to_array();
        let mut walk = Dda::new(local_origin, ray.direction.to_array(), t_enter * scale, [MINI; 3]);

        // Entered through the cell's wall: the first mini-block may be the answer.
        if let Some(normal) = entry_normal {
            let content = mini_at(walk.cell);
            if t_enter >= t_min && is_boundary(content) {
                return Some(Boundary { t: t_enter, point: ray.at(t_enter), normal, entered: content });
            }
        }
        loop {
            let (scaled_t, axis) = walk.step()?; // None = left this cell: back to the big walk
            let t = scaled_t / scale;
            if t >= t_max {
                return None;
            }
            let content = mini_at(walk.cell);
            if t >= t_min && is_boundary(content) {
                let mut normal = [0.0; 3];
                normal[axis] = -walk.step[axis] as f32;
                return Some(Boundary { t, point: ray.at(t), normal: Vec3::from_array(normal), entered: content });
            }
        }
    }
}

/// The grid-walking state ("DDA", Amanatides & Woo 1987), for a grid whose
/// cells are 1 unit wide, starting at 0.
///
/// Idea: along each axis, the ray crosses a cell wall every `t_delta` units
/// of t. We keep, per axis, the t of the NEXT wall (`t_next`). Each step we
/// cross whichever wall comes first and move one cell that way.
struct Dda {
    cell: [i32; 3],
    step: [i32; 3],
    t_next: [f32; 3],
    t_delta: [f32; 3],
    size: [i32; 3],
}

impl Dda {
    /// `origin` and the cell layout are in grid units; the walk starts at
    /// the point origin + t_start × direction.
    fn new(origin: [f32; 3], direction: [f32; 3], t_start: f32, size: [i32; 3]) -> Dda {
        let mut walk = Dda { cell: [0; 3], step: [0; 3], t_next: [f32::INFINITY; 3], t_delta: [f32::INFINITY; 3], size };
        for axis in 0..3 {
            let start = origin[axis] + direction[axis] * t_start;
            // clamp: a start exactly on the far wall would otherwise floor to one past the end
            walk.cell[axis] = (start.floor() as i32).clamp(0, size[axis] - 1);
            if direction[axis] > 0.0 {
                walk.step[axis] = 1;
                walk.t_delta[axis] = 1.0 / direction[axis];
                walk.t_next[axis] = (walk.cell[axis] as f32 + 1.0 - origin[axis]) / direction[axis];
            } else if direction[axis] < 0.0 {
                walk.step[axis] = -1;
                walk.t_delta[axis] = -1.0 / direction[axis];
                walk.t_next[axis] = (walk.cell[axis] as f32 - origin[axis]) / direction[axis];
            }
            // direction 0: never crosses a wall on this axis, t_next stays infinite
        }
        walk
    }

    /// Crosses the nearest wall. Returns (t of that wall, axis crossed), or
    /// None if that step leaves the grid.
    fn step(&mut self) -> Option<(f32, usize)> {
        let axis = if self.t_next[0] < self.t_next[1] {
            if self.t_next[0] < self.t_next[2] { 0 } else { 2 }
        } else if self.t_next[1] < self.t_next[2] {
            1
        } else {
            2
        };
        let t = self.t_next[axis];
        self.cell[axis] += self.step[axis];
        if self.cell[axis] < 0 || self.cell[axis] >= self.size[axis] {
            return None;
        }
        self.t_next[axis] += self.t_delta[axis];
        Some((t, axis))
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

    #[test]
    fn a_mini_block_is_hit_at_its_own_quarter_size_face() {
        let mut grid = BlockGrid::new([-10, -10, -10], [20, 20, 20]);
        // Mini-block (1, 1, 2) spans x 0.25..0.5, y 0.25..0.5, z 0.5..0.75.
        grid.set_mini([1, 1, 2], Some(5));
        let ray = Ray::new(Vec3::new(0.375, 0.375, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = grid.trace(&ray, 0.0, f32::INFINITY).expect("should hit");
        assert!((hit.t - 4.25).abs() < 1e-4); // front face at z = 0.75
        assert_eq!(hit.material, 5);
        // A ray through the same cell that misses the mini-block goes on through.
        let miss = Ray::new(Vec3::new(0.875, 0.875, 5.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(grid.trace(&miss, 0.0, f32::INFINITY).is_none());
    }

    /// A tiny random number generator for test data.
    fn random_numbers(mut seed: u32) -> impl FnMut() -> f32 {
        move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 // 0..1
        }
    }

    /// The strongest check: for hundreds of random rays, the grid walk must
    /// find the same distance as the slow "test every box" method from Phase 3,
    /// with a mix of whole blocks and mini-blocks.
    #[test]
    fn grid_walk_matches_brute_force() {
        let mut random = random_numbers(12345);
        let mut grid = BlockGrid::new([-5, -5, -5], [10, 10, 10]);
        let mut boxes = Vec::new();
        for _ in 0..60 {
            let block = [0, 0, 0].map(|_: i32| (random() * 10.0) as i32 - 5);
            if random() < 0.5 {
                grid.set(block, Some(1));
                let min = Vec3::new(block[0] as f32, block[1] as f32, block[2] as f32);
                boxes.push(Cube::new(min, min + Vec3::new(1.0, 1.0, 1.0), 1));
            } else {
                // Only use cells that are still air, so whole and mini don't overlap.
                if !grid.is_empty(block) && grid.get(block).is_some() {
                    continue;
                }
                for _ in 0..6 {
                    let mini = [0, 1, 2].map(|axis| block[axis] * MINI + (random() * MINI as f32) as i32);
                    grid.set_mini(mini, Some(1));
                    let min = Vec3::new(mini[0] as f32, mini[1] as f32, mini[2] as f32) * 0.25;
                    boxes.push(Cube::new(min, min + Vec3::new(0.25, 0.25, 0.25), 1));
                }
            }
        }

        for _ in 0..800 {
            // Start outside the grid, aim at a random point inside it.
            let origin = Vec3::new(random() - 0.5, random() - 0.5, random() - 0.5).normalized() * 20.0;
            let target = Vec3::new(random() * 10.0 - 5.0, random() * 10.0 - 5.0, random() * 10.0 - 5.0);
            let ray = Ray::new(origin, target - origin);

            let brute_force = boxes
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
