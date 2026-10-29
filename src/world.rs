use std::f32::consts::TAU;

use crate::gravity::{dir_vec, octant, GravitySource, UniformGravity, DIRS, STANDARD_GRAVITY};
use crate::material::Material;

// Tuning knobs. Everything is in cells and frames.
const DRAG: f32 = 0.97; // velocity kept each frame (terminal speed ~4.8)
const MAX_SPEED: f32 = 6.0; // hard cap, so nothing crosses the whole grid at once
const DEFLECT_KEEP: f32 = 0.6; // speed kept when sliding around an obstacle
const PUSH_THROUGH_KEEP: f32 = 0.8; // speed kept when plowing through something
const SPLASH: f32 = 0.3; // how much of that momentum kicks the displaced particle back

#[derive(Clone, Copy)]
pub struct Cell {
    pub material: Material,
    vx: f32,
    vy: f32,
    rx: f32, // fractional movement not yet spent
    ry: f32,
    updated: bool,
}

impl Cell {
    const fn new(material: Material) -> Cell {
        Cell {
            material,
            vx: 0.0,
            vy: 0.0,
            rx: 0.0,
            ry: 0.0,
            updated: false,
        }
    }

    const EMPTY: Cell = Cell::new(Material::Empty);

    fn speed(&self) -> f32 {
        self.vx.hypot(self.vy)
    }
}

pub struct World {
    width: i32,
    height: i32,
    cells: Vec<Cell>,
    field: Vec<(f32, f32)>, // gravity acceleration at each cell
    sources: Vec<Box<dyn GravitySource>>,
    scan_focus: (i32, i32),
    frame: u64,
    rng_state: u32,
}

impl World {
    pub fn new(width: i32, height: i32) -> Self {
        let n = (width * height) as usize;
        let mut world = World {
            width,
            height,
            cells: vec![Cell::EMPTY; n],
            field: vec![(0.0, 0.0); n],
            sources: Vec::new(),
            scan_focus: (width / 2, height + 1000),
            frame: 0,
            rng_state: 0x1234_5678,
        };
        world.add_source(Box::new(UniformGravity {
            gx: 0.0,
            gy: STANDARD_GRAVITY,
        }));
        world
    }

