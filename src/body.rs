/// The solid part of a container: a small bitmap in local space.
/// Local space has its origin at the centre, +x right, +y down (like the screen).
pub struct Shape {
    pub w: i32,
    pub h: i32,
    solid: Vec<bool>,
}

impl Shape {
    /// A cup with half-width `top_half` at the rim and `bottom_half` at the base.
    /// Use equal values for a tumbler, a small `bottom_half` for a cone.
    pub fn cup(top_half: f32, bottom_half: f32, height: i32, wall: f32) -> Shape {
        let w = (top_half.max(bottom_half) * 2.0).ceil() as i32 + 2;
        let h = height;
        let mut solid = vec![false; (w * h) as usize];
        for iy in 0..h {
            for ix in 0..w {
                let lx = ix as f32 + 0.5 - w as f32 / 2.0;
                let v = (iy as f32 + 0.5) / h as f32; // 0 at the rim, 1 at the base
                let half = top_half + (bottom_half - top_half) * v;
                let ax = lx.abs();
                let side = ax <= half && ax > half - wall;
                let base = iy as f32 + 0.5 > h as f32 - wall && ax <= half;
                solid[(iy * w + ix) as usize] = side || base;
            }
        }
        Shape { w, h, solid }
    }

    /// Distance from the centre to the farthest corner of the bitmap.
    pub fn radius(&self) -> f32 {
        (self.w as f32).hypot(self.h as f32) / 2.0
    }

    fn is_solid(&self, lx: f32, ly: f32) -> bool {
        let ix = (lx + self.w as f32 / 2.0).floor() as i32;
        let iy = (ly + self.h as f32 / 2.0).floor() as i32;
        ix >= 0
            && ix < self.w
            && iy >= 0
            && iy < self.h
            && self.solid[(iy * self.w + ix) as usize]
    }
}

pub struct Body {
    pub shape: Shape,
    pub pos: (f32, f32), // where the shape's centre is, in grid cells
    pub angle: f32,      // radians, clockwise on screen
    pub cells: Vec<(i32, i32)>, // the grid cells currently stamped by this body
    pub grab: Option<(f32, f32)>, // held at this point in local space
    pub fall_speed: f32,
}

/// Rotate a world point back into a body's local space.
fn local_point(pos: (f32, f32), angle: f32, p: (f32, f32)) -> (f32, f32) {
    let (dx, dy) = (p.0 - pos.0, p.1 - pos.1);
    let (c, s) = (angle.cos(), angle.sin());
    (dx * c + dy * s, -dx * s + dy * c)
}

impl Body {
    pub fn new(shape: Shape, pos: (f32, f32)) -> Body {
        Body {
            shape,
            pos,
            angle: 0.0,
            cells: Vec::new(),
            grab: None,
            fall_speed: 0.0,
        }
    }

    pub fn to_local(&self, p: (f32, f32)) -> (f32, f32) {
        local_point(self.pos, self.angle, p)
    }

    /// Is this point over the glass (including its hollow inside)?
    pub fn hit_test(&self, p: (f32, f32)) -> bool {
        let (lx, ly) = self.to_local(p);
        lx.abs() <= self.shape.w as f32 / 2.0 && ly.abs() <= self.shape.h as f32 / 2.0
    }

    /// The grid cells this body would cover at the given pose.
    pub fn footprint(
        &self,
        pos: (f32, f32),
        angle: f32,
        width: i32,
        height: i32,
    ) -> Vec<(i32, i32)> {
        let r = self.shape.radius() + 1.0;
        let (min_x, max_x) = ((pos.0 - r).floor() as i32, (pos.0 + r).ceil() as i32);
        let (min_y, max_y) = ((pos.1 - r).floor() as i32, (pos.1 + r).ceil() as i32);

        let mut cells = Vec::new();
        for y in min_y.max(0)..=max_y.min(height - 1) {
            for x in min_x.max(0)..=max_x.min(width - 1) {
                let centre = (x as f32 + 0.5, y as f32 + 0.5);
                let (lx, ly) = local_point(pos, angle, centre);
                if self.shape.is_solid(lx, ly) {
                    cells.push((x, y));
                }
            }
        }
        cells
    }
}