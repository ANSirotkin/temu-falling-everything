use std::f32::consts::FRAC_PI_4;

/// Normal downward gravity, in cells per frame squared.
pub const STANDARD_GRAVITY: f32 = 0.15;

/// The 8 neighbour offsets, ordered by angle (y points down on screen).
pub const DIRS: [(i32, i32); 8] = [
    (1, 0), (1, 1), (0, 1), (-1, 1),
    (-1, 0), (-1, -1), (0, -1), (1, -1),
];

/// Offset an octant by `turn` steps of 45 degrees (wraps around).
pub fn dir_vec(octant: u8, turn: i32) -> (i32, i32) {
    DIRS[(octant as i32 + turn).rem_euclid(8) as usize]
}

/// Which of the 8 directions is closest to this vector?
pub fn octant(gx: f32, gy: f32) -> u8 {
    let angle = gy.atan2(gx); // -PI..PI
    let steps = (angle / FRAC_PI_4).round() as i32;
    steps.rem_euclid(8) as u8
}

/// Anything that can pull on particles.
pub trait GravitySource {
    /// The acceleration at grid position (x, y), in cells per frame squared.
    fn pull_at(&self, x: f32, y: f32) -> (f32, f32);

    /// Where this source "is", if it has a location (used for drawing
    /// and for choosing the scan order). Most sources don't.
    fn focus(&self) -> Option<(f32, f32)> {
        None
    }
}

pub struct UniformGravity {
    pub gx: f32,
    pub gy: f32,
}

impl GravitySource for UniformGravity {
    fn pull_at(&self, _x: f32, _y: f32) -> (f32, f32) {
        (self.gx, self.gy)
    }
}

/// Pulls toward a point. `strength` = acceleration * distance squared, so
/// strength 40 matches STANDARD_GRAVITY at a radius of about 16 cells.
pub struct GravityWell {
    pub x: f32,
    pub y: f32,
    pub strength: f32,
}

impl GravitySource for GravityWell {
    fn pull_at(&self, x: f32, y: f32) -> (f32, f32) {
        let dx = self.x - x;
        let dy = self.y - y;
        let dist_sq = dx * dx + dy * dy + 4.0; // +4 softens the centre
        let dist = dist_sq.sqrt();
        let magnitude = self.strength / dist_sq; // inverse-square falloff
        (dx / dist * magnitude, dy / dist * magnitude)
    }

    fn focus(&self) -> Option<(f32, f32)> {
        Some((self.x, self.y))
    }
}