    // ---------- helpers ----------

    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height
    }

    fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    /// Out-of-bounds counts as Wall, so nothing leaves the grid.
    pub fn get(&self, x: i32, y: i32) -> Material {
        if self.in_bounds(x, y) {
            self.cells[self.idx(x, y)].material
        } else {
            Material::Wall
        }
    }

    /// Placing a material always gives a fresh, motionless cell.
    pub fn set(&mut self, x: i32, y: i32, material: Material) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.cells[i] = Cell::new(material);
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        x
    }

    fn rand_bool(&mut self) -> bool {
        (self.next_u32() >> 16) & 1 == 1
    }

    /// Uniform in [0, 1).
    fn rand_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    // ---------- player interaction ----------

    pub fn paint(&mut self, cx: i32, cy: i32, radius: i32, material: Material) {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius {
                    let (x, y) = (cx + dx, cy + dy);
                    if material == Material::Empty || self.get(x, y) == Material::Empty {
                        self.set(x, y, material);
                    }
                }
            }
        }
    }

    // ---------- gravity management ----------

    pub fn add_source(&mut self, source: Box<dyn GravitySource>) {
        self.sources.push(source);
        self.rebuild_gravity();
    }

    pub fn clear_sources(&mut self) {
        self.sources.clear();
        self.rebuild_gravity();
    }

    pub fn focus_points(&self) -> Vec<(f32, f32)> {
        self.sources.iter().filter_map(|s| s.focus()).collect()
    }

    fn rebuild_gravity(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let (mut gx, mut gy) = (0.0_f32, 0.0_f32);
                for source in &self.sources {
                    let (px, py) = source.pull_at(x as f32, y as f32);
                    gx += px;
                    gy += py;
                }
                let i = self.idx(x, y);
                self.field[i] = (gx, gy);
            }
        }

        // Scan outward from the newest well; with none, pretend there is a
        // focus far away in the direction gravity points.
        let (cx, cy) = (self.width / 2, self.height / 2);
        let (gx, gy) = self.field[self.idx(cx, cy)];
        let len = gx.hypot(gy).max(1e-6);
        let far = (gx / len * 1000.0, gy / len * 1000.0);
        self.scan_focus = match self.sources.iter().rev().find_map(|s| s.focus()) {
            Some((fx, fy)) => (fx as i32, fy as i32),
            None => (cx + far.0 as i32, cy + far.1 as i32),
        };
    }

    // ---------- the simulation ----------

    pub fn step(&mut self) {
        for cell in self.cells.iter_mut() {
            cell.updated = false;
        }

        let flip = self.frame % 2 == 0;
        let rows = scan_order(self.height, self.scan_focus.1, flip);
        let cols = scan_order(self.width, self.scan_focus.0, flip);

        for &y in &rows {
            for &x in &cols {
                self.update_cell(x, y);
            }
        }

        self.frame += 1;
    }

    fn update_cell(&mut self, x: i32, y: i32) {
        let i = self.idx(x, y);
        let mut cell = self.cells[i];
        let material = cell.material;
        if cell.updated || !material.is_movable() {
            return;
        }

        // This material's acceleration here.
        let scale = material.gravity_scale();
        let (gx, gy) = self.field[i];
        let (ax, ay) = (gx * scale, gy * scale);

        // 1. Accelerate, apply drag, cap the speed.
        cell.vx = (cell.vx + ax) * DRAG;
        cell.vy = (cell.vy + ay) * DRAG;
        let speed = cell.vx.hypot(cell.vy);
        if speed > MAX_SPEED {
            cell.vx *= MAX_SPEED / speed;
            cell.vy *= MAX_SPEED / speed;
        }

        // 2. Bank the movement: spend whole cells, keep the fraction.
        cell.rx += cell.vx;
        cell.ry += cell.vy;
        let (whole_x, whole_y) = (cell.rx.trunc(), cell.ry.trunc());
        cell.rx -= whole_x;
        cell.ry -= whole_y;
        self.cells[i] = cell; // write back BEFORE moving anything

        let down = octant(ax, ay);
        let (mx, my) = (whole_x as i32, whole_y as i32);
        if mx == 0 && my == 0 {
            if ax.hypot(ay) > 0.01 {
                self.settle(x, y, down);
            }
        } else {
            self.travel(x, y, mx, my, down);
        }
    }

    /// 3. Walk the path one cell at a time until blocked.
    fn travel(&mut self, x: i32, y: i32, mx: i32, my: i32, down: u8) {
        let steps = mx.abs().max(my.abs());
        let (mut cx, mut cy) = (x, y);
        for s in 1..=steps {
            let t = s as f32 / steps as f32;
            let tx = x + (mx as f32 * t).round() as i32;
            let ty = y + (my as f32 * t).round() as i32;
            let (dx, dy) = (tx - cx, ty - cy);
            if self.try_step(cx, cy, dx, dy) {
                cx = tx;
                cy = ty;
            } else {
                self.collide(cx, cy, dx, dy, down);
                return;
            }
        }
    }

    /// 4. Blocked while moving: tell the material, then slide or stop.
    fn collide(&mut self, x: i32, y: i32, dx: i32, dy: i32, down: u8) {
        let i = self.idx(x, y);
        let speed = self.cells[i].speed();

        if self.on_impact(x, y, speed) {
            return; // the particle was transformed
        }

        let toward = octant(dx as f32, dy as f32);
        if self.get_around(x, y, toward, down, speed) {
            return;
        }

        // Nowhere to go: stop dead.
        let cell = &mut self.cells[i];
        cell.vx = 0.0;
        cell.vy = 0.0;
        cell.rx = 0.0;
        cell.ry = 0.0;
    }

    /// Not moving this frame. If something is underneath, spread out
    /// (piles slump, liquids level). If not, we're just still speeding up.
    fn settle(&mut self, x: i32, y: i32, down: u8) {
        let (bx, by) = dir_vec(down, 0);
        let mover = self.get(x, y);
        if self.get(x + bx, y + by).density() < mover.density() {
            return;
        }
        self.get_around(x, y, down, down, 0.0);
    }

    /// Try the two diagonals next to the blocked direction, then (for
    /// liquids and gases) slide sideways relative to gravity.
    fn get_around(&mut self, x: i32, y: i32, toward: u8, down: u8, speed: f32) -> bool {
        let side = if self.rand_bool() { 1 } else { -1 };

        for turn in [side, -side] {
            let (dx, dy) = dir_vec(toward, turn);
            if self.try_step(x, y, dx, dy) {
                self.set_velocity(x + dx, y + dy, dx as f32, dy as f32, speed * DEFLECT_KEEP);
                return true;
            }
        }

        let dispersion = self.get(x, y).dispersion();
        if dispersion > 0 {
            for turn in [2 * side, -2 * side] {
                let (dx, dy) = dir_vec(down, turn);
                if let Some((nx, ny)) = self.slide(x, y, dx, dy, dispersion) {
                    self.set_velocity(nx, ny, 0.0, 0.0, 0.0);
                    return true;
                }
            }
        }
        false
    }

    // ---------- impacts ----------

    /// Called when a particle hits something at `speed`. Returns true if
    /// the particle was destroyed or transformed.
    fn on_impact(&mut self, x: i32, y: i32, speed: f32) -> bool {
        if let Some((threshold, becomes)) = self.get(x, y).shatter_threshold() {
            if speed >= threshold {
                self.shatter(x, y, becomes, speed);
                return true;
            }
        }
        false
    }

    fn shatter(&mut self, x: i32, y: i32, becomes: Material, speed: f32) {
        self.spawn(x, y, becomes, speed * 0.5);
        for (dx, dy) in DIRS {
            let (nx, ny) = (x + dx, y + dy);
            if self.get(nx, ny) == Material::Empty && self.rand_f32() < 0.4 {
                self.spawn(nx, ny, becomes, speed * 0.5);
            }
        }
    }

    /// Place a particle flying off in a random direction.
    fn spawn(&mut self, x: i32, y: i32, material: Material, speed: f32) {
        let angle = self.rand_f32() * TAU;
        let mut cell = Cell::new(material);
        cell.vx = angle.cos() * speed;
        cell.vy = angle.sin() * speed;
        cell.updated = true; // don't also move it this frame
        let i = self.idx(x, y);
        self.cells[i] = cell;
    }

    // ---------- low-level moves ----------

    fn set_velocity(&mut self, x: i32, y: i32, dir_x: f32, dir_y: f32, speed: f32) {
        let len = dir_x.hypot(dir_y).max(0.0001);
        let i = self.idx(x, y);
        self.cells[i].vx = dir_x / len * speed;
        self.cells[i].vy = dir_y / len * speed;
    }

    /// Move one cell by (dx, dy) if the target is lighter. Plowing through
    /// another material slows the mover and kicks the displaced particle.
    fn try_step(&mut self, x: i32, y: i32, dx: i32, dy: i32) -> bool {
        let (nx, ny) = (x + dx, y + dy);
        let mover = self.get(x, y);
        let target = self.get(nx, ny);
        if target.density() >= mover.density() {
            return false;
        }

        self.swap_cells(x, y, nx, ny);

        if target != Material::Empty {
            let (old_i, new_i) = (self.idx(x, y), self.idx(nx, ny));
            let (vx, vy) = (self.cells[new_i].vx, self.cells[new_i].vy);
            self.cells[old_i].vx = -vx * SPLASH;
            self.cells[old_i].vy = -vy * SPLASH;
            self.cells[new_i].vx *= PUSH_THROUGH_KEEP;
            self.cells[new_i].vy *= PUSH_THROUGH_KEEP;
        }
        true
    }

    /// Slide up to `max` cells while the path is clear. Returns where we ended up.
    fn slide(&mut self, x: i32, y: i32, dx: i32, dy: i32, max: i32) -> Option<(i32, i32)> {
        let mover = self.get(x, y);
        let (mut tx, mut ty) = (x, y);
        for step in 1..=max {
            let (nx, ny) = (x + dx * step, y + dy * step);
            if self.get(nx, ny).density() < mover.density() {
                tx = nx;
                ty = ny;
            } else {
                break;
            }
        }
        if (tx, ty) != (x, y) {
            self.swap_cells(x, y, tx, ty);
            Some((tx, ty))
        } else {
            None
        }
    }

    fn swap_cells(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let a = self.idx(x1, y1);
        let b = self.idx(x2, y2);
        self.cells.swap(a, b);
        self.cells[a].updated = true;
        self.cells[b].updated = true;
    }

    // ---------- output ----------

    /// Fill an RGBA buffer. With `show_speed`, fast particles glow red.
    pub fn write_pixels(&self, buf: &mut [u8], show_speed: bool) {
        for (pixel, cell) in buf.chunks_exact_mut(4).zip(self.cells.iter()) {
            let mut color = cell.material.color();
            if show_speed {
                let boost = (cell.speed() * 40.0).min(255.0) as u8;
                color[0] = color[0].saturating_add(boost);
                color[2] = color[2].saturating_sub(boost);
            }
            pixel.copy_from_slice(&color);
        }
    }
}

/// Indices 0..len ordered outward from `focus` (nearest first).
fn scan_order(len: i32, focus: i32, flip: bool) -> Vec<i32> {
    let mut order: Vec<i32> = (0..len).collect();
    order.sort_by_key(|&i| ((i - focus).abs(), if flip { i } else { -i }));
    order
}